//! Validate bd's resolved JSON before reading or writing graph state.

use crate::contract::{
    BeadComposeRequest, BeadEndpoint, BeadGraphMode, BeadGraphProvenance, BeadId, BeadRelation,
    GraphRef, PROVENANCE_KEY, Sha256Digest, StepId,
};
use crate::error::{
    BeadComposeError, GraphFormulaUnsupportedReason as Unsupported,
    GraphRelationInvalidReason as Invalid,
};
use sc_composer::{HashInput, calculate_hash};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
pub(super) struct CookedFormula {
    pub formula: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    steps: Vec<CookedStep>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct CookedStep {
    id: String,
    title: String,
    #[serde(default)]
    needs: Vec<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    metadata: Map<String, Value>,
    #[serde(flatten)]
    fields: BTreeMap<String, Value>,
}

pub(super) struct Step {
    pub id: StepId,
    pub fields: Map<String, Value>,
    pub needs: Vec<StepId>,
}

pub(super) struct ValidatedGraph {
    pub mode: BeadGraphMode,
    pub parent: Option<BeadId>,
    pub reference: Option<GraphRef>,
    pub formula: crate::FormulaName,
    pub description: String,
    pub revision: Sha256Digest,
    pub inputs: Sha256Digest,
    pub steps: Vec<Step>,
    pub relations: Vec<BeadRelation>,
}

impl ValidatedGraph {
    pub fn provenance(&self, step: Option<StepId>) -> BeadGraphProvenance {
        BeadGraphProvenance {
            v: 1,
            mode: self.mode,
            formula: self.formula.clone(),
            revision: self.revision.clone(),
            inputs: self.inputs.clone(),
            parent: self.parent.clone(),
            ref_: self.reference.clone(),
            step,
        }
    }
}

pub(super) fn unsupported(reason: Unsupported) -> BeadComposeError {
    BeadComposeError::GraphFormulaUnsupported { reason }
}

pub(super) fn scope(req: &BeadComposeRequest) -> Result<(), BeadComposeError> {
    for (field, expected) in [
        ("parent", req.parent.as_ref().map(BeadId::as_str)),
        ("ref", req.ref_.as_ref().map(GraphRef::as_str)),
    ] {
        if let (Some(expected), Some(actual)) = (expected, req.compose_variables.get(field))
            && actual.as_str() != Some(expected)
        {
            return Err(BeadComposeError::GraphScopeMismatch {
                field: field.into(),
                value: actual.to_string(),
            });
        }
    }
    Ok(())
}

fn nonempty(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(v) => !v.is_empty(),
        Value::Object(v) => !v.is_empty(),
        Value::String(v) => !v.is_empty(),
        _ => true,
    }
}

pub(super) fn digest(bytes: &[u8]) -> Result<Sha256Digest, BeadComposeError> {
    let hash = calculate_hash(HashInput::TextFileBytes {
        utf8_file_bytes: bytes,
    })
    .map_err(|_error| unsupported(Unsupported::NotUtf8))?;
    Sha256Digest::new(format!("sha256:{}", hash.template()))
}

