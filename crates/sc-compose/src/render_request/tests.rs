use super::blocks::{read_block_pair_with_extra_stdin_reads, read_optional_block_with};
use super::mode::{build_mode, build_root};
use super::request::{build_multi_pass_request, build_named_request, build_request};
use super::vars::{collect_pass_union, load_cli_and_file_vars, load_prefixed_env_vars};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sc_composer::{
    BUILTIN_VARIABLE_NAMES, ComposeMode, InputValue, PassConfig, ProfileKind, RuntimeKind,
    UnknownVariablePolicy, VariableName,
};

use crate::cli::{
    Ai, CommonArgs, InputArgs, Kind, Mode, PassInputArgs, RenderBehaviorArgs, UnknownVarMode,
};
use crate::template_store::TemplatePack;

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sc-compose-render-request-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn input_args() -> InputArgs {
    InputArgs {
        vars: Vec::new(),
        var_file: Vec::new(),
        env_prefix: None,
        strict: false,
        unknown_var_mode: UnknownVarMode::Ignore,
    }
}

fn common_args(root: &Path) -> CommonArgs {
    CommonArgs {
        mode: Mode::File,
        kind: Kind::Agent,
        agent: None,
        agent_type: None,
        runtime: None,
        input: input_args(),
        root: root.to_path_buf(),
        file: Some(PathBuf::from("template.md.j2")),
        json_escape_mode: None,
    }
}

#[test]
fn compose_mode_maps_file_and_profile_requests() {
    let root = temp_root("compose-mode");

    let file_args = common_args(&root);
    assert_eq!(
        build_mode(&file_args).unwrap(),
        ComposeMode::File {
            template_path: PathBuf::from("template.md.j2"),
        }
    );

    let mut profile_args = common_args(&root);
    profile_args.mode = Mode::Profile;
    profile_args.kind = Kind::Skill;
    profile_args.agent = Some("skill.rx".to_owned());
    profile_args.file = None;

    assert_eq!(
        build_mode(&profile_args).unwrap(),
        ComposeMode::Profile {
            kind: ProfileKind::Skill,
            name: sc_composer::ProfileName::new("skill.rx").unwrap(),
        }
    );
}

#[test]
fn confining_root_canonicalizes_root_path() {
    let root = temp_root("confining-root");
    let nested = root.join("nested");
    fs::create_dir_all(&nested).unwrap();

    let confining = build_root(&nested).unwrap();

    assert_eq!(confining.as_path(), nested.canonicalize().unwrap());
}

#[test]
fn read_block_pair_rejects_conflicting_sources_and_double_stdin() {
    let input = InputArgs {
        var_file: vec!["-".to_owned()],
        ..input_args()
    };
    let render = RenderBehaviorArgs {
        guidance_file: Some("-".to_owned()),
        ..RenderBehaviorArgs::default()
    };
    let error = read_block_pair_with_extra_stdin_reads(&input, &render, 0).unwrap_err();
    assert_eq!(
        error.diagnostic_code,
        Some(sc_composer::DiagnosticCode::ErrRenderStdinDoubleRead)
    );

    let render = RenderBehaviorArgs {
        guidance: Some("inline".to_owned()),
        guidance_file: Some("guidance.txt".to_owned()),
        ..RenderBehaviorArgs::default()
    };
    let error = read_block_pair_with_extra_stdin_reads(&input_args(), &render, 0).unwrap_err();
    assert_eq!(
        error.diagnostic_code,
        Some(sc_composer::DiagnosticCode::ErrConfigParse)
    );
    assert!(error.error.to_string().contains("mutually exclusive"));
}

