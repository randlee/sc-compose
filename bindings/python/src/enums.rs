use pyo3::prelude::*;
use sc_composer::{
    DiagnosticSeverity, ExtractFormat, JsonEscapeMode, ProfileKind, RuntimeKind,
    UnknownVariablePolicy, VariableSource,
};

use crate::errors::config_error;

#[pyclass(name = "RuntimeKind")]
pub(crate) struct PyRuntimeKind;

#[pymethods]
impl PyRuntimeKind {
    #[classattr]
    const CLAUDE: &'static str = "claude";
    #[classattr]
    const CODEX: &'static str = "codex";
    #[classattr]
    const GEMINI: &'static str = "gemini";
    #[classattr]
    const OPENCODE: &'static str = "opencode";
}

#[pyclass(name = "ProfileKind")]
pub(crate) struct PyProfileKind;

#[pymethods]
impl PyProfileKind {
    #[classattr]
    const AGENT: &'static str = "agent";
    #[classattr]
    const COMMAND: &'static str = "command";
    #[classattr]
    const SKILL: &'static str = "skill";
}

#[pyclass(name = "UnknownVariablePolicy")]
pub(crate) struct PyUnknownVariablePolicy;

#[pymethods]
impl PyUnknownVariablePolicy {
    #[classattr]
    const ERROR: &'static str = "error";
    #[classattr]
    const WARN: &'static str = "warn";
    #[classattr]
    const IGNORE: &'static str = "ignore";
}

#[pyclass(name = "JsonEscapeMode")]
pub(crate) struct PyJsonEscapeMode;

#[pymethods]
impl PyJsonEscapeMode {
    #[classattr]
    const AUTO: &'static str = "auto";
    #[classattr]
    const LEGACY: &'static str = "legacy";
}

#[pyclass(name = "VariableSource")]
pub(crate) struct PyVariableSource;

#[pymethods]
impl PyVariableSource {
    #[classattr]
    const EXPLICIT_INPUT: &'static str = "explicit_input";
    #[classattr]
    const ENVIRONMENT: &'static str = "environment";
    #[classattr]
    const BUILTIN: &'static str = "builtin";
    #[classattr]
    const TEMPLATE_INPUT_DEFAULT: &'static str = "template_input_default";
    #[classattr]
    const FRONTMATTER_DEFAULT: &'static str = "frontmatter_default";
    #[classattr]
    const INCLUDED_DEFAULT: &'static str = "included_default";
}

#[pyclass(name = "DiagnosticSeverity")]
pub(crate) struct PyDiagnosticSeverity;

#[pymethods]
impl PyDiagnosticSeverity {
    #[classattr]
    const ERROR: &'static str = "error";
    #[classattr]
    const WARNING: &'static str = "warning";
    #[classattr]
    const INFO: &'static str = "info";
}

#[pyclass(name = "DiagnosticCode")]
pub(crate) struct PyDiagnosticCode;

