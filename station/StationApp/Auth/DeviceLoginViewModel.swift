import Foundation
#if os(macOS)
import AppKit
#endif

@MainActor
public final class DeviceLoginViewModel: ObservableObject {
    @Published public private(set) var phase: DeviceLoginPhase = .idle
    @Published public private(set) var userCode: String?
    @Published public private(set) var verificationURIComplete: String?
    @Published public private(set) var signedInEmail: String?
    @Published public private(set) var errorMessage: String?

    private let client: any DeviceLoginClient
    private let openURL: (URL) -> Void

    public init(
        client: any DeviceLoginClient,
        openURL: @escaping (URL) -> Void = { url in
            #if os(macOS)
            NSWorkspace.shared.open(url)
            #endif
        }
    ) {
        self.client = client
        self.openURL = openURL
    }

    public func refreshSession() async {
        errorMessage = nil
        do {
            if let session = try await client.currentSession() {
                applySignedIn(session)
            } else if phase != .awaitingBrowser && phase != .completing && phase != .starting {
                phase = .idle
                signedInEmail = nil
            }
        } catch {
            if phase == .signedIn {
                phase = .error
                errorMessage = error.localizedDescription
            }
        }
    }

    public func beginLogin() async {
        phase = .starting
        errorMessage = nil
        userCode = nil
        verificationURIComplete = nil
        signedInEmail = nil

        do {
            let challenge = try await client.begin()
            userCode = challenge.userCode
            verificationURIComplete = challenge.verificationURIComplete
            phase = .awaitingBrowser
            let browserTarget = challenge.browserURL ?? challenge.verificationURIComplete
            if let url = URL(string: browserTarget) {
                openURL(url)
            }
            // Poll WorkOS until the browser flow completes — no extra "I've confirmed" click.
            Task { await completeLogin() }
        } catch {
            phase = .error
            userCode = nil
            verificationURIComplete = nil
            errorMessage = error.localizedDescription
        }
    }

    public func completeLogin() async {
        guard phase == .awaitingBrowser || phase == .error else { return }
        phase = .completing
        errorMessage = nil

        do {
            let session = try await client.complete()
            applySignedIn(session)
        } catch {
            phase = .error
            errorMessage = error.localizedDescription
        }
    }

    public func signOut() async {
        do {
            try await client.signOut()
        } catch {
            errorMessage = error.localizedDescription
        }
        phase = .idle
        userCode = nil
        verificationURIComplete = nil
        signedInEmail = nil
    }

    private func applySignedIn(_ session: StationSessionIdentity) {
        phase = .signedIn
        signedInEmail = session.email
        userCode = nil
        verificationURIComplete = nil
        errorMessage = nil
    }
}
