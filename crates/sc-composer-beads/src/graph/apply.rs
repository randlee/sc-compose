//! Ordered application of a planned graph revision.

use super::{
    BTreeMap, BeadComposeError, BeadEdgeAction, BeadGraph, BeadId, BeadNodeAction, CommandSpec,
    GraphDependencyType, InputSnapshot, PendingCreate, PlanKey, Runtime, Value, graph_plan_path,
    partial_error, process_failure_cause, repour, short_cause, take_changes, write,
};

fn apply_removals(
    runtime: &mut Runtime<'_>,
    removals: Vec<(BeadId, BeadId, GraphDependencyType)>,
    done: &mut Vec<crate::GraphChange>,
    pending: &mut Vec<crate::GraphChange>,
    applied_count: &mut usize,
    total: usize,
) -> Result<(), BeadComposeError> {
    for (from, to, _) in removals {
        let args = vec![
            "dep".into(),
            "remove".into(),
            "--".into(),
            from.to_string(),
            to.to_string(),
        ];
        write(runtime, args.clone()).map_err(|error| {
            partial_error(error, done, pending, &args, *applied_count, total, runtime)
        })?;
        take_changes(
            pending,
            done,
            |change| matches!(change, crate::GraphChange::RemoveInternalEdge { from: f, to: t, .. } if f == &from && t == &to),
        );
        *applied_count += 1;
    }
    Ok(())
}

fn apply_updates_and_edges(
    runtime: &mut Runtime<'_>,
    updates: Vec<repour::NodeUpdate>,
    dep_adds: Vec<(BeadId, BeadId, GraphDependencyType)>,
    done: &mut Vec<crate::GraphChange>,
    pending: &mut Vec<crate::GraphChange>,
    applied_count: &mut usize,
    total: usize,
) -> Result<(), BeadComposeError> {
    for update in updates {
        for call in update.calls {
            let mut args = vec!["update".into()];
            args.extend(call.clone());
            args.extend(["--".into(), update.id.to_string()]);
            write(runtime, args.clone()).map_err(|error| {
                partial_error(error, done, pending, &args, *applied_count, total, runtime)
            })?;
            take_changes(pending, done, |change| match change {
                crate::GraphChange::FieldChange { bead, field, .. } if bead == &update.id => call
                    .iter()
                    .any(|arg| arg.starts_with(&format!("{}=", field.as_flag()))),
                crate::GraphChange::MarkRemovedNode { bead, .. } => {
                    bead == &update.id && call.iter().any(|arg| arg == "--add-label=repour:removed")
                }
                _ => false,
            });
            *applied_count += 1;
        }
    }
    for (from, to, kind) in dep_adds {
        let args = vec![
            "dep".into(),
            "add".into(),
            "--type".into(),
            kind.to_string(),
            "--".into(),
            from.to_string(),
            to.to_string(),
        ];
        write(runtime, args.clone()).map_err(|error| {
            partial_error(error, done, pending, &args, *applied_count, total, runtime)
        })?;
        take_changes(
            pending,
            done,
            |change| matches!(change, crate::GraphChange::AddEdge { from: crate::GraphEndpoint::Bead(f), to: crate::GraphEndpoint::Bead(t), .. } if f == &from && t == &to),
        );
        *applied_count += 1;
    }
    Ok(())
}

impl PendingCreate {
    pub(super) fn preview(
        mut self,
        runtime: &mut Runtime<'_>,
    ) -> Result<BeadGraph, BeadComposeError> {
        // Without new nodes there is nothing for `bd create --graph` to check.
        if !self.keys.is_empty() {
            self.create(runtime, true)?;
        }
        Ok(self.graph)
    }

