//! Universal Broker Interface host (ADR 0001).
//!
//! Phase 1: Wasmtime sandbox + `broker_http_call` stub + FillEvent contract surface.
//! Phase 2: catalog descriptors, tagged credential blobs, host credential vault.
//! See `docs/adr/0001-uniform-wasm-sandboxed-broker-adapters.md`.

mod allowlist;
mod catalog;
mod components;
mod credential_vault;
#[cfg(target_os = "macos")]
mod macos_keychain_acl;
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
    fyers_path_allowed, groww_path_allowed, host_allowed, is_fyers_cash_sym_path, is_fyers_nfo_sym_path,
    is_upstox_cash_bod_path, is_upstox_complete_bod_path, is_upstox_nfo_bod_path,
    okx_path_allowed, upstox_exchange_bod_stem, upstox_path_allowed,
    upstox_path_refused, zerodha_kite_path_allowed, ALLOWED_BROKER_HOSTS,
    FYERS_API_HOST, FYERS_BOOK_ID, FYERS_NFO_BOOK_ID, FYERS_PUBLIC_HOST, GROWW_API_HOST,
    GROWW_API_VERSION_HEADER, GROWW_ASSETS_HOST, GROWW_BOOK_ID, GROWW_NFO_BOOK_ID, KITE_API_HOST,
    OKX_API_HOST,
    OKX_COM_SPOT_BOOK_ID, UPSTOX_BOOK_ID,
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
    dhan_path_allowed, DHAN_API_HOST, DHAN_BOOK_ID,
    DHAN_NFO_BOOK_ID,
};
pub use okx_session::OKX_REFUSED_HOSTS;
pub use bybit_session::{BYBIT_API_HOST, BYBIT_BOOK_ID, BYBIT_RECV_WINDOW};
pub use kraken_session::{KRAKEN_API_HOST, KRAKEN_BOOK_ID};
pub use kotak_session::{mint_totp_session, KotakMintRequest, ReqwestKotakSessionHttp};
pub use zerodha_session::{
    begin_connect, exchange_request_token,
    take_pending_connect, take_single_active_pending_connect, zerodha_callback_base_url,
    ReqwestKiteSessionHttp,
};
pub use wasm_adapter::{fill_event_to_broker_fill, WasmBrokerAdapter};
