use std::process::Command;

#[test]
fn reference_exports_live_command_tree_in_diagnostic_envelope() {
    let output = Command::new(env!("CARGO_BIN_EXE_sc-compose"))
        .args(["reference", "--json"])
        .output()
        .expect("run CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("JSON envelope");
    let model = &document["payload"]["model"];
    assert_eq!(model["name"], "sc-compose");
    let commands = model["subcommands"].as_array().expect("command tree");
    let reference = commands
        .iter()
        .find(|c| c["name"] == "reference")
        .expect("reference command");
    assert!(
        reference["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["long"] == "--json")
    );
    let bead = commands
        .iter()
        .find(|c| c["name"] == "bead")
        .expect("bead command");
    assert!(
        bead["subcommands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "attach")
    );
}
