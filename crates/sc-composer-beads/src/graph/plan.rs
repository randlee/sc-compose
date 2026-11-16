//! Read-only conflict planning; only a pending create can reach the writer.

use super::repour::{self, Existing, NodeUpdate};
use super::validate::{ProvenanceV2, Step, ValidatedGraph};
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
    fn dependents(
        &mut self,
        _id: &BeadId,
    ) -> Result<Vec<(BeadId, GraphDependencyType)>, BeadComposeError> {
        Ok(Vec::new())
    }
    /// The stored rows of the direct children of `root`, keyed by bead id.
    fn children(&mut self, root: &BeadId) -> Result<BTreeMap<BeadId, Value>, BeadComposeError>;
}

pub(super) enum GraphPlan {
    Noop(BeadGraph),
    Create(PendingCreate),
}

/// Everything an apply writes: new nodes, edges among existing beads and
/// plan-owned field updates of existing beads.
pub(super) struct PendingCreate {
    pub(super) graph: BeadGraph,
    pub(super) payload: Value,
    pub(super) keys: BTreeMap<PlanKey, Option<StepId>>,
    pub(super) endpoints: Vec<(PlannedEndpoint, PlannedEndpoint)>,
    /// Missing edges between two existing beads; `bd create --graph` cannot add them.
    pub(super) dep_adds: Vec<(BeadId, BeadId, GraphDependencyType)>,
    pub(super) dep_removes: Vec<(BeadId, BeadId, GraphDependencyType)>,
    /// Field updates of existing beads (repour only).
    pub(super) updates: Vec<NodeUpdate>,
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
    advanced: bool,
    reader: &mut impl GraphReader,
) -> Result<GraphPlan, BeadComposeError> {
    // A repour anchors new nodes under the pour root exactly as attach does
    // under its parent: no root node is created and step keys are bare.
    let rooted = v.mode == BeadGraphMode::Attach || v.root.is_some();
    let (mut ids, requested) = requested_beads(v)?;
    let mut found = if requested.is_empty() {
        BTreeMap::new()
    } else {
        reader.issues(&requested.into_iter().collect::<Vec<_>>())?
    };
    let mut existing = match &v.root {
        Some(root) => Some(repour::discover(v, root, reader)?),
        None => None,
    };
    if let Some(existing) = &existing {
        ids.clone_from(&existing.ids);
        found.extend(existing.rows.clone());
    }
    if let Some(parent) = &v.parent
        && !found.contains_key(parent)
    {
        return Err(BeadComposeError::GraphParentNotFound {
            parent: parent.clone(),
        });
    }
    check_relation_endpoints(v, &found)?;
    if v.repour && v.root.is_none() {
        // An attach repour matches its beads by their deterministic ids.
        existing = Some(repour::discover_attach(v, &ids, &found, reader)?);
    }
    let missing = if let Some(existing) = &existing {
        // Existing step beads were matched by their ownership, not by id alone.
        v.steps
            .iter()
            .filter(|step| !existing.ids.contains_key(&step.id))
            .map(|step| step.id.clone())
            .collect()
    } else {
        check_ownership(v, advanced, &ids, &found)?
    };
    let mut pending = create_nodes(v, advanced, &ids, &missing, rooted);
    let edges = plan_edges(
        v,
        (&ids, &missing),
        rooted,
        advanced,
        existing.as_ref(),
        reader,
        &mut pending,
    )?;
    if let Some(payload) = pending.payload.as_object_mut() {
        payload.insert("edges".into(), json!(edges));
    }
    if let Some(existing) = existing {
        mark_repour(v, &existing, &mut pending);
    }
    if pending.keys.is_empty()
        && pending.dep_adds.is_empty()
        && pending.dep_removes.is_empty()
        && pending.updates.is_empty()
    {
        Ok(GraphPlan::Noop(pending.graph))
    } else {
        Ok(GraphPlan::Create(pending))
    }
}

/// The deterministic attach ids and every bead the plan must read.
fn requested_beads(
    v: &ValidatedGraph,
) -> Result<(BTreeMap<StepId, BeadId>, BTreeSet<BeadId>), BeadComposeError> {
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
    Ok((ids, requested))
}

