import AppKit
import Foundation
import SwiftUI
import WebKit

/// Runs broker OAuth in an in-app web view so loopback TLS (`127.0.0.1:9140`) never surfaces
/// Chrome's "connection is not private" interstitial.
public typealias BrokerOAuthWebLoginRunner = @Sendable (URL, String) async -> Bool

@MainActor
public final class BrokerOAuthWebLoginPresenter: ObservableObject {
    public static let shared = BrokerOAuthWebLoginPresenter()

    public struct Request: Identifiable {
        public let id = UUID()
        public let loginURL: URL
        public let callbackPrefix: String
        public let title: String

        public init(loginURL: URL, callbackPrefix: String, title: String) {
            self.loginURL = loginURL
            self.callbackPrefix = callbackPrefix
            self.title = title
        }
    }

    @Published public private(set) var activeRequest: Request?

    private var continuation: CheckedContinuation<Bool, Never>?

    public init() {}

    public func run(loginURL: URL, callbackPrefix: String, title: String) async -> Bool {
        await withCheckedContinuation { cont in
            continuation = cont
            activeRequest = Request(
                loginURL: loginURL,
                callbackPrefix: callbackPrefix,
                title: title
            )
        }
    }

    public func finish(success: Bool) {
        activeRequest = nil
        continuation?.resume(returning: success)
        continuation = nil
    }

    public static let sharedRun: BrokerOAuthWebLoginRunner = { loginURL, callbackPrefix in
        await BrokerOAuthWebLoginPresenter.shared.run(
            loginURL: loginURL,
            callbackPrefix: callbackPrefix,
            title: "Broker sign-in"
        )
    }
}

// MARK: - Loopback callback (agent TLS)

/// Kite redirects to `https://127.0.0.1:9140/...` — WKWebView rejects our self-signed cert (-1202).
/// Cancel that navigation and GET the callback URL from the app process instead.
enum LoopbackOAuthCallbackFetcher {
    private final class TLSDelegate: NSObject, URLSessionDelegate, @unchecked Sendable {
        func urlSession(
            _ session: URLSession,
            didReceive challenge: URLAuthenticationChallenge,
            completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
        ) {
            LoopbackOAuthTLS.acceptLoopbackServerTrust(challenge: challenge, completionHandler: completionHandler)
        }
    }

    private static let delegate = TLSDelegate()
    private static let session: URLSession = {
        URLSession(configuration: .ephemeral, delegate: delegate, delegateQueue: nil)
    }()

    static func fire(_ url: URL) async {
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        _ = try? await session.data(for: request)
    }
}

enum LoopbackOAuthTLS {
    static func acceptLoopbackServerTrust(
        challenge: URLAuthenticationChallenge,
        completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        guard challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust,
              challenge.protectionSpace.host == "127.0.0.1",
              let trust = challenge.protectionSpace.serverTrust
        else {
            completionHandler(.performDefaultHandling, nil)
            return
        }
        completionHandler(.useCredential, URLCredential(trust: trust))
    }
}

// MARK: - Sheet

public struct BrokerOAuthLoginSheet: View {
    let request: BrokerOAuthWebLoginPresenter.Request
    let isFinishing: Bool
    let onFinish: (Bool) -> Void

    @State private var didCompleteCallback = false

    public init(
        request: BrokerOAuthWebLoginPresenter.Request,
        isFinishing: Bool = false,
        onFinish: @escaping (Bool) -> Void
    ) {
        self.request = request
        self.isFinishing = isFinishing
        self.onFinish = onFinish
    }

    public var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(request.title)
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                Spacer()
                if !isFinishing {
                    Button("Cancel") {
                        onFinish(false)
                    }
                }
            }
            .padding()
            if isFinishing {
                VStack(spacing: 12) {
                    ProgressView()
                    Text("Saving broker session…")
                        .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .regular))
                        .foregroundStyle(StationDS.Text.muted)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                BrokerOAuthWebView(
                    loginURL: request.loginURL,
                    callbackPrefix: request.callbackPrefix,
                    onCallbackReached: {
                        guard !didCompleteCallback else { return }
                        didCompleteCallback = true
                        onFinish(true)
                    }
                )
            }
        }
        .frame(minWidth: 520, minHeight: 640)
    }
}

// MARK: - WebKit

private struct BrokerOAuthWebView: NSViewRepresentable {
    let loginURL: URL
    let callbackPrefix: String
    let onCallbackReached: () -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(callbackPrefix: callbackPrefix, onCallbackReached: onCallbackReached)
    }

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .default()
        let webView = WKWebView(frame: .zero, configuration: config)
        webView.navigationDelegate = context.coordinator
        webView.load(URLRequest(url: loginURL))
        return webView
    }

    func updateNSView(_ nsView: WKWebView, context: Context) {}

    final class Coordinator: NSObject, WKNavigationDelegate {
        private let callbackPrefix: String
        private let onCallbackReached: () -> Void
        private var sawCallback = false

        init(callbackPrefix: String, onCallbackReached: @escaping () -> Void) {
            self.callbackPrefix = callbackPrefix
            self.onCallbackReached = onCallbackReached
        }

        private func isLoopbackCallback(_ url: URL) -> Bool {
            url.absoluteString.hasPrefix(callbackPrefix)
        }

        private func completeCallback(at url: URL, webView: WKWebView) {
            guard !sawCallback else { return }
            sawCallback = true
            webView.stopLoading()
            Task { @MainActor in
                await LoopbackOAuthCallbackFetcher.fire(url)
                onCallbackReached()
            }
        }

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction,
            decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
        ) {
            if let url = navigationAction.request.url, isLoopbackCallback(url) {
                completeCallback(at: url, webView: webView)
                decisionHandler(.cancel)
                return
            }
            decisionHandler(.allow)
        }

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationResponse: WKNavigationResponse,
            decisionHandler: @escaping (WKNavigationResponsePolicy) -> Void
        ) {
            if let url = navigationResponse.response.url, isLoopbackCallback(url) {
                completeCallback(at: url, webView: webView)
                decisionHandler(.cancel)
                return
            }
            decisionHandler(.allow)
        }

        func webView(
            _ webView: WKWebView,
            didReceive challenge: URLAuthenticationChallenge,
            completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
        ) {
            LoopbackOAuthTLS.acceptLoopbackServerTrust(challenge: challenge, completionHandler: completionHandler)
        }

        func webView(
            _ webView: WKWebView,
            didFailProvisionalNavigation navigation: WKNavigation!,
            withError error: Error
        ) {
            let nsError = error as NSError
            guard nsError.domain == NSURLErrorDomain,
                  nsError.code == NSURLErrorServerCertificateUntrusted
            else { return }
            if let url = nsError.userInfo[NSURLErrorFailingURLErrorKey] as? URL,
               isLoopbackCallback(url) {
                completeCallback(at: url, webView: webView)
            } else if let urlString = nsError.userInfo[NSURLErrorFailingURLStringErrorKey] as? String,
                      let url = URL(string: urlString),
                      isLoopbackCallback(url) {
                completeCallback(at: url, webView: webView)
            }
        }
    }
}
