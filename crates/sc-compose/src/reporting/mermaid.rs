use std::fmt;

use crate::reporting::spec::{ReportSpec, SqlQuerySpec, StateMachineSpec};

#[derive(Debug)]
pub(crate) struct MermaidRenderError {
    message: String,
}

pub(crate) fn render_mermaid(spec: &ReportSpec) -> Result<String, MermaidRenderError> {
    match spec {
        ReportSpec::StateMachine(spec) => render_state_machine(spec),
        ReportSpec::SqlQuery(spec) => render_sql_query(spec),
    }
}

impl fmt::Display for MermaidRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MermaidRenderError {}

fn render_state_machine(spec: &StateMachineSpec) -> Result<String, MermaidRenderError> {
    if spec.states.is_empty() {
        return Err(MermaidRenderError {
            message: "state_machine specs must define at least one state".to_owned(),
        });
    }

    let mut lines = vec!["stateDiagram-v2".to_owned()];
    for state in &spec.states {
        if let Some(label) = &state.label {
            lines.push(format!(
                "  state \"{label}\" as {}",
                sanitize_node_id(&state.id)
            ));
        } else {
            lines.push(format!("  state {}", sanitize_node_id(&state.id)));
        }
        if state.terminal {
            lines.push(format!("  {} --> [*]", sanitize_node_id(&state.id)));
        }
    }
    for transition in &spec.transitions {
        let mut label_parts = Vec::new();
        if let Some(event) = &transition.event {
            label_parts.push(event.clone());
        }
        if let Some(guard) = &transition.guard {
            label_parts.push(format!("[{guard}]"));
        }
        if let Some(actor) = &transition.actor {
            label_parts.push(format!("actor:{actor}"));
        }
        if let Some(effect) = &transition.effect {
            label_parts.push(format!("/ {effect}"));
        }
        let label = if label_parts.is_empty() {
            String::new()
        } else {
            format!(" : {}", label_parts.join(" "))
        };
        lines.push(format!(
            "  {} --> {}{}",
            sanitize_node_id(&transition.from),
            sanitize_node_id(&transition.to),
            label
        ));
    }
    Ok(lines.join("\n"))
}

fn render_sql_query(spec: &SqlQuerySpec) -> Result<String, MermaidRenderError> {
    if spec.tables_read.is_empty() && spec.tables_written.is_empty() {
        return Err(MermaidRenderError {
            message: "sql_query specs must define at least one read or write table".to_owned(),
        });
    }

    let query_id = sanitize_node_id(&spec.spec.id);
    let mut lines = vec!["flowchart TD".to_owned()];
    lines.push(format!("  {query_id}[\"{}\"]", spec.spec.title));

    for table in &spec.tables_read {
        let node = sanitize_node_id(&format!("read-{table}"));
        lines.push(format!("  {node}[\"read: {table}\"]"));
        lines.push(format!("  {query_id} --> {node}"));
    }
    for table in &spec.tables_written {
        let node = sanitize_node_id(&format!("write-{table}"));
        lines.push(format!("  {node}[\"write: {table}\"]"));
        lines.push(format!("  {query_id} --> {node}"));
    }
    if !spec.filters.is_empty() {
        let node = sanitize_node_id(&format!("filters-{}", spec.spec.id));
        lines.push(format!(
            "  {node}[\"filters: {}\"]",
            spec.filters.join("; ")
        ));
        lines.push(format!("  {query_id} -.-> {node}"));
    }
    if !spec.ordering.is_empty() {
        let node = sanitize_node_id(&format!("ordering-{}", spec.spec.id));
        lines.push(format!(
            "  {node}[\"ordering: {}\"]",
            spec.ordering.join("; ")
        ));
        lines.push(format!("  {query_id} -.-> {node}"));
    }
    if !spec.transactional_assumptions.is_empty() {
        let node = sanitize_node_id(&format!("tx-{}", spec.spec.id));
        lines.push(format!(
            "  {node}[\"tx: {}\"]",
            spec.transactional_assumptions.join("; ")
        ));
        lines.push(format!("  {query_id} -.-> {node}"));
    }
    lines.push(format!(
        "  {}[\"cardinality: {}\"]",
        sanitize_node_id(&format!("cardinality-{}", spec.spec.id)),
        spec.cardinality
    ));
    lines.push(format!(
        "  {query_id} -.-> {}",
        sanitize_node_id(&format!("cardinality-{}", spec.spec.id))
    ));
    Ok(lines.join("\n"))
}

fn sanitize_node_id(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{render_mermaid, sanitize_node_id};
    use crate::reporting::spec::{
        ReportSpec, ReportSpecMetadata, SpecHeader, SqlQuerySpec, StateMachineSpec,
    };

    fn spec_header(kind: &str, id: &str, title: &str) -> SpecHeader {
        SpecHeader {
            kind: kind.to_owned(),
            id: id.to_owned(),
            title: title.to_owned(),
            renderer_targets: vec!["mermaid".to_owned()],
        }
    }

    #[test]
    fn state_machine_specs_require_at_least_one_state() {
        let error = render_mermaid(&ReportSpec::StateMachine(StateMachineSpec {
            spec: spec_header("state_machine", "state-empty", "State Empty"),
            states: Vec::new(),
            transitions: Vec::new(),
            metadata: ReportSpecMetadata::default(),
        }))
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "state_machine specs must define at least one state"
        );
    }

    #[test]
    fn sql_query_specs_require_at_least_one_table() {
        let error = render_mermaid(&ReportSpec::SqlQuery(SqlQuerySpec {
            spec: spec_header("sql_query", "sql-empty", "SQL Empty"),
            purpose: "test".to_owned(),
            tables_read: Vec::new(),
            tables_written: Vec::new(),
            filters: Vec::new(),
            ordering: Vec::new(),
            cardinality: "1 row".to_owned(),
            transactional_assumptions: Vec::new(),
            metadata: ReportSpecMetadata::default(),
        }))
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "sql_query specs must define at least one read or write table"
        );
    }

    #[test]
    fn sanitize_node_id_collides_for_distinct_inputs() {
        assert_eq!(sanitize_node_id("build:ok"), sanitize_node_id("build/ok"));
        assert_eq!(sanitize_node_id("build:ok"), "build_ok");
    }
}
