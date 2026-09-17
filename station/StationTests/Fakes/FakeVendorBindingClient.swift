import Foundation
@testable import Station

@MainActor
final class FakeVendorBindingClient: VendorBindingClienting {
    private(set) var puts: [VendorBindingPut] = []
    var error: VendorBindingError?

    func putBinding(_ body: VendorBindingPut) async throws {
        if let error {
            throw error
        }
        puts.append(body)
    }
}
