//! The hosts file has more than one owner. No owner may clear another's block.
//!
//! Before VenueEgress, `dns_block` assumed a single owner: `apply_hosts_block`
//! skipped entirely if any marker was present, and `disable_block` deleted every
//! line matching `tradeautopsy-killswitch`. Hanging a venue ban on that would have
//! meant a ban expiring at 15:12 silently disarming the kill switch.

use serial_test::serial;
use std::path::PathBuf;
use tradeautopsy_agent::{
    arm_venue_ban, clear_venue_ban, hosts_for_broker, BlockReason, BLOCK_MARKER,
};

fn temp_hosts() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "tradeautopsy-hosts-{}-{}",
        std::process::id(),
        ulid::Ulid::new()
    ));
    std::fs::write(&path, "127.0.0.1 localhost\n").expect("seed hosts file");
    std::env::set_var("TRADEAUTOPSY_HOSTS_FILE", &path);
    path
}

fn read(path: &PathBuf) -> String {
    std::fs::read_to_string(path).expect("read hosts file")
}

fn cleanup(path: PathBuf) {
    let _ = tradeautopsy_agent::dns_disable_block_for_tests();
    let _ = std::fs::remove_file(&path);
    std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
}

#[test]
#[serial]
fn a_venue_ban_does_not_look_like_an_armed_kill_switch() {
    let path = temp_hosts();
    arm_venue_ban("binance_com").expect("arm ban");

    let content = read(&path);
    assert!(
        content.contains("api.binance.com"),
        "ban should sinkhole the venue: {content}"
    );
    // Every existing assertion in the kill-switch suites is
    // `content.contains(BLOCK_MARKER)`. A ban must not make those true.
    assert!(
        !content.contains(BLOCK_MARKER),
        "a venue ban must not read as an armed kill switch: {content}"
    );
    assert!(!tradeautopsy_agent::dns_is_block_active_for_tests());

    cleanup(path);
}

#[test]
#[serial]
fn a_ban_expiring_leaves_an_armed_kill_switch_standing() {
    let path = temp_hosts();

    arm_venue_ban("binance_com").expect("arm ban");
    tradeautopsy_agent::dns_apply_hosts_block_for_tests("kotak_neo").expect("arm kill switch");

    let both = read(&path);
    assert!(both.contains(BLOCK_MARKER), "kill switch armed: {both}");
    assert!(both.contains("api.binance.com"), "ban armed: {both}");
    assert!(both.contains("gw-napi.kotaksecurities.com"));

    // The ban expires. This is the case that would previously have wiped the
    // kill switch off the disk.
    clear_venue_ban("binance_com").expect("clear ban");

    let after = read(&path);
    assert!(
        after.contains(BLOCK_MARKER),
        "kill switch must survive a ban expiring: {after}"
    );
    assert!(
        after.contains("gw-napi.kotaksecurities.com"),
        "kill switch hosts must survive: {after}"
    );
    assert!(
        !after.contains("api.binance.com"),
        "expired ban must be lifted: {after}"
    );
    assert!(tradeautopsy_agent::dns_is_block_active_for_tests());

    cleanup(path);
}

#[test]
#[serial]
fn disarming_the_kill_switch_leaves_an_active_ban_standing() {
    let path = temp_hosts();

    tradeautopsy_agent::dns_apply_hosts_block_for_tests("kotak_neo").expect("arm kill switch");
    arm_venue_ban("binance_com").expect("arm ban");

    tradeautopsy_agent::dns_disable_block_for_tests().expect("disarm kill switch");

    let after = read(&path);
    assert!(
        !after.contains(BLOCK_MARKER),
        "kill switch should be gone: {after}"
    );
    assert!(
        !after.contains("gw-napi.kotaksecurities.com"),
        "kill switch hosts should be gone: {after}"
    );
    assert!(
        after.contains("api.binance.com"),
        "an active IP ban must survive the kill switch being dismissed: {after}"
    );

    cleanup(path);
}

#[test]
#[serial]
fn arming_a_ban_while_the_kill_switch_is_armed_blocks_both() {
    let path = temp_hosts();

    // The old `apply_hosts_block` early-returned when any marker was present, so
    // the second reason was silently skipped.
    tradeautopsy_agent::dns_apply_hosts_block_for_tests("kotak_neo").expect("kill switch");
    arm_venue_ban("binance_com").expect("ban");

    let content = read(&path);
    for host in hosts_for_broker("kotak_neo") {
        assert!(content.contains(host), "missing kill-switch host {host}");
    }
    for host in hosts_for_broker("binance_com") {
        assert!(content.contains(host), "missing banned host {host}");
    }

    cleanup(path);
}

#[test]
#[serial]
fn unmanaged_lines_are_never_touched() {
    let path = temp_hosts();
    std::fs::write(
        &path,
        "127.0.0.1 localhost\n255.255.255.255 broadcasthost\n",
    )
    .unwrap();

    arm_venue_ban("binance_com").expect("arm");
    tradeautopsy_agent::dns_apply_hosts_block_for_tests("kotak_neo").expect("arm");
    clear_venue_ban("binance_com").expect("clear");
    tradeautopsy_agent::dns_disable_block_for_tests().expect("clear");

    let after = read(&path);
    assert!(after.contains("127.0.0.1 localhost"));
    assert!(after.contains("255.255.255.255 broadcasthost"));
    assert!(!after.contains("tradeautopsy-killswitch"));

    cleanup(path);
}

#[test]
#[serial]
fn an_unknown_slug_is_refused_rather_than_blocking_nothing() {
    let path = temp_hosts();
    // R8: an empty host set must never be reported as a successful block.
    assert!(arm_venue_ban("not-a-broker").is_err());
    let content = read(&path);
    assert!(!content.contains("tradeautopsy-killswitch"));
    cleanup(path);
}

#[test]
#[serial]
fn both_markers_are_removable_by_the_existing_sudoers_rule() {
    // `sudoers.d/99-tradeautopsy-dns` permits `sed /tradeautopsy-killswitch/d`.
    // Both markers must match that pattern or a ban could not be lifted without
    // a sudoers change.
    assert!(BlockReason::KillSwitch
        .marker()
        .contains("tradeautopsy-killswitch"));
    assert!(BlockReason::VenueBan
        .marker()
        .contains("tradeautopsy-killswitch"));
    // …while staying distinguishable from the kill switch.
    assert!(!BlockReason::VenueBan.marker().contains(BLOCK_MARKER));
}
