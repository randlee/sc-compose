//! Read-only conflict planning; only a pending create can reach the writer.

use super::validate::ValidatedGraph;
use crate::contract::{
    BeadEdgeAction, BeadEndpoint, BeadGraph, BeadGraphEdge, BeadGraphMode, BeadGraphNode,
    BeadGraphProvenance, BeadId, BeadNodeAction, MissingEdge, PROVENANCE_KEY, StepId,
};
use crate::error::{BeadComposeError, GraphConflictReason, GraphRelationInvalidReason};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) trait GraphReader {
    fn issues(&mut self, ids: &[BeadId]) -> Result<BTreeMap<BeadId, Value>, BeadComposeError>;
    fn dependencies(&mut self, id: &BeadId) -> Result<Vec<(BeadId, String)>, BeadComposeError>;
}

pub(super) enum GraphPlan {
    Noop(BeadGraph),
    Create(PendingCreate),
}

pub(super) struct PendingCreate {
    pub(super) graph: BeadGraph,
    pub(super) payload: Value,
    pub(super) keys: BTreeMap<String, Option<StepId>>,
    pub(super) endpoints: Vec<(PlannedEndpoint, PlannedEndpoint)>,
}

/// Keep identities until apply returns ids; receipt strings are only presentation.
pub(super) enum PlannedEndpoint {
    Named(BeadEndpoint),
    Root,
}

impl PlannedEndpoint {
    pub(super) fn resolve(&self, graph: &BeadGraph) -> String {
        match self {
            Self::Named(BeadEndpoint::Bead(id)) => id.to_string(),
            Self::Named(BeadEndpoint::Step(step)) => graph.ids[step].to_string(),
            Self::Root => graph.parent.as_ref().expect("created root").to_string(),
        }
    }
}

struct Endpoint {
    display: String,
    key: Option<String>,
    id: Option<BeadId>,
}

fn endpoint(
    ep: &BeadEndpoint,
    ids: &BTreeMap<StepId, BeadId>,
    missing: &BTreeSet<StepId>,
    attach: bool,
) -> Endpoint {
    match ep {
        BeadEndpoint::Bead(id) => Endpoint {
            display: id.to_string(),
            key: None,
            id: Some(id.clone()),
        },
        BeadEndpoint::Step(step) => Endpoint {
            display: ids
                .get(step)
                .map_or_else(|| format!("step:{step}"), ToString::to_string),
            key: missing.contains(step).then(|| step_key(step, attach)),
            id: ids.get(step).cloned(),
        },
    }
}

fn step_key(step: &StepId, attach: bool) -> String {
    if attach {
        step.to_string()
    } else {
        format!("step:{step}")
    }
}

pub(super) fn plan(
    v: &ValidatedGraph,
    reader: &mut impl GraphReader,
) -> Result<GraphPlan, BeadComposeError> {
    let attach = v.mode == BeadGraphMode::Attach;
    let mut ids = BTreeMap::new();
    let mut requested = BTreeSet::new();
    if let (Some(parent), Some(reference)) = (&v.parent, &v.reference) {
        requested.insert(parent.clone());
        for step in &v.steps {
            let id = BeadId::new(format!("{parent}.{reference}-{}", step.id))?;
            requested.insert(id.clone());
            ids.insert(step.id.clone(), id);
        }
    }
    for relation in &v.relations {
        for ep in [&relation.from, &relation.to] {
            if let BeadEndpoint::Bead(id) = ep {
                requested.insert(id.clone());
            }
        }
    }
    let found = if requested.is_empty() {
        BTreeMap::new()
    } else {
        reader.issues(&requested.into_iter().collect::<Vec<_>>())?
    };
    if let Some(parent) = &v.parent
        && !found.contains_key(parent)
    {
        return Err(BeadComposeError::GraphParentNotFound {
            parent: parent.clone(),
        });
    }
    for (index, relation) in v.relations.iter().enumerate() {
        for ep in [&relation.from, &relation.to] {
            if let BeadEndpoint::Bead(id) = ep
                && !found.contains_key(id)
            {
                return Err(BeadComposeError::GraphRelationInvalid {
                    index,
                    reason: GraphRelationInvalidReason::BeadNotFound,
                });
            }
        }
    }
    let missing = check_ownership(v, &ids, &found)?;
    let mut pending = create_nodes(v, &ids, &missing, attach);
    let edges = plan_edges(v, &ids, &missing, attach, reader, &mut pending)?;
    pending.payload["edges"] = json!(edges);
    if pending.keys.is_empty() {
        Ok(GraphPlan::Noop(pending.graph))
    } else {
        Ok(GraphPlan::Create(pending))
    }
}

