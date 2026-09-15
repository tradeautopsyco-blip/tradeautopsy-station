//! Derived greeks dispatcher. Two book modules (or zero), never one
//! `calculate()` with `if CRYPTO`. The two books light by different rules, and
//! neither may borrow the other's.
//!
//! Binance options: [`greeks_binance_options`] — **lit**. `GET /eapi/v1/mark` is
//! allowlisted (Slice 3) and publishes delta/theta/gamma/vega plus the IVs.
//! Station copies those strings verbatim, attributes them
//! [`GreeksSource::VenuePublished`], and this is the one path that carries
//! `Rights::desk_display()`. With no snapshot in hand the extract is its own
//! `mark_snapshot_unavailable` hole — not inherited-dark, because mark is a
//! pass-through rather than a chain+contracts compute.
//!
//! NFO: [`greeks_nfo`] — **still dark**. Kotak Quotes has no greeks field, so
//! there is nothing to copy, and `pricing_model_unspecified` stands until
//! OPTIONS-PRICING.md is a real lock. Do not give this book a lit path.
//! Source: `docs/reference/india/nfo/OPTIONS-PRICING.md` BLOCKER.
//!
//! Two entries, one chokepoint: [`extract_greeks`] (no snapshot) and
//! [`extract_greeks_from_mark`] (row in hand) both end on
//! [`refuse_misfiled_source`].

use super::binance_options_mark::OptionsMarkRow;
use super::descriptor::{
    BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID,
};
use super::greeks_binance_options::{
    extract_binance_options_greeks, extract_binance_options_greeks_from,
};
use super::greeks_nfo::extract_nfo_greeks;
use super::honesty::{HonestyStatus, InputHonesty};
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::provenance::ProvenanceLine;
use super::rights::Rights;
use serde::Serialize;

pub const PRICING_MODEL_UNSPECIFIED: &str = "pricing_model_unspecified";
/// No `/eapi/v1/mark` snapshot in hand. The path is allowlisted (Slice 3); this
/// says the fetch has not produced a row, not that the capability is fenced.
pub const MARK_SNAPSHOT_UNAVAILABLE: &str = "mark_snapshot_unavailable";
pub const CASH_IS_NOT_GREEKS: &str = "cash_is_not_greeks";
pub const SPOT_IS_NOT_GREEKS: &str = "spot_is_not_greeks";
pub const BOOK_REQUIRED: &str = "greeks_book_required";
/// A book produced a source class it is structurally not allowed to produce.
/// Fail closed: a greek whose origin is misfiled is not a greek we may show.
pub const SOURCE_CLASS_MISMATCH: &str = "greeks_source_class_mismatch";
/// `Success` + `data` that [`greeks_may_render_number`] refuses. The extract must
/// not hand Notch a number the gate would hide.
pub const GREEKS_NOT_RENDERABLE: &str = "greeks_not_renderable";

/// Where a greek number came from. There are exactly two honest origins, and each
/// book may produce only one of them.
///
/// - `VenuePublished` — the **venue** computed it and Station copied the fields.
///   Binance options only: `GET /eapi/v1/mark` names delta/theta/gamma/vega and the
///   IVs. Kotak Quotes `quote_type` has no greeks field, so NFO can never be this.
/// - `ModelComputed` — **Station** computed it from a named model under a named
///   lock. NFO only. Station does not run a pricer over USDT options, so Binance
///   options can never be this.
///
/// This is the two-module law (`greeks_nfo` vs `greeks_binance_options`) written as
/// a type instead of a comment. `extract_greeks` is the single chokepoint that sees
/// both books, so it is where the pairing is enforced — see [`source_class_allowed`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GreeksSource {
    /// Copied from the venue. `path` is the upstream path the fields came from.
    VenuePublished { path: String, adapter: String },
    /// Computed by Station. `model` names the pricer; `lock` names the doc that
    /// authorises it. Neither may be empty — an unnamed model is not a model.
    ModelComputed { model: String, lock: String },
}

