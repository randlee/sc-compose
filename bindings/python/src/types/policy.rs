use std::collections::BTreeMap;
use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use sc_composer::{
    ComposeMode, ComposePolicy, ComposeRequest, ConfiningRoot, PassConfig, ResolverPolicy,
    VariableName,
};

use crate::convert::{
    coerce_path_like, extract_allowed_roots, extract_metadata_map, extract_pass_configs,
    extract_profile_name, extract_var_map, extract_variable_names, json_to_py,
};
use crate::enums::{
    parse_profile_kind, parse_unknown_variable_policy, profile_kind_str,
    unknown_variable_policy_str,
};
use crate::errors::{compose_error_to_pyerr, config_error, validation_error};

pub(crate) fn python_string_repr(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

pub(crate) fn python_option_string_repr(value: Option<&str>) -> String {
    value.map_or_else(|| "None".to_owned(), python_string_repr)
}

pub(crate) fn python_bool_repr(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

pub(crate) fn compose_mode_repr(mode: &ComposeMode) -> String {
    match mode {
        ComposeMode::File { template_path } => {
            format!(
                "ComposeMode.file({:?})",
                template_path.display().to_string()
            )
        }
        ComposeMode::Profile { kind, name } => format!(
            "ComposeMode.profile(kind={}, name={})",
            python_string_repr(profile_kind_str(*kind)),
            python_string_repr(name.as_str())
        ),
    }
}

pub(crate) fn compose_policy_repr(policy: &ComposePolicy) -> String {
    format!(
        "ComposePolicy(strict_undeclared_variables={}, unknown_variable_policy={}, unbound_variable_policy={}, max_include_depth={}, allowed_roots={:?}, resolver_policy={}, passes={})",
        python_bool_repr(policy.strict_undeclared_variables),
        python_string_repr(unknown_variable_policy_str(policy.unknown_variable_policy)),
        policy.unbound_variable_policy.map_or_else(
            || "None".to_owned(),
            |value| python_string_repr(unknown_variable_policy_str(value)),
        ),
        policy.max_include_depth.get(),
        policy
            .allowed_roots
            .iter()
            .map(|root| root.as_path().display().to_string())
            .collect::<Vec<_>>(),
        PyResolverPolicy {
            inner: policy.resolver_policy.clone(),
        }
        .__repr__(),
        policy.passes.len()
    )
}

#[pyclass(name = "VariableName", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyVariableName {
    pub(crate) inner: VariableName,
}

#[pymethods]
impl PyVariableName {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        VariableName::new(value)
            .map(|inner| Self { inner })
            .map_err(|error| validation_error(error.to_string(), None))
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("VariableName({:?})", self.inner.as_str())
    }
}

#[pyclass(name = "ProfileName", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyProfileName {
    pub(crate) inner: sc_composer::ProfileName,
}

#[pymethods]
impl PyProfileName {
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        sc_composer::ProfileName::new(value)
            .map(|inner| Self { inner })
            .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_MODE")))
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ProfileName({:?})", self.inner.as_str())
    }
}

#[pyclass(name = "ConfiningRoot", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyConfiningRoot {
    pub(crate) inner: ConfiningRoot,
}

#[pymethods]
impl PyConfiningRoot {
    #[new]
    fn new(path: &Bound<'_, PyAny>) -> PyResult<Self> {
        let path = coerce_path_like(path)?;
        ConfiningRoot::new(&path)
            .map(|inner| Self { inner })
            .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))
    }

    fn confine(&self, candidate: &Bound<'_, PyAny>) -> PyResult<String> {
        let candidate = coerce_path_like(candidate)?;
        let canonical = sc_composer::resolve_template_path(&ComposeRequest {
            runtime: None,
            mode: ComposeMode::File {
                template_path: PathBuf::from(candidate),
            },
            root: self.inner.clone(),
            vars_input: BTreeMap::new(),
            vars_env: BTreeMap::new(),
            vars_defaults: BTreeMap::new(),
            guidance_block: None,
            user_prompt: None,
            policy: ComposePolicy::default(),
        })
        .map_err(compose_error_to_pyerr)?;
        Ok(canonical.resolved_path.display().to_string())
    }

    fn __str__(&self) -> String {
        self.inner.as_path().display().to_string()
    }
}

