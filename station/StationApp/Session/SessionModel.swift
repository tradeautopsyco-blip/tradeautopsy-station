import Darwin
import Foundation
import Notch

/// Station-owned live session poller — positions, broker sync honesty, and kill-switch.
/// Replaces the Notch package's `NotchViewModel` for Station surfaces (pulse strip + Today).
/// Intentionally minimal: no floating panel, tabs, bar phase, journal, or workflows.
@MainActor
public final class SessionModel: ObservableObject {
    @Published public var positions: [DeskPosition] = []
    @Published public var killSwitchActive: Bool = false
    @Published public var killSwitchCountdownSecs: Int?
    @Published public var killSwitchExpiresAtMs: Int64?
    @Published public var killSwitchDismissBusy: Bool = false
    @Published public var brokerSessionActive: Bool = false
    @Published public var activeBrokerSlug: String?
    @Published public var deskQuoteCurrency: String?
    @Published public var deskCalcProfileId: String?
    @Published public private(set) var brokerSyncClass: String = "not_connected"

    /// Seconds since the last kill-switch state update (SSE or daemon poll). `Int.max` if never received.
    public var killSwitchStateAgeSecs: Int {
        guard let killSwitchStateReceivedAt else { return Int.max }
        return max(0, Int(Date().timeIntervalSince(killSwitchStateReceivedAt)))
    }

    public var totalUnrealizedPnL: Double {
        positions.compactMap(\.unrealizedPnL).reduce(0, +)
    }

    /// True while broker sync is running or has fresh/stale data — drives Today session mirror polling.
    public var isBrokerSyncActiveForTodayMirror: Bool {
        switch brokerSyncClass {
        case "syncing", "synced", "stale":
            return true
        default:
            return false
        }
    }

    // MARK: - Config (Station-owned; replaces NotchLauncher.configure indirection)

    public private(set) var daemonSecret: String = ""
    public private(set) var daemonPort: UInt16 = 9137
    public private(set) var webBaseURL: String = "https://localhost:3000"

    public func configure(secret: String, port: UInt16, webBase: String) {
        daemonSecret = secret
        daemonPort = port
        let base = webBase.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        if base.hasPrefix("http://") || base.hasPrefix("https://") {
            webBaseURL = base
        } else {
            webBaseURL = "https://\(base)"
        }
    }

    // MARK: - Private runtime state

    private var killSwitchStateReceivedAt: Date?
    private var killSwitchLevel: String?
    private var killSwitchRequiresAck: Bool = false
    private var openOrders: Int = 0

    private var pollPositions: Timer?
    private var connectionTick: Timer?
    private var killSwitchCountdownTimer: Timer?
    private var daemonEventsTask: Task<Void, Never>?
    private var connectionFSM = DaemonConnectionFSM()
    private var daemonConnectionState: DaemonConnectionState = .idle
    private var daemonProtocolError: DaemonProtocolErrorClass?

    private var pinnedAgentBootId: String?
    private var pinnedAgentSsePubKeyB64: String?

    private let urlSession: URLSession

    public init(urlSession: URLSession = .shared) {
        self.urlSession = urlSession
    }

    /// Inject a cached kill-switch timestamp (unit tests only).
    public func setKillSwitchStateReceivedAt(_ date: Date?) {
        killSwitchStateReceivedAt = date
    }

    // MARK: - Polling lifecycle