/// The source class each book is allowed to produce. Anything else is a
/// programming error that this chokepoint refuses rather than renders.
fn source_class_allowed(book: &str, source: &GreeksSource) -> bool {
    matches!(
        (book, source),
        (
            BINANCE_COM_OPTIONS_BOOK_ID,
            GreeksSource::VenuePublished { .. }
        ) | (KOTAK_NSE_NFO_BOOK_ID, GreeksSource::ModelComputed { .. })
    )
}

/// Extract status on the greeks wire. Not [`HonestyStatus`] — Success is live
/// data, and `HonestyStatus` deliberately has no Success variant.
///
/// Mirrors [`GlanceStatus`](super::glance::GlanceStatus), which solved the same
/// problem for the chain wire, plus `InheritedDark`: the NFO book inherits
/// darkness from its named chain and contracts inputs, which a glance never does.
///
/// Writing a venue-published greek as `Empty` — "live, legitimately zero" — would
/// be exactly the kind of mislabelled state this envelope exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GreeksStatus {
    Success,
    Unavailable,
    Empty,
    Unusable,
    InheritedDark,
}

impl GreeksStatus {
    /// Inherited-dark extracts must not carry a payload — same invariant as
    /// [`HonestyStatus::requires_data_none`].
    pub fn requires_data_none(self) -> bool {
        matches!(self, Self::InheritedDark)
    }
}