    /// Apply the plan: create missing nodes, update drifted plan-owned fields,
    /// then add the remaining missing edges. Each write is independent, so a
    /// repeat run recomputes the plan from the stored beads and finishes
    /// whatever an interrupted run left.
    pub(super) fn apply(
        mut self,
        runtime: &mut Runtime<'_>,
    ) -> Result<BeadGraph, BeadComposeError> {
        let creation_count = usize::from(!self.keys.is_empty());
        let updates = std::mem::take(&mut self.updates);
        let dep_adds = std::mem::take(&mut self.dep_adds);
        let dep_removes = std::mem::take(&mut self.dep_removes);
        let total = creation_count
            + updates
                .iter()
                .map(|update| update.calls.len())
                .sum::<usize>()
            + dep_adds.len()
            + dep_removes.len();
        let mut applied_count = 0;
        let mut done = Vec::new();
        let mut pending: Vec<_> = self
            .graph
            .changes
            .iter()
            .filter(|change| !matches!(change, crate::GraphChange::RetainExternalEdge { .. }))
            .cloned()
            .collect();
        // Already-marked removed nodes are reports, not outstanding writes.
        pending.retain(|change| match change {
            crate::GraphChange::MarkRemovedNode { bead, .. } => updates.iter().any(|u| {
                &u.id == bead
                    && u.calls
                        .iter()
                        .flatten()
                        .any(|flag| flag == "--add-label=repour:removed")
            }),
            _ => true,
        });
        apply_removals(
            runtime,
            dep_removes,
            &mut done,
            &mut pending,
            &mut applied_count,
            total,
        )?;
        self.apply_creation(runtime, &mut done, &mut pending, &mut applied_count, total)?;
        apply_updates_and_edges(
            runtime,
            updates,
            dep_adds,
            &mut done,
            &mut pending,
            &mut applied_count,
            total,
        )?;
        for node in &mut self.graph.nodes {
            if node.action == BeadNodeAction::Update {
                node.action = BeadNodeAction::Updated;
            }
        }
        Ok(self.graph)
    }

    fn apply_creation(
        &mut self,
        runtime: &mut Runtime<'_>,
        done: &mut Vec<crate::GraphChange>,
        pending: &mut Vec<crate::GraphChange>,
        applied_count: &mut usize,
        total: usize,
    ) -> Result<(), BeadComposeError> {
        if self.keys.is_empty() {
            self.mark_created()
                .map_err(|cause| self.unconfirmed(Vec::new(), cause))?;
        } else {
            let created: Vec<_> = self
                .graph
                .nodes
                .iter()
                .filter(|node| node.action == BeadNodeAction::Create)
                .cloned()
                .collect();
            self.create(runtime, false).map_err(|error| {
                partial_error(
                    error,
                    done,
                    pending,
                    &["bd create --graph".into()],
                    *applied_count,
                    total,
                    runtime,
                )
            })?;
            take_changes(pending, done, |change| match change {
                crate::GraphChange::AddNode { .. } => true,
                crate::GraphChange::AddEdge { from, to, .. } => created.iter().any(|node| {
                    [from, to].iter().any(|endpoint| match endpoint {
                        crate::GraphEndpoint::Step(step) => node.step.as_ref() == Some(step),
                        crate::GraphEndpoint::Bead(id) => node.id.as_ref() == Some(id),
                        _ => false,
                    })
                }),
                _ => false,
            });
            *applied_count += 1;
        }
        Ok(())
    }

    fn unconfirmed(&self, command: Vec<String>, cause: &str) -> BeadComposeError {
        BeadComposeError::GraphApplyUnconfirmed {
            command,
            cause: short_cause(cause),
            ids: self
                .graph
                .parent
                .iter()
                .chain(self.graph.ids.values())
                .cloned()
                .collect(),
        }
    }