#[pyclass(name = "ResolverPolicy", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyResolverPolicy {
    pub(crate) inner: ResolverPolicy,
}

#[pymethods]
impl PyResolverPolicy {
    fn __repr__(&self) -> String {
        format!(
            "ResolverPolicy(candidate_directories={}, filename_probes={}, ambiguous_without_runtime_is_error={})",
            self.inner.candidate_directories.len(),
            self.inner.filename_probes.len(),
            self.inner.ambiguous_without_runtime_is_error
        )
    }
}

#[pyclass(name = "ComposeMode", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyComposeMode {
    pub(crate) inner: ComposeMode,
}

#[pymethods]
impl PyComposeMode {
    #[staticmethod]
    fn file(template_path: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: ComposeMode::File {
                template_path: PathBuf::from(coerce_path_like(template_path)?),
            },
        })
    }

    #[staticmethod]
    fn profile(kind: &Bound<'_, PyAny>, name: &Bound<'_, PyAny>) -> PyResult<Self> {
        let kind = parse_profile_kind(kind)?;
        let name = extract_profile_name(name)?;
        Ok(Self {
            inner: ComposeMode::Profile { kind, name },
        })
    }

    #[getter]
    fn template_path(&self) -> Option<String> {
        match &self.inner {
            ComposeMode::File { template_path } => Some(template_path.display().to_string()),
            ComposeMode::Profile { .. } => None,
        }
    }

    #[getter]
    fn kind(&self) -> Option<String> {
        match &self.inner {
            ComposeMode::Profile { kind, .. } => Some(profile_kind_str(*kind).to_owned()),
            ComposeMode::File { .. } => None,
        }
    }

    #[getter]
    fn name(&self) -> Option<String> {
        match &self.inner {
            ComposeMode::Profile { name, .. } => Some(name.to_string()),
            ComposeMode::File { .. } => None,
        }
    }

    fn __repr__(&self) -> String {
        compose_mode_repr(&self.inner)
    }
}

#[pyclass(name = "ComposePolicy", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyComposePolicy {
    pub(crate) inner: ComposePolicy,
}

#[pymethods]
impl PyComposePolicy {
    #[new]
    #[pyo3(signature = (strict_undeclared_variables=false, unknown_variable_policy="ignore", unbound_variable_policy=None, max_include_depth=32, allowed_roots=None, passes=None))]
    fn new(
        strict_undeclared_variables: bool,
        unknown_variable_policy: &str,
        unbound_variable_policy: Option<&str>,
        max_include_depth: u16,
        allowed_roots: Option<&Bound<'_, PyAny>>,
        passes: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: ComposePolicy {
                strict_undeclared_variables,
                unknown_variable_policy: parse_unknown_variable_policy(unknown_variable_policy)?,
                unbound_variable_policy: unbound_variable_policy
                    .map(parse_unknown_variable_policy)
                    .transpose()?,
                max_include_depth: sc_composer::IncludeDepth::new(max_include_depth),
                allowed_roots: extract_allowed_roots(allowed_roots)?,
                resolver_policy: ResolverPolicy::default(),
                passes: extract_pass_configs(passes)?,
            },
        })
    }

    #[getter]
    fn strict_undeclared_variables(&self) -> bool {
        self.inner.strict_undeclared_variables
    }

    #[getter]
    fn unknown_variable_policy(&self) -> String {
        unknown_variable_policy_str(self.inner.unknown_variable_policy).to_owned()
    }

    #[getter]
    fn unbound_variable_policy(&self) -> Option<String> {
        self.inner
            .unbound_variable_policy
            .map(|value| unknown_variable_policy_str(value).to_owned())
    }

    #[getter]
    fn max_include_depth(&self) -> u16 {
        self.inner.max_include_depth.get()
    }

    #[getter]
    fn allowed_roots(&self) -> Vec<String> {
        self.inner
            .allowed_roots
            .iter()
            .map(|root| root.as_path().display().to_string())
            .collect()
    }

    #[getter]
    fn resolver_policy(&self) -> PyResolverPolicy {
        PyResolverPolicy {
            inner: self.inner.resolver_policy.clone(),
        }
    }

    #[getter]
    fn passes(&self) -> Vec<PyPassConfig> {
        self.inner
            .passes
            .iter()
            .cloned()
            .map(|inner| PyPassConfig { inner })
            .collect()
    }

    fn __repr__(&self) -> String {
        compose_policy_repr(&self.inner)
    }
}

