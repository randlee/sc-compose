//! Drift-verification entrypoints for deployed template outputs.

use std::fs;
use std::path::Path;

use similar::TextDiff;

use crate::diagnostics::DiagnosticCode;
use crate::error::{ConfigError, RecoveryHint, RecoveryHintKind};
use crate::observer::{CompositionObserver, NoopObserver, VerifyEndEvent, VerifyStartEvent};
use crate::types::{ComposeRequest, VerifyResult};
use crate::{ComposeError, compose_with_observer};

/// Verify that a deployed file matches the rendered output of a compose request.
///
/// # Errors
///
/// Returns [`ComposeError`] when rendering fails or when the deployed file
/// cannot be read.
pub fn verify(
    request: &ComposeRequest,
    deployed_path: impl AsRef<Path>,
) -> Result<VerifyResult, ComposeError> {
    let mut observer = NoopObserver;
    verify_with_observer(request, deployed_path, &mut observer)
}

/// Verify that a deployed file matches the rendered output while emitting
/// observer callbacks.
///
/// # Errors
///
/// Returns [`ComposeError`] when rendering fails or when the deployed file
/// cannot be read.
pub fn verify_with_observer(
    request: &ComposeRequest,
    deployed_path: impl AsRef<Path>,
    observer: &mut dyn CompositionObserver,
) -> Result<VerifyResult, ComposeError> {
    observer.on_verify_start(&VerifyStartEvent);

    let result = (|| {
        let compose_result = compose_with_observer(request, observer)?;
        let deployed_path = deployed_path.as_ref();
        let deployed_text = read_deployed_text(deployed_path)?;
        let diff = render_diff(
            deployed_path,
            &compose_result.resolve_result.resolved_path,
            &deployed_text,
            &compose_result.rendered_text,
        );

        Ok(VerifyResult {
            clean: diff.is_none(),
            resolved_template_path: compose_result.resolve_result.resolved_path,
            deployed_path: deployed_path.to_path_buf(),
            rendered_text: compose_result.rendered_text,
            deployed_text,
            diff,
            warnings: compose_result.warnings,
        })
    })();

    observer.on_verify_end(&VerifyEndEvent);
    result
}

fn read_deployed_text(deployed_path: &Path) -> Result<String, ComposeError> {
    fs::read_to_string(deployed_path).map_err(|error| {
        let code = if error.kind() == std::io::ErrorKind::NotFound {
            DiagnosticCode::ErrResolveNotFound
        } else if error.kind() == std::io::ErrorKind::InvalidData {
            DiagnosticCode::ErrConfigRead
        } else {
            DiagnosticCode::ErrConfigParse
        };
        let message = if code == DiagnosticCode::ErrResolveNotFound {
            format!("deployed file not found: {}", deployed_path.display())
        } else if code == DiagnosticCode::ErrConfigRead {
            format!(
                "deployed file is not valid UTF-8 text: {}",
                deployed_path.display()
            )
        } else {
            format!("failed to read deployed file: {}", deployed_path.display())
        };
        ConfigError::new(code, message)
            .with_source(error)
            .with_recovery_hint(RecoveryHint::new(RecoveryHintKind::InspectPath {
                path: deployed_path.to_path_buf(),
            }))
            .into()
    })
}

fn render_diff(
    deployed_path: &Path,
    template_path: &Path,
    deployed_text: &str,
    rendered_text: &str,
) -> Option<String> {
    let normalized_deployed = normalize_text_for_compare(deployed_text);
    let normalized_rendered = normalize_text_for_compare(rendered_text);
    if normalized_deployed == normalized_rendered {
        return None;
    }

    let diff = TextDiff::from_lines(&normalized_deployed, &normalized_rendered);
    Some(
        diff.unified_diff()
            .header(
                &deployed_path.display().to_string(),
                &format!("rendered({})", template_path.display()),
            )
            .to_string(),
    )
}

