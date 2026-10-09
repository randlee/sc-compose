//! Read-only conflict planning; only a pending create can reach the writer.

use super::validate::ValidatedGraph;
use crate::contract::{
    BeadDependencyType, BeadEdgeAction, BeadEndpoint, BeadGraph, BeadGraphEdge, BeadGraphMode,
    BeadGraphNode, BeadGraphProvenance, BeadId, BeadNodeAction, GraphDependencyType, GraphEndpoint,
    MissingEdge, PROVENANCE_KEY, StepId,
};
use crate::error::{BeadComposeError, GraphConflictReason, GraphRelationInvalidReason};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(super) trait GraphReader {
    fn issues(&mut self, ids: &[BeadId]) -> Result<BTreeMap<BeadId, Value>, BeadComposeError>;
    fn dependencies(
        &mut self,
        id: &BeadId,
    ) -> Result<Vec<(BeadId, GraphDependencyType)>, BeadComposeError>;
}

pub(super) enum GraphPlan {
    Noop(BeadGraph),
    Create(PendingCreate),
}

pub(super) struct PendingCreate {
    pub(super) graph: BeadGraph,
    pub(super) payload: Value,
    pub(super) keys: BTreeMap<PlanKey, Option<StepId>>,
    pub(super) endpoints: Vec<(PlannedEndpoint, PlannedEndpoint)>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct PlanKey(String);

impl PlanKey {
    fn root() -> Self {
        Self("_root".to_owned())
    }