impl From<HonestyStatus> for GreeksStatus {
    /// Every dark state maps across unchanged. There is no inverse: `Success` has
    /// no `HonestyStatus` to become.
    fn from(status: HonestyStatus) -> Self {
        match status {
            HonestyStatus::Empty => Self::Empty,
            HonestyStatus::Unavailable => Self::Unavailable,
            HonestyStatus::Unusable => Self::Unusable,
            HonestyStatus::InheritedDark => Self::InheritedDark,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GreeksEnvelope {
    pub identity: Identity,
    pub status: GreeksStatus,
    pub data: Option<serde_json::Value>,
    pub provenance: ProvenanceLine,
    /// License grant on this number. Holes are `research_fetch_only`; only the
    /// venue-published lit path carries `desk_display`, which is what
    /// [`greeks_may_render_number`] reads.
    pub rights: Rights,
    /// `None` on every hole. Present only when a number is actually carried, and
    /// then it names which of the two origins produced it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<GreeksSource>,
    pub ineligible: Vec<String>,
    pub canonical: bool,
    pub persist_canonical: bool,
}

pub(crate) fn greeks_identity() -> Identity {
    Identity::new(
        Family::Derived,
        CapabilityId::new("greeks").expect("canonical greeks id"),
        Physics::BoundedSnapshot,
    )
}

pub(crate) fn dark_fence(ineligible: &str) -> GreeksEnvelope {
    let identity = greeks_identity();
    GreeksEnvelope {
        identity: identity.clone(),
        status: GreeksStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        rights: Rights::research_fetch_only(),
        source: None,
        ineligible: vec![ineligible.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

/// Book-routed greeks extract. `book_id` is required — NFO is not the default.
pub fn extract_greeks(book_id: Option<&str>, chain: InputHonesty) -> GreeksEnvelope {
    let Some(book) = book_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return dark_fence(BOOK_REQUIRED);
    };

    let envelope = match book {
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID => dark_fence(CASH_IS_NOT_GREEKS),
        id if id == BINANCE_COM_SPOT_BOOK_ID => dark_fence(SPOT_IS_NOT_GREEKS),
        id if id == BINANCE_COM_OPTIONS_BOOK_ID => extract_binance_options_greeks(),
        id if id == KOTAK_NSE_NFO_BOOK_ID => extract_nfo_greeks(chain),
        _ => return dark_fence(BOOK_REQUIRED),
    };

    refuse_misfiled_source(book, envelope)
}

/// The single source-class chokepoint, shared by both entries.
///
/// A book that hands back the other book's source class has mixed the two pricers.
/// Refuse the envelope rather than let a mislabelled origin render.
fn refuse_misfiled_source(book: &str, envelope: GreeksEnvelope) -> GreeksEnvelope {
    match &envelope.source {
        Some(source) if !source_class_allowed(book, source) => dark_fence(SOURCE_CLASS_MISMATCH),
        _ => envelope,
    }
}

fn refuse_unrenderable(envelope: GreeksEnvelope) -> GreeksEnvelope {
    if envelope.data.is_none() || greeks_may_render_number(&envelope) {
        return envelope;
    }
    dark_fence(GREEKS_NOT_RENDERABLE)
}

/// Book-routed greeks extract with a `/eapi/v1/mark` row in hand.
///
/// Only `binance-com-options` reads the row: those greeks are venue-published, and
/// a USDT-options delta is not an NFO greek. NFO ignores the row and stays on its
/// own dark path — this entry must not give it a lit one. `chain` is that book's
/// named chain input; the Binance arm has no use for it.
///
/// Ends on the same [`refuse_misfiled_source`] chokepoint as [`extract_greeks`].
pub fn extract_greeks_from_mark(
    book_id: Option<&str>,
    row: Option<&OptionsMarkRow>,
    as_of: Option<&str>,
    chain: InputHonesty,
) -> GreeksEnvelope {
    let Some(book) = book_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return dark_fence(BOOK_REQUIRED);
    };

    let envelope = match book {
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID => dark_fence(CASH_IS_NOT_GREEKS),
        id if id == BINANCE_COM_SPOT_BOOK_ID => dark_fence(SPOT_IS_NOT_GREEKS),
        id if id == BINANCE_COM_OPTIONS_BOOK_ID => extract_binance_options_greeks_from(row, as_of),
        id if id == KOTAK_NSE_NFO_BOOK_ID => extract_nfo_greeks(chain),
        _ => return dark_fence(BOOK_REQUIRED),
    };

    refuse_unrenderable(refuse_misfiled_source(book, envelope))
}

/// May this envelope's number be shown to a trader?
///
/// Reads the envelope, not just a status code. Four things must all hold:
/// the identity is greeks, data is actually carried, the input is stamped, the
/// model is not the `"raw"` hole marker, and the origin is named. A figure
/// missing any of those is either a hole or an unattributed number, and neither
/// may render.
///
/// It also reads the license grant and the status: a number Station may fetch but
/// is not licensed to display is not renderable, and a status that is not
/// `Success` is not a live number at all.
///
/// It deliberately does **not** consult [`HonestyStatus`]. All four of its
/// variants are dark states — Success is `InputHonesty::Lit`, not a fifth variant
/// — so a match on it could only ever return false. That is precisely what made
/// the previous version of this gate dead code: every arm returned false, so no
/// caller could have rendered a number even once the data was real. The "no fifth
/// state" tripwire lives in `honesty.rs::four_snake_case_states_and_no_fifth`,
/// which is a stronger check than an exhaustive match here.
pub fn greeks_may_render_number(envelope: &GreeksEnvelope) -> bool {
    if envelope.identity.capability_id.as_str() != "greeks" {
        return false;
    }
    if envelope.data.is_none() {
        return false;
    }
    if envelope.provenance.input_at.is_none() {
        return false;
    }
    let model = envelope.provenance.model.trim();
    if model.is_empty() || model == "raw" {
        return false;
    }
    if !envelope.rights.display {
        return false;
    }
    if envelope.status != GreeksStatus::Success {
        return false;
    }
    envelope.source.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;
    use crate::data::{
        obtain, BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
        KOTAK_NSE_NFO_BOOK_ID,
    };

    fn assert_no_fixture_greeks(envelope: &GreeksEnvelope) {
        assert!(envelope.data.is_none());
        let json = serde_json::to_value(envelope).unwrap();
        assert!(json["data"].is_null());
        assert!(json.get("delta").is_none());
        assert!(json.get("gamma").is_none());
        assert!(json.get("theta").is_none());
        assert!(json.get("exchange_rate").is_none());
        assert!(!greeks_may_render_number(envelope));
    }

    fn assert_dual_no_blend(envelope: &GreeksEnvelope) {
        let json = serde_json::to_value(envelope).unwrap();
        assert!(json.get("exchange_rate").is_none());
        assert!(json.get("net_pnl").is_none());
        let dumped = json.to_string();
        assert!(
            !dumped.contains("exchange_rate"),
            "DualNoBlend: greeks envelope must not blend USD+INR"
        );
    }

    /// A fully-formed venue-published envelope: data, stamp, named model, named
    /// origin. Built by hand because no book produces one yet.
    fn lit_venue_envelope() -> GreeksEnvelope {
        let identity = greeks_identity();
        GreeksEnvelope {
            identity: identity.clone(),
            status: GreeksStatus::Success,
            data: Some(serde_json::json!({ "delta": "0.5231" })),
            provenance: ProvenanceLine {
                identity,
                model: "venue_published".to_string(),
                input_at: Some("2026-08-31T09:00:00Z".to_string()),
                adapter_id: "binance_com".to_string(),
                path: "/eapi/v1/mark".to_string(),
            },
            rights: Rights::desk_display(),
            source: Some(GreeksSource::VenuePublished {
                path: "/eapi/v1/mark".into(),
                adapter: "binance_com".into(),
            }),
            ineligible: Vec::new(),
            canonical: false,
            persist_canonical: false,
        }
    }

    const OFFICIAL_EXAMPLE: &str = r#"[ { "symbol": "BTC-200730-9000-C", "markPrice": "1343.2883", "bidIV": "1.40000077", "askIV": "1.50000153", "markIV": "1.45000000", "delta": "0.55937056", "theta": "3739.82509871", "gamma": "0.00010969", "vega": "978.58874732", "highPriceLimit": "1618.241", "lowPriceLimit": "1068.3356", "riskFreeInterest": "0.1" } ]"#;

    fn official_row() -> OptionsMarkRow {
        crate::data::mark_row_for_symbol(OFFICIAL_EXAMPLE, "BTC-200730-9000-C").expect("row")
    }

    #[test]
    fn a_mark_row_lights_only_the_binance_options_book() {
        let row = official_row();
        let lit = extract_greeks_from_mark(
            Some(BINANCE_COM_OPTIONS_BOOK_ID),
            Some(&row),
            Some("2026-08-31T09:00:00Z"),
            InputHonesty::Lit,
        );
        assert_eq!(lit.status, GreeksStatus::Success);
        assert_eq!(lit.data.as_ref().expect("data")["delta"], "0.55937056");
        assert!(lit.rights.display);
        assert!(greeks_may_render_number(&lit));
    }

    #[test]
    fn extract_from_mark_does_not_leak_unrenderable_data() {
        let envelope = extract_greeks_from_mark(
            Some(BINANCE_COM_OPTIONS_BOOK_ID),
            Some(&official_row()),
            None,
            InputHonesty::Lit,
        );
        assert!(
            envelope.data.is_none(),
            "unstamped mark must not carry a number"
        );
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == GREEKS_NOT_RENDERABLE));
        assert!(!greeks_may_render_number(&envelope));
    }

    #[test]
    fn the_same_row_never_lights_any_other_book() {
        let row = official_row();
        // NFO ignores the row entirely — a USDT-options delta is not an NFO greek.
        let nfo = extract_greeks_from_mark(
            Some(KOTAK_NSE_NFO_BOOK_ID),
            Some(&row),
            Some("2026-08-31T09:00:00Z"),
            InputHonesty::Dark(HonestyStatus::Unavailable),
        );
        assert_ne!(nfo.status, GreeksStatus::Success);
        assert!(!greeks_may_render_number(&nfo));
        assert_no_fixture_greeks(&nfo);
        assert!(!serde_json::to_string(&nfo).unwrap().contains("0.55937056"));

        for (book, reason) in [
            (Some(KOTAK_NSE_BSE_CASH_BOOK_ID), CASH_IS_NOT_GREEKS),
            (Some(BINANCE_COM_SPOT_BOOK_ID), SPOT_IS_NOT_GREEKS),
            (None, BOOK_REQUIRED),
            (Some("binance-com-usdm"), BOOK_REQUIRED),
        ] {
            let envelope = extract_greeks_from_mark(
                book,
                Some(&row),
                Some("2026-08-31T09:00:00Z"),
                InputHonesty::Lit,
            );
            assert!(envelope.ineligible.iter().any(|s| s == reason), "{book:?}");
            assert_no_fixture_greeks(&envelope);
            assert!(!serde_json::to_string(&envelope)
                .unwrap()
                .contains("0.55937056"));
        }
    }

    #[test]
    fn no_row_on_the_options_book_is_still_the_mark_snapshot_hole() {
        let envelope = extract_greeks_from_mark(
            Some(BINANCE_COM_OPTIONS_BOOK_ID),
            None,
            Some("2026-08-31T09:00:00Z"),
            InputHonesty::Lit,
        );
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == MARK_SNAPSHOT_UNAVAILABLE));
        assert!(!envelope.rights.display);
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn a_misfiled_source_class_is_refused_by_both_entries() {
        // Same chokepoint, one implementation. A Binance envelope that somehow came
        // back ModelComputed is refused rather than rendered.
        let mut smuggled = lit_venue_envelope();
        smuggled.source = Some(GreeksSource::ModelComputed {
            model: "black_76".into(),
            lock: "OPTIONS-PRICING.md".into(),
        });
        let refused = refuse_misfiled_source(BINANCE_COM_OPTIONS_BOOK_ID, smuggled);
        assert_eq!(refused.ineligible, vec![SOURCE_CLASS_MISMATCH]);
        assert!(!greeks_may_render_number(&refused));
        assert_no_fixture_greeks(&refused);
    }

    #[test]
    fn a_dark_fence_carries_no_display_right() {
        // Every hole is research-fetch-only: nothing about a fence may render.
        for reason in [BOOK_REQUIRED, CASH_IS_NOT_GREEKS, SPOT_IS_NOT_GREEKS] {
            let envelope = dark_fence(reason);
            assert_eq!(envelope.rights, Rights::research_fetch_only());
            assert!(!envelope.rights.display, "{reason}");
        }
    }

    #[test]
    fn the_gate_also_reads_the_display_right_and_the_status() {
        // A number with no display grant is licensed data Station may fetch but not
        // show, and a non-Success status is not a live number at all.
        let mut undisplayable = lit_venue_envelope();
        undisplayable.rights = Rights::research_fetch_only();
        assert!(
            !greeks_may_render_number(&undisplayable),
            "no display right"
        );

        let mut not_success = lit_venue_envelope();
        not_success.status = GreeksStatus::Empty;
        assert!(!greeks_may_render_number(&not_success), "not success");

        assert!(greeks_may_render_number(&lit_venue_envelope()));
    }

    #[test]
    fn success_is_on_the_greeks_wire_but_never_on_honesty_status() {
        // glance.rs made the same split: Success is live data, so it cannot be a
        // HonestyStatus variant, but the wire still has to be able to say it.
        let wire: Vec<String> = [
            GreeksStatus::Success,
            GreeksStatus::Unavailable,
            GreeksStatus::Empty,
            GreeksStatus::Unusable,
            GreeksStatus::InheritedDark,
        ]
        .iter()
        .map(|s| {
            serde_json::to_value(s)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
        assert_eq!(
            wire,
            vec![
                "success",
                "unavailable",
                "empty",
                "unusable",
                "inherited_dark"
            ]
        );

        // HonestyStatus still has exactly four, none of them success.
        assert_eq!(HonestyStatus::ALL.len(), 4);
        for status in HonestyStatus::ALL {
            assert_ne!(GreeksStatus::from(status), GreeksStatus::Success);
        }

        // Only InheritedDark forbids a payload, same as HonestyStatus.
        assert!(GreeksStatus::InheritedDark.requires_data_none());
        assert!(!GreeksStatus::Success.requires_data_none());
    }

    #[test]
    fn a_lit_greek_is_success_not_empty() {
        // "Empty" means live-and-legitimately-zero. A published delta is neither.
        let envelope = lit_venue_envelope();
        assert_eq!(envelope.status, GreeksStatus::Success);
        assert_ne!(envelope.status, GreeksStatus::Empty);
        assert!(greeks_may_render_number(&envelope));
    }

    #[test]
    fn the_gate_is_not_dead_code() {
        // The previous gate matched HonestyStatus with every arm false, so it could
        // never return true no matter how real the data was. This is the proof it
        // now can.
        assert!(greeks_may_render_number(&lit_venue_envelope()));
    }

    #[test]
    fn every_missing_piece_refuses_the_number() {
        let mut no_data = lit_venue_envelope();
        no_data.data = None;
        assert!(!greeks_may_render_number(&no_data), "no data");

        let mut unstamped = lit_venue_envelope();
        unstamped.provenance.input_at = None;
        assert!(!greeks_may_render_number(&unstamped), "no input_at");

        let mut raw_model = lit_venue_envelope();
        raw_model.provenance.model = "raw".to_string();
        assert!(
            !greeks_may_render_number(&raw_model),
            "raw is the hole marker"
        );

        let mut blank_model = lit_venue_envelope();
        blank_model.provenance.model = "   ".to_string();
        assert!(!greeks_may_render_number(&blank_model), "blank model");

        let mut unsourced = lit_venue_envelope();
        unsourced.source = None;
        assert!(!greeks_may_render_number(&unsourced), "unattributed number");

        let mut wrong_identity = lit_venue_envelope();
        wrong_identity.identity = Identity::new(
            Family::Derived,
            CapabilityId::new("synthetic_future").expect("id"),
            Physics::BoundedSnapshot,
        );
        assert!(!greeks_may_render_number(&wrong_identity), "not greeks");
    }

    #[test]
    fn inherited_dark_refuses_the_number() {
        // Carried over from provenance.rs: a stamped input is not enough on its own.
        let mut envelope = lit_venue_envelope();
        envelope.status = GreeksStatus::InheritedDark;
        envelope.data = None;
        envelope.source = None;
        envelope.provenance.model = "raw".to_string();
        assert!(!greeks_may_render_number(&envelope));
    }

    #[test]
    fn each_book_may_produce_only_its_own_source_class() {
        let venue = GreeksSource::VenuePublished {
            path: "/eapi/v1/mark".into(),
            adapter: "binance_com".into(),
        };
        let model = GreeksSource::ModelComputed {
            model: "black_76".into(),
            lock: "OPTIONS-PRICING.md".into(),
        };

        // The two legal pairings.
        assert!(source_class_allowed(BINANCE_COM_OPTIONS_BOOK_ID, &venue));
        assert!(source_class_allowed(KOTAK_NSE_NFO_BOOK_ID, &model));

        // Kotak has no greeks field, so NFO can never be venue-published.
        assert!(!source_class_allowed(KOTAK_NSE_NFO_BOOK_ID, &venue));
        // Station does not price USDT options, so eapi can never be model-computed.
        assert!(!source_class_allowed(BINANCE_COM_OPTIONS_BOOK_ID, &model));

        // And no other book may carry either class.
        for book in [KOTAK_NSE_BSE_CASH_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, ""] {
            assert!(!source_class_allowed(book, &venue), "{book} venue");
            assert!(!source_class_allowed(book, &model), "{book} model");
        }
    }

    #[test]
    fn holes_carry_no_source_and_omit_the_key() {
        for book in [
            Some(KOTAK_NSE_NFO_BOOK_ID),
            Some(BINANCE_COM_OPTIONS_BOOK_ID),
            Some(KOTAK_NSE_BSE_CASH_BOOK_ID),
            Some(BINANCE_COM_SPOT_BOOK_ID),
            None,
        ] {
            let envelope = extract_greeks(book, InputHonesty::Lit);
            assert!(
                envelope.source.is_none(),
                "{book:?} is dark and must name no source"
            );
            let json = serde_json::to_value(&envelope).unwrap();
            assert!(
                json.get("source").is_none(),
                "an absent source must not serialize as null: {json}"
            );
        }
    }

    #[test]
    fn greeks_identity_is_derived_bounded_snapshot() {
        let envelope = extract_greeks(
            Some(KOTAK_NSE_NFO_BOOK_ID),
            InputHonesty::Dark(HonestyStatus::Unavailable),
        );
        assert_eq!(envelope.identity.family, Family::Derived);
        assert_eq!(envelope.identity.capability_id.as_str(), "greeks");
        assert_eq!(envelope.identity.physics, Physics::BoundedSnapshot);
        assert_eq!(envelope.provenance.model, "raw");
        assert!(envelope.provenance.input_at.is_none());
    }

    #[test]
    fn missing_book_is_unavailable_fence() {
        let envelope = extract_greeks(None, InputHonesty::Lit);
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert!(envelope.ineligible.iter().any(|s| s == BOOK_REQUIRED));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn cash_book_is_not_greeks() {
        let envelope = extract_greeks(Some(KOTAK_NSE_BSE_CASH_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert!(envelope.ineligible.iter().any(|s| s == CASH_IS_NOT_GREEKS));
        assert_no_fixture_greeks(&envelope);
        assert_dual_no_blend(&envelope);
    }

    #[test]
    fn spot_book_is_not_greeks() {
        let envelope = extract_greeks(Some(BINANCE_COM_SPOT_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert!(envelope.ineligible.iter().any(|s| s == SPOT_IS_NOT_GREEKS));
        assert_no_fixture_greeks(&envelope);
        assert_dual_no_blend(&envelope);
    }

    #[test]
    fn binance_options_never_fills_nfo_delta() {
        let envelope = extract_greeks(Some(BINANCE_COM_OPTIONS_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == MARK_SNAPSHOT_UNAVAILABLE));
        assert!(!envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
        assert_dual_no_blend(&envelope);
    }

    /// Only the Binance options book ships `optiongreeks`, and even there an
    /// unplanted snapshot is Unavailable — never Unsupported, never an empty
    /// success. NFO stays Unsupported: it has no venue-published greeks to copy
    /// and no named pricing model to compute one.
    #[test]
    fn optiongreeks_ships_on_the_options_book_only_and_never_lit_by_default() {
        use crate::data::source_manifest::binance_com_options_manifest;
        use crate::data::{kotak_neo_nfo_manifest, ObtainStatus};
        assert_eq!(
            obtain(&kotak_neo_nfo_manifest(), "optiongreeks").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&binance_com_options_manifest(), "optiongreeks").status,
            ObtainStatus::Unavailable
        );
        assert_ne!(
            obtain(&binance_com_options_manifest(), "optiongreeks").status,
            ObtainStatus::Unsupported
        );
        // Spot is not a greeks book either.
        assert_eq!(
            obtain(
                &crate::data::source_manifest::binance_com_s1_manifest(),
                "optiongreeks"
            )
            .status,
            ObtainStatus::Unsupported
        );
        // The multi-leg noun is still nobody's.
        assert_eq!(
            obtain(&binance_com_options_manifest(), "multioptiongreeks").status,
            ObtainStatus::Unsupported
        );
    }

    /// Slice 1 research (2026-08-31) closed exercise/settlement, not trader greeks.
    /// Fail-closed: neither book shows a number.
    #[test]
    fn no_book_shows_a_number_without_its_own_input() {
        let nfo = extract_greeks(Some(KOTAK_NSE_NFO_BOOK_ID), InputHonesty::Lit);
        assert!(nfo.data.is_none());
        assert!(!greeks_may_render_number(&nfo));
        let eapi = extract_greeks(Some(BINANCE_COM_OPTIONS_BOOK_ID), InputHonesty::Lit);
        assert_eq!(eapi.status, GreeksStatus::Unavailable);
        assert!(eapi
            .ineligible
            .iter()
            .any(|s| s == MARK_SNAPSHOT_UNAVAILABLE));
        assert!(eapi.data.is_none());
        assert_dual_no_blend(&eapi);
        assert_dual_no_blend(&nfo);
    }
}
