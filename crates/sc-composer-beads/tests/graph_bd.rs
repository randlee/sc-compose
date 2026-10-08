//! Production bd graph use cases from ADR-0023; opt in with `BD_EXECUTABLE`.
use sc_composer_beads::{
    BEADS_SCHEMA_V1, BeadComposeReceipt, BeadComposeRequest, BeadGraphMode, BeadId, BeadNodeAction,
    BeadOperation, BeadOutcome, BeadPourMode, GraphRef, PourAuthorization, execute_bead_request,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

static SERIAL: Mutex<()> = Mutex::new(());
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
const FORMULA: &str = "formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"build\"\ntitle = \"Build {{literal}}\"\n[[steps]]\nid = \"verify\"\ntitle = \"Verify\"\nneeds = [\"build\"]\n[[steps]]\nid = \"publish\"\ntitle = \"Publish\"\nneeds = [\"verify\"]\n";
struct Workspace {
    root: PathBuf,
    bd: PathBuf,
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Workspace {
    fn new(binary: &Path) -> Self {
        let root = std::env::temp_dir().join(format!(
            "sc-graph-bd-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("build")).expect("workspace");
        let ws = Self {
            root: fs::canonicalize(root).expect("canonical root"),
            bd: binary.to_path_buf(),
        };
        ws.command(&[
            "init",
            "--non-interactive",
            "--quiet",
            "--skip-agents",
            "--skip-hooks",
            "--prefix",
            "graph",
        ]);
        fs::write(ws.root.join("release.formula.toml.j2"), FORMULA).expect("template");
        ws
    }
    fn command(&self, args: &[&str]) -> String {
        let mut child = Command::new(&self.bd)
            .args(args)
            .current_dir(&self.root)
            .env("BEADS_NO_DAEMON", "1")
            .env("BEADS_DIR", self.root.join(".beads"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("bd spawn");
        let started = Instant::now();
        loop {
            if child.try_wait().expect("bd status").is_some() {
                break;
            }
            if started.elapsed() > Duration::from_secs(30) {
                let _ = child.kill();
                let _ = child.wait();
                panic!("bd command timed out: {args:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let out = child.wait_with_output().expect("bd output");
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).expect("UTF-8")
    }
    fn json(&self, args: &[&str]) -> Value {
        serde_json::from_str(&self.command(args)).expect("bd JSON")
    }
    fn parent(&self) -> BeadId {
        BeadId::new(
            self.json(&["create", "Parent", "--type", "epic", "--json"])["id"]
                .as_str()
                .expect("created id"),
        )
        .expect("id")
    }
    fn request(&self, operation: BeadOperation, parent: Option<BeadId>) -> BeadComposeRequest {
        BeadComposeRequest {
            schema: BEADS_SCHEMA_V1.into(),
            operation,
            working_directory: self.root.clone(),
            template: self.root.join("release.formula.toml.j2"),
            rendered_formula: self.root.join("build/release.formula.toml"),
            compose_variables: serde_json::Map::new(),
            formula_name: Some(
                sc_composer_beads::FormulaName::new("release").expect("formula name"),
            ),
            bead_variables: BTreeMap::new(),
            bd_executable: Some(self.bd.clone()),
            pour_authorization: Some(PourAuthorization::CreatePersistentBeads),
            ref_: parent
                .as_ref()
                .map(|_| GraphRef::new("release").expect("ref")),
            parent,
            relations: Vec::new(),
        }
    }
    fn run(&self, req: &BeadComposeRequest) -> BeadComposeReceipt {
        assert_eq!(req.working_directory, self.root);
        execute_bead_request(req).expect("valid request")
    }
    fn success(&self, req: &BeadComposeRequest) -> BeadComposeReceipt {
        let receipt = self.run(req);
        assert_eq!(receipt.outcome, BeadOutcome::Succeeded, "{receipt:#?}");
        receipt
    }
    fn snapshot(&self) -> Value {
        let beads = self.json(&["list", "--all", "-n", "0", "--json"]);
        let mut edges = BTreeMap::new();
        for bead in beads.as_array().expect("beads") {
            let id = bead["id"].as_str().expect("bead id");
            edges.insert(id, self.json(&["dep", "list", id, "--json"]));
        }
        json!({"beads": beads, "edges": edges})
    }
    fn refuse(&self, req: &BeadComposeRequest, code: &str) {
        let before = self.snapshot();
        let result = self.run(req);
        assert_eq!(
            result.outcome,
            BeadOutcome::Refused { code: code.into() },
            "{result:#?}"
        );
        assert_eq!(self.snapshot(), before, "refusal changes no beads or edges");
    }
}
fn with_workspace(test: impl FnOnce(&Workspace)) {
    let Some(binary) = std::env::var_os("BD_EXECUTABLE") else {
        eprintln!("skipping graph integration: BD_EXECUTABLE not configured");
        return;
    };
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    test(&Workspace::new(Path::new(&binary)));
}
#[test]
fn uc1_render_and_pour_outside_registry() {
    with_workspace(|w| {
        let r = w.request(BeadOperation::Render, None);
        w.success(&r);
        let preview = w.success(&w.request(BeadOperation::PreviewPour, None));
        assert_eq!(preview.pour_mode, Some(BeadPourMode::Graph));
        assert!(preview.graph.expect("graph").ids.is_empty());
        let applied = w.success(&w.request(BeadOperation::Pour, None));
        let graph = applied.graph.expect("graph");
        assert_eq!(graph.ids.len(), 3);
        assert_eq!(graph.mode, BeadGraphMode::Pour);
        let root = w.json(&[
            "show",
            graph.parent.as_ref().expect("root").as_str(),
            "--json",
        ]);
        assert_eq!(root[0]["issue_type"], "molecule");
    });
}
#[test]
fn uc2_attach_children_directly_under_parent() {
    with_workspace(|w| {
        let parent = w.parent();
        let r = w.request(BeadOperation::Attach, Some(parent.clone()));
        let g = w.success(&r).graph.expect("graph");
        assert_eq!(g.ids.len(), 3);
        let children = w.json(&["children", parent.as_str(), "--json"]);
        assert_eq!(children.as_array().expect("children").len(), 3);
        assert_eq!(
            g.ids[&sc_composer_beads::StepId::new("build").expect("step")].as_str(),
            format!("{parent}.release-build")
        );
    });
}
#[test]
fn uc3_preview_creates_no_beads() {
    with_workspace(|w| {
        let r = w.request(BeadOperation::PreviewAttach, Some(w.parent()));
        let before = w.snapshot();
        let g = w.success(&r).graph.expect("graph");
        assert!(g.nodes.iter().all(|n| n.action == BeadNodeAction::Create));
        assert!(g.plan_path.expect("plan").is_file());
        assert_eq!(w.snapshot(), before);
    });
}
#[test]
fn uc4_rerun_preserves_closed_annotated_beads() {
    with_workspace(|w| {
        let r = w.request(BeadOperation::Attach, Some(w.parent()));
        let g = w.success(&r).graph.expect("graph");
        let id = g.ids.values().next().expect("id");
        w.command(&[
            "update",
            id.as_str(),
            "--notes",
            "retained evidence",
            "--assignee",
            "tester",
        ]);
        w.command(&["close", id.as_str(), "--actor", "tester"]);
        let before = w.snapshot();
        let plan = g.plan_path.expect("original plan");
        let bytes = fs::read(&plan).expect("plan bytes");
        let next = w.success(&r).graph.expect("graph");
        assert!(next.plan_path.is_none());
        assert!(
            next.nodes
                .iter()
                .all(|n| n.action == BeadNodeAction::Existing)
        );
        assert_eq!(w.snapshot(), before);
        assert_eq!(fs::read(plan).expect("unchanged plan"), bytes);
    });
}
#[test]
fn uc5_resume_recreates_only_missing_child() {
    with_workspace(|w| {
        let r = w.request(BeadOperation::Attach, Some(w.parent()));
        let g = w.success(&r).graph.expect("graph");
        let id = g.ids[&sc_composer_beads::StepId::new("publish").expect("step")].clone();
        w.command(&["delete", id.as_str(), "--force"]);
        let next = w.success(&r).graph.expect("graph");
        assert_eq!(
            next.nodes
                .iter()
                .filter(|n| n.action == BeadNodeAction::Created)
                .count(),
            1
        );
    });
}
#[test]
fn uc6_changed_revision_scope_and_relations_are_refused() {
    with_workspace(|w| {
        let mut r = w.request(BeadOperation::Attach, Some(w.parent()));
        w.success(&r);
        fs::write(&r.template, FORMULA.replace("Verify", "Verify changed"))
            .expect("changed formula");
        w.refuse(&r, "BEADS_GRAPH_CONFLICT");
        fs::write(&r.template, FORMULA).expect("restore template");
        r.compose_variables.insert("ref".into(), json!("other"));
        w.refuse(&r, "BEADS_GRAPH_SCOPE_MISMATCH");
        r.compose_variables.clear();
        let external = w.parent();
        r.relations = serde_json::from_value(
            json!([{"from":"step:build","to":format!("bead:{external}"),"type":"related"}]),
        )
        .expect("relation");
        w.refuse(&r, "BEADS_GRAPH_CONFLICT");
    });
}
#[test]
fn uc7_relations_in_both_directions_preserve_external_fields() {
    with_workspace(|w| {
        let mut r = w.request(BeadOperation::Attach, Some(w.parent()));
        let external = w.parent();
        r.relations=serde_json::from_value(json!([{"from":"step:build","to":format!("bead:{external}"),"type":"related"},{"from":format!("bead:{external}"),"to":"step:verify","type":"validates"}])).expect("relations");
        w.command(&[
            "update",
            external.as_str(),
            "--notes",
            "unchanged",
            "--assignee",
            "tester",
        ]);
        let before = w.json(&["show", external.as_str(), "--json"]);
        w.success(&r);
        let after = w.json(&["show", external.as_str(), "--json"]);
        for field in ["title", "status", "notes", "assignee"] {
            assert_eq!(before[0][field], after[0][field]);
        }
        w.success(&r);
    });
}
#[test]
fn uc8_template_loop_and_hyphenated_ref() {
    with_workspace(|w| {
        let mut r = w.request(BeadOperation::Attach, Some(w.parent()));
        r.ref_ = Some(GraphRef::new("qa1-f1-r1").expect("ref"));
        fs::write(&r.template,"formula = \"batch\"\nversion = 1\ntype = \"workflow\"\n{% for i in range(1, 11) %}\n[[steps]]\nid = \"item_{{{ i }}}\"\ntitle = \"Item {{{ i }}}\"\n{% if i > 1 %}needs = [\"item_{{{ i - 1 }}}\"]\n{% endif %}{% endfor %}").expect("loop template");
        let g = w.success(&r).graph.expect("graph");
        assert_eq!(g.ids.len(), 10);
        assert!(
            g.ids
                .values()
                .all(|id| id.as_str().contains(".qa1-f1-r1-item_"))
        );
    });
}
#[test]
fn uc9_attach_one_workflow_under_three_parents() {
    with_workspace(|w| {
        let parents = [w.parent(), w.parent(), w.parent()];
        for parent in &parents {
            w.success(&w.request(BeadOperation::Attach, Some(parent.clone())));
        }
        let before = w.snapshot();
        for parent in parents {
            w.success(&w.request(BeadOperation::Attach, Some(parent)));
        }
        assert_eq!(w.snapshot(), before);
    });
}
#[test]
fn uc10_receipt_maps_every_generated_id_and_parent() {
    with_workspace(|w| {
        let g = w
            .success(&w.request(BeadOperation::Pour, None))
            .graph
            .expect("graph");
        for (step, id) in &g.ids {
            let row = w.json(&["show", id.as_str(), "--json"]);
            assert_eq!(
                row[0]["metadata"]["sc_compose_graph"]["step"],
                step.as_str()
            );
            assert_eq!(
                row[0]["parent"],
                g.parent.as_ref().expect("parent").as_str()
            );
        }
    });
}
#[test]
fn inherited_steps_attach_and_bd_loop_is_refused() {
    with_workspace(|w| {
        fs::create_dir_all(w.root.join(".beads/formulas")).expect("registry");
        fs::write(
            w.root.join(".beads/formulas/base.formula.json"),
            include_str!("fixtures/beads/graph/base.formula.json"),
        )
        .expect("base");
        let r = w.request(BeadOperation::Attach, Some(w.parent()));
        fs::write(&r.template,"formula = \"release\"\nversion = 1\ntype = \"workflow\"\nextends = [\"base\"]\n[[steps]]\nid = \"build\"\ntitle = \"Build\"\n").expect("extends");
        assert_eq!(w.success(&r).graph.expect("graph").ids.len(), 2);
        fs::write(&r.template,"formula = \"release\"\nversion = 1\ntype = \"workflow\"\n[[steps]]\nid = \"loop\"\ntitle = \"Loop\"\n[steps.loop]\ncount = 2\n[[steps.loop.body]]\nid = \"check\"\ntitle = \"Check\"\n").expect("loop");
        w.refuse(&r, "BEADS_GRAPH_ID_INVALID");
    });
}

#[test]
fn refusals_preserve_beads_and_edges() {
    with_workspace(|w| {
        let parent = w.parent();
        let mut r = w.request(BeadOperation::Attach, Some(parent.clone()));
        r.parent = Some(BeadId::new("graph-missing").expect("id"));
        w.refuse(&r, "BEADS_GRAPH_PARENT_NOT_FOUND");
        r.parent = Some(parent);
        r.relations = serde_json::from_value(json!([
            {"from":"step:build","to":"step:build","type":"blocks"}
        ]))
        .expect("relation");
        w.refuse(&r, "BEADS_GRAPH_RELATION_INVALID");
        r.relations.clear();
        fs::write(
            &r.template,
            FORMULA.replace(
                "version = 1",
                "version = 1\n[vars.release]\ndefault = \"v1\"",
            ),
        )
        .expect("unsupported formula");
        w.refuse(&r, "BEADS_GRAPH_FORMULA_UNSUPPORTED");
        fs::write(&r.template, FORMULA).expect("restore formula");
        let graph = w.success(&r).graph.expect("graph");
        let build = graph.ids[&sc_composer_beads::StepId::new("build").expect("step")].as_str();
        let verify = graph.ids[&sc_composer_beads::StepId::new("verify").expect("step")].as_str();
        w.command(&["dep", "remove", verify, build]);
        w.refuse(&r, "BEADS_GRAPH_EDGE_MISSING");
        w.command(&["dep", "add", verify, build, "--type", "related"]);
        w.refuse(&r, "BEADS_GRAPH_EDGE_CONFLICT");
        w.command(&[
            "update",
            build,
            "--metadata",
            r#"{"sc_compose_graph":null}"#,
        ]);
        w.refuse(&r, "BEADS_GRAPH_CONFLICT");
    });
}
