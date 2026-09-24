import Testing
@testable import Station

// P1-SWIFT DECISION (B): Notch routing stays source of truth — decided 2026-09-23.
//
// RED PROOF (Swift BrokerCatalog is slug-only, first-book pointers only):
// - station/StationApp/Models/Broker/BrokerCatalog.swift:64-95 — `BrokerCatalog.v1`
//   holds exactly 2 slug rows (binance_com, kotak_neo) with ONE `bookId` each
//   (binance-com-spot, kotak-nse-bse-cash) and a single slug-keyed lookup
//   `descriptor(for slug:)` (:93). No `descriptor(forBookId:)`, no per-book rows.
// - Same file :20-62 — `PlannedBrokerDescriptor.assetClass` is a legacy free String
//   ("crypto_spot" :70, "equities" :83). No instrumentClass, no isInverse.
// - Slug-only use sites: BrokerConnectServices.swift:15, DeskHonesty.swift:26,
//   SessionPulseStripPresentation.swift:112, BrokersViewModel.swift:96.
// - Rust mirror (READ-ONLY, core lane owns the bump): agent/src/ubi/catalog.rs
//   `catalog_v1()` + `descriptor_for_slug` — same 2-row slug-only shape.
//
// WHY B, NOT A:
// 1. No book concept in the Swift catalog to extend — only a first-book `bookId`
//    pointer per slug. Building a 6-book class table here would invent a second,
//    parallel book layer (a fake), not extend an existing one.
// 2. Notch routing ALREADY is the book source of truth and says so in-tree:
//    notch/BarDeskInstruments.swift:355-365 pins all 6 live book_ids, and :356
//    documents kotak-nse-nfo as "not a `BrokerCatalog.v1` row". Routing:
//    `marketBook(for:)` (:391), `deskBookId(slug:assetClass:)` (:405),
//    `InstrumentTickBookId` (:49), quote/chain/OI `book=` extracts in
//    notch/NotchViewModel.swift:2384-2400, :2475-2515. Covered by NotchTests
//    (BrokerBridgeTests, BarOptionsDeclareTests, BarDeskInstrumentsTests).
// 3. A static per-book (asset,instrument) pair is LOSSY for 3 of the 6 books,
//    so any table I write today would be wrong by construction:
//    - kotak-nse-nfo: lock says "equity derivatives", segment nse_fo, pInstType
//      OPTIDX/OPTSTK/FUTIDX/FUTSTK = index+equity x option+future. One pair
//      cannot represent it; Notch routes it via segment+declareAssetClass instead.
//    - binance-com-usdm / binance-com-coinm: locks say "perpetual futures";
//      P1 law splits swap vs future and the books carry perps AND quarterlies.
//      The swap-vs-future ruling belongs to the core lane + ADR 0004, not Swift.
//    - binance-com-options: is_inverse needs the core-lane ruling (options lock:
//      quote/settle names exist, "always USDT" is NOT SPECIFIED IN SOURCE).
// 4. Lane fence: BrokerCatalog.swift lives at StationApp/Models/Broker/ — OUTSIDE
//    this lane's owner files (StationApp/Broker/**). Modifying it is a fence
//    violation; duplicating its role under StationApp/Broker/ would be drift.
//
// DRAFT P1-law mapping (NOT shipped as code — for core-lane cross-check only):
// - binance-com-spot:    cryptocurrency / spot   / false  (clean)
// - kotak-nse-bse-cash:  equity         / spot   / false  (clean; cash lock: lot 1)
// - binance-com-options: cryptocurrency / option / false? (needs is_inverse ruling)
// - binance-com-usdm:    cryptocurrency / swap?  / false  (needs swap-vs-future ruling)
// - binance-com-coinm:   cryptocurrency / swap?  / true   (needs swap-vs-future ruling)
// - kotak-nse-nfo:       ??? — mixed index+equity x option+future (needs representation ruling)
//
// FOLLOW-UP TICKET (named, for the integrator to file/schedule):
//   P1-SWIFT-FOLLOWUP: BrokerCatalog per-book class after core lane lands
//   Preconditions: core lane's book-keyed Rust catalog + ADR 0004 taxonomy
//   rulings landed (USDM/CoinM swap-vs-future; options is_inverse; NFO
//   mixed-class representation — single pair vs per-fill/per-segment stamping).
//   Owner files: station/StationApp/Models/Broker/BrokerCatalog.swift (integrator
//   or a Models-lane agent — NOT this lane's fence) + station/StationTests/**.
//   Work: add P1-law enums (wire snake_case) + per-book rows or
//   `descriptor(forBookId:)`; migrate the 2 legacy assetClass strings; flip
//   `legacyAssetStringsAwaitP1FollowUp` below to the new enums; extend
//   `catalogHasNoBookKeyedLookup` into a positive per-book test. No-drift rule:
//   Station must mirror the Rust book table + CLAIM-REGISTRY, never outrun it;
//   Notch `book=` routing stays the runtime source of truth until this ticket
//   lands and its tests flip green.
//
// RE-VERIFY FLAGS for the integrator (after the core lane lands):
// - R1: Rust `descriptor_for_book_id` (or per-book rows) shape vs the draft
//   mapping above — flip/extend these tests to match, do not hand-edit values.
// - R2: ADR 0004 rulings on the three lossy books (NFO representation, USDM/CoinM
//   swap-vs-future, options is_inverse) before any Station per-book table ships.

