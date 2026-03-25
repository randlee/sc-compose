//! Runtime-aware profile resolution and search tracing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::DiagnosticCode;
use crate::error::{ComposeError, ConfigError, ResolveError};
use crate::observer::{CompositionObserver, NoopObserver, ResolveOutcomeEvent};
use crate::types::{
    ComposeMode, ComposeRequest, ConfiningRoot, ProfileKind, ProfileName, ResolveResult,
    ResolverPolicy, RuntimeKind,
};

const DEFAULT_RUNTIME_ORDER: [RuntimeKind; 4] = [
    RuntimeKind::Claude,
    RuntimeKind::Codex,
    RuntimeKind::Gemini,
    RuntimeKind::Opencode,
];

/// Resolve the template path targeted by a compose request.
///
/// In file mode, this canonicalizes the explicit path under the configured
/// confinement roots. In profile mode, it performs runtime-aware prompt lookup.
///
/// # Errors
///
/// Returns [`ComposeError`] when resolution fails, when the path escapes the
/// configured roots, or when multiple profile candidates are found.
pub fn resolve_template_path(request: &ComposeRequest) -> Result<ResolveResult, ComposeError> {
    match &request.mode {
        ComposeMode::Profile { kind, name } => resolve_profile_impl(
            request.root.as_path(),
            *kind,
            name,
            request.runtime,
            &request.policy.resolver_policy,
        ),
        ComposeMode::File { template_path } => {
            let resolved_path = canonicalize_with_roots(
                template_path,
                request.root.as_path(),
                &request.policy.allowed_roots,
            )?;

            Ok(ResolveResult {
                resolved_path: resolved_path.clone(),
                attempted_paths: vec![resolved_path],
                ambiguity_candidates: Vec::new(),
            })
        }
    }
}

/// Resolve a profile-mode request to a concrete template path.
///
/// # Errors
///
/// Returns [`ComposeError`] when the request is not in profile mode, when no
/// template is found, or when multiple candidates match an omitted-runtime
/// lookup.
pub fn resolve_profile(request: &ComposeRequest) -> Result<ResolveResult, ComposeError> {
    let mut observer = NoopObserver;
    resolve_profile_with_observer(request, &mut observer)
}

/// Resolve a profile-mode request while emitting observer callbacks.
///
/// # Errors
///
/// Returns [`ComposeError`] when the request is not in profile mode, when no
/// template is found, or when multiple candidates match an omitted-runtime
/// lookup.
pub fn resolve_profile_with_observer(
    request: &ComposeRequest,
    observer: &mut dyn CompositionObserver,
) -> Result<ResolveResult, ComposeError> {
    match &request.mode {
        ComposeMode::Profile { kind, name } => {
            let result = resolve_profile_impl(
                request.root.as_path(),
                *kind,
                name,
                request.runtime,
                &request.policy.resolver_policy,
            );
            match &result {
                Ok(resolve_result) => observer.on_resolve_outcome(&ResolveOutcomeEvent {
                    resolved_path: Some(resolve_result.resolved_path.clone()),
                    attempted_paths: resolve_result.attempted_paths.clone(),
                    code: None,
                }),
                Err(ComposeError::Resolve(error)) => {
                    observer.on_resolve_outcome(&ResolveOutcomeEvent {
                        resolved_path: None,
                        attempted_paths: error.attempted_paths().to_vec(),
                        code: error.code(),
                    });
                }
                Err(_) => {}
            }
            result
        }
        ComposeMode::File { .. } => Err(ConfigError::new(
            DiagnosticCode::ErrConfigMode,
            "resolve_profile requires profile mode",
        )
        .into()),
    }
}

pub(crate) fn canonicalize_with_roots(
    path: impl AsRef<Path>,
    root: &Path,
    allowed_roots: &[ConfiningRoot],
) -> Result<PathBuf, ComposeError> {
    let path = path.as_ref();
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
        ResolveError::new(
            DiagnosticCode::ErrResolveNotFound,
            format!("template path not found: {}", candidate.display()),
            vec![candidate.clone()],
        )
        .with_source(error)
    })?;

    let mut allowed = Vec::with_capacity(allowed_roots.len() + 1);
    allowed.push(std::fs::canonicalize(root).map_err(|error| {
        ConfigError::new(
            DiagnosticCode::ErrConfigParse,
            format!("failed to canonicalize root: {}", root.display()),
        )
        .with_source(error)
    })?);
    allowed.extend(
        allowed_roots
            .iter()
            .map(|root| root.as_path().to_path_buf()),
    );

    if allowed
        .iter()
        .any(|allowed_root| canonical.starts_with(allowed_root))
    {
        Ok(canonical)
    } else {
        Err(ResolveError::new(
            DiagnosticCode::ErrResolveNotFound,
            format!(
                "template path escapes configured roots: {}",
                candidate.display()
            ),
            vec![candidate],
        )
        .into())
    }
}

