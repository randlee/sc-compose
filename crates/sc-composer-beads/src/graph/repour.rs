//! Convergent repour: find the beads of an earlier pour and diff plan-owned fields.
//!
//! A repour targets the pour root. The existing step beads are the root's
//! children whose provenance names the same formula and a step id; no other
//! child is adopted. Only fields the plan owns are compared: title,
//! description, design, acceptance criteria, priority, additional labels and
//! plan metadata keys. Runtime state (status, notes, assignee, close state)
//! and every field the plan does not mention are never read as drift and never
//! written.

use super::plan::GraphReader;
use super::validate::ValidatedGraph;
use crate::contract::{BeadGraphMode, BeadId, GraphRef, PROVENANCE_KEY, StepId};
use crate::error::{BeadComposeError, GraphConflictReason};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// The ownership fields shared by provenance v1 and v2.
#[derive(Deserialize)]
struct Owner {
    mode: BeadGraphMode,
    formula: crate::FormulaName,
    step: Option<StepId>,
    #[serde(default)]
    parent: Option<BeadId>,
    #[serde(default, rename = "ref")]
    reference: Option<GraphRef>,
}

fn owner(row: &Value) -> Option<Owner> {
    let raw = row.get("metadata")?.get(PROVENANCE_KEY)?;
    serde_json::from_value(raw.clone()).ok()
}

/// The stored beads of an earlier pour, found through its root.
pub(super) struct Existing {
    /// Plan steps that already have a bead.
    pub ids: BTreeMap<StepId, BeadId>,
    /// Poured beads whose step is no longer in the plan.
    pub retained: Vec<(StepId, BeadId)>,
    /// Stored rows of the root and every poured bead, as read.
    pub rows: BTreeMap<BeadId, Value>,
}

fn conflict(id: &BeadId, reason: GraphConflictReason) -> BeadComposeError {
    BeadComposeError::GraphConflict {
        id: id.clone(),
        reason,
    }
}

/// Read the root and its poured children, refusing a root that is not the
/// molecule of this formula.
pub(super) fn discover(
    v: &ValidatedGraph,
    root: &BeadId,
    reader: &mut impl GraphReader,
) -> Result<Existing, BeadComposeError> {
    let mut rows = reader.issues(std::slice::from_ref(root))?;
    let root_row = rows
        .remove(root)
        .ok_or_else(|| BeadComposeError::GraphParentNotFound {
            parent: root.clone(),
        })?;
    match owner(&root_row) {
        None => return Err(conflict(root, GraphConflictReason::NotOwned)),
        Some(owner)
            if owner.mode != BeadGraphMode::Pour
                || owner.formula != v.formula
                || owner.step.is_some() =>
        {
            return Err(conflict(root, GraphConflictReason::ProvenanceDiffers));
        }
        Some(_) => {}
    }
    let mut existing = Existing {
        ids: BTreeMap::new(),
        retained: Vec::new(),
        rows: BTreeMap::from([(root.clone(), root_row)]),
    };
    for (id, row) in reader.children(root)? {
        let Some(Owner {
            mode: BeadGraphMode::Pour,
            formula,
            step: Some(step),
            ..
        }) = owner(&row)
        else {
            continue;
        };
        if formula != v.formula {
            continue;
        }
        if v.steps.iter().any(|planned| planned.id == step) {
            if existing.ids.insert(step, id.clone()).is_some() {
                return Err(conflict(&id, GraphConflictReason::ProvenanceDiffers));
            }
        } else {
            existing.retained.push((step, id.clone()));
        }
        existing.rows.insert(id, row);
    }
    Ok(existing)
}

/// Find the beads of an earlier attach of the same formula and `ref`.
///
/// Ids are deterministic, so a planned step is matched by id and must be owned
/// by this attachment: same mode, formula, parent, ref and step. Whether its
/// node definition or revision changed is not ownership; that is an update.
/// Siblings of the same attachment that the plan no longer names are retained.
pub(super) fn discover_attach(
    v: &ValidatedGraph,
    ids: &BTreeMap<StepId, BeadId>,
    found: &BTreeMap<BeadId, Value>,
    reader: &mut impl GraphReader,
) -> Result<Existing, BeadComposeError> {
    let mut existing = Existing {
        ids: BTreeMap::new(),
        retained: Vec::new(),
        rows: BTreeMap::new(),
    };
    for step in &v.steps {
        let Some((id, row)) = ids.get(&step.id).and_then(|id| Some((id, found.get(id)?))) else {
            continue;
        };
        let owned = match owner(row) {
            None => return Err(conflict(id, GraphConflictReason::NotOwned)),
            Some(owner) => attach_identity(v, &owner, Some(&step.id)),
        };
        if !owned {
            return Err(conflict(id, GraphConflictReason::ProvenanceDiffers));
        }
        existing.ids.insert(step.id.clone(), id.clone());
        existing.rows.insert(id.clone(), row.clone());
    }
    if let Some(parent) = &v.parent {
        for (id, row) in reader.children(parent)? {
            let Some(Owner {
                step: Some(step), ..
            }) = owner(&row).filter(|owner| attach_identity(v, owner, owner.step.as_ref()))
            else {
                continue;
            };
            if !v.steps.iter().any(|planned| planned.id == step) {
                existing.retained.push((step, id.clone()));
                existing.rows.insert(id, row);
            }
        }
    }
    Ok(existing)
}

fn attach_identity(v: &ValidatedGraph, owner: &Owner, step: Option<&StepId>) -> bool {
    owner.mode == BeadGraphMode::Attach
        && owner.formula == v.formula
        && owner.parent == v.parent
        && owner.reference == v.reference
        && owner.step.as_ref() == step
}

