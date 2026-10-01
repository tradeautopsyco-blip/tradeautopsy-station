//! Hosts-file DNS enforcement for kill switch (macOS). Other platforms no-op.
//!
//! Set `TRADEAUTOPSY_HOSTS_FILE` to a writable path for integration tests (no sudo).

use std::path::{Path, PathBuf};

pub const BLOCK_MARKER: &str = "# tradeautopsy-killswitch";
/// Substring for `sed /.../d` (no `#` — keeps sudoers rules parseable).
const HOSTS_LINE_MATCH: &str = "tradeautopsy-killswitch";

/// Marker for a venue IP-ban block (Ring 3 of VenueEgress).
///
/// The token order is deliberate and load-bearing. It must:
///   - contain `HOSTS_LINE_MATCH`, so the existing
///     `sudoers.d/99-tradeautopsy-dns` rule `sed /tradeautopsy-killswitch/d`
///     still removes it with no sudoers change; and
///   - **not** contain `BLOCK_MARKER`, so `is_block_active()` and every existing
///     `content.contains(BLOCK_MARKER)` assertion keep meaning "the kill switch
///     is armed" and are not made true by a venue ban.
/// Putting `venue-ban-` in front of the token satisfies both; appending it would
/// satisfy only the first.
const VENUE_BAN_MARKER: &str = "# venue-ban-tradeautopsy-killswitch";

/// Why a set of hosts is currently sinkholed. The hosts file has more than one
/// owner, so no single owner may clear it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockReason {
    /// The kill switch. Cleared only by the kill switch.
    KillSwitch,
    /// A venue told us we are IP-banned. Cleared when the ban expires.
    VenueBan,
}

impl BlockReason {
    pub fn marker(self) -> &'static str {
        match self {
            BlockReason::KillSwitch => BLOCK_MARKER,
            BlockReason::VenueBan => VENUE_BAN_MARKER,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BlockReason::KillSwitch => "kill_switch",
            BlockReason::VenueBan => "venue_ban",
        }
    }
}

/// Every (reason, broker) currently asking for a sinkhole. The hosts file is
/// always rewritten to the **union** of this set — never appended to, never
/// wholesale deleted — so one owner's release cannot lift another's block.
static ACTIVE_BLOCKS: std::sync::Mutex<std::collections::BTreeSet<(BlockReason, String)>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

fn active_blocks() -> Vec<(BlockReason, String)> {
    ACTIVE_BLOCKS
        .lock()
        .map(|g| g.iter().cloned().collect())
        .unwrap_or_default()
}

/// Lines the hosts file should currently carry, for the whole union.
fn union_entries() -> String {
    let mut out = String::new();
    for (reason, broker) in active_blocks() {
        for host in hosts_for_broker(&broker) {
            out.push_str(&format!("127.0.0.1 {host} {}\n", reason.marker()));
            out.push_str(&format!("::1 {host} {}\n", reason.marker()));
        }
    }
    out
}

/// Is this specific reason armed, according to the file on disk?
pub fn is_reason_active(reason: BlockReason) -> bool {
    let Ok(content) = std::fs::read_to_string(hosts_file_path()) else {
        return false;
    };
    content
        .lines()
        .any(|line| line.trim_end().ends_with(reason.marker()))
}

const KOTAK_HOSTS: &[&str] = &[
    "cis.kotaksecurities.com",
    "neo.kotaksecurities.com",
    "mis.kotaksecurities.com",
    // Trading-API gateways from SDK login `baseUrl` (keep aligned with ALLOWED_BROKER_HOSTS).
    "gw-napi.kotaksecurities.com",
    "mnapi.kotaksecurities.com",
    "cnapi.kotaksecurities.com",
    "napi.kotaksecurities.com",
    "e21.kotaksecurities.com",
    "e22.kotaksecurities.com",
    "e41.kotaksecurities.com",
    "e43.kotaksecurities.com",
    "lapi.kotaksecurities.com",
];

