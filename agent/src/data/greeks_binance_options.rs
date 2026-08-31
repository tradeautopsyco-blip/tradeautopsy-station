//! Binance.com European options greeks — **venue-published**.
//!
//! `GET /eapi/v1/mark` publishes `delta` / `theta` / `gamma` / `vega` plus the IVs.
//! Station copies those strings verbatim and attributes them
//! [`GreeksSource::VenuePublished`]. It does **not** price this book: units,
//! day-count and the venue's model are NOT SPECIFIED IN SOURCE, so there is no
//! model to name and `ModelComputed` is structurally unavailable here.
//!
//! Do not copy NFO formulas onto USDT options, and do not carry this book's
//! `riskFreeInterest` or unit conventions the other way.
//!
//! Lock: `locks/binance-com-options.md` Slice 3 ·
//! `docs/reference/crypto/binance-global/options/REST.md` Slice 3.

use super::binance_options_mark::{OptionsMarkRow, OPTIONS_MARK_PATH};
use super::descriptor::BINANCE_COM_ADAPTER_ID;
use super::greeks::{
    dark_fence, greeks_identity, GreeksEnvelope, GreeksSource, GreeksStatus,
    MARK_SNAPSHOT_UNAVAILABLE,
};
use super::provenance::ProvenanceLine;
use super::rights::Rights;

/// Provenance `model` for a copied number. Deliberately not a formula name — the
/// venue did not publish one, and inventing "black_76" here would be a lie about
/// who computed it.
pub const VENUE_PUBLISHED_MODEL: &str = "venue_published";

/// No mark snapshot in hand. Own hole — not inherited-dark from a chain, because
/// venue-published greeks are a `mark` pass-through, not a chain+contracts compute.
pub fn extract_binance_options_greeks() -> GreeksEnvelope {
    dark_fence(MARK_SNAPSHOT_UNAVAILABLE)
}