fn resolve_profile_impl(
    root: &Path,
    kind: ProfileKind,
    name: &ProfileName,
    runtime: Option<RuntimeKind>,
    resolver_policy: &ResolverPolicy,
) -> Result<ResolveResult, ComposeError> {
    let candidate_directories = candidate_directories(root, kind, runtime, resolver_policy);
    let probes = filename_probes(kind, name, resolver_policy);

    let mut attempted_paths = Vec::new();
    let mut attempted_seen = BTreeSet::new();
    let mut matches = Vec::new();
    let mut matched_seen = BTreeSet::new();

    for directory in candidate_directories {
        for probe in &probes {
            let candidate = directory.join(probe);
            if attempted_seen.insert(candidate.clone()) {
                attempted_paths.push(candidate.clone());
            }
            if candidate.is_file() {
                let canonical = std::fs::canonicalize(&candidate).map_err(|error| {
                    ResolveError::new(
                        DiagnosticCode::ErrResolveNotFound,
                        format!("failed to canonicalize candidate: {}", candidate.display()),
                        vec![candidate.clone()],
                    )
                    .with_source(error)
                })?;
                if matched_seen.insert(canonical.clone()) {
                    matches.push(canonical);
                }
            }
        }
    }

    match matches.as_slice() {
        [resolved_path] => Ok(ResolveResult {
            resolved_path: resolved_path.clone(),
            attempted_paths,
            ambiguity_candidates: Vec::new(),
        }),
        [] => Err(ResolveError::new(
            DiagnosticCode::ErrResolveNotFound,
            format!("no {kind:?} profile named `{name}` was found"),
            attempted_paths,
        )
        .into()),
        _ => Err(ResolveError::new(
            DiagnosticCode::ErrResolveAmbiguous,
            format!("multiple {kind:?} profiles named `{name}` matched; specify a runtime"),
            attempted_paths,
        )
        .into()),
    }
}

fn candidate_directories(
    root: &Path,
    kind: ProfileKind,
    runtime: Option<RuntimeKind>,
    resolver_policy: &ResolverPolicy,
) -> Vec<PathBuf> {
    if !resolver_policy.candidate_directories.is_empty() {
        return resolver_policy
            .candidate_directories
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    root.join(path)
                }
            })
            .collect();
    }

    let mut directories = Vec::new();
    let mut seen = BTreeSet::new();

    let runtimes: Vec<RuntimeKind> =
        runtime.map_or_else(|| DEFAULT_RUNTIME_ORDER.to_vec(), |runtime| vec![runtime]);

    for runtime in runtimes {
        for relative in runtime_chain(runtime, kind) {
            let directory = root.join(relative);
            if seen.insert(directory.clone()) {
                directories.push(directory);
            }
        }
    }

    directories
}

fn filename_probes(
    kind: ProfileKind,
    name: &ProfileName,
    resolver_policy: &ResolverPolicy,
) -> Vec<PathBuf> {
    if !resolver_policy.filename_probes.is_empty() {
        return resolver_policy
            .filename_probes
            .iter()
            .map(PathBuf::from)
            .collect();
    }

    match kind {
        ProfileKind::Agent | ProfileKind::Command => vec![
            PathBuf::from(format!("{}.md.j2", name.as_str())),
            PathBuf::from(format!("{}.md", name.as_str())),
            PathBuf::from(format!("{}.j2", name.as_str())),
        ],
        ProfileKind::Skill => vec![
            PathBuf::from(name.as_str()).join("SKILL.md.j2"),
            PathBuf::from(name.as_str()).join("SKILL.md"),
            PathBuf::from(name.as_str()).join("SKILL.j2"),
        ],
    }
}