/// R8 gap closed: COM was reachable during an L3 block because it had no host set here.
/// HTTP hosts stay aligned with `ubi::ALLOWED_BROKER_HOSTS`. `stream.binance.com` is
/// extra: Market Streams WS (9443), not `broker_http_call`. A COM ban must sinkhole it.
const BINANCE_COM_HOSTS: &[&str] = &[
    "api.binance.com",
    "eapi.binance.com",
    "fapi.binance.com",
    "dapi.binance.com",
    "stream.binance.com",
    "fstream.binance.com",
    "dstream.binance.com",
];

const BINANCE_US_HOSTS: &[&str] = &["api.binance.us"];

const ZERODHA_HOSTS: &[&str] = &["kite.zerodha.com", "api.kite.trade", "ws.kite.trade"];

const UPSTOX_HOSTS: &[&str] = &[
    "api.upstox.com",
    "api-v2.upstox.com",
    "api-hft.upstox.com",
    "assets.upstox.com",
];

const FYERS_HOSTS: &[&str] = &[
    "api-t1.fyers.in",
    "api.fyers.in",
    "api-t2.fyers.in",
    "public.fyers.in",
];

const OKX_COM_HOSTS: &[&str] = &["www.okx.com"];

const KRAKEN_HOSTS: &[&str] = &["api.kraken.com"];

const GROWW_HOSTS: &[&str] = &[
    "api.groww.in",
    "growwapi-assets.groww.in",
    // Socket host confirmed at dogfood if Z7 adds it; sinkhole early per lock row 22.
    "socket-api.groww.in",
];

const DHAN_HOSTS: &[&str] = &[
    "api.dhan.co",
    "auth.dhan.co",
    "api-feed.dhan.co",
    "api-order-update.dhan.co",
];

const COINBASE_ADVANCED_HOSTS: &[&str] = &["api.coinbase.com"];

const BYBIT_HOSTS: &[&str] = &["api.bybit.com"];

/// Broker slug → sinkhole hostnames (pure, testable).
///
/// Unknown slugs return **empty** — never default to Kotak (R8). An empty set
/// must not be treated as a successful L3 block.
pub fn hosts_for_broker(broker: &str) -> &'static [&'static str] {
    let slug = broker.trim().to_ascii_lowercase();
    match slug.as_str() {
        // W0.7 (F5): planned slug `zerodha_kite` reuses ZERODHA_HOSTS.
        // UNVERIFIED until B6 row 22 cites them; verified Wave 1 Z7.
        "zerodha" | "kite" | "zerodha_kite" => ZERODHA_HOSTS,
        "upstox" => UPSTOX_HOSTS,
        "fyers" => FYERS_HOSTS,
        "groww" => GROWW_HOSTS,
        "dhan" => DHAN_HOSTS,
        "coinbase_advanced" | "coinbase" => COINBASE_ADVANCED_HOSTS,
        "okx_com" => OKX_COM_HOSTS,
        "kraken" => KRAKEN_HOSTS,
        "kotak" | "kotak_neo" => KOTAK_HOSTS,
        "binance" | "binance_com" => BINANCE_COM_HOSTS,
        "binance_us" => BINANCE_US_HOSTS,
        "bybit" => BYBIT_HOSTS,
        _ => &[],
    }
}

fn hosts_file_path() -> PathBuf {
    std::env::var("TRADEAUTOPSY_HOSTS_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/etc/hosts"))
}

