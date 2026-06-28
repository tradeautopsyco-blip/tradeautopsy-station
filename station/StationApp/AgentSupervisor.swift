import Foundation

/// Spawns, health-checks, supervises, and tears down `tradeautopsy-agent` on loopback.
@MainActor
public final class AgentSupervisor: AgentSupervising {
    nonisolated public static let defaultPort: UInt16 = 9137
    nonisolated public static let healthPath = "/api/daemon/health"
    nonisolated public static let launchTimeout: TimeInterval = 10

    public var onHealthChange: ((Bool) -> Void)?

    public private(set) var isHealthy = false
    public private(set) var currentWarning: AgentHealthWarning?

    private var spawnedPID: Int32?
    private var crashTimestamps: [Date] = []
    private let port: UInt16
    private let session: URLSession

    public init(port: UInt16 = AgentSupervisor.defaultPort, session: URLSession = .shared) {
        self.port = port
        self.session = session
    }

    public func start() async {
        if ProcessInfo.processInfo.environment["STATION_DEV_ATTACH"] == "1" {
            await probeHealthUntilReady()
            return
        }

        if spawnedPID == nil {
            do {
                try spawnAgent()
            } catch AgentSupervisorError.portInUse {
                await handlePortCollision()
                return
            } catch {
                setWarning(
                    AgentHealthWarning(
                        reason: .launchTimeout,
                        message: "Failed to spawn agent: \(error.localizedDescription)",
                        logPath: nil,
                        canRetry: true
                    )
                )
                return
            }
        }

        await probeHealthUntilReady()
    }

    public func retry() async {
        crashTimestamps.removeAll()
        currentWarning = nil
        await start()
    }

    public func shutdown() async {
        guard let pid = spawnedPID else { return }
        kill(pid, SIGTERM)
        try? await Task.sleep(nanoseconds: 3_000_000_000)
        if kill(pid, 0) == 0 {
            kill(pid, SIGKILL)
        }
        spawnedPID = nil
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

    private func handlePortCollision() async {
        if await isTradeAutopsyAgentListening() {
            isHealthy = true
            currentWarning = nil
            onHealthChange?(true)
            return
        }

        setWarning(
            AgentHealthWarning(
                reason: .portCollisionNonAgent,
                message: "Port \(port) is in use by a non-agent process. Check DAEMON_PORT and other listeners.",
                logPath: nil,
                canRetry: true
            )
        )
    }

    private func probeHealthUntilReady() async {
        let deadline = Date().addingTimeInterval(Self.launchTimeout)
        var delay: UInt64 = 200_000_000

        while Date() < deadline {
            if await isTradeAutopsyAgentListening() {
                isHealthy = true
                currentWarning = nil
                onHealthChange?(true)
                return
            }
            try? await Task.sleep(nanoseconds: delay)
            delay = min(delay * 2, 1_000_000_000)
        }

        setWarning(
            AgentHealthWarning(
                reason: .launchTimeout,
                message: "Agent did not respond on loopback within \(Int(Self.launchTimeout)) seconds.",
                logPath: agentLogPath(),
                canRetry: true
            )
        )
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
    case portInUse
}
