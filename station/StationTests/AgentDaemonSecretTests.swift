import Foundation
import Testing
@testable import Station

struct AgentDaemonSecretTests {
    @Test func ephemeralSecretIs64HexChars() {
        let secret = AgentDaemonSecret.generateEphemeral()
        #expect(secret.count == 64)
        #expect(secret.allSatisfy { $0.isHexDigit })
    }

    @Test func ephemeralSecretsAreUnique() {
        let first = AgentDaemonSecret.generateEphemeral()
        let second = AgentDaemonSecret.generateEphemeral()
        #expect(first != second)
    }
}

private extension Character {
    var isHexDigit: Bool {
        ("0"..."9").contains(self) || ("a"..."f").contains(self) || ("A"..."F").contains(self)
    }
}