fn runtime_chain(runtime: RuntimeKind, kind: ProfileKind) -> &'static [&'static str] {
    match (runtime, kind) {
        (RuntimeKind::Claude, ProfileKind::Agent) => &[".claude/agents", ".agents/agents"],
        (RuntimeKind::Codex, ProfileKind::Agent) => {
            &[".codex/agents", ".agents/agents", ".claude/agents"]
        }
        (RuntimeKind::Gemini, ProfileKind::Agent) => {
            &[".gemini/agents", ".agents/agents", ".claude/agents"]
        }
        (RuntimeKind::Opencode, ProfileKind::Agent) => {
            &[".opencode/agents", ".agents/agents", ".claude/agents"]
        }
        (RuntimeKind::Claude, ProfileKind::Command) => &[".claude/commands", ".agents/commands"],
        (RuntimeKind::Codex, ProfileKind::Command) => {
            &[".codex/commands", ".agents/commands", ".claude/commands"]
        }
        (RuntimeKind::Gemini, ProfileKind::Command) => {
            &[".gemini/commands", ".agents/commands", ".claude/commands"]
        }
        (RuntimeKind::Opencode, ProfileKind::Command) => {
            &[".opencode/commands", ".agents/commands", ".claude/commands"]
        }
        (RuntimeKind::Claude, ProfileKind::Skill) => &[".claude/skills", ".agents/skills"],
        (RuntimeKind::Codex, ProfileKind::Skill) => {
            &[".codex/skills", ".agents/skills", ".claude/skills"]
        }
        (RuntimeKind::Gemini, ProfileKind::Skill) => {
            &[".gemini/skills", ".agents/skills", ".claude/skills"]
        }
        (RuntimeKind::Opencode, ProfileKind::Skill) => {
            &[".opencode/skills", ".agents/skills", ".claude/skills"]
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::resolve_profile_impl;
    use crate::observer::{CompositionObserver, ResolveOutcomeEvent};
    use crate::types::{
        ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, ProfileKind, ProfileName,
    };
    use crate::{ComposeError, DiagnosticCode};

    #[derive(Default)]
    struct CapturingObserver {
        resolve: Vec<ResolveOutcomeEvent>,
    }

    impl CompositionObserver for CapturingObserver {
        fn on_resolve_outcome(&mut self, event: &ResolveOutcomeEvent) {
            self.resolve.push(event.clone());
        }
    }

    #[test]
    fn resolves_agent_command_and_skill_profiles_across_runtime_and_shared_roots() {
        let root = temp_root("resolver_profile_matrix");
        write_file(&root.join(".claude/agents/agent.md"), "agent");
        write_file(&root.join(".agents/commands/command.md"), "command");
        write_file(&root.join(".codex/skills/skill/SKILL.md.j2"), "skill");

        let agent = resolve_profile_impl(
            &root,
            ProfileKind::Agent,
            &ProfileName::new("agent").unwrap(),
            None,
            &crate::types::ResolverPolicy::default(),
        )
        .unwrap();
        let command = resolve_profile_impl(
            &root,
            ProfileKind::Command,
            &ProfileName::new("command").unwrap(),
            Some(crate::types::RuntimeKind::Claude),
            &crate::types::ResolverPolicy::default(),
        )
        .unwrap();
        let skill = resolve_profile_impl(
            &root,
            ProfileKind::Skill,
            &ProfileName::new("skill").unwrap(),
            Some(crate::types::RuntimeKind::Codex),
            &crate::types::ResolverPolicy::default(),
        )
        .unwrap();

        assert!(agent.resolved_path.ends_with("agent.md"));
        assert!(command.resolved_path.ends_with("command.md"));
        assert!(skill.resolved_path.ends_with("SKILL.md.j2"));
        assert!(!agent.attempted_paths.is_empty());
    }

    #[test]
    fn omitted_runtime_reports_ambiguity() {
        let root = temp_root("resolver_ambiguity");
        write_file(&root.join(".claude/agents/name.md"), "claude");
        write_file(&root.join(".codex/agents/name.md"), "codex");

        let error = resolve_profile_impl(
            &root,
            ProfileKind::Agent,
            &ProfileName::new("name").unwrap(),
            None,
            &crate::types::ResolverPolicy::default(),
        )
        .unwrap_err();

        match error {
            ComposeError::Resolve(error) => {
                assert_eq!(error.code(), Some(DiagnosticCode::ErrResolveAmbiguous));
                assert!(!error.attempted_paths().is_empty());
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn explicit_file_mode_canonicalizes_within_root() {
        let root = temp_root("resolver_file_mode");
        let file = root.join("nested/template.md.j2");
        write_file(&file, "hello");

        let request = ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("nested/template.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input: BTreeMap::default(),
            vars_env: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        };

        let result = super::resolve_template_path(&request).unwrap();
        assert!(result.resolved_path.ends_with("nested/template.md.j2"));
    }

    #[test]
    fn resolve_profile_rejects_file_mode_with_mode_code() {
        let root = temp_root("resolver_mode_mismatch");
        write_file(&root.join("template.md.j2"), "hello");
        let request = ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from("template.md.j2"),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input: BTreeMap::default(),
            vars_env: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        };

        let error = super::resolve_profile(&request).unwrap_err();

        match error {
            ComposeError::Config(error) => {
                assert_eq!(error.code(), Some(DiagnosticCode::ErrConfigMode));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn resolve_profile_with_observer_emits_failure_outcome() {
        let root = temp_root("resolver_observer_failure");
        let mut observer = CapturingObserver::default();
        let request = ComposeRequest {
            runtime: Some(crate::types::RuntimeKind::Claude),
            mode: ComposeMode::Profile {
                kind: ProfileKind::Agent,
                name: ProfileName::new("missing").unwrap(),
            },
            root: ConfiningRoot::new(&root).unwrap(),
            vars_input: BTreeMap::default(),
            vars_env: BTreeMap::default(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        };

        let error = super::resolve_profile_with_observer(&request, &mut observer).unwrap_err();

        assert!(matches!(error, ComposeError::Resolve(_)));
        assert_eq!(observer.resolve.len(), 1);
        assert_eq!(
            observer.resolve[0].code,
            Some(DiagnosticCode::ErrResolveNotFound)
        );
        assert!(observer.resolve[0].resolved_path.is_none());
    }

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("sc-compose-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}
