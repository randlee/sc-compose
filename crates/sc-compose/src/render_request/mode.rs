use anyhow::{Context, anyhow};
use sc_composer::{
    ComposeMode, ConfiningRoot, DiagnosticCode, ProfileKind, RecoveryHint, RecoveryHintKind,
    RuntimeKind,
};

use crate::CommandError;
use crate::cli::{Ai, CommonArgs, Kind, Mode};

pub(super) fn build_mode(args: &CommonArgs) -> Result<ComposeMode, CommandError> {
    match args.mode {
        Mode::File => Ok(ComposeMode::File {
            template_path: required_file_path(args)?,
        }),
        Mode::Profile => Ok(ComposeMode::Profile {
            kind: profile_kind(args.kind),
            name: required_profile_name(args)?,
        }),
    }
}

pub(super) fn build_root(root: &std::path::Path) -> Result<ConfiningRoot, CommandError> {
    ConfiningRoot::new(root)
        .with_context(|| format!("failed to canonicalize root {}", root.display()))
        .map_err(|error| CommandError::usage_with_code(error, DiagnosticCode::ErrConfigParse))
}

pub(super) fn required_file_path(args: &CommonArgs) -> Result<std::path::PathBuf, CommandError> {
    args.file.clone().ok_or_else(|| {
        CommandError::usage_with_code_and_hints(
            anyhow!("--file is required in file mode"),
            DiagnosticCode::ErrConfigMode,
            vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "pass --file when --mode file is selected".to_owned(),
            })],
        )
    })
}

pub(super) fn required_profile_name(
    args: &CommonArgs,
) -> Result<sc_composer::ProfileName, CommandError> {
    let name = args
        .agent
        .clone()
        .or_else(|| args.agent_type.clone())
        .ok_or_else(|| {
            CommandError::usage_with_code_and_hints(
                anyhow!("--agent/--agent-type is required in profile mode"),
                DiagnosticCode::ErrConfigMode,
                vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                    key: "pass --agent or --agent-type when --mode profile is selected".to_owned(),
                })],
            )
        })?;
    sc_composer::ProfileName::new(name).map_err(|error| {
        CommandError::usage_with_code_and_hints(
            anyhow!("invalid profile name: {error}"),
            DiagnosticCode::ErrConfigParse,
            vec![RecoveryHint::new(RecoveryHintKind::ReviewConfiguration {
                key: "use an alphanumeric profile name with . _ or - only".to_owned(),
            })],
        )
    })
}

pub(super) const fn profile_kind(kind: Kind) -> ProfileKind {
    match kind {
        Kind::Agent => ProfileKind::Agent,
        Kind::Command => ProfileKind::Command,
        Kind::Skill => ProfileKind::Skill,
    }
}

pub(super) fn runtime_kind(runtime: Ai) -> RuntimeKind {
    match runtime {
        Ai::Claude => RuntimeKind::Claude,
        Ai::Hermes => RuntimeKind::Hermes,
        Ai::Codex => RuntimeKind::Codex,
        Ai::Gemini => RuntimeKind::Gemini,
        Ai::Opencode => RuntimeKind::Opencode,
    }
}
