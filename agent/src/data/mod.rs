//! Canonical Station Data runtime (this agent). Console `crates/agent` is a CI mirror only.

mod apply;
mod binance_depth;
mod binance_klines;
mod binance_options_chain;
mod binance_options_oi;
mod binance_options_public;
mod binance_public;
mod binance_spot_ticker;
mod book_identity;
mod connection;
mod contracts;
mod depthbook;
mod descriptor;
mod extract;
mod force_order;
mod glance;
mod greeks;
mod greeks_binance_options;
mod greeks_nfo;
mod history;
mod history_store;
mod historybook;
mod honesty;
mod host_policy;
mod identity;
mod inherit;
mod instrument_master_status;
mod klines_pager;
mod kotak_depth;
mod kotak_quotes;
mod margin_estimate;
mod market_bind;
mod matrix;
mod operations;
mod provenance;
mod quote_subscription;
mod registry;
mod resample;
mod resolve;
mod rights;
mod router;
mod source_manifest;
mod tick;
mod tickbook;

pub use apply::{apply_quote, ApplyError};
#[allow(unused_imports)] // host-facing page walk; live desk fetch is owned elsewhere
pub use binance_depth::{
    depth_snapshot_from_binance_json, ensure_binance_com_depth_stream,
    spawn_binance_com_depth_loop, validate_depth_delta, DepthDelta, DepthDeltaDecision,
    DepthSyncPhase, DEPTH_COM_HOST, DEPTH_PATH,
};
#[allow(unused_imports)] // host-facing page walk; live desk fetch is owned elsewhere
pub use binance_klines::{
    candles_from_klines_json, klines_time_query, klines_url, HistoryCandle, KlineRequestRefuse,
    KLINE_COM_HOST, KLINE_LIMIT_MAX, KLINE_PATH,
};
pub use binance_klines::{
    series_from_klines_json, validate_kline_request, HistorySeries, DEFAULT_HISTORY_INTERVAL,
    KLINE_LIMIT_DEFAULT,
};
pub use binance_options_chain::{
    chain_rows_for_contract, expiration_from_dated_contract,
    option_symbols_from_exchange_info_json, underlying_asset_from_dated_contract, OptionsSymbolRow,
};
pub use binance_options_oi::{oi_rows_from_json, OptionsOiRow};
pub use binance_options_public::{
    ensure_binance_com_options_quote, is_dated_option_contract, normalize_options_instrument,
    quote_tick_from_options_ticker_json,
};
pub use binance_public::{
    ensure_binance_com_trade_stream, normalize_quote_instrument, quote_tick_from_binance_json,
    spawn_binance_com_trade_loop,
};
pub use binance_spot_ticker::await_binance_spot_ticker_price;
pub use book_identity::{book_accepts_symbol, query_symbol};
pub use connection::BrokerConnectionRuntime;
pub use contracts::{extract_contracts, extract_contracts_from_rows, ContractRow};
pub use depthbook::DepthBook;
pub use descriptor::{
    binance_com_quote_descriptor, fixture_quote_descriptor, kotak_neo_quote_descriptor,
    BINANCE_COM_ADAPTER_ID, BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID,
    KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
pub use extract::{
    extract_quote, extract_quote_for, extract_quote_for_book, QuoteEnvelope, QuoteStatus,
};
pub use force_order::{
    extract_force_order, reject_lossy_as_complete, ForceOrderEnvelope, LossyStatus,
    LOSSY_CANNOT_CLAIM_COMPLETE,
};
pub use glance::{
    chain_input_honesty, extract_chain, extract_chain_from, extract_open_interest,
    extract_open_interest_from, ChainRow, GlanceEnvelope, GlanceStatus,
};
pub use greeks::extract_greeks;
pub use history::{
    apply_history_series, extract_history, extract_licensed_history, history_obtain_data,
    HistoryEnvelope, HistoryStatus,
};
pub use history_store::HistoryStore;
pub use historybook::HistoryBook;
pub use honesty::{HonestyStatus, InputHonesty};
pub use host_policy::{
    authorize_book_call, authorize_book_fence, authorize_host_call, authorize_inferred_call,
    infer_capability, is_kotak_cash_scrip_csv_path, is_kotak_fo_scrip_csv_path,
    is_kotak_nse_fo_scrip_csv_path, AuthMode, HostRefuse, R0_ALLOWED_HOSTS,
};
pub use identity::Physics;
pub use inherit::{capital_may_light, inherit};
pub use instrument_master_status::{
    binance_exchange_info_cache_path, json_array_first_object_keys, json_field_object_keys,
    json_first_nested_object_keys, json_object_keys, kotak_csv_cache_path, truncate_body,
    write_raw_cache, InstrumentMasterErrorClass, InstrumentMasterFetchError, InstrumentMasterPhase,
    InstrumentMasterStatus,
};
#[allow(unused_imports)] // host-facing page walk; live desk fetch is owned elsewhere
pub use klines_pager::{
    fetch_klines_page, next_start_time_ms, page_klines, walk_is_complete, walk_klines_page,
    WalkedPage,
};
pub use kotak_depth::{
    depth_obtain_data, depth_snapshot_from_kotak_json, depth_snapshots_from_kotak_json,
    extract_depth, DepthEnvelope, DepthStatus,
};
pub use kotak_quotes::{
    is_cash_segment, is_nfo_segment, kotak_quote_book_id, parse_nfo_instrument_id,
    quote_tick_from_kotak_json, quote_tick_from_kotak_json_for_book, quote_ticks_from_kotak_json,
    quote_ticks_from_kotak_json_for_book, quotes_neosymbol_path, QUOTE_TYPE_ALL, QUOTE_TYPE_DEPTH,
};
pub use margin_estimate::extract_margin_estimate;
pub use market_bind::MarketBind;
pub use provenance::ProvenanceLine;
pub use quote_subscription::{quote_subscription_for, QuoteSubscription};
pub use registry::Registry;
pub use resample::extract_resample;
pub use resolve::{resolve_among, resolve_desk_instrument};
pub use source_manifest::{
    describe, first_party_s0_manifests, kotak_neo_nfo_manifest, kotak_neo_s1k_manifest,
    load_first_party_manifests, manifest_for_book_id, manifest_for_slug, obtain, shared_budget,
    shipping_book_id_for_slug, ObtainEnvelope, ObtainStatus, SourceManifest,
};
pub use tick::{QuoteTick, SessionOhlc, Transport};
pub use tickbook::TickBook;
