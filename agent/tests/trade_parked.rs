//! TRADE is parked — founder has not named a lock amendment for venue place.
//!
//! Lock: `issues/compliance/locks/binance-com-usdm.md`, `binance-com-options.md`,
//! `binance-com-coinm.md`, `binance-com-spot.md`.
//! `POST /fapi/v1/order`, `POST /eapi/v1/order`, `POST /dapi/v1/order`,
//! `POST /api/v3/order`, `POST /fapi/v1/leverage`, `POST /fapi/v1/marginType`,
//! `POST /fapi/v1/algoOrder`, `POST /dapi/v1/algoOrder` stay MutationForbidden.

use tradeautopsy_agent::{authorize_book_call, infer_capability, is_mutation, HostRefuse};

#[test]
fn parked_trade_paths_stay_mutation_forbidden() {
    let parked = [
        ("binance-com-usdm", "fapi.binance.com", "/fapi/v1/order"),
        ("binance-com-usdm", "fapi.binance.com", "/fapi/v1/leverage"),
        ("binance-com-usdm", "fapi.binance.com", "/fapi/v1/marginType"),
        ("binance-com-usdm", "fapi.binance.com", "/fapi/v1/algoOrder"),
        ("binance-com-options", "eapi.binance.com", "/eapi/v1/order"),
        ("binance-com-coinm", "dapi.binance.com", "/dapi/v1/order"),
        ("binance-com-coinm", "dapi.binance.com", "/dapi/v1/algoOrder"),
        ("binance-com-spot", "api.binance.com", "/api/v3/order"),
    ];
    for (book, host, path) in parked {
        assert!(
            is_mutation("POST", path),
            "{path} must stay a mutation until TRADE is named"
        );
        assert_eq!(
            infer_capability("POST", path).unwrap_err(),
            HostRefuse::MutationForbidden,
            "{path} infer stays MutationForbidden"
        );
        assert_eq!(
            authorize_book_call(book, host, "POST", path, true).unwrap_err(),
            HostRefuse::MutationForbidden,
            "{book} {path} stays MutationForbidden"
        );
    }
}