/// A relation to a bead that does not exist is refused before any write.
fn check_relation_endpoints(
    v: &ValidatedGraph,
    found: &BTreeMap<BeadId, Value>,
) -> Result<(), BeadComposeError> {
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
    Ok(())
}

/// Record a repour's field updates, retained nodes and backup on the plan.
fn mark_repour(v: &ValidatedGraph, existing: &Existing, pending: &mut PendingCreate) {
    pending.updates = repour::node_updates(v, existing);
    pending
        .graph
        .changes
        .extend(repour::field_changes(v, existing));
    for update in &pending.updates {
        if let Some(node) = pending
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.id.as_ref() == Some(&update.id))
        {
            node.action = BeadNodeAction::Update;
        }
    }
    for (step, id) in &existing.retained {
        let row = &existing.rows[id];
        let status = repour::row_status(row);
        pending
            .graph
            .changes
            .push(crate::GraphChange::MarkRemovedNode {
                bead: id.clone(),
                status,
                remove_command: format!("bd delete -- {}", crate::error::shell_quote(id.as_str())),
            });
        if !row
            .get("labels")
            .and_then(Value::as_array)
            .is_some_and(|labels| labels.iter().any(|label| label == "repour:removed"))
        {
            pending.updates.push(NodeUpdate {
                id: id.clone(),
                calls: vec![vec!["--add-label=repour:removed".into()]],
            });
        }
        pending.graph.nodes.push(BeadGraphNode {
            step: Some(step.clone()),
            id: Some(id.clone()),
            action: BeadNodeAction::Retained,
        });
    }
}

fn check_ownership(
    v: &ValidatedGraph,
    advanced: bool,
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
                Some(raw) if advanced => {
                    match serde_json::from_value::<ProvenanceV2>(raw.clone()) {
                        Ok(actual) => {
                            let expected = v.provenance_v2(Some(step));
                            if actual.v == 2
                                && actual.mode == expected.mode
                                && actual.formula == expected.formula
                                && actual.parent == expected.parent
                                && actual.reference == expected.reference
                                && actual.step == expected.step
                                && actual.node_definition == expected.node_definition
                            {
                                None
                            } else {
                                Some(GraphConflictReason::ProvenanceDiffers)
                            }
                        }
                        Err(_) if raw.get("v").and_then(Value::as_u64) == Some(1) => {
                            Some(GraphConflictReason::MigrationProofMissing)
                        }
                        Err(_) => Some(GraphConflictReason::ProvenanceDiffers),
                    }
                }
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
    advanced: bool,
    ids: &BTreeMap<StepId, BeadId>,
    missing: &BTreeSet<StepId>,
    attach: bool,
) -> PendingCreate {
    let mut graph = BeadGraph {
        mode: v.mode,
        parent: anchor(v).cloned(),
        ref_: v.reference.clone(),
        formula: v.formula.clone(),
        revision: v.revision.clone(),
        plan_path: None,
        ids: ids.clone(),
        nodes: Vec::new(),
        edges: Vec::new(),
        changes: Vec::new(),
    };
    let mut nodes = Vec::new();
    let mut keys = BTreeMap::new();
    if let Some(root) = &v.root {
        graph.nodes.push(BeadGraphNode {
            step: None,
            id: Some(root.clone()),
            action: BeadNodeAction::Existing,
        });
    } else if !attach {
        let root_key = PlanKey::root();
        let provenance = if advanced {
            json!(v.provenance_v2(None))
        } else {
            json!(v.provenance(None))
        };
        nodes.push(json!({"key":root_key.as_str(),"type":"molecule","title":v.formula,"description":v.description,"metadata":{PROVENANCE_KEY:provenance}}));
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
            if v.repour {
                graph.changes.push(crate::GraphChange::AddNode {
                    step: step.id.clone(),
                });
            }
            let (key, fields) = create_step(v, advanced, step, ids.get(&step.id), attach);
            nodes.push(fields);
            keys.insert(key, Some(step.id.clone()));
        }
    }
    PendingCreate {
        graph,
        payload: json!({"nodes": nodes}),
        endpoints: Vec::new(),
        dep_adds: Vec::new(),
        dep_removes: Vec::new(),
        updates: Vec::new(),
        keys,
    }
}

