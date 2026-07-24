import Foundation
@testable import Station

final class FakeDeviceLoginClient: DeviceLoginClient, @unchecked Sendable {
    var beginResult: Result<DeviceLoginChallenge, Error> = .failure(DeviceLoginClientError.unavailable("unset"))
    var completeResult: Result<StationSessionIdentity, Error> = .failure(DeviceLoginClientError.unavailable("unset"))
    var sessionResult: Result<StationSessionIdentity?, Error> = .success(nil)

    private(set) var beginCallCount = 0
    private(set) var completeCallCount = 0
    private(set) var sessionCallCount = 0

    func begin() async throws -> DeviceLoginChallenge {
        beginCallCount += 1
        return try beginResult.get()
    }

    func complete() async throws -> StationSessionIdentity {
        completeCallCount += 1
        return try completeResult.get()
    }

    func currentSession() async throws -> StationSessionIdentity? {
        sessionCallCount += 1
        return try sessionResult.get()
    }

    func signOut() async throws {
        sessionResult = .success(nil)
    }
}