#[pymethods]
impl PyDiagnosticCode {
    #[classattr]
    const ERR_RESOLVE_NOT_FOUND: &'static str = "ERR_RESOLVE_NOT_FOUND";
    #[classattr]
    const ERR_RESOLVE_AMBIGUOUS: &'static str = "ERR_RESOLVE_AMBIGUOUS";
    #[classattr]
    const ERR_INCLUDE_ESCAPE: &'static str = "ERR_INCLUDE_ESCAPE";
    #[classattr]
    const ERR_INCLUDE_NOT_FOUND: &'static str = "ERR_INCLUDE_NOT_FOUND";
    #[classattr]
    const ERR_INCLUDE_CYCLE: &'static str = "ERR_INCLUDE_CYCLE";
    #[classattr]
    const ERR_INCLUDE_DEPTH: &'static str = "ERR_INCLUDE_DEPTH";
    #[classattr]
    const ERR_VAL_OBJECT_SHAPE: &'static str = "ERR_VAL_OBJECT_SHAPE";
    #[classattr]
    const ERR_VAL_NESTED_ARRAY_UNSUPPORTED: &'static str = "ERR_VAL_NESTED_ARRAY_UNSUPPORTED";
    #[classattr]
    const ERR_VAL_DUPLICATE: &'static str = "ERR_VAL_DUPLICATE";
    #[classattr]
    const WARN_VAL_CONFLICTING_DEFAULT_SECTIONS: &'static str =
        "WARN_VAL_CONFLICTING_DEFAULT_SECTIONS";
    #[classattr]
    const ERR_VAL_EMPTY: &'static str = "ERR_VAL_EMPTY";
    #[classattr]
    const ERR_VAL_MISSING_FRONTMATTER: &'static str = "ERR_VAL_MISSING_FRONTMATTER";
    #[classattr]
    const ERR_VAL_MISSING_REQUIRED: &'static str = "ERR_VAL_MISSING_REQUIRED";
    #[classattr]
    const ERR_VAL_MISSING_NESTED_FIELD: &'static str = "ERR_VAL_MISSING_NESTED_FIELD";
    #[classattr]
    const ERR_VAL_SHAPE_MISMATCH: &'static str = "ERR_VAL_SHAPE_MISMATCH";
    #[classattr]
    const ERR_VAL_UNDECLARED_TOKEN: &'static str = "ERR_VAL_UNDECLARED_TOKEN";
    #[classattr]
    const ERR_VAL_EXTRA_INPUT: &'static str = "ERR_VAL_EXTRA_INPUT";
    #[classattr]
    const ERR_VAL_UNBOUND_VARIABLE: &'static str = "ERR_VAL_UNBOUND_VARIABLE";
    #[classattr]
    const INFO_VAL_DEFAULT_USED: &'static str = "INFO_VAL_DEFAULT_USED";
    #[classattr]
    const ERR_RENDER_STDIN_DOUBLE_READ: &'static str = "ERR_RENDER_STDIN_DOUBLE_READ";
    #[classattr]
    const ERR_RENDER_WRITE: &'static str = "ERR_RENDER_WRITE";
    #[classattr]
    const ERR_CONFIG_READONLY: &'static str = "ERR_CONFIG_READONLY";
    #[classattr]
    const ERR_CONFIG_MODE: &'static str = "ERR_CONFIG_MODE";
    #[classattr]
    const ERR_CONFIG_PARSE: &'static str = "ERR_CONFIG_PARSE";
    #[classattr]
    const ERR_CONFIG_VARFILE: &'static str = "ERR_CONFIG_VARFILE";
    #[classattr]
    const ERR_CONFIG_PACK_NOT_FOUND: &'static str = "ERR_CONFIG_PACK_NOT_FOUND";
    #[classattr]
    const ERR_CONFIG_PACK_NOT_RENDERABLE: &'static str = "ERR_CONFIG_PACK_NOT_RENDERABLE";
    #[classattr]
    const ERR_CONFIG_TEMPLATE_EXISTS: &'static str = "ERR_CONFIG_TEMPLATE_EXISTS";
    #[classattr]
    const ERR_EXTRACT_INVALID_REQUEST: &'static str = "ERR_EXTRACT_INVALID_REQUEST";
    #[classattr]
    const ERR_EXTRACT_MALFORMED: &'static str = "ERR_EXTRACT_MALFORMED";
    #[classattr]
    const ERR_EXTRACT_UNSUPPORTED: &'static str = "ERR_EXTRACT_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_TEMPLATE_UNSUPPORTED: &'static str = "ERR_EXTRACT_TEMPLATE_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_XML_ELEMENT_MISMATCH: &'static str = "ERR_EXTRACT_XML_ELEMENT_MISMATCH";
    #[classattr]
    const ERR_EXTRACT_XML_ATTRIBUTE_MISMATCH: &'static str = "ERR_EXTRACT_XML_ATTRIBUTE_MISMATCH";
    #[classattr]
    const ERR_EXTRACT_XML_CHILD_STRUCTURE_MISMATCH: &'static str =
        "ERR_EXTRACT_XML_CHILD_STRUCTURE_MISMATCH";
    #[classattr]
    const ERR_EXTRACT_XML_STATIC_MISMATCH: &'static str = "ERR_EXTRACT_XML_STATIC_MISMATCH";
    #[classattr]
    const ERR_EXTRACT_XML_CONTROL_FLOW_UNSUPPORTED: &'static str =
        "ERR_EXTRACT_XML_CONTROL_FLOW_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_XML_DYNAMIC_ELEMENT_NAME: &'static str =
        "ERR_EXTRACT_XML_DYNAMIC_ELEMENT_NAME";
    #[classattr]
    const ERR_EXTRACT_XML_NAMESPACE_UNSUPPORTED: &'static str =
        "ERR_EXTRACT_XML_NAMESPACE_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_AMBIGUOUS: &'static str = "ERR_EXTRACT_AMBIGUOUS";
    #[classattr]
    const ERR_EXTRACT_FORMAT_UNSUPPORTED: &'static str = "ERR_EXTRACT_FORMAT_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_JSON_MALFORMED: &'static str = "ERR_EXTRACT_JSON_MALFORMED";
    #[classattr]
    const ERR_EXTRACT_JSON_DUPLICATE_KEY: &'static str = "ERR_EXTRACT_JSON_DUPLICATE_KEY";
    #[classattr]
    const ERR_EXTRACT_JSON_PATH_MISSING: &'static str = "ERR_EXTRACT_JSON_PATH_MISSING";
    #[classattr]
    const ERR_EXTRACT_JSON_SHAPE_MISMATCH: &'static str = "ERR_EXTRACT_JSON_SHAPE_MISMATCH";
    #[classattr]
    const ERR_EXTRACT_JSON_VALUE_UNSUPPORTED: &'static str = "ERR_EXTRACT_JSON_VALUE_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_JSON_AMBIGUOUS: &'static str = "ERR_EXTRACT_JSON_AMBIGUOUS";
    #[classattr]
    const ERR_EXTRACT_YAML_MALFORMED: &'static str = "ERR_EXTRACT_YAML_MALFORMED";
    #[classattr]
    const ERR_EXTRACT_YAML_DUPLICATE_KEY: &'static str = "ERR_EXTRACT_YAML_DUPLICATE_KEY";
    #[classattr]
    const ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED: &'static str = "ERR_EXTRACT_YAML_ALIAS_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_YAML_DOCUMENT_STREAM: &'static str = "ERR_EXTRACT_YAML_DOCUMENT_STREAM";
    #[classattr]
    const ERR_EXTRACT_YAML_PATH_MISSING: &'static str = "ERR_EXTRACT_YAML_PATH_MISSING";
    #[classattr]
    const ERR_EXTRACT_YAML_SHAPE_MISMATCH: &'static str = "ERR_EXTRACT_YAML_SHAPE_MISMATCH";
    #[classattr]
    const ERR_EXTRACT_YAML_VALUE_UNSUPPORTED: &'static str = "ERR_EXTRACT_YAML_VALUE_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_YAML_AMBIGUOUS: &'static str = "ERR_EXTRACT_YAML_AMBIGUOUS";
    #[classattr]
    const ERR_EXTRACT_TOML_MALFORMED: &'static str = "ERR_EXTRACT_TOML_MALFORMED";
    #[classattr]
    const ERR_EXTRACT_INPUT_LIMIT: &'static str = "ERR_EXTRACT_INPUT_LIMIT";
    #[classattr]
    const ERR_EXTRACT_TOML_DUPLICATE_KEY: &'static str = "ERR_EXTRACT_TOML_DUPLICATE_KEY";
    #[classattr]
    const ERR_EXTRACT_TOML_PATH_MISSING: &'static str = "ERR_EXTRACT_TOML_PATH_MISSING";
    #[classattr]
    const ERR_EXTRACT_TOML_SHAPE_MISMATCH: &'static str = "ERR_EXTRACT_TOML_SHAPE_MISMATCH";
    #[classattr]
    const ERR_EXTRACT_TOML_VALUE_UNSUPPORTED: &'static str = "ERR_EXTRACT_TOML_VALUE_UNSUPPORTED";
    #[classattr]
    const ERR_EXTRACT_TOML_AMBIGUOUS: &'static str = "ERR_EXTRACT_TOML_AMBIGUOUS";
    #[classattr]
    const ERR_JSON_ESCAPE_MODE_NON_JSON: &'static str = "ERR_JSON_ESCAPE_MODE_NON_JSON";
    #[classattr]
    const ERR_JSON_LEGACY_NON_STRING: &'static str = "ERR_JSON_LEGACY_NON_STRING";
    #[classattr]
    const ERR_JSON_MODE_CONTRACT: &'static str = "ERR_JSON_MODE_CONTRACT";
    #[classattr]
    const WARN_JSON_LEGACY_ESCAPE_MODE: &'static str = "WARN_JSON_LEGACY_ESCAPE_MODE";
    #[classattr]
    const WARN_JSON_QUOTED_PLACEHOLDER: &'static str = "WARN_JSON_QUOTED_PLACEHOLDER";
    #[classattr]
    const WARN_EXTRACT_NOT_OBSERVED: &'static str = "WARN_EXTRACT_NOT_OBSERVED";
    #[classattr]
    const WARN_EXTRACT_LOW_CONFIDENCE: &'static str = "WARN_EXTRACT_LOW_CONFIDENCE";
    #[classattr]
    const WARN_EXTRACT_DIRTY_PREFIX_STRIPPED: &'static str = "WARN_EXTRACT_DIRTY_PREFIX_STRIPPED";
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyRuntimeKind>()?;
    module.add_class::<PyProfileKind>()?;
    module.add_class::<PyUnknownVariablePolicy>()?;
    module.add_class::<PyJsonEscapeMode>()?;
    module.add_class::<PyVariableSource>()?;
    module.add_class::<PyDiagnosticSeverity>()?;
    module.add_class::<PyDiagnosticCode>()?;
    Ok(())
}