/// The bead new nodes hang under: the attach parent or the repour root.
fn anchor(v: &ValidatedGraph) -> Option<&BeadId> {
    v.parent.as_ref().or(v.root.as_ref())
}

fn create_step(
    v: &ValidatedGraph,
    advanced: bool,
    step: &Step,
    id: Option<&BeadId>,
    attach: bool,
) -> (PlanKey, Value) {
    let mut fields = step.fields.clone();
    let key = PlanKey::step(&step.id, attach);
    fields.insert("key".into(), json!(key.as_str()));
    if let Some(id) = id {
        fields.insert("id".into(), json!(id));
    }
    if let Some(parent) = anchor(v) {
        fields.insert("parent_id".into(), json!(parent));
    } else {
        fields.insert("parent_key".into(), json!("_root"));
    }
    let metadata = fields
        .entry("metadata")
        .or_insert_with(|| Value::Object(Map::default()));
    let provenance = if advanced {
        json!(v.provenance_v2(Some(step)))
    } else {
        json!(v.provenance(Some(step.id.clone())))
    };
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
    (key, Value::Object(fields))
}

fn plan_edges(
    v: &ValidatedGraph,
    (ids, missing): (&BTreeMap<StepId, BeadId>, &BTreeSet<StepId>),
    attach: bool,
    advanced: bool,
    repoured: Option<&Existing>,
    reader: &mut impl GraphReader,
    pending: &mut PendingCreate,
) -> Result<Vec<Value>, BeadComposeError> {
    let (planned_edges, existing_edges) = read_edges(v, ids, missing, attach, repoured, reader)?;
    let mut absent = Vec::new();
    let mut edges = Vec::new();
    let mut declared = BTreeSet::new();
    for (from, to, kind) in planned_edges {
        pending.endpoints.push((
            PlannedEndpoint::Named(from.clone()),
            PlannedEndpoint::Named(to.clone()),
        ));
        let from = endpoint(&from, ids, missing, attach);
        let to = endpoint(&to, ids, missing, attach);
        let existing = check_edge(&from, &to, &kind, &existing_edges, &mut absent)?;
        note_edge(&from, &to, &kind, existing, pending, &mut declared);
        if !existing && repoured.is_some() {
            pending.graph.changes.push(crate::GraphChange::AddEdge {
                from: from.display().clone(),
                to: to.display().clone(),
                kind: kind.clone(),
            });
        }
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
            edges.push(planned_edge_json(from, to, &kind));
        }
    }
    plan_parent_edges(
        v,
        ids,
        missing,
        attach,
        &existing_edges,
        pending,
        &mut absent,
    )?;
    if let Some(existing) = repoured {
        note_revision_edges(existing, &existing_edges, &declared, pending);
    }
    if advanced {
        Ok(edges)
    } else {
        finish_edges(absent, edges)
    }
}

fn note_revision_edges(
    existing: &Existing,
    existing_edges: &ExistingEdges,
    declared: &BTreeSet<(BeadId, BeadId)>,
    pending: &mut PendingCreate,
) {
    let owned: BTreeSet<_> = existing.rows.keys().cloned().collect();
    for ((from, to), kind) in existing_edges {
        if *kind == GraphDependencyType::ParentChild {
            continue;
        }
        let inside_from = owned.contains(from);
        let inside_to = owned.contains(to);
        if inside_from && inside_to && !declared.contains(&(from.clone(), to.clone())) {
            pending
                .dep_removes
                .push((from.clone(), to.clone(), kind.clone()));
            pending
                .graph
                .changes
                .push(crate::GraphChange::RemoveInternalEdge {
                    from: from.clone(),
                    to: to.clone(),
                    kind: kind.clone(),
                });
        } else if inside_from != inside_to {
            pending
                .graph
                .changes
                .push(crate::GraphChange::RetainExternalEdge {
                    from: from.clone(),
                    to: to.clone(),
                    kind: kind.clone(),
                });
        }
    }
}

