//! Recursive include expansion and confinement enforcement.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::ComposeError;
use crate::frontmatter::Frontmatter;
use crate::types::{ComposePolicy, ConfiningRoot};

mod directive;
mod expansion;
mod fingerprint;
mod path;

pub use fingerprint::CompositionFingerprint;

/// Expanded include graph returned from the include engine.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExpandedTemplate {
    /// Final text with all includes expanded in deterministic order.
    pub text: String,
    /// Files visited during include expansion, in first-seen order.
    pub resolved_files: Vec<PathBuf>,
    /// Parsed frontmatter values keyed by the file they came from.
    pub frontmatters: Vec<(PathBuf, Vec<Frontmatter>)>,
    /// Include chain recorded for each resolved file.
    pub include_chains: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// Raw source text keyed by each file visited during include expansion.
    pub source_texts: BTreeMap<PathBuf, String>,
    /// Deterministic source-composition identity and its inspectable manifest.
    pub composition_fingerprint: Option<CompositionFingerprint>,
}

/// Expand `@<path>` directives starting from the provided template path.
///
/// # Errors
///
/// Returns [`ComposeError`] when include resolution fails, when the include
/// graph exceeds the configured depth, or when an include escapes the allowed
/// roots.
pub fn expand_includes(
    template_path: impl AsRef<Path>,
    root: &ConfiningRoot,
    policy: &ComposePolicy,
) -> Result<ExpandedTemplate, ComposeError> {
    let (text, state) = expansion::expand(template_path, root, policy)?;
    let composition_fingerprint = state.composition_fingerprint()?;

    Ok(ExpandedTemplate {
        text,
        resolved_files: state.resolved_files,
        frontmatters: state.frontmatters,
        include_chains: state.include_chains,
        source_texts: state.source_texts,
        composition_fingerprint: Some(composition_fingerprint),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::ops::Deref;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::types::{ComposePolicy, ConfiningRoot, IncludeDepth};
    use crate::{ComposeError, DiagnosticCode};

    use super::expand_includes;

    #[test]
    fn one_level_and_multi_level_includes_have_ordered_manifest() {
        let root = temp_root("include_success");
        write_file(&root.join("root.md.j2"), "top\n@<partials/one.md>\n");
        write_file(&root.join("partials/one.md"), "middle\n@<two.md>\n");
        write_file(&root.join("partials/two.md"), "bottom\n");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert!(expanded.text.contains("top"));
        assert!(expanded.text.contains("middle"));
        assert!(expanded.text.contains("bottom"));
        assert_eq!(expanded.resolved_files.len(), 3);
        let fingerprint = expanded.composition_fingerprint.unwrap();
        assert_eq!(fingerprint.manifest.nodes.len(), 3);
        assert_eq!(fingerprint.manifest.edges.len(), 2);
        assert_eq!(fingerprint.resolved_files.len(), 3);
    }

    #[test]
    fn root_only_template_has_a_single_manifest_node() {
        let root = temp_root("include_root_only");
        write_file(&root.join("root.md"), "root\n");

        let expanded = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();
        let fingerprint = expanded.composition_fingerprint.unwrap();

        assert_eq!(fingerprint.manifest.nodes.len(), 1);
        assert!(fingerprint.manifest.edges.is_empty());
        assert_eq!(fingerprint.resolved_files.len(), 1);
    }

    #[test]
    fn legacy_non_nested_template_behavior_remains_unchanged() {
        let root = temp_root("include_legacy_non_nested");
        write_file(&root.join("legacy.md"), "plain legacy text\n");

        let expanded = expand_includes(
            root.join("legacy.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert_eq!(expanded.text, "plain legacy text\n");
        assert_eq!(
            expanded.resolved_files,
            vec![root.join("legacy.md").canonicalize().unwrap()]
        );
        assert_eq!(
            expanded
                .composition_fingerprint
                .unwrap()
                .manifest
                .edges
                .len(),
            0
        );
    }

    #[test]
    fn duplicate_includes_reuse_cached_source_and_preserve_graph_order() {
        let root = temp_root("include_duplicate");
        write_file(&root.join("root.md.j2"), "@<child.md>\n@<child.md>\n");
        write_file(&root.join("child.md"), "child\n");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert_eq!(expanded.text, "child\nchild\n");
        assert_eq!(expanded.resolved_files.len(), 2);
        assert_eq!(expanded.source_texts.len(), 2);
        assert_eq!(expanded.include_chains.len(), 2);
        assert_eq!(expanded.frontmatters.len(), 2);
        let fingerprint = expanded
            .composition_fingerprint
            .as_ref()
            .expect("fingerprint");
        assert_eq!(fingerprint.manifest.nodes.len(), 2);
        assert_eq!(fingerprint.manifest.edges.len(), 2);
        assert_eq!(fingerprint.manifest.edges[0].occurrence, 0);
        assert_eq!(fingerprint.manifest.edges[1].occurrence, 1);
    }

    #[test]
    fn nested_child_changes_composition_but_unrelated_file_does_not() {
        let root = temp_root("include_fingerprint_changes");
        write_file(&root.join("root.md.j2"), "@<partials/child.md>\n");
        write_file(&root.join("partials/child.md"), "child v1\n");
        write_file(&root.join("unrelated.md"), "outside v1\n");
        let policy = ComposePolicy::default();
        let first = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap()
        .source_sha;

        write_file(&root.join("unrelated.md"), "outside v2\n");
        let unrelated = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap()
        .source_sha;
        assert_eq!(first, unrelated);

        write_file(&root.join("partials/child.md"), "child v2\n");
        let nested = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap()
        .source_sha;
        assert_ne!(first, nested);
    }

    #[test]
    fn diamond_dependency_deduplicates_nodes_and_keeps_edges() {
        let root = temp_root("include_fingerprint_diamond");
        write_file(&root.join("root.md.j2"), "@<a.md>\n@<b.md>\n");
        write_file(&root.join("a.md"), "@<shared.md>\n");
        write_file(&root.join("b.md"), "@<shared.md>\n");
        write_file(&root.join("shared.md"), "shared\n");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();
        let fingerprint = expanded.composition_fingerprint.unwrap();
        assert_eq!(fingerprint.manifest.nodes.len(), 4);
        assert_eq!(fingerprint.manifest.edges.len(), 4);
        assert_eq!(
            fingerprint
                .manifest
                .nodes
                .iter()
                .filter(|node| node.source
                    == sc_sha::CanonicalSource::LocalPath(
                        sc_sha::CanonicalTemplatePath::try_from("shared.md".to_owned()).unwrap()
                    ))
                .count(),
            1
        );
    }

    #[test]
    fn identical_content_at_distinct_paths_remains_distinct_sources() {
        let root = temp_root("include_distinct_sources");
        write_file(&root.join("root.md"), "@<a.md>\n@<b.md>\n");
        write_file(&root.join("a.md"), "same\n");
        write_file(&root.join("b.md"), "same\n");

        let fingerprint = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap()
        .composition_fingerprint
        .unwrap();

        assert_eq!(fingerprint.manifest.nodes.len(), 3);
        assert_eq!(
            fingerprint.manifest.nodes[1].content_hash,
            fingerprint.manifest.nodes[2].content_hash
        );
        assert_ne!(
            fingerprint.manifest.nodes[1].source,
            fingerprint.manifest.nodes[2].source
        );
    }

    #[test]
    fn conditional_path_candidates_are_exhaustive_and_renderable() {
        let root = temp_root("include_conditional_candidates");
        write_file(
            &root.join("root.md"),
            "@<{{ \"item.md\" if mode == \"item\" else \"other-item.md\" }}>\n",
        );
        write_file(&root.join("item.md"), "item\n");
        write_file(&root.join("other-item.md"), "other\n");

        let expanded = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();
        let fingerprint = expanded.composition_fingerprint.unwrap();

        assert_eq!(fingerprint.manifest.nodes.len(), 3);
        assert_eq!(fingerprint.manifest.edges.len(), 2);
        assert!(expanded.text.contains("{% if mode == \"item\" %}"));
        assert!(expanded.text.contains("item\n"));
        assert!(expanded.text.contains("other\n"));

        write_file(&root.join("root.md"), "@<item.md>\n");
        let single_candidate = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap()
        .composition_fingerprint
        .unwrap();
        assert_ne!(fingerprint.source_sha, single_candidate.source_sha);
        assert_eq!(single_candidate.manifest.nodes.len(), 2);
    }

    #[test]
    fn reordering_include_occurrences_changes_composition_identity() {
        let root = temp_root("include_reordering");
        write_file(&root.join("root.md"), "@<a.md>\n@<b.md>\n");
        write_file(&root.join("a.md"), "a\n");
        write_file(&root.join("b.md"), "b\n");
        let policy = ComposePolicy::default();
        let first = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap();

        write_file(&root.join("root.md"), "@<b.md>\n@<a.md>\n");
        let reordered = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap();

        assert_ne!(first.source_sha, reordered.source_sha);
        assert_eq!(first.manifest.nodes.len(), reordered.manifest.nodes.len());
        assert_eq!(first.manifest.edges.len(), reordered.manifest.edges.len());
    }

    #[test]
    fn nested_paths_use_forward_slash_canonical_sources() {
        let root = temp_root("include_nested_paths");
        write_file(&root.join("root.md"), "@<nested/deep/child.md>\n");
        write_file(&root.join("nested/deep/child.md"), "child\n");

        let fingerprint = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap()
        .composition_fingerprint
        .unwrap();

        assert_eq!(
            fingerprint.manifest.nodes[1].source,
            sc_sha::CanonicalSource::LocalPath(
                sc_sha::CanonicalTemplatePath::try_from("nested/deep/child.md".to_owned()).unwrap()
            )
        );
    }

    #[test]
    fn text_hash_boundary_is_stable_for_line_endings_and_sensitive_to_bom() {
        let root = temp_root("include_text_hash_boundary");
        write_file(&root.join("root.md"), "@<child.md>\n");
        write_bytes(&root.join("child.md"), b"line\r\n");
        let policy = ComposePolicy::default();
        let crlf = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap()
        .source_sha;

        write_bytes(&root.join("child.md"), b"line\n");
        let lf = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap()
        .source_sha;
        assert_eq!(crlf, lf);

        write_bytes(&root.join("child.md"), b"\xef\xbb\xbfline\n");
        let bom = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap()
        .source_sha;
        assert_ne!(lf, bom);

        write_bytes(&root.join("child.md"), b"line");
        let no_final_newline = expand_includes(
            root.join("root.md"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap()
        .composition_fingerprint
        .unwrap()
        .source_sha;
        assert_ne!(lf, no_final_newline);
    }

    #[test]
    fn tagged_url_and_local_sources_do_not_collide() {
        let digest = sc_sha::calculate_hash(sc_sha::HashInput::TextFileBytes {
            utf8_file_bytes: b"same\n",
        })
        .unwrap()
        .template()
        .to_owned();
        let local = sc_sha::CanonicalSource::LocalPath(
            sc_sha::CanonicalTemplatePath::try_from("same.md".to_owned()).unwrap(),
        );
        let url = sc_sha::CanonicalSource::Url(
            sc_sha::CanonicalSourceUrl::try_from("https://example.test/same.md".to_owned())
                .unwrap(),
        );
        let local_hash = sc_sha::calculate_composition_hash(&sc_sha::ResolvedTemplateManifest {
            schema: sc_sha::ManifestSchemaVersion::V1,
            nodes: vec![sc_sha::ResolvedTemplateNode {
                source: local,
                content_hash: digest,
            }],
            edges: Vec::new(),
        })
        .unwrap();
        let url_hash = sc_sha::calculate_composition_hash(&sc_sha::ResolvedTemplateManifest {
            schema: sc_sha::ManifestSchemaVersion::V1,
            nodes: vec![sc_sha::ResolvedTemplateNode {
                source: url,
                content_hash: digest,
            }],
            edges: Vec::new(),
        })
        .unwrap();

        assert_ne!(local_hash, url_hash);
    }

    #[test]
    fn dynamic_include_is_explicitly_non_cacheable() {
        let root = temp_root("include_dynamic");
        write_file(&root.join("root.md.j2"), "@<{{ name }}.md>\n");
        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            Some(DiagnosticCode::ErrIncludeDynamicUnresolved)
        );
    }

    #[test]
    fn captures_frontmatter_and_child_body_with_default_policy() {
        let root = temp_root("include_frontmatter");
        write_file(
            &root.join("root.md.j2"),
            "---\ndefaults:\n  greeting: hello\n---\n@<child.md>\n",
        );
        write_file(&root.join("child.md"), "[[ greeting ]]\n");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert_eq!(expanded.frontmatters.len(), 2);
        assert_eq!(expanded.frontmatters[0].1.len(), 1);
        assert_eq!(expanded.text, "[[ greeting ]]\n");
    }

    #[test]
    fn missing_include_reports_not_found() {
        let root = temp_root("include_missing");
        write_file(&root.join("root.md.j2"), "@<missing.md>\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeNotFound);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn directory_include_target_reports_is_a_directory() {
        let root = temp_root("include_directory_target");
        write_file(&root.join("root.md.j2"), "@<partials>\n");
        fs::create_dir(root.join("partials")).unwrap();

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeIsADirectory);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn permission_denied_include_reports_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("include_permission_denied");
        let restricted = root.join("restricted.md");
        write_file(&root.join("root.md.j2"), "@<restricted.md>\n");
        write_file(&restricted, "secret\n");
        let mut permissions = fs::metadata(&restricted).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&restricted, permissions).unwrap();

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        let mut restore = fs::metadata(&restricted).unwrap().permissions();
        restore.set_mode(0o600);
        fs::set_permissions(&restricted, restore).unwrap();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludePermissionDenied);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlink_loop_include_reports_filesystem_loop() {
        use std::os::unix::fs::symlink;

        let root = temp_root("include_filesystem_loop");
        write_file(&root.join("root.md.j2"), "@<loop-a.md>\n");
        symlink("loop-b.md", root.join("loop-a.md")).unwrap();
        symlink("loop-a.md", root.join("loop-b.md")).unwrap();

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeFilesystemLoop);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn cycle_detection_is_rejected() {
        let root = temp_root("include_cycle");
        write_file(&root.join("root.md.j2"), "@<one.md>\n");
        write_file(&root.join("one.md"), "@<root.md.j2>\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeCycle);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn depth_overflow_is_rejected() {
        let root = temp_root("include_depth");
        write_file(&root.join("root.md.j2"), "@<a.md>\n");
        write_file(&root.join("a.md"), "@<b.md>\n");
        write_file(&root.join("b.md"), "@<c.md>\n");
        write_file(&root.join("c.md"), "done\n");

        let policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(1),
            ..ComposePolicy::default()
        };

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeDepth);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn escape_attempts_are_rejected() {
        let root = temp_root("include_escape");
        write_file(&root.join("root.md.j2"), "@<../outside.md>\n");
        let outside = root.parent().unwrap().join("outside.md");
        write_file(&outside, "nope\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn nonexistent_escape_attempts_are_rejected_before_not_found() {
        let root = temp_root("include_escape_missing");
        write_file(&root.join("root.md.j2"), "@<../outside-missing.md>\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn single_line_include_expands_exactly_once() {
        let root = temp_root("include_single_line");
        write_file(&root.join("root.md.j2"), "@<child.md>");
        write_file(&root.join("child.md"), "child body");

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap();

        assert_eq!(expanded.text, "child body");
    }

    #[test]
    fn absolute_escape_attempts_are_rejected() {
        let root = temp_root("include_absolute_escape");
        let outside = root.parent().unwrap().join("absolute-outside.md");
        write_file(
            &root.join("root.md.j2"),
            &format!("@<{}>\n", outside.display()),
        );
        write_file(&outside, "outside\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn symlink_escape_attempts_are_rejected_when_supported() {
        let root = temp_root("include_symlink_escape");
        let outside = root.parent().unwrap().join("symlink-outside.md");
        write_file(&outside, "outside\n");
        let symlink_path = root.join("linked-outside.md");
        if !create_symlink_if_supported(&outside, &symlink_path) {
            return;
        }
        write_file(&root.join("root.md.j2"), "@<linked-outside.md>\n");

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn sibling_prefix_escape_attempts_are_rejected() {
        let root = temp_root("include_sibling_prefix");
        let sibling = root.parent().unwrap().join(format!(
            "{}-evil",
            root.file_name().unwrap().to_string_lossy()
        ));
        write_file(&sibling.join("outside.md"), "outside\n");
        write_file(
            &root.join("root.md.j2"),
            &format!(
                "@<../{}/outside.md>\n",
                sibling.file_name().unwrap().to_string_lossy()
            ),
        );

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &ComposePolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeEscape);
                assert!(
                    error
                        .message()
                        .contains("include path escapes confinement root")
                );
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn configured_allowed_root_remains_available_to_includes() {
        let root = temp_root("include_allowed_root");
        let allowed = temp_root("include_allowed_root_shared");
        write_file(
            &root.join("root.md.j2"),
            &format!(
                "@<../{}/part.md>\n",
                allowed.file_name().unwrap().to_string_lossy()
            ),
        );
        write_file(&allowed.join("part.md"), "shared\n");

        let policy = ComposePolicy {
            allowed_roots: vec![ConfiningRoot::new(&allowed).unwrap()],
            ..ComposePolicy::default()
        };
        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap();

        assert_eq!(expanded.text, "shared\n");
    }

    #[test]
    fn deep_include_chain_above_safe_ceiling_returns_include_depth_error() {
        const CHAIN_DEPTH: usize = 1_900;
        let root = temp_root("include_stack_overflow");
        let chain_root = root.join("chain");
        fs::create_dir_all(&chain_root).unwrap();
        write_file(&root.join("root.md.j2"), "@<chain/0000.md>\n");

        for index in 0..CHAIN_DEPTH {
            let current = chain_root.join(format!("{index:04}.md"));
            let contents = if index + 1 == CHAIN_DEPTH {
                "done\n".to_owned()
            } else {
                format!("@<{:04}.md>\n", index + 1)
            };
            write_file(&current, &contents);
        }

        let policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(1_905),
            ..ComposePolicy::default()
        };

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeDepth);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn include_chain_at_safe_depth_still_expands_with_high_configured_limit() {
        const CHAIN_DEPTH: usize = 50;
        let root = temp_root("include_safe_depth");
        write_linear_chain(&root, CHAIN_DEPTH);

        let policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(1_000),
            ..ComposePolicy::default()
        };

        let expanded = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap();

        assert_eq!(expanded.text, "done\n");
    }

    #[test]
    fn configured_depth_below_safety_ceiling_remains_the_effective_bound() {
        const CHAIN_DEPTH: usize = 10;
        let root = temp_root("include_configured_depth");
        write_linear_chain(&root, CHAIN_DEPTH);

        let policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(5),
            ..ComposePolicy::default()
        };

        let error = expand_includes(
            root.join("root.md.j2"),
            &ConfiningRoot::new(&root).unwrap(),
            &policy,
        )
        .unwrap_err();

        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeDepth);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn include_depth_exact_limit_succeeds_and_one_over_fails() {
        let exact_root = temp_root("include_depth_exact_limit");
        write_linear_chain(&exact_root, 3);
        let exact_policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(3),
            ..ComposePolicy::default()
        };

        let expanded = expand_includes(
            exact_root.join("root.md.j2"),
            &ConfiningRoot::new(&exact_root).unwrap(),
            &exact_policy,
        )
        .unwrap();
        assert_eq!(expanded.text, "done\n");

        let over_root = temp_root("include_depth_one_over");
        write_linear_chain(&over_root, 4);
        let over_policy = ComposePolicy {
            max_include_depth: IncludeDepth::new(3),
            ..ComposePolicy::default()
        };

        let error = expand_includes(
            over_root.join("root.md.j2"),
            &ConfiningRoot::new(&over_root).unwrap(),
            &over_policy,
        )
        .unwrap_err();
        match error {
            ComposeError::Include(error) => {
                assert_eq!(error.code(), DiagnosticCode::ErrIncludeDepth);
                assert_eq!(error.message(), "include depth exceeded maximum of 3");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    struct TempFixture {
        path: PathBuf,
    }

    impl TempFixture {
        fn new(label: &str) -> Self {
            static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let sequence = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "sc-compose-{label}-{}-{nanos}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl AsRef<Path> for TempFixture {
        fn as_ref(&self) -> &Path {
            &self.path
        }
    }

    impl Deref for TempFixture {
        type Target = Path;

        fn deref(&self) -> &Self::Target {
            &self.path
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn temp_root(label: &str) -> TempFixture {
        TempFixture::new(label)
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn write_bytes(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn write_linear_chain(root: &Path, depth: usize) {
        let chain_root = root.join("chain");
        fs::create_dir_all(&chain_root).unwrap();
        write_file(&root.join("root.md.j2"), "@<chain/0000.md>\n");

        for index in 0..depth {
            let current = chain_root.join(format!("{index:04}.md"));
            let contents = if index + 1 == depth {
                "done\n".to_owned()
            } else {
                format!("@<{:04}.md>\n", index + 1)
            };
            write_file(&current, &contents);
        }
    }

    #[cfg(unix)]
    fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[cfg(windows)]
    fn create_symlink_if_supported(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("failed to create symlink: {error}"),
        }
    }
}
