//! Canonical Station Data runtime (this agent). Console `crates/agent` is a CI mirror only.

mod account_book;
mod account_capability;
mod account_split;
mod amfi;
mod apply;
mod binance_coinm_depth;
mod binance_coinm_exchange_info;
mod binance_coinm_klines;
mod binance_coinm_private;
mod binance_coinm_ticker;
mod binance_depth;
mod binance_klines;
mod binance_options_chain;
mod binance_options_depth;
mod binance_options_index;
mod binance_options_klines;
mod binance_options_mark;
mod binance_options_oi;
mod binance_options_private;
mod binance_options_public;
mod binance_options_ticker;
mod binance_public;
mod binance_spot_private;
mod binance_spot_ticker;
mod binance_usdm_depth;
mod binance_usdm_exchange_info;
mod binance_usdm_klines;
mod binance_usdm_private;
mod binance_usdm_ticker;
mod book_identity;
mod candle_builder;
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
mod instrument_search;
mod klines_pager;
mod kotak_depth;
pub(crate) mod kotak_historical;
mod kotak_private;
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
mod source_route;
mod tick;
mod tickbook;
mod vendor_health;
mod vendor_registry;

pub use account_book::AccountBook;
pub use account_capability::{
    account_capability_wire, account_capability_wire_key, ACCOUNT_CAPABILITY_OPS,
};
pub use account_split::{
    fills_provenance_path, merge_poll_book_id, split_fills_by_book, stamp_nfo_fills,
};
pub use amfi::{obtain_amfi_nav, AMFI_ADAPTER_ID, AMFI_NAV_BOOK_ID, AMFI_NAV_HOST};
pub use apply::{apply_quote, ApplyError};
pub use binance_coinm_exchange_info::{
    coinm_step_size_for, coinm_tick_size_for, ensure_coinm_exchange_info, CoinmExchangeInfoCache,
};
pub use binance_coinm_depth::{
    depth_snapshot_from_dapi_json, ensure_coinm_depth, coinm_depth_query, COINM_DEPTH_HOST,
    COINM_DEPTH_PATH,
};
pub use binance_coinm_klines::{
    ensure_coinm_klines, series_from_dapi_klines_json, validate_coinm_kline_request,
    COINM_KLINES_HOST, COINM_KLINES_PATH, COINM_KLINE_INTERVALS, DEFAULT_COINM_HISTORY_INTERVAL,
};
pub use binance_coinm_private::{
    ensure_coinm_balance, ensure_coinm_force_orders, ensure_coinm_positions,
};
pub use binance_coinm_ticker::{
    await_binance_coinm_ticker, normalize_coinm_instrument, quote_tick_from_coinm_ticker_json,
};
pub use binance_depth::{
    await_bound_com_depth_row, ensure_binance_com_depth_stream, spawn_binance_com_depth_loop,
};
pub use binance_klines::{
    series_from_klines_json, validate_kline_request, HistoryCandle, HistorySeries,
    DEFAULT_HISTORY_INTERVAL, KLINE_LIMIT_DEFAULT,
};
pub use binance_options_chain::{
    chain_rows_for_contract, expiration_from_dated_contract,
    option_symbols_from_exchange_info_json, underlying_asset_from_dated_contract, OptionsSymbolRow,
};
pub use binance_options_depth::{
    depth_snapshot_from_eapi_json, options_depth_query, OPTIONS_DEPTH_HOST, OPTIONS_DEPTH_PATH,
};
pub use binance_options_index::{
    cached_index_hit, index_price_from_json_for_underlying, index_underlying_for_contract,
    options_index_query, CachedIndex, OPTIONS_INDEX_HOST, OPTIONS_INDEX_PATH,
};
pub use binance_options_klines::{
    options_klines_query, series_from_eapi_klines_json, validate_options_kline_request,
    DEFAULT_OPTIONS_HISTORY_INTERVAL, OPTIONS_KLINES_HOST, OPTIONS_KLINES_PATH,
    OPTIONS_KLINE_LIMIT_DEFAULT,
};
pub use binance_options_mark::{
    mark_row_for_symbol, mark_rows_from_json, options_mark_query, CachedMark, OPTIONS_MARK_PATH,
};
pub use binance_options_oi::{oi_rows_from_json, OptionsOiRow};
pub use binance_options_private::{
    ensure_options_margin_account, ensure_options_positions, ensure_options_user_trades,
};
pub use binance_options_public::{
    ensure_binance_com_options_quote, is_dated_option_contract, normalize_options_instrument,
    quote_tick_from_options_ticker_json,
};
#[cfg(test)]
pub use binance_options_public::{options_ticker_query, OPTIONS_EAPI_HOST, OPTIONS_TICKER_PATH};
pub use binance_options_ticker::await_binance_options_ticker;
pub use binance_public::{
    ensure_binance_com_trade_stream, normalize_quote_instrument, quote_tick_from_binance_json,
    spawn_binance_com_trade_loop,
};
pub use binance_spot_private::{ensure_spot_account, ensure_spot_open_orders};
pub use binance_spot_ticker::await_binance_spot_ticker_price;
pub use binance_usdm_exchange_info::{
    ensure_usdm_exchange_info, step_size_for, tick_size_for, UsdmExchangeInfoCache,
    UsdmSymbolFilters,
};
pub use binance_usdm_depth::{
    depth_snapshot_from_fapi_json, ensure_usdm_depth, usdm_depth_query, USDM_DEPTH_HOST,
    USDM_DEPTH_PATH,
};
pub use binance_usdm_klines::{
    ensure_usdm_klines, series_from_fapi_klines_json, validate_usdm_kline_request,
    DEFAULT_USDM_HISTORY_INTERVAL, USDM_KLINES_HOST, USDM_KLINES_PATH, USDM_KLINE_INTERVALS,
};
pub use binance_usdm_private::{
    ensure_usdm_balance, ensure_usdm_force_orders, ensure_usdm_positions,
    ensure_usdm_realized_income,
};
pub use binance_usdm_ticker::{
    await_binance_usdm_ticker, normalize_usdm_instrument, quote_tick_from_usdm_ticker_json,
};
pub use book_identity::{book_accepts_symbol, query_symbol};
pub use candle_builder::{
    apply_history_series_and_seed, overlay_forming, overlay_json_candles, seed_builders_from_book,
    CandleBuilder, CandleBuilders,
};
pub use connection::BrokerConnectionRuntime;
pub use contracts::{extract_contracts, extract_contracts_from_rows, ContractRow};
pub use depthbook::DepthBook;
pub use descriptor::{
    binance_com_quote_descriptor, fixture_quote_descriptor, kotak_neo_quote_descriptor,
    BINANCE_COM_ADAPTER_ID, BINANCE_COM_COINM_BOOK_ID, BINANCE_COM_OPTIONS_BOOK_ID,
    BINANCE_COM_SPOT_BOOK_ID, BINANCE_COM_USDM_BOOK_ID, KOTAK_NEO_ADAPTER_ID,
    KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID, ZERODHA_KITE_ADAPTER_ID,
    ZERODHA_NSE_BSE_CASH_BOOK_ID, ZERODHA_NSE_NFO_BOOK_ID,
};
pub use extract::{
    extract_quote, extract_quote_for, extract_quote_for_book, refused_quote_binding, QuoteEnvelope,
    QuoteStatus,
};
pub use force_order::{
    observation_from_rest, ForceOrderBook, ForceOrderEnvelope, LossyStatus,
    LOSSY_CANNOT_CLAIM_COMPLETE,
};
pub use glance::{
    chain_input_honesty, extract_chain, extract_chain_from, extract_index, extract_open_interest,
    extract_open_interest_for_book, extract_open_interest_from, ChainRow, GlanceEnvelope,
    GlanceStatus,
};
pub use greeks::{extract_greeks, extract_greeks_from_mark, GreeksEnvelope, GreeksStatus};
pub use history::{
    apply_history_series, extract_coinm_history, extract_gap_vendor_history, extract_history,
    extract_licensed_history, extract_options_history, extract_usdm_history,
    futures_history_obtain_data, gap_history_obtain_data, history_obtain_data, HistoryEnvelope,
    HistoryStatus,
};
pub use historybook::HistoryBook;
pub use honesty::{HonestyStatus, InputHonesty};
pub use host_policy::{
    authorize_book_call, authorize_book_fence, authorize_host_call, authorize_inferred_call,
    infer_capability, is_kotak_cash_scrip_csv_path, is_kotak_fo_scrip_csv_path,
    is_kotak_nse_fo_scrip_csv_path, is_mutation, AuthMode, HostRefuse, R0_ALLOWED_HOSTS,
};
pub use identity::Physics;
pub use inherit::{capital_may_light, inherit};
pub use instrument_master_status::{
    binance_exchange_info_cache_path, json_array_first_object_keys, json_field_object_keys,
    json_first_nested_object_keys, json_object_keys, kotak_csv_cache_path, truncate_body,
    write_raw_cache, InstrumentMasterErrorClass, InstrumentMasterFetchError, InstrumentMasterPhase,
    InstrumentMasterStatus,
};
pub use instrument_search::{search_identity, search_rows_for_book};
pub use kotak_depth::{
    depth_obtain_data, depth_snapshot_from_kotak_json, depth_snapshots_from_kotak_json,
    extract_depth, extract_depth_on_book, DepthEnvelope, DepthStatus,
};
pub use kotak_private::{
    ensure_kotak_funds, ensure_kotak_holdings, ensure_kotak_orders, ensure_kotak_positions,
    kotak_cash_limits_jdata_body, kotak_jdata_form_body, kotak_nfo_limits_jdata_body,
};
pub use kotak_quotes::{
    is_nfo_segment, kotak_quote_book_id, nfo_oi_session_from_kotak_json, nfo_open_interest_from_kotak_json,
    parse_nfo_instrument_id, quote_tick_from_kotak_json, quote_tick_from_kotak_json_for_book,
    quote_ticks_from_kotak_json_for_book, quotes_neosymbol_path,
    tick_cash_builders_from_kotak_json, NfoOiSessionSlice, NfoOpenInterest, QUOTE_TYPE_ALL,
    QUOTE_TYPE_DEPTH, QUOTE_TYPE_OI,
};
pub use margin_estimate::extract_margin_estimate;
pub use market_bind::MarketBind;
pub use provenance::ProvenanceLine;
pub use quote_subscription::{quote_subscription_for, QuoteSubscription};
pub use registry::Registry;
pub use resample::extract_resample;
pub use resolve::{resolve_among, resolve_desk_instrument};
pub use router::RouteOutcome;
pub use source_manifest::{
    describe, kotak_neo_nfo_manifest, kotak_neo_s1k_manifest, load_first_party_manifests,
    manifest_for_book_id, obtain, shared_budget, shipping_book_id_for_slug, ObtainEnvelope,
    ObtainStatus, SourceManifest,
};
#[cfg(test)]
pub use source_manifest::{first_party_s0_manifests, manifest_for_slug};
pub use source_route::{
    apply_kotak_source_route, decide_kotak_route, secret_looks_like_url, should_source_route,
    GapVendorConfig, LICENSED_HISTORY_ADAPTER_ID, LICENSED_HISTORY_VAULT_CONNECTION_ID,
};
pub use tick::{QuoteTick, Transport};
pub use tickbook::TickBook;
pub use vendor_health::vendor_health_rows;
pub use vendor_registry::{binding_for, clamp_budget};
