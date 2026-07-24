import Foundation
import Testing
@testable import Station

@MainActor
struct DeviceLoginViewModelTests {
    @Test func beginPublishesUserCodeOnlyAndNeverDeviceCode() async {
        let client = FakeDeviceLoginClient()
        client.beginResult = .success(
            DeviceLoginChallenge(
                userCode: "RRGQ-BJVS",
                verificationURI: "https://example.authkit.app/device",
                verificationURIComplete: "https://example.authkit.app/device?user_code=RRGQ-BJVS",
                expiresIn: 300,
                interval: 5
            )
        )
        let viewModel = DeviceLoginViewModel(client: client)

        await viewModel.beginLogin()

        #expect(viewModel.phase == .awaitingBrowser)
        #expect(viewModel.userCode == "RRGQ-BJVS")
        #expect(viewModel.verificationURIComplete?.contains("user_code=RRGQ-BJVS") == true)
        #expect(viewModel.errorMessage == nil)
        // A8: device_code must never appear on UI-facing state.
        let mirror = Mirror(reflecting: viewModel)
        let labels = mirror.children.compactMap(\.label)
        #expect(!labels.contains(where: { $0.lowercased().contains("device") && $0.lowercased().contains("code") }))
        #expect(String(describing: viewModel).contains("device_code") == false)
        #expect(String(describing: viewModel).contains("secret-device") == false)
    }

    @Test func completeTransitionsToSignedInWithSessionEmail() async {
        let client = FakeDeviceLoginClient()
        client.beginResult = .success(
            DeviceLoginChallenge(
                userCode: "AAAA-BBBB",
                verificationURI: "https://example.authkit.app/device",
                verificationURIComplete: "https://example.authkit.app/device?user_code=AAAA-BBBB",
                expiresIn: 300,
                interval: 5
            )
        )
        client.completeResult = .success(
            StationSessionIdentity(
                profileID: "11111111-1111-4111-8111-111111111111",
                email: "trader@example.com",
                aud: "station"
            )
        )
        let viewModel = DeviceLoginViewModel(client: client)
        await viewModel.beginLogin()

        await viewModel.completeLogin()

        #expect(viewModel.phase == .signedIn)
        #expect(viewModel.signedInEmail == "trader@example.com")
        #expect(viewModel.userCode == nil)
        #expect(client.completeCallCount == 1)
    }

    @Test func refreshSessionWhenAlreadySignedInSkipsBegin() async {
        let client = FakeDeviceLoginClient()
        client.sessionResult = .success(
            StationSessionIdentity(
                profileID: "11111111-1111-4111-8111-111111111111",
                email: "existing@example.com",
                aud: "station"
            )
        )
        let viewModel = DeviceLoginViewModel(client: client)

        await viewModel.refreshSession()

        #expect(viewModel.phase == .signedIn)
        #expect(viewModel.signedInEmail == "existing@example.com")
        #expect(client.beginCallCount == 0)
    }

    @Test func beginFailureSurfacesErrorWithoutUserCode() async {
        let client = FakeDeviceLoginClient()
        client.beginResult = .failure(DeviceLoginClientError.unavailable("WorkOS client not configured"))
        let viewModel = DeviceLoginViewModel(client: client)

        await viewModel.beginLogin()

        #expect(viewModel.phase == .error)
        #expect(viewModel.userCode == nil)
        #expect(viewModel.errorMessage?.contains("WorkOS") == true)
    }

    @Test func challengeJSONContractOmitsDeviceCode() throws {
        let challenge = DeviceLoginChallenge(
            userCode: "RRGQ-BJVS",
            verificationURI: "https://example.authkit.app/device",
            verificationURIComplete: "https://example.authkit.app/device?user_code=RRGQ-BJVS",
            expiresIn: 300,
            interval: 5
        )
        let data = try JSONEncoder().encode(challenge)
        let object = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        #expect(object?["user_code"] as? String == "RRGQ-BJVS")
        #expect(object?["device_code"] == nil)
    }
}