fn direct_write_enabled() -> bool {
    std::env::var("TRADEAUTOPSY_HOSTS_FILE").is_ok()
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::io::Write;
    use std::process::Command;
    use tracing::info;

    fn flush_dns_cache() {
        if direct_write_enabled() {
            return;
        }
        let _ = Command::new("sudo")
            .args(["/usr/bin/dscacheutil", "-flushcache"])
            .output();
        let _ = Command::new("sudo")
            .args(["/usr/bin/killall", "-HUP", "mDNSResponder"])
            .output();
    }

    pub fn flush_browser_dns() {
        if direct_write_enabled() {
            return;
        }
        let _ = Command::new("pkill")
            .args(["-x", "Google Chrome Helper (Renderer)"])
            .output();
        let _ = Command::new("pkill")
            .args(["-x", "com.apple.WebKit.Networking"])
            .output();
        let _ = Command::new("pkill").args(["-x", "firefox"]).output();
        std::thread::sleep(std::time::Duration::from_millis(200));
        info!("Browser DNS caches flushed");
    }

    fn append_entries(path: &Path, entries: &str) -> Result<(), String> {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("open hosts file: {e}"))?;
        file.write_all(entries.as_bytes())
            .map_err(|e| format!("write hosts file: {e}"))?;
        Ok(())
    }

    /// Strip every line this process manages, whatever the reason.
    fn strip_managed_lines(path: &Path) -> Result<(), String> {
        if direct_write_enabled() {
            let content =
                std::fs::read_to_string(path).map_err(|e| format!("read hosts file: {e}"))?;
            let filtered: String = content
                .lines()
                .filter(|line| !line.contains(HOSTS_LINE_MATCH))
                .map(|line| format!("{line}\n"))
                .collect();
            std::fs::write(path, filtered).map_err(|e| format!("write hosts file: {e}"))?;
            return Ok(());
        }
        // Same sudoers rule as before: the venue-ban marker was chosen to contain
        // this token so no sudoers change is needed.
        let pattern = format!("/{HOSTS_LINE_MATCH}/d");
        let result = Command::new("sudo")
            .args([
                "/usr/bin/sed",
                "-i",
                "",
                &pattern,
                path.to_string_lossy().as_ref(),
            ])
            .output()
            .map_err(|e| format!("sed failed: {e}"))?;
        if !result.status.success() {
            return Err(format!(
                "sed error: {}",
                String::from_utf8_lossy(&result.stderr)
            ));
        }
        Ok(())
    }

    /// Rewrite the managed region to the union of every active reason.
    ///
    /// Never "append" and never "delete all": those are what made a single owner
    /// able to skip another owner's block on the way in, and tear it down on the
    /// way out.
    pub(super) fn rewrite_managed_region() -> Result<(), String> {
        let path = hosts_file_path();
        strip_managed_lines(&path)?;

        let entries = union_entries();
        if entries.is_empty() {
            flush_dns_cache();
            info!("Hosts block: no active reasons — managed region empty");
            return Ok(());
        }

        if direct_write_enabled() {
            append_entries(&path, &entries)?;
        } else {
            let mut child = Command::new("sudo")
                .args(["/usr/bin/tee", "-a", path.to_string_lossy().as_ref()])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Failed to spawn tee: {e}"))?;
            let mut stdin = child.stdin.take().ok_or("Failed to get stdin")?;
            stdin
                .write_all(entries.as_bytes())
                .map_err(|e| format!("Failed to write entries: {e}"))?;
            drop(stdin);
            let status = child.wait().map_err(|e| format!("tee wait failed: {e}"))?;
            if !status.success() {
                return Err("sudo tee /etc/hosts failed".to_string());
            }
        }

        flush_dns_cache();
        Ok(())
    }

    /// Arm one reason for one broker and rewrite the union.
    pub(super) fn arm(reason: BlockReason, broker: &str) -> Result<(), String> {
        let hosts = hosts_for_broker(broker);
        if hosts.is_empty() {
            // R8: an unknown slug must never be treated as a successful block.
            return Err(format!(
                "no Kill DNS hosts for broker={broker} — refusing empty L3 block (R8)"
            ));
        }
        if let Ok(mut active) = ACTIVE_BLOCKS.lock() {
            active.insert((reason, broker.to_string()));
        }
        info!(
            "Hosts block: arming {} hosts for broker={broker} reason={}",
            hosts.len(),
            reason.as_str()
        );
        let result = rewrite_managed_region();
        if result.is_ok() && reason == BlockReason::KillSwitch {
            flush_browser_dns();
        }
        result
    }

    /// Release one reason for one broker and rewrite the union. Other reasons,
    /// and other brokers under the same reason, stay blocked.
    pub(super) fn disarm(reason: BlockReason, broker: Option<&str>) -> Result<(), String> {
        if let Ok(mut active) = ACTIVE_BLOCKS.lock() {
            match broker {
                Some(b) => {
                    active.remove(&(reason, b.to_string()));
                }
                None => active.retain(|(r, _)| *r != reason),
            }
        }
        info!("Hosts block: released reason={}", reason.as_str());
        rewrite_managed_region()
    }

    /// Back-compat entry point for the kill switch.
    pub fn apply_hosts_block(broker: &str) -> Result<(), String> {
        arm(BlockReason::KillSwitch, broker)
    }

    /// Back-compat entry point for the kill switch. Releases **only** the kill
    /// switch: an active venue ban survives, and vice versa.
    pub fn disable_block() -> Result<(), String> {
        disarm(BlockReason::KillSwitch, None)
    }

    /// True when the **kill switch** is armed. A venue ban does not make this
    /// true — that is why the two markers are distinguishable.
    pub fn is_block_active() -> bool {
        super::is_reason_active(BlockReason::KillSwitch)
    }
}