/// Plan-owned writes for one existing bead, in the order they must run.
pub(super) struct NodeUpdate {
    pub id: BeadId,
    /// `bd update` argument lists: the plan fields first, the refreshed
    /// provenance last. Each flag is in `--flag=value` form.
    pub calls: Vec<Vec<String>>,
}

/// The `bd update` flags that bring `row` in line with the plan `fields`, and
/// the metadata object the bead holds once they are applied.
///
/// A field the plan states, including an explicitly empty one, is applied; a
/// field the plan does not state is left alone.
fn field_args(fields: &Map<String, Value>, row: &Value) -> (Vec<String>, Map<String, Value>) {
    let mut args = Vec::new();
    for (field, flag) in [
        ("title", "--title"),
        ("description", "--description"),
        ("design", "--design"),
        ("acceptance_criteria", "--acceptance"),
    ] {
        let stored = row.get(field).and_then(Value::as_str).unwrap_or_default();
        if let Some(want) = fields.get(field).and_then(Value::as_str)
            && want != stored
        {
            args.push(format!("{flag}={want}"));
        }
    }
    let priority = fields.get("priority").and_then(|value| {
        value.as_i64().or_else(|| {
            value
                .as_str()
                .and_then(|text| text.trim_start_matches(['P', 'p']).parse().ok())
        })
    });
    if let Some(want) = priority
        && row.get("priority").and_then(Value::as_i64) != Some(want)
    {
        args.push(format!("--priority={want}"));
    }
    // Labels are only added: a label someone attached at runtime stays.
    let stored: BTreeSet<&str> = row
        .get("labels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for label in fields
        .get("labels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|label| !stored.contains(label))
    {
        args.push(format!("--add-label={label}"));
    }
    let want = fields.get("metadata").and_then(Value::as_object);
    let (merged, changed) = merge_metadata(row, want);
    if changed {
        args.push(format!("--metadata={}", Value::Object(merged.clone())));
    }
    (args, merged)
}

/// Merge `want` into the stored metadata object, keeping every other key.
fn merge_metadata(row: &Value, want: Option<&Map<String, Value>>) -> (Map<String, Value>, bool) {
    let mut merged = row
        .get("metadata")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut changed = false;
    for (key, value) in want.into_iter().flatten() {
        if merged.get(key) != Some(value) {
            merged.insert(key.clone(), value.clone());
            changed = true;
        }
    }
    (merged, changed)
}

/// Updates for the root and every existing step bead whose plan fields or
/// provenance drifted.
///
/// Provenance is written after the plan fields in a call of its own. A bead
/// whose fields changed but whose provenance did not yet catch up is
/// recognised as out of date by the next run, which finishes it.
pub(super) fn node_updates(
    v: &ValidatedGraph,
    existing: &Existing,
) -> Result<Vec<NodeUpdate>, BeadComposeError> {
    let mut targets = Vec::new();
    if let Some(root) = &v.root {
        let mut molecule = Map::new();
        molecule.insert("title".into(), Value::String(v.formula.to_string()));
        molecule.insert("description".into(), Value::String(v.description.clone()));
        targets.push((root, molecule, v.provenance_v2(None)?));
    }
    for step in &v.steps {
        if let Some(id) = existing.ids.get(&step.id) {
            targets.push((id, step.fields.clone(), v.provenance_v2(Some(step))?));
        }
    }
    let mut updates = Vec::new();
    for (id, fields, expected) in targets {
        let Some(row) = existing.rows.get(id) else {
            continue;
        };
        let (mut calls, stored) = {
            let (args, merged) = field_args(&fields, row);
            let calls = if args.is_empty() { vec![] } else { vec![args] };
            (calls, merged)
        };
        let mut after = row.clone();
        after["metadata"] = Value::Object(stored);
        let want = provenance(&expected);
        let (merged, stale) = merge_metadata(&after, Some(&want));
        if stale {
            calls.push(vec![format!("--metadata={}", Value::Object(merged))]);
        }
        if !calls.is_empty() {
            updates.push(NodeUpdate {
                id: id.clone(),
                calls,
            });
        }
    }
    Ok(updates)
}

fn provenance(value: &impl serde::Serialize) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert(
        PROVENANCE_KEY.into(),
        serde_json::to_value(value).expect("provenance serializes"),
    );
    object
}

/// The plan-owned field differences of existing nodes.
pub(super) fn field_changes(v: &ValidatedGraph, existing: &Existing) -> Vec<crate::GraphChange> {
    let mut result = Vec::new();
    let mut targets = Vec::new();
    if let Some(root) = &v.root {
        let mut fields = Map::new();
        fields.insert("title".into(), Value::String(v.formula.to_string()));
        fields.insert("description".into(), Value::String(v.description.clone()));
        targets.push((root, fields));
    }
    for step in &v.steps {
        if let Some(id) = existing.ids.get(&step.id) {
            targets.push((id, step.fields.clone()));
        }
    }
    for (id, fields) in targets {
        let Some(row) = existing.rows.get(id) else {
            continue;
        };
        let status = row
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let (args, _) = field_args(&fields, row);
        for arg in args {
            let Some((flag, _)) = arg.split_once('=') else {
                continue;
            };
            let field = match flag {
                "--title" => "title",
                "--description" => "description",
                "--design" => "design",
                "--acceptance" => "acceptance_criteria",
                "--priority" => "priority",
                "--add-label" => "labels",
                "--metadata" => "metadata",
                _ => continue,
            };
            let before = row.get(field).cloned().unwrap_or(Value::Null);
            let after = fields.get(field).cloned().unwrap_or(Value::Null);
            let change = crate::GraphChange::FieldChange {
                bead: id.clone(),
                status: status.clone(),
                field: field.into(),
                before,
                after,
            };
            if !result.contains(&change) {
                result.push(change);
            }
        }
    }
    result
}