pub(crate) fn parse_runtime_kind(value: &Bound<'_, PyAny>) -> PyResult<RuntimeKind> {
    match value.extract::<String>()?.as_str() {
        "claude" => Ok(RuntimeKind::Claude),
        "codex" => Ok(RuntimeKind::Codex),
        "gemini" => Ok(RuntimeKind::Gemini),
        "opencode" => Ok(RuntimeKind::Opencode),
        other => Err(config_error(
            format!("unknown runtime kind: {other}"),
            Some("ERR_CONFIG_MODE"),
        )),
    }
}

pub(crate) fn parse_profile_kind(value: &Bound<'_, PyAny>) -> PyResult<ProfileKind> {
    match value.extract::<String>()?.as_str() {
        "agent" => Ok(ProfileKind::Agent),
        "command" => Ok(ProfileKind::Command),
        "skill" => Ok(ProfileKind::Skill),
        other => Err(config_error(
            format!("unknown profile kind: {other}"),
            Some("ERR_CONFIG_MODE"),
        )),
    }
}

pub(crate) fn parse_unknown_variable_policy(value: &str) -> PyResult<UnknownVariablePolicy> {
    match value {
        "error" => Ok(UnknownVariablePolicy::Error),
        "warn" => Ok(UnknownVariablePolicy::Warn),
        "ignore" => Ok(UnknownVariablePolicy::Ignore),
        other => Err(config_error(
            format!("unknown unknown-variable policy: {other}"),
            Some("ERR_CONFIG_MODE"),
        )),
    }
}

