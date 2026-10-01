import Foundation

/// Spawns, health-checks, supervises, and tears down `tradeautopsy-agent` on loopback.
@MainActor
public final class AgentSupervisor: AgentSupervising {
    nonisolated public static let defaultPort: UInt16 = 9137
    nonisolated public static let healthPath = "/api/daemon/health"
    nonisolated public static let killSwitchStatePath = "/api/daemon/kill-switch/state"
    /// Agent HTTP bind is after ~10s of boot (DBs, adapters). 10s raced and killed a live child.
    nonisolated public static let launchTimeout: TimeInterval = 30
    nonisolated public static let runtimePollInterval: TimeInterval = 2

    public var onHealthChange: ((Bool) -> Void)?

    public private(set) var isHealthy = false
    public private(set) var currentWarning: AgentHealthWarning?
    public var ownsSpawnedAgent: Bool { spawnedProcess != nil || spawnedPID != nil }

    private var spawnedPID: Int32?
    /// Retained so we can terminate the child on quit (PID alone is easy to lose ownership of).
    private var spawnedProcess: Process?
    private var restartTracker = AgentRestartTracker()
    private var supervisionTask: Task<Void, Never>?
    private let port: UInt16
    private let session: URLSession
    private let daemonSecret: String
    private let healthLaunchTimeout: TimeInterval
    private let attachOnly: Bool

    public init(
        port: UInt16 = AgentSupervisor.defaultPort,
        session: URLSession = .shared,
        daemonSecret: String,
        launchTimeout: TimeInterval = AgentSupervisor.launchTimeout,
        attachOnly: Bool? = nil
    ) {
        self.port = port
        self.session = session
        self.daemonSecret = daemonSecret
        self.healthLaunchTimeout = launchTimeout
        self.attachOnly = attachOnly
            ?? (ProcessInfo.processInfo.environment["STATION_DEV_ATTACH"] == "1")
    }

    public func start() async {
        if attachOnly {
            await attachIfVerified()
            return
        }

        if await tryAttachToExistingListener() {
            return
        }

        await launchWithAutoRestart()
    }

    public func retry() async {
        restartTracker.resetOnSuccessfulAttach()
        currentWarning = nil
        await start()
    }

    /// Stop and respawn the bundled agent on loopback. Refuses when Wave 3 kill latch is active.
    public func restartAgent() async -> AgentManualRestartOutcome {
        if await isKillLatched() {
            return .blockedKillLatched
        }
        await shutdown()
        await retry()
        return .restarted
    }

    public func shutdown() async {
        supervisionTask?.cancel()
        supervisionTask = nil

        if await isKillLatched() {
            // Quit / Stop is not consent — leave the Enforcer running with teeth in force.
            spawnedProcess = nil
            spawnedPID = nil
            isHealthy = false
            return
        }

        if let process = spawnedProcess {
            await terminateOwnedProcess(process)
            spawnedProcess = nil
            spawnedPID = nil
        } else if let pid = spawnedPID {
            forceKillPID(pid)
            spawnedPID = nil
        }

        // Reap orphans from this app bundle (attach / unclean Stop). Skip in DEV attach mode.
        if !attachOnly {
            killBundledAgentOrphans()
        }
        isHealthy = false
    }