#[test]
fn read_optional_block_accepts_inline_file_and_stdin_success_paths() {
    assert_eq!(
        read_optional_block_with(
            Some("inline".to_owned()),
            Some("ignored.txt"),
            || Ok("stdin".to_owned()),
            |_path| Ok("file".to_owned()),
        )
        .unwrap(),
        Some("inline".to_owned())
    );

    assert_eq!(
        read_optional_block_with(
            None,
            Some("guidance.txt"),
            || Ok(String::new()),
            |_path| { Ok("from-file".to_owned()) }
        )
        .unwrap(),
        Some("from-file".to_owned())
    );

    assert_eq!(
        read_optional_block_with(
            None,
            Some("-"),
            || Ok("from-stdin".to_owned()),
            |_path| { Ok(String::new()) }
        )
        .unwrap(),
        Some("from-stdin".to_owned())
    );
}

#[test]
fn load_vars_merges_inline_and_var_file_inputs() {
    let root = temp_root("load-vars");
    let vars_path = root.join("vars.yaml");
    write_file(&vars_path, "name: yaml-world\ncount: 2\n");

    let args = InputArgs {
        vars: vec![("name".to_owned(), "inline-world".to_owned())],
        var_file: vec![vars_path.display().to_string()],
        ..input_args()
    };

    let vars = load_cli_and_file_vars(&args).unwrap();

    assert_eq!(
        vars.get(&VariableName::new("name".to_owned()).unwrap()),
        Some(&InputValue::String("yaml-world".to_owned()))
    );
    assert_eq!(
        vars.get(&VariableName::new("count".to_owned()).unwrap()),
        Some(&serde_json::json!(2))
    );
}

#[test]
fn load_env_uses_prefix_and_preserves_builtin_casing() {
    let (source_key, source_value) = std::env::vars()
        .find(|(key, _)| key == "PATH")
        .or_else(|| std::env::vars().next())
        .expect("process environment must contain at least one variable");
    let prefix = source_key
        .chars()
        .next()
        .map(|ch| ch.to_string())
        .unwrap_or_default();
    let trimmed = source_key.strip_prefix(&prefix).unwrap_or(&source_key);
    let expected_name = if BUILTIN_VARIABLE_NAMES.contains(&trimmed) {
        trimmed.to_owned()
    } else {
        trimmed.to_ascii_lowercase()
    };

    let args = InputArgs {
        env_prefix: Some(prefix),
        ..input_args()
    };

    let vars = load_prefixed_env_vars(&args).unwrap();

    assert_eq!(
        vars.get(&VariableName::new(expected_name).unwrap()),
        Some(&InputValue::String(source_value))
    );
}

#[test]
fn collect_pass_union_reads_inline_and_var_file_values() {
    let root = temp_root("collect-pass-union");
    let vars_path = root.join("pass.json");
    write_file(&vars_path, "{ \"branch\": \"develop\" }\n");

    let pass_inputs = vec![
        PassInputArgs {
            pass_number: 0,
            vars: vec![("name".to_owned(), "world".to_owned())],
            var_files: vec![vars_path.display().to_string()],
        },
        PassInputArgs {
            pass_number: 2,
            vars: vec![("owner".to_owned(), "team".to_owned())],
            var_files: Vec::new(),
        },
    ];

    let vars = collect_pass_union(&pass_inputs).unwrap();

    assert_eq!(
        vars.get(&VariableName::new("name".to_owned()).unwrap()),
        Some(&InputValue::String("world".to_owned()))
    );
    assert_eq!(
        vars.get(&VariableName::new("branch".to_owned()).unwrap()),
        Some(&InputValue::String("develop".to_owned()))
    );
    assert_eq!(
        vars.get(&VariableName::new("owner".to_owned()).unwrap()),
        Some(&InputValue::String("team".to_owned()))
    );
}