/// Does the file on disk still carry every line the union asks for?
#[cfg(target_os = "macos")]
fn managed_region_is_stale() -> bool {
    let expected = union_entries();
    if expected.is_empty() {
        return false;
    }
    let Ok(content) = std::fs::read_to_string(hosts_file_path()) else {
        return true;
    };
    expected
        .lines()
        .any(|line| !line.is_empty() && !content.contains(line))
}

#[cfg(target_os = "macos")]
static WATCHER_HANDLE: std::sync::Mutex<Option<notify::RecommendedWatcher>> =
    std::sync::Mutex::new(None);

#[cfg(target_os = "macos")]
static ARMED_BROKER: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

#[cfg(target_os = "macos")]
fn start_hosts_watcher(broker: String) {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use tracing::{info, warn};

    if let Ok(mut armed) = ARMED_BROKER.lock() {
        *armed = Some(broker.clone());
    }

    let mut guard = WATCHER_HANDLE.lock().expect("watcher lock");
    if guard.is_some() {
        return;
    }

    let watch_path = hosts_file_path();
    let broker_clone = broker.clone();
    let watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_)) {
                    // Re-apply the **union**, not one broker: a venue ban armed
                    // alongside the kill switch must survive tampering too.
                    if managed_region_is_stale() {
                        let _ = &broker_clone;
                        warn!("Hosts block: /etc/hosts tampered — re-applying union");
                        let _ = macos::rewrite_managed_region();
                    }
                }
            }
        },
        Config::default(),
    );

    match watcher {
        Ok(mut w) => {
            if let Err(e) = w.watch(&watch_path, RecursiveMode::NonRecursive) {
                warn!("KillSwitch: failed to watch hosts file: {e}");
                return;
            }
            info!("KillSwitch: hosts watcher started for broker={broker}");
            *guard = Some(w);
        }
        Err(e) => warn!("KillSwitch: failed to create watcher: {e}"),
    }
}

#[cfg(target_os = "macos")]
fn stop_hosts_watcher() {
    let mut guard = WATCHER_HANDLE.lock().expect("watcher lock");
    if guard.take().is_some() {
        tracing::info!("KillSwitch: hosts watcher stopped");
    }
    if let Ok(mut armed) = ARMED_BROKER.lock() {
        *armed = None;
    }
}