    /// Reads `GET /api/daemon/kill-switch/state` (Wave 3). Missing endpoint or errors → not latched.
    public func isKillLatched() async -> Bool {
        guard let request = signedKillSwitchStateRequest() else { return false }
        do {
            let (data, response) = try await session.data(for: request)
            guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                return false
            }
            guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                return false
            }
            return json["active"] as? Bool == true
        } catch {
            return false
        }
    }

    private func terminateOwnedProcess(_ process: Process) async {
        guard process.isRunning else { return }
        process.terminate() // SIGTERM
        let deadline = Date().addingTimeInterval(2)
        while process.isRunning, Date() < deadline {
            try? await Task.sleep(nanoseconds: 50_000_000)
        }
        if process.isRunning {
            forceKillPID(process.processIdentifier)
        }
    }

    private func forceKillPID(_ pid: Int32) {
        guard pid > 1 else { return }
        kill(pid, SIGTERM)
        usleep(200_000)
        if kill(pid, 0) == 0 {
            kill(pid, SIGKILL)
        }
    }

    /// Kill `tradeautopsy-agent` processes whose path is our bundled binary.
    private func killBundledAgentOrphans() {
        guard let expectedPath = bundledAgentPath() else { return }
        for pid in pidsMatchingExecutablePath(expectedPath) {
            if let owned = spawnedPID, pid == owned { continue }
            forceKillPID(pid)
        }
    }

    private func bundledAgentPath() -> String? {
        let url = Bundle.main.url(forAuxiliaryExecutable: "tradeautopsy-agent")
            ?? Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("tradeautopsy-agent")
        guard let url, FileManager.default.isExecutableFile(atPath: url.path) else { return nil }
        return url.path
    }

    private func pidsMatchingExecutablePath(_ expectedPath: String) -> [Int32] {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/pgrep")
        process.arguments = ["-f", expectedPath]
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = Pipe()
        do {
            try process.run()
            process.waitUntilExit()
        } catch {
            return []
        }
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        guard let text = String(data: data, encoding: .utf8) else { return [] }
        return text.split(whereSeparator: \.isNewline).compactMap { Int32($0.trimmingCharacters(in: .whitespaces)) }
    }

    private func launchWithAutoRestart() async {
        while !Task.isCancelled {
            if restartTracker.crashLoopExceeded(at: Date()) {
                setCrashLoopExceededWarning()
                return
            }

            spawnedPID = nil
            spawnedProcess = nil
            do {
                try await spawnAgent()
            } catch {
                await handleFailure(message: "Failed to spawn agent: \(error.localizedDescription)")
                continue
            }

            let becameHealthy = await probeHealthUntilReady()
            if becameHealthy {
                restartTracker.resetOnSuccessfulAttach()
                beginRuntimeSupervision()
                return
            }

            if spawnedProcess != nil || spawnedPID != nil {
                await shutdown()
            }
            await handleFailure(message: "Agent did not respond on loopback within \(Int(healthLaunchTimeout)) seconds.")
        }
    }

    private func attachIfVerified() async {
        if await probeHealthUntilReady() {
            markHealthyAttached()
        } else {
            setWarning(
                AgentHealthWarning(
                    reason: .launchTimeout,
                    message: "Dev attach mode: no TradeAutopsy agent responded on loopback.",
                    logPath: agentLogPath(),
                    canRetry: true
                )
            )
            beginRuntimeSupervision()
        }
    }

    private func tryAttachToExistingListener() async -> Bool {
        guard await portHasListener() else { return false }

        if await probeHealthUntilReady() {
            markHealthyAttached()
            return true
        }

        switch await classifyListener() {
        case .tradeAutopsyAgent:
            markHealthyAttached()
            return true
        case .noListener:
            return false
        case .agentWireRejected:
            setWarning(
                AgentHealthWarning(
                    reason: .launchTimeout,
                    message: "Port \(port) has a TradeAutopsy agent that rejected this session. Quit the orphaned tradeautopsy-agent process and click Retry.",
                    logPath: agentLogPath(),
                    canRetry: true
                )
            )
        case .foreignProcess:
            setWarning(
                AgentHealthWarning(
                    reason: .portCollisionNonAgent,
                    message: "Port \(port) is in use by a non-agent process. Check DAEMON_PORT and other listeners.",
                    logPath: nil,
                    canRetry: true
                )
            )
        }

        beginRuntimeSupervision()
        return true
    }

    private func markHealthyAttached() {
        isHealthy = true
        currentWarning = nil
        restartTracker.resetOnSuccessfulAttach()
        onHealthChange?(true)
        beginRuntimeSupervision()
    }

    private func spawnAgent() async throws {
        guard let binaryURL = Bundle.main.url(forAuxiliaryExecutable: "tradeautopsy-agent")
            ?? Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("tradeautopsy-agent")
        else {
            throw AgentSupervisorError.binaryNotFound
        }

        // Reap any leftover agent from a previous Station that didn't shut down cleanly.
        // Never reap a latched orphan — attach on the next start instead.
        let portListening = await portHasListener()
        let killLatched = await isKillLatched()
        if !(portListening && killLatched) {
            killBundledAgentOrphans()
        }

        let process = Process()
        process.executableURL = binaryURL
        var environment = StationDotEnv.merge(into: ProcessInfo.processInfo.environment)
        StationDotEnv.applyShippedDefaults(&environment)
        environment[AgentDaemonSecret.envKey] = daemonSecret
        process.environment = environment
        process.terminationHandler = { [weak self] proc in
            Task { @MainActor in
                guard let self else { return }
                if self.spawnedPID == proc.processIdentifier {
                    self.spawnedProcess = nil
                    self.spawnedPID = nil
                }
            }
        }

        try process.run()
        spawnedProcess = process
        spawnedPID = process.processIdentifier
    }

    @discardableResult
    private func probeHealthUntilReady() async -> Bool {
        let deadline = Date().addingTimeInterval(healthLaunchTimeout)
        var delay: UInt64 = 200_000_000

        while Date() < deadline {
            if await isTradeAutopsyAgentListening() {
                isHealthy = true
                currentWarning = nil
                onHealthChange?(true)
                return true
            }
            try? await Task.sleep(nanoseconds: delay)
            delay = min(delay * 2, 1_000_000_000)
        }

        return false
    }

    private func beginRuntimeSupervision() {
        supervisionTask?.cancel()
        supervisionTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: UInt64(Self.runtimePollInterval * 1_000_000_000))
                guard let self, !Task.isCancelled else { return }
                await self.runtimeSupervisionTick()
            }
        }
    }

    private func runtimeSupervisionTick() async {
        if let pid = spawnedPID, kill(pid, 0) != 0 {
            spawnedPID = nil
            isHealthy = false
            await handleFailure(message: "Agent process exited unexpectedly.")
            if !restartTracker.crashLoopExceeded(at: Date()), !isHealthy, currentWarning?.reason != .crashLoopExceeded {
                await launchWithAutoRestart()
            }
            return
        }

        if await isTradeAutopsyAgentListening() {
            if !isHealthy || currentWarning != nil {
                recoverFromRuntimeDisconnect()
            }
            return
        }

        if isHealthy {
            handleRuntimeDisconnect()
        }
    }

    private func handleRuntimeDisconnect() {
        setWarning(
            AgentHealthWarning(
                reason: .runtimeDisconnected,
                message: "Agent disconnected during session. Live data unavailable until reconnected.",
                logPath: agentLogPath(),
                canRetry: true
            )
        )
    }

    private func recoverFromRuntimeDisconnect() {
        isHealthy = true
        currentWarning = nil
        onHealthChange?(true)
    }

    private func handleFailure(message: String) async {
        restartTracker.recordCrash(at: Date())

        if restartTracker.crashLoopExceeded(at: Date()) {
            setCrashLoopExceededWarning()
            return
        }

        let delay = restartTracker.nextBackoffDelay()
        try? await Task.sleep(nanoseconds: UInt64(delay * 1_000_000_000))
    }

    private func setCrashLoopExceededWarning() {
        setWarning(
            AgentHealthWarning(
                reason: .crashLoopExceeded,
                message: "Agent crashed repeatedly. Manual retry required.",
                logPath: agentLogPath(),
                canRetry: true
            )
        )
    }

    private enum ListenerClassification {
        case tradeAutopsyAgent
        case agentWireRejected
        case foreignProcess
        case noListener
    }

    private func portHasListener() async -> Bool {
        switch await classifyListener() {
        case .noListener:
            return false
        case .tradeAutopsyAgent, .agentWireRejected, .foreignProcess:
            return true
        }
    }

    private func classifyListener() async -> ListenerClassification {
        guard let request = signedHealthRequest() else { return .noListener }

        do {
            let (data, response) = try await session.data(for: request)
            guard let http = response as? HTTPURLResponse else { return .foreignProcess }

            if http.statusCode == 200,
               let decoded = try? JSONDecoder().decode(AgentHealthResponse.self, from: data),
               decoded.status == "ok",
               decoded.daemon == "agent" {
                return .tradeAutopsyAgent
            }

            if looksLikeAgentWireError(data: data, statusCode: http.statusCode) {
                return .agentWireRejected
            }

            return .foreignProcess
        } catch {
            return .noListener
        }
    }

    private func looksLikeAgentWireError(data: Data, statusCode: Int) -> Bool {
        guard statusCode == 401 || statusCode == 412 || statusCode == 428 else { return false }
        guard let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              json["error_class"] is String else {
            return false
        }
        return true
    }

    private func isTradeAutopsyAgentListening() async -> Bool {
        guard let request = signedHealthRequest() else { return false }

        do {
            let (data, response) = try await session.data(for: request)
            guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                return false
            }
            let decoded = try JSONDecoder().decode(AgentHealthResponse.self, from: data)
            return decoded.status == "ok" && decoded.daemon == "agent"
        } catch {
            return false
        }
    }

    private func signedHealthRequest() -> URLRequest? {
        signedWireGetRequest(path: Self.healthPath)
    }

    private func signedKillSwitchStateRequest() -> URLRequest? {
        signedWireGetRequest(path: Self.killSwitchStatePath)
    }

    private func signedWireGetRequest(path: String) -> URLRequest? {
        var request = StationWireClient.signedRequest(
            method: "GET",
            path: path,
            body: Data(),
            daemonSecret: daemonSecret
        )
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        return request
    }

    private func setWarning(_ warning: AgentHealthWarning) {
        isHealthy = false
        currentWarning = warning
        onHealthChange?(false)
    }

    private func agentLogPath() -> URL? {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Logs/tradeautopsy-agent.log")
    }
}

private struct AgentHealthResponse: Decodable {
    let status: String
    let daemon: String
}

private enum AgentSupervisorError: Error {
    case binaryNotFound
}