pub(super) fn validate(
    req: &BeadComposeRequest,
    cooked: CookedFormula,
    rendered: &[u8],
    mode: BeadGraphMode,
) -> Result<ValidatedGraph, BeadComposeError> {
    if !req.bead_variables.is_empty() {
        return Err(unsupported(Unsupported::BeadVariablesSet));
    }
    if cooked.kind != "workflow" {
        return Err(unsupported(Unsupported::Composition));
    }
    for (key, value) in &cooked.extra {
        match key.as_str() {
            "vars" if nonempty(value) => return Err(unsupported(Unsupported::VarsDeclared)),
            "template" | "compose" | "advice" | "pointcuts" if nonempty(value) => {
                return Err(unsupported(Unsupported::Composition));
            }
            "vars" | "template" | "compose" | "advice" | "pointcuts" | "version"
            | "schema_version" | "source" | "phase" | "pour" | "intent" => {}
            _ => return Err(unsupported(Unsupported::UnknownKey)),
        }
    }
    if cooked.steps.is_empty() {
        return Err(unsupported(Unsupported::StepGraph));
    }
    let mut steps = Vec::new();
    let mut ids = BTreeSet::new();
    for step in cooked.steps {
        let id = StepId::new(step.id)?;
        if !ids.insert(id.clone()) {
            return Err(unsupported(Unsupported::StepGraph));
        }
        if step.metadata.contains_key(PROVENANCE_KEY) {
            return Err(unsupported(Unsupported::ReservedMetadata));
        }
        if step.labels.iter().any(|s| s.contains(',')) {
            return Err(unsupported(Unsupported::LabelComma));
        }
        let mut fields = Map::new();
        for (key, value) in step.fields {
            match key.as_str() {
                "description" | "notes" | "type" | "priority" | "assignee" => {
                    fields.insert(key, value);
                }
                "children" | "expand_vars" | "condition" | "gate" | "on_complete" | "waits_for" => {
                    if nonempty(&value) {
                        return Err(unsupported(Unsupported::StepConstruct));
                    }
                }
                _ => return Err(unsupported(Unsupported::UnknownKey)),
            }
        }
        fields.insert("title".into(), json!(step.title));
        if !step.labels.is_empty() {
            fields.insert("labels".into(), json!(step.labels));
        }
        fields.insert("metadata".into(), Value::Object(step.metadata));
        let mut needs = Vec::new();
        for dep in step.needs.into_iter().chain(step.depends_on) {
            let dep = StepId::new(dep)?;
            if !needs.contains(&dep) {
                needs.push(dep);
            }
        }
        steps.push(Step { id, fields, needs });
    }
    validate_relations(req, &steps, &ids)?;
    let mut relations = req.relations.clone();
    relations.sort_by_cached_key(|r| {
        (
            String::from(r.from.clone()),
            String::from(r.to.clone()),
            serde_json::to_string(&r.kind).expect("enum serializes"),
        )
    });
    let inputs = serde_json::to_vec(&json!({"mode": mode, "relations": relations}))
        .expect("contract serializes");
    Ok(ValidatedGraph {
        mode,
        parent: req.parent.clone(),
        reference: req.ref_.clone(),
        formula: req
            .formula_name
            .clone()
            .map_or_else(|| crate::FormulaName::new(cooked.formula), Ok)?,
        description: cooked.description,
        revision: digest(rendered)?,
        inputs: digest(&inputs)?,
        steps,
        relations: req.relations.clone(),
    })
}

fn validate_relations(
    req: &BeadComposeRequest,
    steps: &[Step],
    ids: &BTreeSet<StepId>,
) -> Result<(), BeadComposeError> {
    let mut pairs = BTreeSet::new();
    for step in steps {
        for dep in &step.needs {
            if !ids.contains(dep) {
                return Err(unsupported(Unsupported::StepGraph));
            }
            pairs.insert((format!("step:{}", step.id), format!("step:{dep}")));
        }
    }
    for (index, relation) in req.relations.iter().enumerate() {
        let invalid = |reason| BeadComposeError::GraphRelationInvalid { index, reason };
        for endpoint in [&relation.from, &relation.to] {
            if let BeadEndpoint::Step(step) = endpoint
                && !ids.contains(step)
            {
                return Err(invalid(Invalid::UnknownStep));
            }
            if let BeadEndpoint::Bead(id) = endpoint
                && req.parent.as_ref() == Some(id)
            {
                return Err(invalid(Invalid::ParentPair));
            }
        }
        if relation.from == relation.to {
            return Err(invalid(Invalid::SelfEdge));
        }
        if matches!(
            (&relation.from, &relation.to),
            (BeadEndpoint::Bead(_), BeadEndpoint::Bead(_))
        ) {
            return Err(invalid(Invalid::NoStep));
        }
        let pair = (
            String::from(relation.from.clone()),
            String::from(relation.to.clone()),
        );
        if !pairs.insert(pair) {
            return Err(invalid(Invalid::Duplicate));
        }
    }
    Ok(())
}