pub(crate) fn parse_json_escape_mode(value: &str) -> PyResult<JsonEscapeMode> {
    match value {
        "auto" => Ok(JsonEscapeMode::Auto),
        "legacy" => Ok(JsonEscapeMode::Legacy),
        other => Err(config_error(
            format!("unknown JSON escape mode: {other}"),
            Some("ERR_CONFIG_MODE"),
        )),
    }
}

pub(crate) fn parse_extract_format(value: &str) -> PyResult<ExtractFormat> {
    match value {
        "xml" => Ok(ExtractFormat::Xml),
        "json" => Ok(ExtractFormat::Json),
        "yaml" => Ok(ExtractFormat::Yaml),
        "toml" => Ok(ExtractFormat::Toml),
        "raw" => Ok(ExtractFormat::Raw),
        other => Err(crate::errors::config_error_with_recovery_hints(
            format!(
                "unsupported extraction format `{other}`; use `xml`, `json`, `yaml`, `toml`, or `raw`"
            ),
            Some("ERR_EXTRACT_FORMAT_UNSUPPORTED"),
            vec!["set format to `xml`, `json`, `yaml`, `toml`, or `raw`".to_owned()],
        )),
    }
}

pub(crate) const fn runtime_kind_str(value: RuntimeKind) -> &'static str {
    match value {
        RuntimeKind::Claude => "claude",
        RuntimeKind::Codex => "codex",
        RuntimeKind::Gemini => "gemini",
        RuntimeKind::Opencode => "opencode",
    }
}