fn check_ownership(
    v: &ValidatedGraph,
    ids: &BTreeMap<StepId, BeadId>,
    found: &BTreeMap<BeadId, Value>,
) -> Result<BTreeSet<StepId>, BeadComposeError> {
    let mut missing = BTreeSet::new();
    for step in &v.steps {
        if let Some((id, bead)) = ids
            .get(&step.id)
            .and_then(|id| found.get(id).map(|bead| (id, bead)))
        {
            let provenance = bead.get("metadata").and_then(|m| m.get(PROVENANCE_KEY));
            let reason = match provenance {
                None => Some(GraphConflictReason::NotOwned),
                Some(raw) => match serde_json::from_value::<BeadGraphProvenance>(raw.clone()) {
                    Ok(actual) if actual == v.provenance(Some(step.id.clone())) => None,
                    _ => Some(GraphConflictReason::ProvenanceDiffers),
                },
            };
            if let Some(reason) = reason {
                return Err(BeadComposeError::GraphConflict {
                    id: id.clone(),
                    reason,
                });
            }
        } else {
            missing.insert(step.id.clone());
        }
    }
    Ok(missing)
}

fn create_nodes(
    v: &ValidatedGraph,
    ids: &BTreeMap<StepId, BeadId>,
    missing: &BTreeSet<StepId>,
    attach: bool,
) -> PendingCreate {
    let mut graph = BeadGraph {
        mode: v.mode,
        parent: v.parent.clone(),
        ref_: v.reference.clone(),
        formula: v.formula.clone(),
        revision: v.revision.clone(),
        plan_path: None,
        ids: ids.clone(),
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    let mut nodes = Vec::new();
    let mut keys = BTreeMap::new();
    if !attach {
        nodes.push(json!({"key":"_root","type":"molecule","title":v.formula,"description":v.description,"metadata":{PROVENANCE_KEY:v.provenance(None)}}));
        keys.insert("_root".into(), None);
        graph.nodes.push(BeadGraphNode {
            step: None,
            id: None,
            action: BeadNodeAction::Create,
        });
    }
    for step in &v.steps {
        let create = missing.contains(&step.id);
        graph.nodes.push(BeadGraphNode {
            step: Some(step.id.clone()),
            id: ids.get(&step.id).cloned(),
            action: if create {
                BeadNodeAction::Create
            } else {
                BeadNodeAction::Existing
            },
        });
        if create {
            let mut fields = step.fields.clone();
            let key = step_key(&step.id, attach);
            fields.insert("key".into(), json!(key));
            if let Some(id) = ids.get(&step.id) {
                fields.insert("id".into(), json!(id));
            }
            if let Some(parent) = &v.parent {
                fields.insert("parent_id".into(), json!(parent));
            } else {
                fields.insert("parent_key".into(), json!("_root"));
            }
            fields
                .get_mut("metadata")
                .and_then(Value::as_object_mut)
                .expect("validated metadata")
                .insert(
                    PROVENANCE_KEY.into(),
                    json!(v.provenance(Some(step.id.clone()))),
                );
            nodes.push(Value::Object(fields));
            keys.insert(key, Some(step.id.clone()));
        }
    }
    PendingCreate {
        graph,
        payload: json!({"nodes": nodes}),
        endpoints: Vec::new(),
        keys,
    }
}

fn plan_edges(
    v: &ValidatedGraph,
    ids: &BTreeMap<StepId, BeadId>,
    missing: &BTreeSet<StepId>,
    attach: bool,
    reader: &mut impl GraphReader,
    pending: &mut PendingCreate,
) -> Result<Vec<Value>, BeadComposeError> {
    let (planned_edges, existing_edges) = read_edges(v, ids, missing, attach, reader)?;
    let mut absent = Vec::new();
    let mut edges = Vec::new();
    for (from, to, kind) in planned_edges {
        pending.endpoints.push((
            PlannedEndpoint::Named(from.clone()),
            PlannedEndpoint::Named(to.clone()),
        ));
        let from = endpoint(&from, ids, missing, attach);
        let to = endpoint(&to, ids, missing, attach);
        let existing = check_edge(&from, &to, &kind, &existing_edges, &mut absent)?;
        pending.graph.edges.push(BeadGraphEdge {
            from: from.display.clone(),
            to: to.display.clone(),
            kind: kind.clone(),
            action: if existing {
                BeadEdgeAction::Existing
            } else {
                BeadEdgeAction::Add
            },
        });
        if from.key.is_some() || to.key.is_some() {
            let mut edge = serde_json::Map::new();
            for (prefix, ep) in [("from", from), ("to", to)] {
                if let Some(key) = ep.key {
                    edge.insert(format!("{prefix}_key"), json!(key));
                } else {
                    edge.insert(
                        format!("{prefix}_id"),
                        json!(ep.id.expect("existing endpoint")),
                    );
                }
            }
            edge.insert("type".into(), json!(kind));
            edges.push(Value::Object(edge));
        }
    }
    // Parent edges are supplied by parent_id/parent_key, never duplicated in the plan.
    for step in &v.steps {
        pending.endpoints.push((
            PlannedEndpoint::Named(BeadEndpoint::Step(step.id.clone())),
            v.parent.as_ref().map_or(PlannedEndpoint::Root, |id| {
                PlannedEndpoint::Named(BeadEndpoint::Bead(id.clone()))
            }),
        ));
        let from = endpoint(&BeadEndpoint::Step(step.id.clone()), ids, missing, attach);
        let to = Endpoint {
            display: v
                .parent
                .as_ref()
                .map_or_else(|| "_root".into(), ToString::to_string),
            key: (!attach).then(|| "_root".into()),
            id: v.parent.clone(),
        };
        let existing = check_edge(&from, &to, "parent-child", &existing_edges, &mut absent)?;
        pending.graph.edges.push(BeadGraphEdge {
            from: from.display,
            to: to.display,
            kind: "parent-child".into(),
            action: if existing {
                BeadEdgeAction::Existing
            } else {
                BeadEdgeAction::Add
            },
        });
    }
    if !absent.is_empty() {
        return Err(BeadComposeError::GraphEdgeMissing { edges: absent });
    }
    Ok(edges)
}

type PlannedEdges = Vec<(BeadEndpoint, BeadEndpoint, String)>;
type ExistingEdges = BTreeMap<(BeadId, BeadId), String>;

fn read_edges(
    v: &ValidatedGraph,
    ids: &BTreeMap<StepId, BeadId>,
    missing: &BTreeSet<StepId>,
    attach: bool,
    reader: &mut impl GraphReader,
) -> Result<(PlannedEdges, ExistingEdges), BeadComposeError> {
    let mut planned_edges = Vec::new();
    for step in &v.steps {
        for dep in &step.needs {
            planned_edges.push((
                BeadEndpoint::Step(step.id.clone()),
                BeadEndpoint::Step(dep.clone()),
                "blocks".to_owned(),
            ));
        }
    }
    for relation in &v.relations {
        let kind = serde_json::to_value(relation.kind)
            .expect("enum serializes")
            .as_str()
            .expect("string enum")
            .to_owned();
        planned_edges.push((relation.from.clone(), relation.to.clone(), kind));
    }
    let mut existing_edges = BTreeMap::new();
    // Include external sources: their dependency rows must also be checked.
    let mut sources = BTreeSet::new();
    for (from, _, _) in &planned_edges {
        let ep = endpoint(from, ids, missing, attach);
        if ep.key.is_none()
            && let Some(id) = ep.id
        {
            sources.insert(id);
        }
    }
    for step in &v.steps {
        if !missing.contains(&step.id) {
            sources.insert(ids[&step.id].clone());
        }
    }
    for id in sources {
        for (to, kind) in reader.dependencies(&id)? {
            existing_edges.insert((id.clone(), to), kind);
        }
    }
    Ok((planned_edges, existing_edges))
}

fn check_edge(
    from: &Endpoint,
    to: &Endpoint,
    kind: &str,
    existing: &BTreeMap<(BeadId, BeadId), String>,
    absent: &mut Vec<MissingEdge>,
) -> Result<bool, BeadComposeError> {
    if from.key.is_some() || to.key.is_some() {
        return Ok(false);
    }
    let (from, to) = (
        from.id.as_ref().expect("existing from"),
        to.id.as_ref().expect("existing to"),
    );
    match existing.get(&(from.clone(), to.clone())) {
        Some(actual) if actual == kind => Ok(true),
        Some(actual) => Err(BeadComposeError::GraphEdgeConflict {
            from: from.clone(),
            to: to.clone(),
            existing: actual.clone(),
            requested: kind.into(),
        }),
        None => {
            absent.push(MissingEdge {
                from: from.clone(),
                to: to.clone(),
                kind: kind.into(),
            });
            Ok(false)
        }
    }
}
