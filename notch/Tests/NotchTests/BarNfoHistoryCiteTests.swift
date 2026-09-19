import Testing
@testable import Notch

/// Cite lock: Kotak SDK historical_data names nse_cm, refuses mcx_fo and nse_com,
/// and does not name nse_fo. NFO obtain(history) stays the declared gap.
struct BarNfoHistoryCiteTests {
    @Test func sdkNamesNseFoOnHistoricalDetailsIsCiteMiss() {
        #expect(BarNfoHistoryCopy.sdkNamesNseFoOnHistoricalDetails == false)
    }

    @Test func sessionHoleBodyKeepsKotakHistoryUnsupported() {
        #expect(BarNfoHistoryCopy.sessionHoleBody.contains("kotak_history_unsupported"))
        #expect(BarNfoHistoryCopy.sessionHoleBody.contains("does not compose"))
    }

    @Test func sessionHoleBodyDoesNotNameEapiOrLicensedHistory() {
        #expect(!BarNfoHistoryCopy.sessionHoleBody.contains("eapi"))
        #expect(!BarNfoHistoryCopy.sessionHoleBody.contains("sumOpenInterest"))
        #expect(!BarNfoHistoryCopy.sessionHoleBody.contains("licensed-history"))
    }
}
