import Foundation

/// Spawns, health-checks, supervises, and tears down `tradeautopsy-agent` on loopback.
@MainActor
public final class AgentSupervisor: AgentSupervising {
    nonisolated public static let defaultPort: UInt16 = 9137
    nonisolated public static let healthPath = "/api/daemon/health"
    nonisolated public static let launchTimeout: TimeInterval = 10
    nonisolated public static let runtimePollInterval: TimeInterval = 2

    public var onHealthChange: ((Bool) -> Void)?

    public private(set) var isHealthy = false
    public private(set) var currentWarning: AgentHealthWarning?
    public var ownsSpawnedAgent: Bool { spawnedPID != nil }

    private var spawnedPID: Int32?
    private var restartTracker = AgentRestartTracker()
    private var supervisionTask: Task<Void, Never>?
    private let port: UInt16
    private let session: URLSession

    public init(port: UInt16 = AgentSupervisor.defaultPort, session: URLSession = .shared) {
        self.port = port
        self.session = session
    }

    public func start() async {
        if ProcessInfo.processInfo.environment["STATION_DEV_ATTACH"] == "1" {
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

    public func shutdown() async {
        supervisionTask?.cancel()
        supervisionTask = nil

        guard let pid = spawnedPID else { return }
        kill(pid, SIGTERM)
        try? await Task.sleep(nanoseconds: 3_000_000_000)
        if kill(pid, 0) == 0 {
            kill(pid, SIGKILL)
        }
        spawnedPID = nil
    }

    private func launchWithAutoRestart() async {
        while !Task.isCancelled {
            if restartTracker.crashLoopExceeded(at: Date()) {
                setCrashLoopExceededWarning()
                return
            }

            spawnedPID = nil
            do {
                try spawnAgent()
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

            if spawnedPID != nil {
                await shutdown()
            }
            await handleFailure(message: "Agent did not respond on loopback within \(Int(Self.launchTimeout)) seconds.")
        }
    }

    private func attachIfVerified() async {
        if await isTradeAutopsyAgentListening() {
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
        }
    }

    private func tryAttachToExistingListener() async -> Bool {
        guard await portHasListener() else { return false }

        if await isTradeAutopsyAgentListening() {
            markHealthyAttached()
            return true
        }

        setWarning(
            AgentHealthWarning(
                reason: .portCollisionNonAgent,
                message: "Port \(port) is in use by a non-agent process. Check DAEMON_PORT and other listeners.",
                logPath: nil,
                canRetry: true
            )
        )
        return true
    }

    private func markHealthyAttached() {
        isHealthy = true
        currentWarning = nil
        restartTracker.resetOnSuccessfulAttach()
        onHealthChange?(true)
        beginRuntimeSupervision()
    }

    private func spawnAgent() throws {
        guard let binaryURL = Bundle.main.url(forAuxiliaryExecutable: "tradeautopsy-agent")
            ?? Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent("tradeautopsy-agent")
        else {
            throw AgentSupervisorError.binaryNotFound
        }

        let process = Process()
        process.executableURL = binaryURL
        process.environment = ProcessInfo.processInfo.environment

        try process.run()
        spawnedPID = process.processIdentifier
    }

    @discardableResult
    private func probeHealthUntilReady() async -> Bool {
        let deadline = Date().addingTimeInterval(Self.launchTimeout)
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
            if !isHealthy {
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

    private func portHasListener() async -> Bool {
        guard let url = URL(string: "http://127.0.0.1:\(port)\(Self.healthPath)") else {
            return false
        }

        do {
            let (_, response) = try await session.data(from: url)
            return response is HTTPURLResponse
        } catch {
            return false
        }
    }

    private func isTradeAutopsyAgentListening() async -> Bool {
        guard let url = URL(string: "http://127.0.0.1:\(port)\(Self.healthPath)") else {
            return false
        }

        do {
            let (data, response) = try await session.data(from: url)
            guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                return false
            }
            let decoded = try JSONDecoder().decode(AgentHealthResponse.self, from: data)
            return decoded.status == "ok" && decoded.daemon == "agent"
        } catch {
            return false
        }
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