struct BrokerBookClassDecisionTests {
    /// The 6 SHIPPING books per issues/compliance/CLAIM-REGISTRY.md (2026-09-23).
    private static let liveBookIds = [
        "binance-com-spot",
        "binance-com-options",
        "binance-com-usdm",
        "binance-com-coinm",
        "kotak-nse-bse-cash",
        "kotak-nse-nfo",
    ]

    @Test func catalogIsSlugOnlyFirstPair() {
        #expect(BrokerCatalog.v1.count == 5)
        #expect(
            BrokerCatalog.v1.map(\.slug) == [
                "binance_com", "kotak_neo", "zerodha_kite", "upstox", "fyers",
            ]
        )
        // First-book pointers only — the 4 second books have no catalog row.
        #expect(
            BrokerCatalog.v1.map(\.bookId) == [
                "binance-com-spot", "kotak-nse-bse-cash", "zerodha-nse-bse-cash",
                "upstox-nse-bse-cash", "fyers-nse-bse-cash",
            ]
        )
        #expect(BrokerCatalog.descriptor(for: "zerodha_kite")?.availability == .planned)
        #expect(BrokerCatalog.descriptor(for: "upstox")?.availability == .planned)
        #expect(BrokerCatalog.descriptor(for: "upstox")?.authScheme == .upstoxOAuthBearerSession)
        #expect(BrokerCatalog.descriptor(for: "fyers")?.availability == .planned)
        #expect(BrokerCatalog.descriptor(for: "fyers")?.authScheme == .fyersOAuthJsonAppIdHashSession)
        #expect(BrokerCatalog.descriptor(for: "binance_com")?.bookId == "binance-com-spot")
        #expect(BrokerCatalog.descriptor(for: "kotak_neo")?.bookId == "kotak-nse-bse-cash")
    }

    @Test func catalogHasNoBookKeyedLookup() {
        // Every live book_id — INCLUDING the two first-books — fails slug lookup.
        // Only slug strings resolve. If a per-book API ever lands, this test must
        // be consciously flipped by P1-SWIFT-FOLLOWUP, never silently broken.
        for bookId in Self.liveBookIds {
            #expect(
                BrokerCatalog.descriptor(for: bookId) == nil,
                "book_id \(bookId) must not resolve via slug lookup while decision B holds"
            )
        }
    }

    @Test func legacyAssetStringsAwaitP1FollowUp() {
        // F3-style pin: Station still carries pre-P1 legacy strings. The core lane
        // + ADR 0004 rule first; P1-SWIFT-FOLLOWUP migrates these to P1-law enums.
        #expect(BrokerCatalog.descriptor(for: "binance_com")?.assetClass == "crypto_spot")
        #expect(BrokerCatalog.descriptor(for: "kotak_neo")?.assetClass == "equities")
    }
}