fn normalize_text_for_compare(text: &str) -> String {
    text.lines().collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::observer::{CompositionObserver, VerifyEndEvent, VerifyStartEvent};
    use crate::types::{
        ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, PassConfig, VariableName,
    };
    use crate::{DiagnosticCode, verify, verify_with_observer};

    #[derive(Default)]
    struct CapturingObserver {
        verify_started: usize,
        verify_ended: usize,
    }

    impl CompositionObserver for CapturingObserver {
        fn on_verify_start(&mut self, _event: &VerifyStartEvent) {
            self.verify_started += 1;
        }

        fn on_verify_end(&mut self, _event: &VerifyEndEvent) {
            self.verify_ended += 1;
        }
    }

    #[test]
    fn verify_returns_clean_when_render_matches_deployed() {
        let root = temp_root("verify-clean");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
        );
        write_file(&root.join("deployed.md"), "hello world");

        let result = verify(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            root.join("deployed.md"),
        )
        .unwrap();

        assert!(result.clean);
        assert!(result.diff.is_none());
    }

    #[test]
    fn verify_returns_diff_when_render_does_not_match_deployed() {
        let root = temp_root("verify-drift");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
        );
        write_file(&root.join("deployed.md"), "hello drift\n");

        let result = verify(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            root.join("deployed.md"),
        )
        .unwrap();

        assert!(!result.clean);
        let diff = result.diff.unwrap();
        assert!(diff.contains("--- "));
        assert!(diff.contains("+++ "));
        assert!(diff.contains("-hello drift"));
        assert!(diff.contains("+hello world"));
    }

    #[test]
    fn verify_reports_multi_pass_drift_without_expanding_higher_brace_literals() {
        let root = temp_root("verify-multi-pass-drift");
        write_file(
            &root.join("template.md.j2"),
            "---\npass: 1\ndefaults:\n  team: platform\n---\n---\npass: 2\ndefaults:\n  feature: nested templates\n---\nrelease {{{ feature }}} for {{ team }}\n",
        );
        write_file(
            &root.join("deployed.md"),
            "release nested templates for wrong-team\n",
        );

        let mut pass_two_defaults = BTreeMap::new();
        pass_two_defaults.insert(
            VariableName::new("feature").unwrap(),
            "nested templates".into(),
        );
        let policy = ComposePolicy {
            passes: vec![
                PassConfig::default(),
                PassConfig {
                    pass_number: 2,
                    defaults: pass_two_defaults,
                    ..PassConfig::default()
                },
            ],
            ..ComposePolicy::default()
        };

        let result = verify(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy,
            },
            root.join("deployed.md"),
        )
        .unwrap();

        assert!(!result.clean);
        assert_eq!(
            result.rendered_text,
            "release nested templates for platform"
        );
        let diff = result.diff.unwrap();
        assert!(diff.contains("-release nested templates for wrong-team"));
        assert!(diff.contains("+release nested templates for platform"));
        assert!(!diff.contains("{% raw %}"));
        assert!(!diff.contains("{{{ feature }}}"));
    }

    #[test]
    fn verify_with_observer_emits_start_and_end_events() {
        let root = temp_root("verify-observer");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
        );
        write_file(&root.join("deployed.md"), "hello world");

        let mut observer = CapturingObserver::default();
        let _ = verify_with_observer(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            root.join("deployed.md"),
            &mut observer,
        )
        .unwrap();

        assert_eq!(observer.verify_started, 1);
        assert_eq!(observer.verify_ended, 1);
    }

    #[test]
    fn verify_normalizes_crlf_line_endings_when_comparing() {
        let root = temp_root("verify-crlf");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}\nnext line",
        );
        write_bytes(&root.join("deployed.md"), b"hello world\r\nnext line");

        let result = verify(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            root.join("deployed.md"),
        )
        .unwrap();

        assert!(result.clean);
        assert!(result.diff.is_none());
    }

    #[test]
    fn verify_invalid_utf8_uses_read_failure_code() {
        let root = temp_root("verify-invalid-utf8");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
        );
        write_bytes(&root.join("deployed.md"), &[0xff, 0xfe, 0xfd]);

        let error = verify(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            root.join("deployed.md"),
        )
        .unwrap_err();

        match error {
            crate::ComposeError::Config(config) => {
                assert_eq!(config.code(), DiagnosticCode::ErrConfigRead);
                assert!(config.message().contains("not valid UTF-8 text"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn verify_treats_trailing_newline_difference_as_clean() {
        let root = temp_root("verify-trailing-newline");
        write_file(
            &root.join("template.md.j2"),
            "---\ndefaults:\n  name: world\n---\nhello {{ name }}\n",
        );
        write_file(&root.join("deployed.md"), "hello world\n");

        let result = verify(
            &ComposeRequest {
                runtime: None,
                mode: ComposeMode::File {
                    template_path: PathBuf::from("template.md.j2"),
                },
                root: ConfiningRoot::new(&root).unwrap(),
                vars_input: BTreeMap::default(),
                vars_env: BTreeMap::default(),
                vars_defaults: BTreeMap::default(),
                guidance_block: None,
                user_prompt: None,
                policy: ComposePolicy::default(),
            },
            root.join("deployed.md"),
        )
        .unwrap();

        assert!(result.clean);
        assert!(result.diff.is_none());
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "sc-composer-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &std::path::Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn write_bytes(path: &std::path::Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}