#[test]
fn build_request_maps_runtime_mode_and_policy() {
    let root = temp_root("build-request");
    let vars_path = root.join("vars.json");
    write_file(&vars_path, "{ \"name\": \"json-world\" }\n");

    let mut args = common_args(&root);
    args.runtime = Some(Ai::Gemini);
    args.input.strict = true;
    args.input.unknown_var_mode = UnknownVarMode::Warn;
    args.input.var_file = vec![vars_path.display().to_string()];

    let request = build_request(
        &args,
        (Some("guidance".to_owned()), Some("prompt".to_owned())),
        BTreeMap::new(),
    )
    .unwrap();

    assert_eq!(request.runtime, Some(RuntimeKind::Gemini));
    assert_eq!(
        request.mode,
        ComposeMode::File {
            template_path: PathBuf::from("template.md.j2"),
        }
    );
    assert_eq!(request.guidance_block.as_deref(), Some("guidance"));
    assert_eq!(request.user_prompt.as_deref(), Some("prompt"));
    assert!(request.policy.strict_undeclared_variables);
    assert_eq!(
        request.policy.unknown_variable_policy,
        UnknownVariablePolicy::Warn
    );
    assert_eq!(
        request
            .vars_input
            .get(&VariableName::new("name".to_owned()).unwrap()),
        Some(&InputValue::String("json-world".to_owned()))
    );
}

#[test]
fn build_request_maps_hermes_runtime() {
    let root = temp_root("build-request-hermes");
    let mut args = common_args(&root);
    args.runtime = Some(Ai::Hermes);

    let request = build_request(&args, (None, None), BTreeMap::new()).unwrap();

    assert_eq!(request.runtime, Some(RuntimeKind::Hermes));
}

#[test]
fn build_multi_pass_request_normalizes_pass_zero_and_builds_union() {
    let root = temp_root("build-multi-pass-request");
    let vars_path = root.join("pass.yaml");
    write_file(&vars_path, "owner: qa\n");

    let pass_inputs = vec![
        PassInputArgs {
            pass_number: 0,
            vars: vec![("name".to_owned(), "world".to_owned())],
            var_files: vec![vars_path.display().to_string()],
        },
        PassInputArgs {
            pass_number: 3,
            vars: vec![("branch".to_owned(), "develop".to_owned())],
            var_files: Vec::new(),
        },
    ];

    let request = build_multi_pass_request(
        &common_args(&root),
        (None, None),
        BTreeMap::new(),
        &pass_inputs,
    )
    .unwrap();

    assert_eq!(
        request
            .vars_input
            .get(&VariableName::new("owner".to_owned()).unwrap()),
        Some(&InputValue::String("qa".to_owned()))
    );
    assert_eq!(request.policy.passes.len(), 2);
    assert_eq!(
        request.policy.passes[0],
        PassConfig {
            pass_number: sc_composer::types::default_pass_number(),
            defaults: BTreeMap::from([
                (
                    VariableName::new("name".to_owned()).unwrap(),
                    InputValue::String("world".to_owned()),
                ),
                (
                    VariableName::new("owner".to_owned()).unwrap(),
                    InputValue::String("qa".to_owned()),
                ),
            ]),
            ..PassConfig::default()
        }
    );
    assert_eq!(request.policy.passes[1].pass_number, 3);
}

#[test]
fn build_named_request_uses_pack_root_template_and_defaults() {
    let root = temp_root("build-named-request");
    let pack = TemplatePack {
        root: root.clone(),
        template_path: root.join("template.md.j2"),
        input_defaults: BTreeMap::from([(
            VariableName::new("name".to_owned()).unwrap(),
            InputValue::String("pack-default".to_owned()),
        )]),
    };

    let request = build_named_request(
        &pack,
        &InputArgs {
            unknown_var_mode: UnknownVarMode::Error,
            ..input_args()
        },
        (Some("guidance".to_owned()), None),
    )
    .unwrap();

    assert_eq!(request.runtime, None);
    assert_eq!(
        request.mode,
        ComposeMode::File {
            template_path: pack.template_path.clone(),
        }
    );
    assert_eq!(request.root.as_path(), root.canonicalize().unwrap());
    assert_eq!(
        request
            .vars_defaults
            .get(&VariableName::new("name".to_owned()).unwrap()),
        Some(&InputValue::String("pack-default".to_owned()))
    );
    assert_eq!(
        request.policy.unknown_variable_policy,
        UnknownVariablePolicy::Error
    );
}
