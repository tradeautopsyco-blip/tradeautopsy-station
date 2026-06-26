//! Hosts-file DNS enforcement for kill switch (macOS). Other platforms no-op.
//!
//! Set `TRADEAUTOPSY_HOSTS_FILE` to a writable path for integration tests (no sudo).

use std::path::{Path, PathBuf};

pub const BLOCK_MARKER: &str = "# tradeautopsy-killswitch";
/// Substring for `sed /.../d` (no `#` — keeps sudoers rules parseable).
const HOSTS_LINE_MATCH: &str = "tradeautopsy-killswitch";

const KOTAK_HOSTS: &[&str] = &[
    "cis.kotaksecurities.com",
    "neo.kotaksecurities.com",
    "mis.kotaksecurities.com",
];

const ZERODHA_HOSTS: &[&str] = &["kite.zerodha.com", "api.kite.trade"];

const UPSTOX_HOSTS: &[&str] = &["api.upstox.com", "api-v2.upstox.com"];

/// Broker slug → sinkhole hostnames (pure, testable).
pub fn hosts_for_broker(broker: &str) -> &'static [&'static str] {
    let slug = broker.trim().to_ascii_lowercase();
    match slug.as_str() {
        "zerodha" | "kite" => ZERODHA_HOSTS,
        "upstox" => UPSTOX_HOSTS,
        "kotak" | "kotak_neo" => KOTAK_HOSTS,
        _ => KOTAK_HOSTS,
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

    fn build_block_entries(broker: &str) -> String {
        hosts_for_broker(broker)
            .iter()
            .flat_map(|h| {
                [
                    format!("127.0.0.1 {h} {BLOCK_MARKER}\n"),
                    format!("::1 {h} {BLOCK_MARKER}\n"),
                ]
            })
            .collect()
    }

    /// Append broker block lines to hosts file and flush caches (blocking thread).
    pub fn apply_hosts_block(broker: &str) -> Result<(), String> {
        if is_block_active() {
            info!("KillSwitch DNS: already active, skipping hosts write");
            return Ok(());
        }

        let hosts = hosts_for_broker(broker);
        let entries = build_block_entries(broker);
        let path = hosts_file_path();

        info!(
            "KillSwitch DNS: blocking {} hosts for broker={}",
            hosts.len(),
            broker
        );

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

            let status = child
                .wait()
                .map_err(|e| format!("tee wait failed: {e}"))?;

            if !status.success() {
                return Err("sudo tee /etc/hosts failed".to_string());
            }
        }

        flush_dns_cache();
        flush_browser_dns();
        info!("KillSwitch DNS: block ENABLED for {broker}");
        Ok(())
    }

    pub fn disable_block() -> Result<(), String> {
        if !is_block_active() {
            info!("KillSwitch DNS: no active block, skipping");
            return Ok(());
        }

        let path = hosts_file_path();

        if direct_write_enabled() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("read hosts file: {e}"))?;
            let filtered: String = content
                .lines()
                .filter(|line| !line.contains(HOSTS_LINE_MATCH))
                .map(|line| format!("{line}\n"))
                .collect();
            std::fs::write(&path, filtered).map_err(|e| format!("write hosts file: {e}"))?;
        } else {
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
        }

        flush_dns_cache();
        info!("KillSwitch DNS: block DISABLED");
        Ok(())
    }

    pub fn is_block_active() -> bool {
        std::fs::read_to_string(hosts_file_path())
            .map(|c| c.contains(BLOCK_MARKER))
            .unwrap_or(false)
    }
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
                    if !macos::is_block_active() {
                        let armed_broker = ARMED_BROKER
                            .lock()
                            .ok()
                            .and_then(|g| g.clone())
                            .unwrap_or(broker_clone.clone());
                        warn!(
                            "KillSwitch: /etc/hosts tampered — re-applying block for broker={armed_broker}"
                        );
                        let _ = macos::apply_hosts_block(&armed_broker);
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
    }

    #[test]
    fn unknown_broker_falls_back_to_kotak_hosts() {
        let hosts = hosts_for_broker("unknown");
        assert!(hosts.contains(&"neo.kotaksecurities.com"));
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
        std::env::set_var("TRADEAUTOPSY_HOSTS_FILE", path.to_string_lossy().to_string());

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
