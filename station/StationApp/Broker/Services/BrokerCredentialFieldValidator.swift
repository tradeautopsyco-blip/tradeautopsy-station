import Foundation

public enum BrokerCredentialFieldValidator {
    public static func invalidFields(apiKey: String, apiSecret: String) -> Set<BrokerCredentialField> {
        var invalid = Set<BrokerCredentialField>()
        if apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.apiKey)
        }
        if apiSecret.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.apiSecret)
        }
        return invalid
    }

    public static func invalidKotakLoginFields(
        consumerKey: String,
        mobileNumber: String,
        ucc: String,
        totp: String,
        mpin: String
    ) -> Set<BrokerCredentialField> {
        var invalid = Set<BrokerCredentialField>()
        if consumerKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.consumerKey)
        }
        if mobileNumber.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.mobileNumber)
        }
        if ucc.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.ucc)
        }
        if totp.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.totp)
        }
        if mpin.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.mpin)
        }
        return invalid
    }

    public static func invalidDhanFields(
        dhanClientId: String,
        apiKey: String,
        apiSecret: String
    ) -> Set<BrokerCredentialField> {
        var invalid = Set<BrokerCredentialField>()
        if dhanClientId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.consumerKey)
        }
        if apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.apiKey)
        }
        if apiSecret.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.apiSecret)
        }
        return invalid
    }

    public static func invalidOkxPassphraseFields(
        apiKey: String,
        apiSecret: String,
        passphrase: String
    ) -> Set<BrokerCredentialField> {
        var invalid = invalidFields(apiKey: apiKey, apiSecret: apiSecret)
        if passphrase.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.passphrase)
        }
        return invalid
    }

    public static func invalidCoinbaseJwtFields(
        apiKey: String,
        pemPrivateKey: String
    ) -> Set<BrokerCredentialField> {
        var invalid = Set<BrokerCredentialField>()
        if apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.apiKey)
        }
        let pem = pemPrivateKey.trimmingCharacters(in: .whitespacesAndNewlines)
        if pem.isEmpty {
            invalid.insert(.pemPrivateKey)
        } else if !pem.contains("BEGIN") || !pem.contains("PRIVATE KEY") {
            invalid.insert(.pemPrivateKey)
        }
        return invalid
    }
}