    fn step(step: &StepId, attach: bool) -> Self {
        if attach {
            Self(step.to_string())
        } else {
            Self(crate::contract::GraphEndpoint::Step(step.clone()).encoded())
        }
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// Keep identities until apply returns ids; receipt strings are only presentation.
pub(super) enum PlannedEndpoint {
    Named(BeadEndpoint),
    Root,
}

impl PlannedEndpoint {
    pub(super) fn resolve(&self, graph: &BeadGraph) -> Option<GraphEndpoint> {
        match self {
            Self::Named(BeadEndpoint::Bead(id)) => Some(GraphEndpoint::Bead(id.clone())),
            Self::Named(BeadEndpoint::Step(step)) => {
                graph.ids.get(step).cloned().map(GraphEndpoint::Bead)
            }
            Self::Root => graph.parent.clone().map(GraphEndpoint::Bead),
        }
    }
}

enum Endpoint {
    Existing {
        display: GraphEndpoint,
        id: BeadId,
    },
    Planned {
        display: GraphEndpoint,
        key: PlanKey,
    },
}

impl Endpoint {
    fn display(&self) -> &GraphEndpoint {
        match self {
            Self::Existing { display, .. } | Self::Planned { display, .. } => display,
        }
    }
}

fn endpoint(
    ep: &BeadEndpoint,
    ids: &BTreeMap<StepId, BeadId>,
    missing: &BTreeSet<StepId>,
    attach: bool,
) -> Endpoint {
    match ep {
        BeadEndpoint::Bead(id) => Endpoint::Existing {
            display: GraphEndpoint::Bead(id.clone()),
            id: id.clone(),
        },
        BeadEndpoint::Step(step) => match (missing.contains(step), ids.get(step)) {
            (false, Some(id)) => Endpoint::Existing {
                display: GraphEndpoint::Bead(id.clone()),
                id: id.clone(),
            },
            _ => Endpoint::Planned {
                display: ids.get(step).map_or_else(
                    || GraphEndpoint::Step(step.clone()),
                    |id| GraphEndpoint::Bead(id.clone()),
                ),
                key: PlanKey::step(step, attach),
            },
        },
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
    if let Some(payload) = pending.payload.as_object_mut() {
        payload.insert("edges".into(), json!(edges));
    }
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
        let root_key = PlanKey::root();
        nodes.push(json!({"key":root_key.as_str(),"type":"molecule","title":v.formula,"description":v.description,"metadata":{PROVENANCE_KEY:v.provenance(None)}}));
        keys.insert(root_key, None);
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
            let key = PlanKey::step(&step.id, attach);
            fields.insert("key".into(), json!(key.as_str()));
            if let Some(id) = ids.get(&step.id) {
                fields.insert("id".into(), json!(id));
            }
            if let Some(parent) = &v.parent {
                fields.insert("parent_id".into(), json!(parent));
            } else {
                fields.insert("parent_key".into(), json!("_root"));
            }
            let metadata = fields
                .entry("metadata")
                .or_insert_with(|| Value::Object(Map::default()));
            let provenance = json!(v.provenance(Some(step.id.clone())));
            match metadata {
                Value::Object(metadata) => {
                    metadata.insert(PROVENANCE_KEY.into(), provenance);
                }
                metadata => {
                    let mut object = Map::new();
                    object.insert(PROVENANCE_KEY.into(), provenance);
                    *metadata = Value::Object(object);
                }
            }
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
            from: from.display().clone(),
            to: to.display().clone(),
            kind: kind.clone(),
            action: if existing {
                BeadEdgeAction::Existing
            } else {
                BeadEdgeAction::Add
            },
        });
        if matches!(from, Endpoint::Planned { .. }) || matches!(to, Endpoint::Planned { .. }) {
            let mut edge = serde_json::Map::new();
            for (prefix, ep) in [("from", from), ("to", to)] {
                match ep {
                    Endpoint::Planned { key, .. } => {
                        edge.insert(format!("{prefix}_key"), json!(key.as_str()));
                    }
                    Endpoint::Existing { id, .. } => {
                        edge.insert(format!("{prefix}_id"), json!(id));
                    }
                }
            }
            edge.insert("type".into(), json!(kind.to_string()));
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
        let to = if attach {
            v.parent.as_ref().map_or_else(
                || Endpoint::Planned {
                    display: GraphEndpoint::Root,
                    key: PlanKey::root(),
                },
                |id| Endpoint::Existing {
                    display: GraphEndpoint::Bead(id.clone()),
                    id: id.clone(),
                },
            )
        } else {
            Endpoint::Planned {
                display: GraphEndpoint::Root,
                key: PlanKey::root(),
            }
        };
        let existing = check_edge(
            &from,
            &to,
            &GraphDependencyType::ParentChild,
            &existing_edges,
            &mut absent,
        )?;
        pending.graph.edges.push(BeadGraphEdge {
            from: from.display().clone(),
            to: to.display().clone(),
            kind: GraphDependencyType::ParentChild,
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

type PlannedEdges = Vec<(BeadEndpoint, BeadEndpoint, GraphDependencyType)>;
type ExistingEdges = BTreeMap<(BeadId, BeadId), GraphDependencyType>;

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
                BeadDependencyType::Blocks.into(),
            ));
        }
    }
    for relation in &v.relations {
        let kind = relation.kind.into();
        planned_edges.push((relation.from.clone(), relation.to.clone(), kind));
    }
    let mut existing_edges = BTreeMap::new();
    // Include external sources: their dependency rows must also be checked.
    let mut sources = BTreeSet::new();
    for (from, _, _) in &planned_edges {
        let ep = endpoint(from, ids, missing, attach);
        if let Endpoint::Existing { id, .. } = ep {
            sources.insert(id);
        }
    }
    for step in &v.steps {
        if !missing.contains(&step.id)
            && let Some(id) = ids.get(&step.id)
        {
            sources.insert(id.clone());
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
    kind: &GraphDependencyType,
    existing: &BTreeMap<(BeadId, BeadId), GraphDependencyType>,
    absent: &mut Vec<MissingEdge>,
) -> Result<bool, BeadComposeError> {
    let (Endpoint::Existing { id: from, .. }, Endpoint::Existing { id: to, .. }) = (from, to)
    else {
        return Ok(false);
    };
    match existing.get(&(from.clone(), to.clone())) {
        Some(actual) if actual == kind => Ok(true),
        Some(actual) => Err(BeadComposeError::GraphEdgeConflict {
            from: from.clone(),
            to: to.clone(),
            existing: actual.clone(),
            requested: kind.clone(),
        }),
        None => {
            absent.push(MissingEdge {
                from: from.clone(),
                to: to.clone(),
                kind: kind.clone(),
            });
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph() -> BeadGraph {
        let step = StepId::new("build").unwrap();
        BeadGraph {
            mode: BeadGraphMode::Pour,
            parent: Some(BeadId::new("proj-1").unwrap()),
            ref_: None,
            formula: crate::FormulaName::new("example").unwrap(),
            revision: crate::Sha256Digest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
            plan_path: None,
            ids: BTreeMap::from([(step, BeadId::new("proj-1.build").unwrap())]),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    #[test]
    fn planned_endpoints_resolve_when_their_identity_is_available() {
        let graph = graph();
        let step = StepId::new("build").unwrap();
        let endpoint = PlannedEndpoint::Named(BeadEndpoint::Step(step));

        assert_eq!(
            endpoint.resolve(&graph),
            Some(GraphEndpoint::Bead(BeadId::new("proj-1.build").unwrap()))
        );
        assert_eq!(
            PlannedEndpoint::Root.resolve(&graph),
            graph.parent.clone().map(GraphEndpoint::Bead)
        );
        let bead = BeadId::new("proj-9").unwrap();
        assert_eq!(
            PlannedEndpoint::Named(BeadEndpoint::Bead(bead.clone())).resolve(&graph),
            Some(GraphEndpoint::Bead(bead))
        );
    }

    #[test]
    fn unresolved_planned_endpoints_return_none() {
        let mut graph = graph();
        graph.ids.clear();
        graph.parent = None;

        assert_eq!(
            PlannedEndpoint::Named(BeadEndpoint::Step(StepId::new("build").unwrap()))
                .resolve(&graph),
            None
        );
        assert_eq!(PlannedEndpoint::Root.resolve(&graph), None);
    }

    #[test]
    fn planned_endpoint_pair_is_not_treated_as_an_existing_dependency() {
        let from = Endpoint::Planned {
            display: GraphEndpoint::Step(StepId::new("build").unwrap()),
            key: PlanKey::step(&StepId::new("build").unwrap(), false),
        };
        let id = BeadId::new("proj-1").unwrap();
        let to = Endpoint::Existing {
            display: GraphEndpoint::Bead(id.clone()),
            id,
        };
        let mut absent = Vec::new();

        let result = check_edge(
            &from,
            &to,
            &GraphDependencyType::Known(BeadDependencyType::Blocks),
            &BTreeMap::new(),
            &mut absent,
        );

        assert!(!result.unwrap());
        assert!(absent.is_empty());
    }
}