/// Lit greeks from one `/eapi/v1/mark` row.
///
/// `as_of` is the RFC3339 stamp of the fetch. Without it the envelope cannot pass
/// `greeks_may_render_number`, which is the intended behaviour: an unstamped
/// number has no freshness and must not render.
pub fn extract_binance_options_greeks_from(
    row: Option<&OptionsMarkRow>,
    as_of: Option<&str>,
) -> GreeksEnvelope {
    let Some(row) = row else {
        return extract_binance_options_greeks();
    };
    let identity = greeks_identity();

    // Only what the venue actually published. Absent IVs stay absent rather than
    // serializing as null — `-1.0` was already refused upstream in the parser.
    let mut data = serde_json::Map::new();
    data.insert("symbol".into(), row.symbol.clone().into());
    data.insert("mark_price".into(), row.mark_price.clone().into());
    data.insert("delta".into(), row.delta.clone().into());
    data.insert("gamma".into(), row.gamma.clone().into());
    data.insert("theta".into(), row.theta.clone().into());
    data.insert("vega".into(), row.vega.clone().into());
    for (key, value) in [
        ("mark_iv", &row.mark_iv),
        ("bid_iv", &row.bid_iv),
        ("ask_iv", &row.ask_iv),
        ("risk_free_interest", &row.risk_free_interest),
    ] {
        if let Some(value) = value {
            data.insert(key.into(), value.clone().into());
        }
    }

    GreeksEnvelope {
        identity: identity.clone(),
        status: GreeksStatus::Success,
        data: Some(serde_json::Value::Object(data)),
        provenance: ProvenanceLine {
            identity,
            model: VENUE_PUBLISHED_MODEL.to_string(),
            input_at: as_of.map(str::to_string),
            adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
            path: OPTIONS_MARK_PATH.to_string(),
        },
        // The one display grant in this module: Station is licensed to show what
        // the venue published. It is still not licensed to store or redistribute it.
        rights: Rights::desk_display(),
        source: Some(GreeksSource::VenuePublished {
            path: OPTIONS_MARK_PATH.to_string(),
            adapter: BINANCE_COM_ADAPTER_ID.to_string(),
        }),
        ineligible: Vec::new(),
        canonical: false,
        persist_canonical: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::binance_options_mark::mark_row_for_symbol;
    use crate::data::greeks::{
        extract_greeks, greeks_may_render_number, PRICING_MODEL_UNSPECIFIED,
    };
    use crate::data::honesty::{HonestyStatus, InputHonesty};
    use crate::data::BINANCE_COM_OPTIONS_BOOK_ID;

    const OFFICIAL_EXAMPLE: &str = r#"[ { "symbol": "BTC-200730-9000-C", "markPrice": "1343.2883", "bidIV": "1.40000077", "askIV": "1.50000153", "markIV": "1.45000000", "delta": "0.55937056", "theta": "3739.82509871", "gamma": "0.00010969", "vega": "978.58874732", "highPriceLimit": "1618.241", "lowPriceLimit": "1068.3356", "riskFreeInterest": "0.1" } ]"#;
    const LIVE_NO_BID: &str = r#"[{"symbol":"BTC-260925-145000-C","markPrice":"2.439","bidIV":"-1.0","askIV":"0.74930251","markIV":"0.709","delta":"0.00065535","theta":"-0.84070424","gamma":"0.00000012","vega":"0.46744754","highPriceLimit":"750","lowPriceLimit":"5","riskFreeInterest":"0.0454"}]"#;

    fn official_row() -> OptionsMarkRow {
        mark_row_for_symbol(OFFICIAL_EXAMPLE, "BTC-200730-9000-C").expect("row")
    }

    fn lit() -> GreeksEnvelope {
        extract_binance_options_greeks_from(Some(&official_row()), Some("2026-08-31T09:00:00Z"))
    }

    #[test]
    fn a_stamped_mark_row_renders_a_number() {
        let envelope = lit();
        assert!(greeks_may_render_number(&envelope));
        let data = envelope.data.as_ref().expect("data");
        assert_eq!(data["delta"], "0.55937056");
        assert_eq!(data["vega"], "978.58874732");
    }

    #[test]
    fn the_number_is_venue_published_never_model_computed() {
        let envelope = lit();
        assert_eq!(
            envelope.source,
            Some(GreeksSource::VenuePublished {
                path: "/eapi/v1/mark".into(),
                adapter: "binance_com".into(),
            })
        );
        // Station named no pricer, because the venue published none.
        assert_eq!(envelope.provenance.model, "venue_published");
        assert_ne!(envelope.provenance.model, "black_76");
        assert_eq!(envelope.provenance.path, "/eapi/v1/mark");
    }

    #[test]
    fn an_unstamped_row_must_not_render() {
        let envelope = extract_binance_options_greeks_from(Some(&official_row()), None);
        assert!(envelope.data.is_some(), "the data is there");
        assert!(
            !greeks_may_render_number(&envelope),
            "but with no input_at it has no freshness and must stay dark"
        );
    }

    #[test]
    fn no_snapshot_is_this_extracts_own_hole() {
        let envelope = extract_binance_options_greeks();
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert_eq!(envelope.ineligible, vec![MARK_SNAPSHOT_UNAVAILABLE]);
        assert!(envelope.data.is_none());
        assert!(envelope.source.is_none());
        assert!(!greeks_may_render_number(&envelope));
        // Not inherited-dark: mark is a pass-through, not a chain+contracts compute.
        assert_ne!(envelope.status, GreeksStatus::InheritedDark);
    }

    #[test]
    fn a_refused_iv_is_absent_not_null() {
        let row = mark_row_for_symbol(LIVE_NO_BID, "BTC-260925-145000-C").expect("row");
        let envelope =
            extract_binance_options_greeks_from(Some(&row), Some("2026-08-31T09:00:00Z"));
        let data = envelope.data.as_ref().expect("data");
        assert!(
            data.get("bid_iv").is_none(),
            "-1.0 must vanish, not serialize as null"
        );
        assert_eq!(data["ask_iv"], "0.74930251");
        let wire = serde_json::to_string(&envelope).unwrap();
        assert!(
            !wire.contains("-1.0"),
            "the sentinel must not reach the wire"
        );
    }

    #[test]
    fn this_book_never_carries_a_rho() {
        let wire = serde_json::to_string(&lit()).unwrap();
        assert!(!wire.contains("rho"), "four greeks, not five");
    }

    #[test]
    fn never_calls_nfo_pricing_model_unspecified() {
        // The NFO hole belongs to the other book. This one has no model to miss.
        for envelope in [lit(), extract_binance_options_greeks()] {
            assert!(!envelope
                .ineligible
                .iter()
                .any(|reason| reason == PRICING_MODEL_UNSPECIFIED));
        }
    }

    #[test]
    fn dual_no_blend_no_inr_field() {
        let wire = serde_json::to_string(&lit()).unwrap();
        assert!(!wire.contains("exchange_rate"));
        assert!(!wire.contains("INR"));
        assert!(!wire.contains("NIFTY"));
    }

    #[test]
    fn the_dispatcher_still_refuses_without_a_snapshot() {
        for chain in [InputHonesty::Lit, InputHonesty::Dark(HonestyStatus::Empty)] {
            let envelope = extract_greeks(Some(BINANCE_COM_OPTIONS_BOOK_ID), chain);
            assert_eq!(envelope.ineligible, vec![MARK_SNAPSHOT_UNAVAILABLE]);
            assert!(!greeks_may_render_number(&envelope));
        }
    }
}
