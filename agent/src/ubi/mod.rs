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
mod wasm_adapter;

pub use allowlist::{host_allowed, ALLOWED_BROKER_HOSTS};
pub use catalog::{
    calc_profile, catalog_v1, compliance_profile, descriptor_for_slug, AdapterOrigin, AuthScheme,
    BrokerAvailability, BrokerDescriptor, CalcProfile, ComplianceProfile,
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
    run_describe, run_fetch_fills, run_obtain, BrokerHttpFixture, BrokerHttpMode, FillCursor,
    FillEvent, HostCredentialBlob, UbiHostConfig, UbiHostError, UbiHostState,
    FORBIDDEN_COMPONENT_HEADERS,
};
pub use http::{
    attach_kotak_file_paths_session, classify_response, effective_host, kotak_base_host,
    prepare_kotak_catalog_get, prepare_kotak_file_paths_get, prepare_request,
    prepare_unsigned_request, redact_response_headers, BrokerHttpTransport, PreparedHttpRequest,
    RecordingTransport, ReqwestBrokerHttpTransport, TransportResponse, RESPONSE_HEADER_ALLOWLIST,
};
pub use kotak_session::{mint_totp_session, KotakMintRequest, ReqwestKotakSessionHttp};
pub use wasm_adapter::{fill_event_to_broker_fill, WasmBrokerAdapter};