    public func startPolling() {
        stopPolling()
        startDaemonEventsStream()
        connectionTick = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                self.daemonConnectionState = self.connectionFSM.onTick()
            }
        }
        // Station serves equities + crypto desks — poll positions always (no NSE IST gate).
        pollPositions = Timer.scheduledTimer(withTimeInterval: 10, repeats: true) { [weak self] _ in
            Task { @MainActor in
                await self?.fetchPositions()
            }
        }
        Task {
            await fetchPositions()
        }
    }

    public func stopPolling() {
        daemonEventsTask?.cancel()
        daemonEventsTask = nil
        pollPositions?.invalidate()
        pollPositions = nil
        connectionTick?.invalidate()
        connectionTick = nil
        stopKillSwitchCountdownTimer()
        daemonConnectionState = .idle
        daemonProtocolError = nil
        connectionFSM = DaemonConnectionFSM()
        pinnedAgentBootId = nil
        pinnedAgentSsePubKeyB64 = nil
    }

    // MARK: - SSE

    private func startDaemonEventsStream() {
        daemonEventsTask?.cancel()
        daemonEventsTask = Task { @MainActor [weak self] in
            guard let self else { return }
            while !Task.isCancelled {
                self.connectionFSM.onConnectStart()
                self.daemonConnectionState = self.connectionFSM.state
                await self.refreshPinnedAgentTrust()
                guard self.pinnedAgentSsePubKeyB64 != nil else {
                    self.connectionFSM.onTransportFailure()
                    self.daemonConnectionState = self.connectionFSM.state
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                    continue
                }
                guard let url = URL(string: self.baseURL() + "/api/daemon/events/stream") else {
                    self.connectionFSM.onTransportFailure()
                    self.daemonConnectionState = self.connectionFSM.state
                    return
                }
                var req = self.authorizedRequest(url: url)
                req.setValue("text/event-stream", forHTTPHeaderField: "Accept")
                do {
                    let (bytes, resp) = try await self.urlSession.bytes(for: req)
                    let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
                    if statusCode != 200 {
                        var body = Data()
                        for try await line in bytes.lines {
                            body.append(contentsOf: line.utf8)
                            if body.count >= 2048 { break }
                        }
                        self.handleDaemonErrorResponse(
                            data: body.isEmpty ? nil : body,
                            statusCode: statusCode
                        )
                        self.connectionFSM.onTransportFailure()
                        self.daemonConnectionState = self.connectionFSM.state
                        try? await Task.sleep(nanoseconds: 2_000_000_000)
                        continue
                    }
                    self.daemonProtocolError = nil
                    self.connectionFSM.onStreamOpened()
                    self.daemonConnectionState = self.connectionFSM.state
                    for try await line in bytes.lines {
                        if Task.isCancelled { return }
                        guard line.hasPrefix("data:") else { continue }
                        let raw = String(line.dropFirst(5)).trimmingCharacters(in: .whitespaces)
                        if self.consumeDaemonEvent(raw) {
                            self.daemonConnectionState = self.connectionFSM.state
                        }
                    }
                    self.connectionFSM.onTransportFailure()
                    self.daemonConnectionState = self.connectionFSM.state
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                } catch {
                    self.connectionFSM.onTransportFailure()
                    self.daemonConnectionState = self.connectionFSM.state
                    try? await Task.sleep(nanoseconds: 2_000_000_000)
                }
            }
        }
    }

    private func consumeDaemonEvent(_ jsonString: String) -> Bool {
        guard
            let data = jsonString.data(using: .utf8),
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return false }
        guard let type = obj["type"] as? String else { return false }

        if let pk = pinnedAgentSsePubKeyB64 {
            guard
                let sig = obj["sig"] as? String,
                let eventId = obj["event_id"] as? String
            else {
                daemonProtocolError = .sigInvalid
                connectionFSM.onTransportFailure()
                daemonConnectionState = connectionFSM.state
                return false
            }
            guard SessionSseTrust.verifySseEd25519(
                rawEnvelopeJson: jsonString,
                eventId: eventId,
                typeStr: type,
                sigB64: sig,
                pubKeyB64: pk
            ) else {
                daemonProtocolError = .sigInvalid
                connectionFSM.onTransportFailure()
                daemonConnectionState = connectionFSM.state
                return false
            }
        }

        if type == "agent_health" {
            daemonProtocolError = nil
            connectionFSM.onHeartbeat()
            return true
        }

        guard let payload = obj["payload"] as? [String: Any] else { return false }
        return applyDaemonEventPayload(type: type, payload: payload)
    }

    /// Applies verified SSE payload-only updates (production after verify, and unit tests).
    @discardableResult
    public func applyDaemonEventPayload(type: String, payload: [String: Any]) -> Bool {
        switch type {
        case "kill_switch_state":
            let active = payload["active"] as? Bool ?? false
            killSwitchActive = active
            recordKillSwitchStateReceived()
            if active {
                if let lv = payload["level"] as? String, !lv.isEmpty {
                    killSwitchLevel = lv
                }
                killSwitchRequiresAck = payload["requires_ack"] as? Bool ?? false
                killSwitchExpiresAtMs = KillSwitchCountdown.parseMs(payload["expires_at_ms"])
                let wireCountdown: Int?
                if let c = payload["countdown_secs"] as? Int {
                    wireCountdown = c
                } else if let d = payload["countdown_secs"] as? Double {
                    wireCountdown = Int(d)
                } else {
                    wireCountdown = nil
                }
                killSwitchCountdownSecs = KillSwitchCountdown.remainingSecs(
                    expiresAtMs: killSwitchExpiresAtMs,
                    countdownSecs: wireCountdown,
                    nowMs: KillSwitchCountdown.nowMs()
                )
                startKillSwitchCountdownTimerIfNeeded()
            } else {
                killSwitchActive = false
                killSwitchCountdownSecs = nil
                killSwitchExpiresAtMs = nil
                killSwitchLevel = nil
                killSwitchRequiresAck = false
                stopKillSwitchCountdownTimer()
            }
            return true

        case "broker_sync_state":
            let c = (payload["class"] as? String)?.lowercased() ?? "not_connected"
            brokerSyncClass = c
            brokerSessionActive = (brokerSyncClass == "synced" || brokerSyncClass == "stale")
            applyDeskHonesty(from: payload)
            return true

        default:
            return false
        }
    }

    // MARK: - Trust pin

    private func refreshPinnedAgentTrust() async {
        guard let url = URL(string: baseURL() + "/api/daemon/health") else { return }
        let req = authorizedRequest(url: url)
        do {
            let (data, resp) = try await urlSession.data(for: req)
            guard (resp as? HTTPURLResponse)?.statusCode == 200 else { return }
            guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
            guard let boot = json["boot_id"] as? String else { return }
            guard let pk = json["sse_signing_pubkey_b64"] as? String else { return }

            if let prevBoot = pinnedAgentBootId, prevBoot == boot,
               let prevPk = pinnedAgentSsePubKeyB64, prevPk != pk {
                daemonProtocolError = .sigInvalid
                pinnedAgentSsePubKeyB64 = nil
                return
            }

            if pinnedAgentBootId != boot {
                pinnedAgentBootId = boot
            }
            pinnedAgentSsePubKeyB64 = pk
        } catch {}
    }

    // MARK: - Wire

    private func baseURL() -> String {
        "http://127.0.0.1:\(daemonPort)"
    }

    private func authorizedRequest(url: URL, method: String = "GET", body: Data? = nil) -> URLRequest {
        let payload = body ?? Data()
        let path = url.path.isEmpty ? "/" : url.path
        var r = StationWireClient.signedRequest(
            method: method,
            path: path,
            body: payload,
            daemonSecret: daemonSecret
        )
        r.url = url
        // Keep "notch" for agent wire compatibility until source allow-list is updated.
        r.setValue("notch", forHTTPHeaderField: "x-daemon-source")
        if body != nil {
            r.setValue("application/json", forHTTPHeaderField: "Content-Type")
            r.httpBody = body
        }
        #if os(macOS)
        if let bundle = Bundle.main.bundleIdentifier {
            r.setValue(bundle, forHTTPHeaderField: "x-tradeautopsy-caller-bundle-id")
        }
        r.setValue(String(getpid()), forHTTPHeaderField: "x-tradeautopsy-caller-pid")
        #endif
        return r
    }

    private func handleDaemonErrorResponse(data: Data?, statusCode: Int) {
        if let data, let envelope = try? JSONDecoder().decode(DaemonErrorEnvelope.self, from: data) {
            daemonProtocolError = envelope.errorClass
        } else {
            daemonProtocolError = statusCode == 429 ? .rateLimited : .unknown
        }
        daemonConnectionState = .disconnected
    }

    public func fetchPositions() async {
        guard let url = URL(string: baseURL() + "/api/daemon/positions") else { return }
        do {
            let (data, resp) = try await urlSession.data(for: authorizedRequest(url: url))
            let statusCode = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard statusCode == 200 else {
                handleDaemonErrorResponse(data: data, statusCode: statusCode)
                return
            }
            daemonProtocolError = nil
            let decoded = try SessionPositionsDecoder.decode(data)
            if let k = decoded.killSwitchActive {
                killSwitchActive = k
                recordKillSwitchStateReceived()
            }
            if let o = decoded.openOrders { openOrders = o }
            positions = decoded.positions
        } catch {}
    }

    // MARK: - Kill-switch dismiss

    public func dismissKillSwitchFromOverlay() async {
        guard KillSwitchOverlayPresentation.calmButtonEnabled(countdownSecs: killSwitchCountdownSecs) else {
            return
        }
        killSwitchDismissBusy = true
        defer { killSwitchDismissBusy = false }
        if killSwitchRequiresAck {
            await ackKillSwitchOverlay()
        }
        guard let url = URL(string: baseURL() + "/api/daemon/dismiss-kill-switch") else { return }
        let body: [String: Any] = ["reason": "user_calm"]
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else { return }
        do {
            let (_, resp) = try await urlSession.data(
                for: authorizedRequest(url: url, method: "POST", body: payload)
            )
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard (200...299).contains(code) else { return }
            killSwitchActive = false
            killSwitchCountdownSecs = nil
            killSwitchExpiresAtMs = nil
            killSwitchLevel = nil
            killSwitchRequiresAck = false
            recordKillSwitchStateReceived()
            stopKillSwitchCountdownTimer()
        } catch {}
    }

    private func ackKillSwitchOverlay() async {
        guard let url = URL(string: baseURL() + "/api/daemon/kill-switch/ack") else { return }
        var body: [String: Any] = [
            "countdown_remaining_secs": killSwitchCountdownSecs ?? 0,
        ]
        if let lv = killSwitchLevel, !lv.isEmpty {
            body["level"] = lv
        }
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else { return }
        let req = authorizedRequest(url: url, method: "POST", body: payload)
        _ = try? await urlSession.data(for: req)
    }

    // MARK: - Desk honesty (from SSE broker_sync_state)

    func applyDeskHonesty(from payload: [String: Any]) {
        let slug = (payload["brokerSlug"] as? String)
            ?? (payload["active_broker_slug"] as? String)
            ?? (payload["broker_slug"] as? String)
        if let slug, !slug.isEmpty {
            activeBrokerSlug = slug
        }
        if let ccy = payload["quoteCurrency"] as? String, !ccy.isEmpty {
            deskQuoteCurrency = ccy.uppercased()
        } else if let ccy = payload["quote_currency"] as? String, !ccy.isEmpty {
            deskQuoteCurrency = ccy.uppercased()
        } else if let mapped = DeskMoneyFormatting.quoteCurrency(forBrokerSlug: activeBrokerSlug) {
            deskQuoteCurrency = mapped
        }
        if let calc = payload["calcProfileId"] as? String, !calc.isEmpty {
            deskCalcProfileId = calc
        } else if let calc = payload["calc_profile_id"] as? String, !calc.isEmpty {
            deskCalcProfileId = calc
        } else if let mapped = DeskMoneyFormatting.calcProfileId(forBrokerSlug: activeBrokerSlug) {
            deskCalcProfileId = mapped
        }
        if brokerSyncClass == "not_connected" || brokerSyncClass == "disconnected" {
            activeBrokerSlug = nil
        }
    }

    // MARK: - Countdown timer

    private func startKillSwitchCountdownTimerIfNeeded() {
        guard killSwitchCountdownSecs != nil || killSwitchExpiresAtMs != nil else { return }
        stopKillSwitchCountdownTimer()
        let t = Timer(timeInterval: 1, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                let remaining = KillSwitchCountdown.remainingSecs(
                    expiresAtMs: self.killSwitchExpiresAtMs,
                    countdownSecs: self.killSwitchCountdownSecs.map { max(0, $0 - 1) },
                    nowMs: KillSwitchCountdown.nowMs()
                )
                self.killSwitchCountdownSecs = remaining
                if remaining == nil || remaining == 0 {
                    self.stopKillSwitchCountdownTimer()
                }
            }
        }
        RunLoop.main.add(t, forMode: .common)
        killSwitchCountdownTimer = t
    }

    private func stopKillSwitchCountdownTimer() {
        killSwitchCountdownTimer?.invalidate()
        killSwitchCountdownTimer = nil
    }

    private func recordKillSwitchStateReceived() {
        killSwitchStateReceivedAt = Date()
    }
}
