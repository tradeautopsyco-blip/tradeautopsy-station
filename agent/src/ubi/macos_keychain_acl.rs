//! macOS Keychain writes with `SecAccess` trusting the agent + Station (issue #1).
//!
//! Mirrors `KeychainBrokerCredentialStore.makeTrustedAccessIncludingAgent` in Station.

use anyhow::{Context, Result};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::data::CFData;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation_sys::base::{CFRelease, CFTypeRef};
use core_foundation_sys::string::CFStringRef;
use security_framework::os::macos::access::SecAccess;
use security_framework::passwords::{delete_generic_password_options, PasswordOptions};
use security_framework_sys::base::{errSecItemNotFound, errSecSuccess, SecAccessRef};
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecValueData,
};
use security_framework_sys::keychain_item::SecItemAdd;
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

use crate::ubi::credentials::{
    BROKER_CREDENTIAL_KEYCHAIN_SERVICE, KOTAK_SESSION_KEYCHAIN_SERVICE,
};

/// macOS executable name inside `TradeAutopsy Station.app/Contents/MacOS/` (`project.yml`).
pub const STATION_MACOS_EXECUTABLE_NAME: &str = "TradeAutopsy Station";

type SecTrustedApplicationRef = *mut std::ffi::c_void;

#[link(name = "Security", kind = "framework")]
extern "C" {
    static kSecAttrAccess: CFStringRef;
    fn SecAccessCreate(
        descriptor: CFStringRef,
        trustedlist: core_foundation_sys::array::CFArrayRef,
        accessRef: *mut SecAccessRef,
    ) -> i32;
    fn SecTrustedApplicationCreateFromPath(
        path: *const c_char,
        app: *mut SecTrustedApplicationRef,
    ) -> i32;
}

struct TrustedApplication(SecTrustedApplicationRef);

impl Drop for TrustedApplication {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0 as CFTypeRef) };
        }
    }
}

fn cvt_os_status(status: i32) -> Result<()> {
    if status == errSecSuccess {
        Ok(())
    } else {
        Err(security_framework::base::Error::from_code(status).into())
    }
}

/// Services that should be written with Station-aligned SecAccess (not default keyring ACL).
pub fn service_uses_trusted_acl(service: &str) -> bool {
    service == KOTAK_SESSION_KEYCHAIN_SERVICE || service == BROKER_CREDENTIAL_KEYCHAIN_SERVICE
}

/// Resolve `TradeAutopsy Station.app/Contents/MacOS/TradeAutopsy Station` when the agent runs
/// bundled next to the Station executable (`station/project.yml` post-build copy).
pub fn station_executable_adjacent_to_agent(agent_exe: &Path) -> Option<PathBuf> {
    if agent_exe.file_name()?.to_str()? != "tradeautopsy-agent" {
        return None;
    }
    let macos_dir = agent_exe.parent()?;
    if macos_dir.file_name().and_then(|n| n.to_str()) != Some("MacOS") {
        return None;
    }
    let station = macos_dir.join(STATION_MACOS_EXECUTABLE_NAME);
    if station.is_file() {
        Some(station)
    } else {
        None
    }
}

pub fn discover_station_executable_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    station_executable_adjacent_to_agent(&exe)
}

fn access_descriptor_label(service: &str) -> &'static str {
    if service == KOTAK_SESSION_KEYCHAIN_SERVICE {
        "TradeAutopsy Kotak session vault"
    } else {
        "TradeAutopsy broker credentials"
    }
}

fn trusted_application_from_path(path: Option<&Path>) -> Result<Option<TrustedApplication>> {
    let owned_path;
    let c_path = match path {
        None => ptr::null(),
        Some(p) => {
            owned_path = CString::new(p.as_os_str().as_bytes())
                .context("trusted application path")?;
            owned_path.as_ptr()
        }
    };
    let mut app: SecTrustedApplicationRef = ptr::null_mut();
    let status = unsafe { SecTrustedApplicationCreateFromPath(c_path, &mut app) };
    if status == errSecSuccess && !app.is_null() {
        Ok(Some(TrustedApplication(app)))
    } else {
        Ok(None)
    }
}