pub(crate) const fn profile_kind_str(value: ProfileKind) -> &'static str {
    match value {
        ProfileKind::Agent => "agent",
        ProfileKind::Command => "command",
        ProfileKind::Skill => "skill",
    }
}

pub(crate) const fn unknown_variable_policy_str(value: UnknownVariablePolicy) -> &'static str {
    match value {
        UnknownVariablePolicy::Error => "error",
        UnknownVariablePolicy::Warn => "warn",
        UnknownVariablePolicy::Ignore => "ignore",
    }
}

pub(crate) const fn variable_source_str(value: &VariableSource) -> &'static str {
    match value {
        VariableSource::ExplicitInput => "explicit_input",
        VariableSource::Environment => "environment",
        VariableSource::Builtin => "builtin",
        VariableSource::TemplateInputDefault => "template_input_default",
        VariableSource::FrontmatterDefault => "frontmatter_default",
        VariableSource::IncludedDefault => "included_default",
    }
}

pub(crate) const fn diagnostic_severity_str(value: DiagnosticSeverity) -> &'static str {
    match value {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_and_profile_kinds_round_trip() {
        Python::initialize();
        Python::attach(|py| {
            let runtime = "claude".into_pyobject(py).unwrap();
            let profile = "agent".into_pyobject(py).unwrap();

            assert_eq!(
                parse_runtime_kind(runtime.as_any()).unwrap(),
                RuntimeKind::Claude
            );
            assert_eq!(
                parse_profile_kind(profile.as_any()).unwrap(),
                ProfileKind::Agent
            );
        });

        assert_eq!(runtime_kind_str(RuntimeKind::Codex), "codex");
        assert_eq!(profile_kind_str(ProfileKind::Skill), "skill");
        assert_eq!(
            unknown_variable_policy_str(UnknownVariablePolicy::Warn),
            "warn"
        );
        assert_eq!(diagnostic_severity_str(DiagnosticSeverity::Info), "info");
        assert_eq!(
            variable_source_str(&VariableSource::TemplateInputDefault),
            "template_input_default"
        );
    }

    #[test]
    fn invalid_unknown_variable_policy_maps_to_config_error() {
        Python::initialize();
        Python::attach(|py| {
            let err = parse_unknown_variable_policy("bogus").unwrap_err();
            let exc = err.value(py);
            let message = exc.getattr("message").unwrap().extract::<String>().unwrap();

            assert_eq!(exc.get_type().name().unwrap(), "ScConfigError");
            assert!(message.contains("unknown unknown-variable policy: bogus"));
        });
    }
}
