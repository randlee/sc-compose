use pyo3::prelude::*;

mod policy;
mod request;
mod results;

pub(crate) use policy::{
    PyComposeMode, PyComposePolicy, PyConfiningRoot, PyPassConfig, PyProfileName, PyResolverPolicy,
    PyVariableName,
};
pub(crate) use request::{PyComposeRequest, PyLoadedTemplateRequest, PyNamedTemplateAsset};
pub(crate) use results::{
    PyComposeResult, PyDiagnostic, PyExpandedTemplate, PyFrontmatter, PyFrontmatterInitResult,
    PyInitResult, PyParsedTemplate, PyRenderedArtifact, PyRenderer, PyResolveResult,
    PyValidationReport, PyVerifyResult,
};

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyVariableName>()?;
    module.add_class::<PyProfileName>()?;
    module.add_class::<PyConfiningRoot>()?;
    module.add_class::<PyResolverPolicy>()?;
    module.add_class::<PyComposeMode>()?;
    module.add_class::<PyComposePolicy>()?;
    module.add_class::<PyPassConfig>()?;
    module.add_class::<PyComposeRequest>()?;
    module.add_class::<PyDiagnostic>()?;
    module.add_class::<PyResolveResult>()?;
    module.add_class::<PyComposeResult>()?;
    module.add_class::<PyValidationReport>()?;
    module.add_class::<PyVerifyResult>()?;
    module.add_class::<PyNamedTemplateAsset>()?;
    module.add_class::<PyLoadedTemplateRequest>()?;
    module.add_class::<PyRenderedArtifact>()?;
    module.add_class::<PyFrontmatter>()?;
    module.add_class::<PyParsedTemplate>()?;
    module.add_class::<PyExpandedTemplate>()?;
    module.add_class::<PyFrontmatterInitResult>()?;
    module.add_class::<PyInitResult>()?;
    module.add_class::<PyRenderer>()?;
    Ok(())
}