// Parent edges are supplied by parent_id/parent_key, never duplicated in the plan.
fn plan_parent_edges(
    v: &ValidatedGraph,
    ids: &BTreeMap<StepId, BeadId>,
    missing: &BTreeSet<StepId>,
    attach: bool,
    existing_edges: &ExistingEdges,
    pending: &mut PendingCreate,
    absent: &mut Vec<MissingEdge>,
) -> Result<(), BeadComposeError> {
    for step in &v.steps {
        pending.endpoints.push((
            PlannedEndpoint::Named(BeadEndpoint::Step(step.id.clone())),
            anchor(v).map_or(PlannedEndpoint::Root, |id| {
                PlannedEndpoint::Named(BeadEndpoint::Bead(id.clone()))
            }),
        ));
        let from = endpoint(&BeadEndpoint::Step(step.id.clone()), ids, missing, attach);
        let to = if attach {
            anchor(v).map_or_else(
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
            existing_edges,
            absent,
        )?;
        let mut declared = BTreeSet::new();
        note_edge(
            &from,
            &to,
            &GraphDependencyType::ParentChild,
            existing,
            pending,
            &mut declared,
        );
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
    Ok(())
}

/// Track an edge between two existing beads: it is declared by the plan, and
/// when absent it must be added by a separate `bd dep add`.
fn note_edge(
    from: &Endpoint,
    to: &Endpoint,
    kind: &GraphDependencyType,
    existing: bool,
    pending: &mut PendingCreate,
    declared: &mut BTreeSet<(BeadId, BeadId)>,
) {
    if let (Endpoint::Existing { id: from, .. }, Endpoint::Existing { id: to, .. }) = (from, to) {
        declared.insert((from.clone(), to.clone()));
        if !existing {
            pending
                .dep_adds
                .push((from.clone(), to.clone(), kind.clone()));
        }
    }
}

fn finish_edges(
    absent: Vec<crate::contract::MissingEdge>,
    edges: Vec<Value>,
) -> Result<Vec<Value>, BeadComposeError> {
    if absent.is_empty() {
        Ok(edges)
    } else {
        Err(BeadComposeError::GraphEdgeMissing { edges: absent })
    }
}

fn planned_edge_json(from: Endpoint, to: Endpoint, kind: &GraphDependencyType) -> Value {
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
    Value::Object(edge)
}

type PlannedEdges = Vec<(BeadEndpoint, BeadEndpoint, GraphDependencyType)>;
type ExistingEdges = BTreeMap<(BeadId, BeadId), GraphDependencyType>;

fn read_edges(
    v: &ValidatedGraph,
    ids: &BTreeMap<StepId, BeadId>,
    missing: &BTreeSet<StepId>,
    attach: bool,
    existing: Option<&Existing>,
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
    if let Some(existing) = existing {
        sources.extend(existing.rows.keys().cloned());
    }
    for id in sources {
        for (to, kind) in reader.dependencies(&id)? {
            existing_edges.insert((id.clone(), to), kind);
        }
        if existing.is_some() {
            for (from, kind) in reader.dependents(&id)? {
                existing_edges.insert((from, id.clone()), kind);
            }
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

    #[test]
    fn unowned_node_conflict_has_execution_error_class() {
        let step = StepId::new("build").unwrap();
        let id = BeadId::new("proj-1.rel-build").unwrap();
        let digest = crate::Sha256Digest::new(format!("sha256:{}", "a".repeat(64))).unwrap();
        let graph = ValidatedGraph {
            mode: BeadGraphMode::Attach,
            parent: Some(BeadId::new("proj-1").unwrap()),
            root: None,
            repour: true,
            reference: Some(crate::GraphRef::new("rel").unwrap()),
            formula: crate::FormulaName::new("example").unwrap(),
            description: String::new(),
            revision: digest.clone(),
            inputs: digest,
            steps: vec![Step {
                id: step.clone(),
                fields: Map::new(),
                needs: vec![],
            }],
            relations: vec![],
        };
        let error = check_ownership(
            &graph,
            true,
            &BTreeMap::from([(step, id.clone())]),
            &BTreeMap::from([(id, json!({"metadata": {}}))]),
        )
        .unwrap_err();
        assert_eq!(error.code(), "BEADS_GRAPH_CONFLICT");
        assert_eq!(error.class(), crate::BeadErrorClass::Execution);
    }

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
            changes: Vec::new(),
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
