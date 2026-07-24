//! station-wire/v1.json contract invariants (architecture-deepening Issue 1 + A8 IV).

#[test]
fn station_wire_documents_bearer_upstream_not_daemon_secret_identity() {
    let raw = include_str!("../../station-wire/v1.json");
    let v: serde_json::Value = serde_json::from_str(raw).expect("station-wire JSON");
    let auth = &v["endpoints"]["auth"];
    assert_eq!(
        auth["agent_to_brain"]["kind"].as_str(),
        Some("station_caller_bearer")
    );
    assert_eq!(
        auth["notch_to_agent"]["kind"].as_str(),
        Some("wire_v1_machine_integrity")
    );
    let hops = &v["endpoints"]["hops"];
    assert!(hops["notch_to_agent_bar"]["declare"]["path"]
        .as_str()
        .unwrap()
        .starts_with("/api/daemon/bar/"));
    assert!(hops["agent_to_brain_bar"]["declarations"]["path"]
        .as_str()
        .unwrap()
        .starts_with("/api/bar/v1/"));
    let cmds = hops["kill"]["command_types"].as_array().expect("command_types");
    assert!(cmds.iter().any(|c| c == "fog_of_war"));
    assert!(cmds.iter().any(|c| c == "clear_fog"));
    assert_eq!(
        v["endpoints"]["compatibility"]["phase"].as_str(),
        Some("architecture-deepening-v1")
    );
}
