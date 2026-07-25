//! Universal Broker Interface host (ADR 0001).
//!
//! Phase 1: Wasmtime sandbox + `broker_http_call` stub + FillEvent contract surface.
//! Phase 2: catalog descriptors, tagged credential blobs, host credential vault.
//! See `docs/adr/0001-uniform-wasm-sandboxed-broker-adapters.md`.

mod allowlist;
mod catalog;
mod credential_vault;
mod credentials;
mod host;

pub use allowlist::{host_allowed, ALLOWED_BROKER_HOSTS};
pub use catalog::{
    calc_profile, catalog_v1, compliance_profile, descriptor_for_slug, AdapterOrigin, AuthScheme,
    BrokerAvailability, BrokerDescriptor, CalcProfile, ComplianceProfile,
};
pub use credential_vault::{
    BrokerCredentialVault, KeyringBrokerCredentialVault, MemoryBrokerCredentialVault,
};
pub use credentials::{
    decode_credential_blob, CredentialBlob, BROKER_CREDENTIAL_KEYCHAIN_SERVICE,
};
pub use host::{
    run_fetch_fills, BrokerHttpFixture, FillCursor, FillEvent, HostCredentialBlob, UbiHostConfig,
    UbiHostError, UbiHostState, FORBIDDEN_COMPONENT_HEADERS,
};