    fn create(&mut self, runtime: &mut Runtime<'_>, preview: bool) -> Result<(), BeadComposeError> {
        let (path, public_path) = graph_plan_path(runtime)?;
        let bytes = serde_json::to_vec(&self.payload).expect("plan JSON serializes");
        let plan_input = InputSnapshot::write(&path, &bytes)
            .map_err(|error| crate::snapshot::output_error(&path, error))?;
        plan_input.publish_copy(&path)?;
        self.graph.plan_path = Some(public_path.clone());
        let mut args = vec![
            "create".into(),
            "--graph".into(),
            plan_input.path().to_string_lossy().into_owned(),
        ];
        if preview {
            args.push("--dry-run".into());
        }
        args.push("--json".into());
        let mut presented_args = args.clone();
        presented_args[2] = public_path.to_string_lossy().into_owned();
        let command = CommandSpec {
            executable: runtime.bd.clone(),
            args: presented_args,
            working_directory: runtime.normalized.working_directory.clone(),
        }
        .argv();
        let output = runtime.invoke_presenting(args, Some((plan_input.path(), &public_path)))?;
        let present = |text: &str| {
            text.replace(
                plan_input.path().to_string_lossy().as_ref(),
                public_path.to_string_lossy().as_ref(),
            )
        };
        let failure = |cause: String| BeadComposeError::GraphApplyFailed {
            command: command.clone(),
            status: output.exit_status,
            cause: short_cause(&present(&cause)),
        };
        if output.exit_status != Some(0) {
            let mut presented_output = output.clone();
            presented_output.stdout = present(&output.stdout);
            presented_output.stderr = present(&output.stderr);
            return Err(failure(process_failure_cause(&presented_output)));
        }
        if preview {
            return Ok(());
        }
        // bd exited 0: its transaction may have committed, so from here on a
        // failure must not claim that nothing was written.
        self.record_created(&output.stdout)
            .map_err(|cause| self.unconfirmed(command, &present(&cause)))
    }

    /// Consume an exit-0 apply response, returning the cause it is unusable.
    fn record_created(&mut self, stdout: &str) -> Result<(), String> {
        let value: Value = serde_json::from_str(stdout).map_err(|error| error.to_string())?;
        let assigned: BTreeMap<PlanKey, BeadId> = serde_json::from_value(
            value
                .get("ids")
                .cloned()
                .ok_or("bd create --graph response is missing ids")?,
        )
        .map_err(|error| error.to_string())?;
        if assigned.len() != self.keys.len() || self.keys.keys().any(|k| !assigned.contains_key(k))
        {
            return Err("bd create --graph response ids do not match the planned steps".into());
        }
        for (key, step) in &self.keys {
            let id = assigned
                .get(key)
                .cloned()
                .ok_or("bd create --graph response is missing a planned key")?;
            if let Some(step) = step {
                if self
                    .graph
                    .ids
                    .get(step)
                    .is_some_and(|expected| *expected != id)
                {
                    return Err(
                        "bd create --graph assigned an id different from the planned id".into(),
                    );
                }
                self.graph.ids.insert(step.clone(), id);
            } else {
                self.graph.parent = Some(id);
            }
        }
        self.mark_created().map_err(str::to_owned)
    }

    fn mark_created(&mut self) -> Result<(), &'static str> {
        for node in &mut self.graph.nodes {
            if node.action == BeadNodeAction::Create {
                node.action = BeadNodeAction::Created;
                node.id = node
                    .step
                    .as_ref()
                    .and_then(|s| self.graph.ids.get(s))
                    .cloned()
                    .or_else(|| self.graph.parent.clone());
            }
        }
        let endpoints: Vec<_> = self
            .endpoints
            .iter()
            .map(|(from, to)| {
                let from = from
                    .resolve(&self.graph)
                    .ok_or("planned edge source did not resolve")?;
                let to = to
                    .resolve(&self.graph)
                    .ok_or("planned edge destination did not resolve")?;
                Ok((from, to))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (edge, (from, to)) in self.graph.edges.iter_mut().zip(endpoints) {
            edge.from = from;
            edge.to = to;
            if edge.action == BeadEdgeAction::Add {
                edge.action = BeadEdgeAction::Added;
            }
        }
        Ok(())
    }
}
