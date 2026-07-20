use pyo3::prelude::*;
use sc_composer::{
    ComposePolicy, ComposeRequest, ConfiningRoot, LoadedTemplateRequest, NamedTemplateAsset,
};

use super::policy::{
    PyComposeMode, PyComposePolicy, compose_mode_repr, compose_policy_repr, python_bool_repr,
    python_option_string_repr,
};
use crate::convert::{
    coerce_path_like, extract_runtime_kind, extract_string_map, extract_supporting_templates,
    extract_var_map,
};
use crate::errors::config_error;

#[pyclass(name = "ComposeRequest", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyComposeRequest {
    pub(crate) inner: ComposeRequest,
}

pub(crate) fn compose_request_repr(request: &ComposeRequest) -> String {
    format!(
        "ComposeRequest(root={:?}, mode={}, runtime={}, vars_input={}, vars_env={}, vars_defaults={}, guidance_block={}, user_prompt={}, policy={})",
        request.root.as_path().display().to_string(),
        compose_mode_repr(&request.mode),
        python_option_string_repr(request.runtime.map(crate::enums::runtime_kind_str)),
        request.vars_input.len(),
        request.vars_env.len(),
        request.vars_defaults.len(),
        python_bool_repr(request.guidance_block.is_some()),
        python_bool_repr(request.user_prompt.is_some()),
        compose_policy_repr(&request.policy)
    )
}

#[pymethods]
impl PyComposeRequest {
    #[new]
    #[pyo3(signature = (root, mode, vars_input=None, vars_env=None, vars_defaults=None, guidance_block=None, user_prompt=None, policy=None, runtime=None))]
    #[allow(
        clippy::too_many_arguments,
        clippy::needless_pass_by_value,
        reason = "Python constructor shape is part of the planned public API and PyO3 extracts owned PyRef arguments."
    )]
    fn new(
        root: &Bound<'_, PyAny>,
        mode: PyRef<'_, PyComposeMode>,
        vars_input: Option<&Bound<'_, PyAny>>,
        vars_env: Option<&Bound<'_, PyAny>>,
        vars_defaults: Option<&Bound<'_, PyAny>>,
        guidance_block: Option<String>,
        user_prompt: Option<String>,
        policy: Option<PyRef<'_, PyComposePolicy>>,
        runtime: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let root_string = coerce_path_like(root)?;
        Ok(Self {
            inner: ComposeRequest {
                runtime: extract_runtime_kind(runtime)?,
                mode: mode.inner.clone(),
                root: ConfiningRoot::new(&root_string)
                    .map_err(|error| config_error(error.to_string(), Some("ERR_CONFIG_PARSE")))?,
                vars_input: extract_var_map(vars_input)?,
                vars_env: extract_var_map(vars_env)?,
                vars_defaults: extract_var_map(vars_defaults)?,
                guidance_block,
                user_prompt,
                policy: policy
                    .as_ref()
                    .map_or_else(ComposePolicy::default, |policy| policy.inner.clone()),
            },
        })
    }

    #[getter]
    fn root(&self) -> String {
        self.inner.root.as_path().display().to_string()
    }

    #[getter]
    fn runtime(&self) -> Option<String> {
        self.inner
            .runtime
            .map(|runtime| crate::enums::runtime_kind_str(runtime).to_owned())
    }

    #[getter]
    fn mode(&self) -> PyComposeMode {
        PyComposeMode {
            inner: self.inner.mode.clone(),
        }
    }

    #[getter]
    fn policy(&self) -> PyComposePolicy {
        PyComposePolicy {
            inner: self.inner.policy.clone(),
        }
    }

    fn __repr__(&self) -> String {
        compose_request_repr(&self.inner)
    }
}

#[pyclass(name = "NamedTemplateAsset", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyNamedTemplateAsset {
    pub(crate) inner: NamedTemplateAsset,
}

#[pymethods]
impl PyNamedTemplateAsset {
    #[new]
    fn new(template_name: String, template_text: String) -> Self {
        Self {
            inner: NamedTemplateAsset {
                template_name,
                template_text,
            },
        }
    }

    #[getter]
    fn template_name(&self) -> String {
        self.inner.template_name.clone()
    }

    #[getter]
    fn template_text(&self) -> String {
        self.inner.template_text.clone()
    }
}

#[pyclass(name = "LoadedTemplateRequest", skip_from_py_object)]
#[derive(Clone, Debug)]
pub(crate) struct PyLoadedTemplateRequest {
    pub(crate) inner: LoadedTemplateRequest,
}

#[pymethods]
impl PyLoadedTemplateRequest {
    #[new]
    #[pyo3(signature = (template_name, template_text, context, supporting_templates=None))]
    fn new(
        template_name: String,
        template_text: String,
        context: &Bound<'_, PyAny>,
        supporting_templates: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: LoadedTemplateRequest {
                template_name,
                template_text,
                context: extract_string_map(context)?,
                supporting_templates: extract_supporting_templates(supporting_templates)?,
            },
        })
    }
}
