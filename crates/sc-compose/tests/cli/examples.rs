//! Example discovery and recovery use controlled, private catalogs.

use crate::support::*;
use serde_json::Value;

fn catalog() -> TempFixture {
    let fixture = TempFixture::new("examples-discovery");
    write_file(
        &fixture.path.join("examples/greeting.md.j2"),
        "---\nrequired_variables: [name]\n---\nHello {{ name }}!\n",
    );
    fixture
}

#[test]
fn bare_examples_matches_list_and_succeeds_without_error_logs() {
    let fixture = catalog();
    let listed = sc_compose()
        .args(["examples", "list"])
        .env("SC_COMPOSE_DATA_DIR", &fixture.path)
        .bounded_output()
        .unwrap();
    let bare = sc_compose()
        .arg("examples")
        .current_dir(&fixture.path)
        .env("SC_COMPOSE_DATA_DIR", &fixture.path)
        .bounded_output()
        .unwrap();
    assert_eq!(listed.status.code(), Some(0), "{listed:?}");
    assert_eq!(bare.status.code(), Some(0), "{bare:?}");
    assert_eq!(bare.stdout, listed.stdout);
    assert!(String::from_utf8_lossy(&bare.stdout).contains("greeting"));
    assert!(
        !String::from_utf8_lossy(&bare.stderr).contains("ERROR"),
        "{bare:?}"
    );
}

#[test]
fn bare_examples_json_matches_explicit_list_and_empty_catalog_succeeds() {
    let fixture = catalog();
    let listed = sc_compose()
        .args(["examples", "list", "--json"])
        .env("SC_COMPOSE_DATA_DIR", &fixture.path)
        .bounded_output()
        .unwrap();
    let bare = sc_compose()
        .args(["examples", "--json"])
        .env("SC_COMPOSE_DATA_DIR", &fixture.path)
        .bounded_output()
        .unwrap();
    assert_eq!(bare.status.code(), Some(0), "{bare:?}");
    assert_eq!(listed.status.code(), Some(0), "{listed:?}");
    assert_eq!(bare.stdout, listed.stdout);
    assert!(
        !String::from_utf8_lossy(&bare.stderr).contains("ERROR"),
        "{bare:?}"
    );
    let empty = sc_compose()
        .arg("examples")
        .env("SC_COMPOSE_DATA_DIR", fixture.path.join("absent"))
        .bounded_output()
        .unwrap();
    assert_eq!(empty.status.code(), Some(0), "{empty:?}");
    assert!(empty.stdout.is_empty());
}

#[test]
fn examples_list_advertises_inputs_and_its_sample_path_renders() {
    let fixture = catalog();
    let root = fixture.path.join("installed data");
    write_file(
        &root.join("examples/report.html.j2"),
        "---\nrequired_variables: [title]\n---\n<h1>{{ title }}</h1>\n",
    );
    write_file(
        &root.join("examples/report.sample-vars.json"),
        "{\"title\":\"Sample report\"}",
    );
    // A schema-only pack has no sample and must advertise its declared inputs.
    write_file(
        &root.join("examples/greeting.md.j2"),
        "---\nrequired_variables: [name]\n---\nHello {{ name }}!\n",
    );
    let text = sc_compose()
        .args(["examples", "list"])
        .env("SC_COMPOSE_DATA_DIR", &root)
        .bounded_output()
        .unwrap();
    assert_eq!(text.status.code(), Some(0), "{text:?}");
    let stdout = String::from_utf8(text.stdout).unwrap();
    assert!(stdout.contains("required variables: name"), "{stdout}");
    assert!(stdout.contains("report.sample-vars.json"), "{stdout}");
    assert!(stdout.contains("--var-file"), "{stdout}");
    let json = sc_compose()
        .args(["examples", "list", "--json"])
        .env("SC_COMPOSE_DATA_DIR", &root)
        .bounded_output()
        .unwrap();
    assert_eq!(json.status.code(), Some(0), "{json:?}");
    let value: Value = serde_json::from_slice(&json.stdout).unwrap();
    let packs = value["payload"]["packs"].as_array().unwrap();
    assert_eq!(packs[0]["required_variables"], serde_json::json!(["name"]));
    let sample = packs[1]["sample_vars"]
        .as_str()
        .expect("advertised sample path");
    let rendered = sc_compose()
        .args(["examples", "report", "--var-file", sample])
        .env("SC_COMPOSE_DATA_DIR", &root)
        .bounded_output()
        .unwrap();
    assert_eq!(rendered.status.code(), Some(0), "{rendered:?}");
    assert!(String::from_utf8_lossy(&rendered.stdout).contains("<h1>Sample report</h1>"));
}

#[test]
fn missing_example_suggests_a_close_pack_name_in_text_and_json() {
    let fixture = catalog();
    let text = sc_compose()
        .args(["examples", "greetng"])
        .env("SC_COMPOSE_DATA_DIR", &fixture.path)
        .bounded_output()
        .unwrap();
    assert_eq!(text.status.code(), Some(3), "{text:?}");
    let stderr = String::from_utf8_lossy(&text.stderr);
    assert!(stderr.contains("ERR_CONFIG_PACK_NOT_FOUND"), "{stderr}");
    assert!(stderr.contains("did you mean `greeting`?"), "{stderr}");
    assert!(stderr.contains("sc-compose examples list"), "{stderr}");
    let json = sc_compose()
        .args(["examples", "greetng", "--json"])
        .env("SC_COMPOSE_DATA_DIR", &fixture.path)
        .bounded_output()
        .unwrap();
    let value: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert!(
        value["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("did you mean `greeting`?"),
        "{value}"
    );
}

#[test]
fn extension_suggestions_use_filenames_and_mentions_without_hard_coded_pack_names() {
    for (file, content) in [
        ("novel.xhtml.j2", "<p>example</p>"),
        ("novel.html.j2", "<!-- supported: .html.j2/.xhtml.j2 -->"),
    ] {
        let fixture = TempFixture::new("examples-extension");
        write_file(&fixture.path.join("examples").join(file), content);
        let result = sc_compose()
            .args(["examples", "xhtml"])
            .env("SC_COMPOSE_DATA_DIR", &fixture.path)
            .bounded_output()
            .unwrap();
        assert_eq!(result.status.code(), Some(3), "{result:?}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains("did you mean `novel`?"), "{stderr}");
    }
}

#[test]
fn suggestion_ties_use_name_order_and_unrelated_names_get_only_list_recovery() {
    let fixture = TempFixture::new("examples-suggestion-tie");
    for name in ["zebra", "alpha"] {
        write_file(
            &fixture
                .path
                .join("examples")
                .join(format!("{name}.xhtml.j2")),
            "markup",
        );
    }
    let tied = sc_compose()
        .args(["examples", "xhtml"])
        .env("SC_COMPOSE_DATA_DIR", &fixture.path)
        .bounded_output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&tied.stderr).contains("did you mean `alpha`?"),
        "{tied:?}"
    );
    let unknown = sc_compose()
        .args(["examples", "unrelated-name"])
        .env("SC_COMPOSE_DATA_DIR", &fixture.path)
        .bounded_output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(3), "{unknown:?}");
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(!stderr.contains("did you mean"), "{stderr}");
    assert!(stderr.contains("sc-compose examples list"), "{stderr}");
}