#[pyclass(name = "PassConfig", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyPassConfig {
    pub(crate) inner: PassConfig,
}

#[pymethods]
impl PyPassConfig {
    #[new]
    #[pyo3(signature = (pass_number, required_variables=None, defaults=None, metadata=None))]
    fn new(
        pass_number: u8,
        required_variables: Option<&Bound<'_, PyAny>>,
        defaults: Option<&Bound<'_, PyAny>>,
        metadata: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: PassConfig {
                pass_number: if pass_number == 0 {
                    PassConfig::default().pass_number
                } else {
                    pass_number
                },
                required_variables: extract_variable_names(required_variables)?,
                defaults: extract_var_map(defaults)?,
                metadata: extract_metadata_map(metadata)?,
            },
        })
    }

    #[getter]
    fn pass_number(&self) -> u8 {
        self.inner.pass_number
    }

    #[getter]
    fn required_variables(&self) -> Vec<PyVariableName> {
        self.inner
            .required_variables
            .iter()
            .cloned()
            .map(|inner| PyVariableName { inner })
            .collect()
    }

    #[getter]
    fn defaults(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.defaults {
            dict.set_item(key.as_str(), json_to_py(py, value)?)?;
        }
        Ok(dict.into_any().unbind())
    }

    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.metadata {
            dict.set_item(
                key,
                json_to_py(
                    py,
                    &value.to_json_value().map_err(|error| {
                        config_error(error.to_string(), Some("ERR_CONFIG_PARSE"))
                    })?,
                )?,
            )?;
        }
        Ok(dict.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "PassConfig(pass_number={}, required_variables={:?}, defaults={}, metadata={})",
            self.inner.pass_number,
            self.required_variables()
                .into_iter()
                .map(|variable| variable.inner.to_string())
                .collect::<Vec<_>>(),
            self.inner.defaults.len(),
            self.inner.metadata.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use sc_composer::{ProfileKind, ProfileName};

    use super::super::request::{PyComposeRequest, compose_request_repr};
    use super::*;

    #[test]
    fn python_repr_helpers_match_python_style_output() {
        assert_eq!(python_string_repr("claude"), "'claude'");
        assert_eq!(python_option_string_repr(Some("claude")), "'claude'");
        assert_eq!(python_option_string_repr(None), "None");
        assert_eq!(python_bool_repr(true), "True");
        assert_eq!(python_bool_repr(false), "False");
    }

    #[test]
    fn compose_wrappers_emit_informative_repr_strings() {
        let mode = PyComposeMode {
            inner: ComposeMode::Profile {
                kind: ProfileKind::Agent,
                name: ProfileName::new("reviewer").unwrap(),
            },
        };
        let policy = PyComposePolicy {
            inner: ComposePolicy {
                strict_undeclared_variables: true,
                ..ComposePolicy::default()
            },
        };
        let request = PyComposeRequest {
            inner: ComposeRequest {
                runtime: Some(sc_composer::RuntimeKind::Claude),
                mode: mode.inner.clone(),
                root: ConfiningRoot::new(std::env::temp_dir()).unwrap(),
                vars_input: BTreeMap::from([(
                    VariableName::new("name").unwrap(),
                    serde_json::json!("world"),
                )]),
                vars_env: BTreeMap::new(),
                vars_defaults: BTreeMap::new(),
                guidance_block: Some("use the guide".to_owned()),
                user_prompt: Some("render this".to_owned()),
                policy: policy.inner.clone(),
            },
        };

        assert_eq!(
            mode.__repr__(),
            "ComposeMode.profile(kind='agent', name='reviewer')"
        );
        assert!(
            policy
                .__repr__()
                .contains("unknown_variable_policy='ignore'")
        );
        assert!(compose_request_repr(&request.inner).contains("runtime='claude'"));
        assert!(compose_request_repr(&request.inner).contains("guidance_block=True"));
        assert!(compose_request_repr(&request.inner).contains("user_prompt=True"));
    }
}