/// Boot reconcile: latch armed and hosts already blocked — re-arm watcher without
/// rewriting a correct hosts file.
#[cfg(target_os = "macos")]
pub fn reconcile_armed_blocked(broker: &str) -> Result<(), String> {
    if let Ok(mut active) = ACTIVE_BLOCKS.lock() {
        active.insert((BlockReason::KillSwitch, broker.to_string()));
    }
    if managed_region_is_stale() {
        macos::rewrite_managed_region()?;
    }
    start_hosts_watcher(broker.to_string());
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn reconcile_armed_blocked(_broker: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn enable_block_with_watcher(broker: &str) -> Result<(), String> {
    let broker_owned = broker.to_string();
    let result = macos::apply_hosts_block(&broker_owned);
    if result.is_ok() {
        start_hosts_watcher(broker_owned);
    }
    result
}

#[cfg(target_os = "macos")]
pub fn disable_block_and_watcher() -> Result<(), String> {
    stop_hosts_watcher();
    macos::disable_block()
}

#[cfg(target_os = "macos")]
pub use macos::{apply_hosts_block, disable_block, is_block_active};

/// Ring 3 of VenueEgress: sinkhole a banned venue's hosts for the life of the
/// ban, so nothing on this Mac — including a process that never learned about
/// the engine — can keep knocking and extend it.
///
/// Best effort by design. This shells out to `sudo`; if that prompts, fails, or
/// hangs, the in-process freeze has already refused the call and the ban is
/// already being respected. Ring 3 is a backstop, never the mechanism.
#[cfg(target_os = "macos")]
pub fn arm_venue_ban(slot_id: &str) -> Result<(), String> {
    macos::arm(BlockReason::VenueBan, slot_id)
}

#[cfg(target_os = "macos")]
pub fn clear_venue_ban(slot_id: &str) -> Result<(), String> {
    macos::disarm(BlockReason::VenueBan, Some(slot_id))
}

#[cfg(not(target_os = "macos"))]
pub fn arm_venue_ban(_slot_id: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn clear_venue_ban(_slot_id: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn enable_block_with_watcher(_broker: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn disable_block_and_watcher() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn flush_browser_dns() {}

#[cfg(not(target_os = "macos"))]
pub fn apply_hosts_block(_broker: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn disable_block() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn is_block_active() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zerodha_broker_maps_to_kite_hosts() {
        let hosts = hosts_for_broker("zerodha");
        assert!(hosts.contains(&"kite.zerodha.com"));
        assert!(hosts.contains(&"api.kite.trade"));
        assert!(hosts.contains(&"ws.kite.trade"));
    }

    #[test]
    fn upstox_broker_maps_to_upstox_hosts() {
        let hosts = hosts_for_broker("upstox");
        assert!(hosts.contains(&"api.upstox.com"));
    }

    #[test]
    fn kotak_neo_maps_to_kotak_hosts() {
        let hosts = hosts_for_broker("kotak_neo");
        assert!(hosts.contains(&"neo.kotaksecurities.com"));
        assert!(hosts.contains(&"cis.kotaksecurities.com"));
        assert!(hosts.contains(&"e21.kotaksecurities.com"));
    }

    #[test]
    fn binance_com_maps_to_com_host_never_the_us_venue() {
        let hosts = hosts_for_broker("binance_com");
        assert!(hosts.contains(&"api.binance.com"));
        assert!(!hosts.contains(&"api.binance.us"));
        assert!(hosts_for_broker("binance_us").contains(&"api.binance.us"));
    }

    #[test]
    fn binance_com_kill_hosts_never_include_kotak() {
        let hosts = hosts_for_broker("binance_com");
        for kotak in KOTAK_HOSTS {
            assert!(
                !hosts.contains(kotak),
                "binance_com Kill must not sinkhole Kotak host {kotak}"
            );
        }
    }

    #[test]
    fn kill_dns_covers_every_ubi_allowlisted_host() {
        for host in crate::ubi::ALLOWED_BROKER_HOSTS {
            let covered = [
                "binance_com",
                "kotak_neo",
                "zerodha_kite",
                "upstox",
                "fyers",
                "groww",
                "dhan",
                "bybit",
                "okx_com",
                "coinbase_advanced",
                "kraken",
            ]
                .iter()
                .any(|slug| hosts_for_broker(slug).contains(host));
            assert!(covered, "no Kill DNS entry for allowlisted host {host}");
        }
    }

    #[test]
    fn unknown_broker_does_not_silently_use_kotak_hosts() {
        let hosts = hosts_for_broker("unknown");
        assert!(
            hosts.is_empty(),
            "unknown slug must not default to Kotak (R8)"
        );
        for kotak in KOTAK_HOSTS {
            assert!(!hosts.contains(kotak));
        }
    }

    #[test]
    fn first_pair_kill_dogfood_hosts_are_disjoint_and_non_empty() {
        let com = hosts_for_broker("binance_com");
        let kotak = hosts_for_broker("kotak_neo");
        assert!(!com.is_empty(), "COM Kill dogfood requires Binance hosts");
        assert!(!kotak.is_empty(), "Kotak Kill dogfood requires Kotak hosts");
        assert!(com.contains(&"api.binance.com"));
        assert!(kotak.contains(&"mis.kotaksecurities.com"));
        for h in com {
            assert!(
                !kotak.contains(h),
                "host {h} must not be shared across desks"
            );
        }
    }

    #[test]
    fn w07_every_catalog_slug_plus_zerodha_kite_resolves_non_empty() {
        // W0.7 (F5): every catalog_v1 slug + planned `zerodha_kite` must resolve
        // NON-EMPTY, or L3 Kill silently no-ops for that slug.
        for descriptor in crate::ubi::catalog_v1() {
            let hosts = hosts_for_broker(&descriptor.slug);
            assert!(
                !hosts.is_empty(),
                "catalog slug {} must resolve to a non-empty Kill host set (W0.7)",
                descriptor.slug
            );
        }
        assert!(
            !hosts_for_broker("zerodha_kite").is_empty(),
            "planned slug zerodha_kite must resolve to a non-empty Kill host set (W0.7)"
        );
    }

    #[test]
    fn w07_unknown_slug_stays_empty_r8() {
        // R8 by design: unknown slugs return empty, never a default host set.
        let hosts = hosts_for_broker("definitely_not_a_broker_xyz");
        assert!(
            hosts.is_empty(),
            "unknown slug must stay empty by design (R8)"
        );
    }

    #[test]
    fn w07_zerodha_kite_set_equals_zerodha_set() {
        assert_eq!(
            hosts_for_broker("zerodha_kite"),
            hosts_for_broker("zerodha"),
            "zerodha_kite must reuse the Zerodha host set (W0.7)"
        );
    }

    #[test]
    fn kill_dns_named_book_hosts_match_spot_and_cash_no_eapi_or_fo() {
        // Kill is slug-scoped: named books on `binance_com` share this host set.
        // Spot's R0 fence still refuses fapi/dapi/eapi (host_policy). NFO shares Kotak.
        let com = hosts_for_broker("binance_com");
        assert_eq!(com, BINANCE_COM_HOSTS);
        assert!(com.contains(&"eapi.binance.com"));
        assert!(com.contains(&"fapi.binance.com"));
        assert!(com.contains(&"dapi.binance.com"));
        assert!(com.contains(&"fstream.binance.com"));
        assert!(com.contains(&"dstream.binance.com"));
        assert!(com.contains(&"stream.binance.com"));

        let kotak = hosts_for_broker("kotak_neo");
        assert_eq!(kotak, KOTAK_HOSTS);
        assert!(kotak.contains(&"lapi.kotaksecurities.com"));
        assert!(!kotak.contains(&"mlhsm.kotaksecurities.com"));
        assert!(!kotak.iter().any(|h| h.contains("eapi")));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn apply_and_disable_block_with_test_hosts_file() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "rta-killswitch-test-{}.hosts",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&path, "127.0.0.1 localhost\n").expect("seed hosts");
        std::env::set_var(
            "TRADEAUTOPSY_HOSTS_FILE",
            path.to_string_lossy().to_string(),
        );

        apply_hosts_block("zerodha").expect("apply");
        assert!(is_block_active());
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains(BLOCK_MARKER));
        assert!(content.contains("kite.zerodha.com"));

        disable_block().expect("disable");
        assert!(!is_block_active());
        let after = std::fs::read_to_string(&path).expect("read after");
        assert!(!after.contains(BLOCK_MARKER));

        std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
        let _ = std::fs::remove_file(path);
    }
}