fn make_trusted_access(service: &str) -> Result<SecAccess> {
    let mut holders: Vec<TrustedApplication> = Vec::new();

    if let Some(self_app) = trusted_application_from_path(None)? {
        holders.push(self_app);
    }
    if let Some(station) = discover_station_executable_path() {
        if let Some(station_app) = trusted_application_from_path(Some(&station))? {
            holders.push(station_app);
        }
    }

    if holders.is_empty() {
        anyhow::bail!("SecAccess: no trusted applications (agent or Station)");
    }

    let cf_apps: Vec<CFType> = holders
        .iter()
        .map(|h| unsafe { CFType::wrap_under_get_rule(h.0 as CFTypeRef) })
        .collect();
    let trusted_list = CFArray::from_CFTypes(&cf_apps);

    let descriptor = CFString::new(access_descriptor_label(service));
    let mut access_ref: SecAccessRef = ptr::null_mut();
    cvt_os_status(unsafe {
        SecAccessCreate(
            descriptor.as_concrete_TypeRef(),
            trusted_list.as_concrete_TypeRef(),
            &mut access_ref,
        )
    })
    .context("SecAccessCreate")?;

    Ok(unsafe { SecAccess::wrap_under_create_rule(access_ref) })
}

/// Delete + `SecItemAdd` generic password with `kSecAttrAccess` (readable by agent + Station).
pub fn write_generic_password_with_trusted_acl(
    service: &str,
    account: &str,
    secret: &str,
) -> Result<()> {
    let delete_opts = PasswordOptions::new_generic_password(service, account);
    match delete_generic_password_options(delete_opts) {
        Ok(()) => {}
        Err(e) if e.code() == errSecItemNotFound as i32 => {}
        Err(e) => return Err(e.into()),
    }

    let access = make_trusted_access(service)?;
    let add_params = CFDictionary::from_CFType_pairs(&[
        (
            unsafe { CFString::wrap_under_get_rule(kSecClass).into_CFType() },
            unsafe { CFString::wrap_under_get_rule(kSecClassGenericPassword).into_CFType() },
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrService).into_CFType() },
            CFString::new(service).into_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccount).into_CFType() },
            CFString::new(account).into_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecAttrAccess).into_CFType() },
            access.into_CFType(),
        ),
        (
            unsafe { CFString::wrap_under_get_rule(kSecValueData).into_CFType() },
            CFData::from_buffer(secret.as_bytes()).into_CFType(),
        ),
    ]);

    cvt_os_status(unsafe { SecItemAdd(add_params.as_concrete_TypeRef(), ptr::null_mut()) })
        .context("SecItemAdd broker vault")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn trusted_acl_services_include_kotak_and_hmac_vaults() {
        assert!(service_uses_trusted_acl(KOTAK_SESSION_KEYCHAIN_SERVICE));
        assert!(service_uses_trusted_acl(BROKER_CREDENTIAL_KEYCHAIN_SERVICE));
        assert!(!service_uses_trusted_acl("other.service"));
    }

    #[test]
    fn station_path_rejects_non_bundled_agent_layout() {
        assert!(station_executable_adjacent_to_agent(Path::new(
            "/tmp/target/debug/tradeautopsy-agent"
        ))
        .is_none());
    }

    #[test]
    fn station_path_finds_station_binary_in_app_bundle_macos() {
        let root = std::env::temp_dir().join(format!(
            "ta-station-acl-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let macos = root.join("TradeAutopsy Station.app/Contents/MacOS");
        fs::create_dir_all(&macos).expect("macos dir");
        let agent = macos.join("tradeautopsy-agent");
        fs::write(&agent, b"\0").expect("agent stub");
        let station = macos.join(STATION_MACOS_EXECUTABLE_NAME);
        fs::write(&station, b"\0").expect("station stub");

        assert_eq!(
            station_executable_adjacent_to_agent(&agent),
            Some(station)
        );
        let _ = fs::remove_dir_all(&root);
    }
}
