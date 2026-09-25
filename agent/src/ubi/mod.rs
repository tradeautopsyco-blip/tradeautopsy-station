//! Universal Broker Interface host (ADR 0001).
//!
//! Phase 1: Wasmtime sandbox + `broker_http_call` stub + FillEvent contract surface.
//! Phase 2: catalog descriptors, tagged credential blobs, host credential vault.
//! See `docs/adr/0001-uniform-wasm-sandboxed-broker-adapters.md`.

mod allowlist;
mod catalog;
mod components;
mod credential_vault;
mod credentials;
mod desk;
mod host;
mod http;
mod kotak_session;
pub mod dhan_session;
pub mod groww_session;
pub mod okx_session;
pub mod fyers_session;
pub mod upstox_session;
mod zerodha_session;
pub mod bybit_session;
pub mod coinbase_session;
pub mod kraken_session;
mod wasm_adapter;

pub use allowlist::{
    fyers_path_allowed, fyers_path_refused, fyers_sym_details_stem, groww_path_allowed,
    groww_path_refused, host_allowed, is_fyers_cash_sym_path, is_fyers_nfo_sym_path,
    is_upstox_cash_bod_path, is_upstox_complete_bod_path, is_upstox_nfo_bod_path,
    okx_path_allowed, okx_path_refused, upstox_exchange_bod_stem, upstox_path_allowed,
    upstox_path_refused, zerodha_kite_path_allowed, zerodha_kite_path_refused, ALLOWED_BROKER_HOSTS,
    FYERS_API_HOST, FYERS_BOOK_ID, FYERS_NFO_BOOK_ID, FYERS_PUBLIC_HOST, GROWW_API_HOST,
    GROWW_API_VERSION_HEADER, GROWW_ASSETS_HOST, GROWW_BOOK_ID, GROWW_NFO_BOOK_ID, KITE_API_HOST,
    OKX_API_HOST,
    OKX_COM_SPOT_BOOK_ID, UPSTOX_API_HOST, UPSTOX_ASSETS_HOST, UPSTOX_BOOK_ID, UPSTOX_HFT_HOST,
    UPSTOX_NFO_BOOK_ID, ZERODHA_KITE_BOOK_ID, ZERODHA_KITE_NFO_BOOK_ID,
};
pub use catalog::{
    book_id_for_slug_calc_profile, calc_profile, catalog_books, catalog_v1, compliance_profile,
    descriptor_for_book_id, descriptor_for_slug,
    AdapterOrigin, AssetClass, AuthScheme, BrokerAvailability, BrokerDescriptor, CalcProfile,
    ComplianceProfile, InstrumentClass,
};
pub use components::{
    component_candidate_paths, component_crate_dir, component_file_name, component_path_for_slug,
    COMPONENT_DIR_ENV,
};
pub use credential_vault::{
    BrokerCredentialVault, KeyringBrokerCredentialVault, MemoryBrokerCredentialVault,
};
pub use credentials::{
    decode_credential_blob, keychain_service_for, CredentialBlob,
    BROKER_CREDENTIAL_KEYCHAIN_SERVICE, KOTAK_SESSION_KEYCHAIN_SERVICE,
};
pub use desk::desk_profile_for_slug;
pub use host::{
    run_describe, run_fetch_fills, run_obtain, stamp_fill_identity, BrokerHttpFixture,
    BrokerHttpMode, FillCursor, FillEvent, HostCredentialBlob, UbiHostConfig, UbiHostError,
    UbiHostState, WitAssetClass, WitInstrumentClass, FORBIDDEN_COMPONENT_HEADERS,
};
pub use http::{
    attach_kotak_file_paths_session, classify_response, effective_host, kotak_base_host,
    prepare_kotak_catalog_get, prepare_kotak_file_paths_get, prepare_kotak_market_data_get,
    prepare_request, prepare_unsigned_request, redact_response_headers, BrokerHttpTransport,
    PreparedHttpRequest, RecordingTransport, ReqwestBrokerHttpTransport, TransportResponse,
    RESPONSE_HEADER_ALLOWLIST,
};
pub use dhan_session::{
    begin_connect as dhan_begin_connect, dhan_callback_base_url, dhan_path_allowed,
    exchange_token_id, take_pending_connect as take_dhan_pending_connect, truncate_state,
    MintedDhanSession, PendingDhanConnect, ReqwestDhanSessionHttp, DHAN_API_HOST, DHAN_BOOK_ID,
    DHAN_NFO_BOOK_ID,
};
pub use coinbase_session::{
    build_rest_jwt, coinbase_bearer_authorization, coinbase_host_refused, coinbase_path_allowed,
    coinbase_rest_uri, prepare_coinbase_authenticated_request, COINBASE_ACCOUNTS_PATH,
    COINBASE_API_HOST, COINBASE_BOOK_ID, COINBASE_FILLS_PATH, COINBASE_PRODUCTS_PATH,
    COINBASE_REST_PREFIX, JWT_EXPIRY_SECS, JWT_ISSUER,
};
pub use groww_session::{
    connect_mint, groww_approval_checksum, ReqwestGrowwSessionHttp, GrowwMintError,
    MintedGrowwSession,
};
pub use okx_session::{
    connect_store as okx_connect_store, okx_sign_request, validate_credentials as okx_validate_credentials,
    OkxSessionHttp, OKX_REFUSED_HOSTS,
};
pub use bybit_session::{BYBIT_API_HOST, BYBIT_BOOK_ID, BYBIT_RECV_WINDOW};
pub use kraken_session::{KRAKEN_API_HOST, KRAKEN_BOOK_ID};
pub use kotak_session::{mint_totp_session, KotakMintRequest, ReqwestKotakSessionHttp};
pub use fyers_session::{
    begin_connect as fyers_begin_connect, exchange_auth_code as fyers_exchange_auth_code,
    fyers_app_id_hash, fyers_authorization_header_value, fyers_authorize_url,
    fyers_callback_base_url, take_pending_connect as take_fyers_pending_connect, MintedFyersSession,
    ReqwestFyersSessionHttp, FyersExchangeError,
};
pub use upstox_session::{
    begin_connect as upstox_begin_connect, exchange_auth_code, take_pending_connect as take_upstox_pending_connect,
    upstox_authorize_url, upstox_bearer_authorization_header_value, upstox_callback_base_url,
    MintedUpstoxSession, ReqwestUpstoxSessionHttp, UpstoxExchangeError,
};
pub use zerodha_session::{
    begin_connect, exchange_request_token, kite_authorization_header_value, kite_login_checksum,
    take_pending_connect, take_single_active_pending_connect, zerodha_callback_base_url,
    KiteExchangeError, MintedKiteSession,
    ReqwestKiteSessionHttp,
};
pub use wasm_adapter::{fill_event_to_broker_fill, WasmBrokerAdapter};
