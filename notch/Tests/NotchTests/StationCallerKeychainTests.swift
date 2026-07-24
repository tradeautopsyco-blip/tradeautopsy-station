import Foundation
import Testing
@testable import Notch

struct StationCallerKeychainTests {
    @Test func tokensJSONContractMatchesAgentKeyringShape() throws {
        let json = """
        {"access_token":"station.access","refresh_token":"station.refresh","expires_in":900,"refresh_expires_in":1000}
        """.data(using: .utf8)!
        let tokens = try JSONDecoder().decode(StationCallerKeychain.Tokens.self, from: json)
        #expect(tokens.accessToken == "station.access")
        #expect(tokens.refreshToken == "station.refresh")
        #expect(tokens.expiresIn == 900)
    }

    @MainActor
    @Test func loopbackWireUserIdIsNotBrainIdentityConstantName() {
        // A8: wire UUID is machine integrity only — not a profile / WorkOS user id.
        let viewModel = NotchViewModel()
        #expect(UUID(uuidString: viewModel.loopbackWireUserId) != nil)
        #expect(viewModel.loopbackWireUserId == "00000000-0000-4000-8000-000000000002")
    }
}
