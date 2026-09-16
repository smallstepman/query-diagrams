use std::collections::{BTreeMap, BTreeSet};

use crate::{DqError, Result, ViewGraph, ViewNode};

pub fn emit(graph: &ViewGraph) -> Result<String> {
    let direction = graph
        .direction
        .as_deref()
        .map(mermaid_direction)
        .unwrap_or("TD");
    let mut out = format!("flowchart {direction}\n");
    let ids = IdMap::new(graph);

    let child_groups = child_group_map(graph);
    let grouped_nodes = grouped_node_set(graph);
    let nested_groups = nested_group_set(graph);

    for group_id in graph
        .groups
        .keys()
        .filter(|id| !nested_groups.contains(id.as_str()))
    {
        emit_group(&mut out, graph, &ids, group_id, 1, &child_groups)?;
    }

    for node in graph.nodes.values() {
        if !grouped_nodes.contains(node.id.as_str()) {
            emit_node(&mut out, &ids, node, 1);
        }
    }

    let note_ids: BTreeMap<_, _> = graph
        .notes
        .keys()
        .enumerate()
        .map(|(i, id)| (id.clone(), format!("dq_note_{i}")))
        .collect();
    for note in graph.notes.values() {
        let note_id = &note_ids[&note.id];
        out.push_str(&format!(
            "    {note_id}[\"{}\"]\n",
            mermaid_label(&note.text)
        ));
    }

    let mut styled_edges = Vec::new();
    for (edge_index, edge) in graph.edges.values().enumerate() {
        let from = ids.get(&edge.from)?;
        let to = ids.get(&edge.to)?;
        let op = if edge.directed { "-->" } else { "---" };
        if edge.label.is_empty() {
            out.push_str(&format!("    {from} {op} {to}\n"));
        } else if edge.directed {
            out.push_str(&format!(
                "    {from} -->|{}| {to}\n",
                mermaid_edge_label(&edge.label)
            ));
        } else {
            out.push_str(&format!(
                "    {from} ---|{}| {to}\n",
                mermaid_edge_label(&edge.label)
            ));
        }
        if !edge.styles.is_empty() {
            styled_edges.push((edge_index, edge.styles.clone()));
        }
    }

    for note in graph.notes.values() {
        let note_id = &note_ids[&note.id];
        if let Ok(target) = ids.get(&note.target_id) {
            out.push_str(&format!("    {note_id} -.-> {target}\n"));
        }
    }

    for node in graph.nodes.values() {
        if let Some(class_name) = node.properties.get("native.mermaid.class") {
            out.push_str(&format!(
                "    class {} {}\n",
                ids.get(&node.id)?,
                class_name
            ));
        }
        if let Some(raw_style) = node.properties.get("native.mermaid.style") {
            out.push_str(&format!("    style {} {}\n", ids.get(&node.id)?, raw_style));
        } else if !node.styles.is_empty()
            && let Some(style) = mermaid_style(&node.styles)
        {
            out.push_str(&format!("    style {} {style}\n", ids.get(&node.id)?));
        }
    }
    for group in graph.groups.values() {
        if let Some(class_name) = group.properties.get("native.mermaid.class") {
            out.push_str(&format!(
                "    class {} {}\n",
                ids.get(&group.id)?,
                class_name
            ));
        }
        if let Some(raw_style) = group.properties.get("native.mermaid.style") {
            out.push_str(&format!(
                "    style {} {}\n",
                ids.get(&group.id)?,
                raw_style
            ));
        } else if !group.styles.is_empty()
            && let Some(style) = mermaid_style(&group.styles)
        {
            out.push_str(&format!("    style {} {style}\n", ids.get(&group.id)?));
        }
    }
    for (index, styles) in styled_edges {
        if let Some(style) = mermaid_edge_style(&styles) {
            out.push_str(&format!("    linkStyle {index} {style}\n"));
        }
    }
    for note_id in note_ids.values() {
        out.push_str(&format!(
            "    style {note_id} fill:#fff7cc,stroke:#a58b00,stroke-dasharray:3 3\n"
        ));
    }

    Ok(out)
}

fn emit_group(
    out: &mut String,
    graph: &ViewGraph,
    ids: &IdMap,
    id: &str,
    depth: usize,
    child_groups: &BTreeMap<String, Vec<String>>,
) -> Result<()> {
    let group = graph
        .groups
        .get(id)
        .ok_or_else(|| DqError::InvalidView(format!("missing group {id:?}")))?;
    let indent = "    ".repeat(depth);
    out.push_str(&format!(
        "{indent}subgraph {}[\"{}\"]\n",
        ids.get(id)?,
        mermaid_label(&group.label)
    ));

    for (member, kind) in &group.members {
        match kind.as_str() {
            "node" => {
                if let Some(node) = graph.nodes.get(member) {
                    emit_node(out, ids, node, depth + 1);
                }
            }
            "group" => emit_group(out, graph, ids, member, depth + 1, child_groups)?,
            _ => {}
        }
    }
    // Some source adapters may record nesting in the child map without an explicit
    // member entry. Keep this fallback deterministic.
    if let Some(children) = child_groups.get(id) {
        for child in children {
            if !group.members.contains_key(child) {
                emit_group(out, graph, ids, child, depth + 1, child_groups)?;
            }
        }
    }
    out.push_str(&format!("{indent}end\n"));
    Ok(())
}

fn emit_node(out: &mut String, ids: &IdMap, node: &ViewNode, depth: usize) {
    let indent = "    ".repeat(depth);
    let id = ids.map.get(&node.id).expect("id map complete");
    let label = mermaid_node_label(node);
    let expr = match node.shape.as_str() {
        "diamond" | "rhombus" => format!("{id}{{\"{label}\"}}"),
        "circle" => format!("{id}((\"{label}\"))"),
        "cylinder" | "database" => format!("{id}[(\"{label}\")]"),
        "rounded" | "roundrect" | "round_rect" => format!("{id}(\"{label}\")"),
        "stadium" => format!("{id}([\"{label}\"])"),
        "hexagon" => format!("{id}{{{{\"{label}\"}}}}"),
        "subroutine" => format!("{id}[[\"{label}\"]]"),
        _ => format!("{id}[\"{label}\"]"),
    };
    out.push_str(&indent);
    out.push_str(&expr);
    out.push('\n');
}

struct IdMap {
    map: BTreeMap<String, String>,
}
impl IdMap {
    fn new(graph: &ViewGraph) -> Self {
        let mut map = BTreeMap::new();
        for id in graph.nodes.keys().chain(graph.groups.keys()) {
            map.insert(id.clone(), mermaid_id(id));
        }
        Self { map }
    }
    fn get(&self, id: &str) -> Result<&str> {
        self.map
            .get(id)
            .map(String::as_str)
            .ok_or_else(|| DqError::InvalidView(format!("no Mermaid id for {id:?}")))
    }
}

fn grouped_node_set(graph: &ViewGraph) -> BTreeSet<&str> {
    graph
        .groups
        .values()
        .flat_map(|g| g.members.iter())
        .filter_map(|(id, kind)| (kind == "node").then_some(id.as_str()))
        .collect()
}
fn nested_group_set(graph: &ViewGraph) -> BTreeSet<&str> {
    graph
        .groups
        .values()
        .flat_map(|g| g.members.iter())
        .filter_map(|(id, kind)| (kind == "group").then_some(id.as_str()))
        .collect()
}
fn child_group_map(graph: &ViewGraph) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for group in graph.groups.values() {
        for (id, kind) in &group.members {
            if kind == "group" {
                map.entry(group.id.clone()).or_default().push(id.clone());
            }
        }
    }
    map
}

fn mermaid_style(styles: &BTreeMap<String, String>) -> Option<String> {
    let mut parts = Vec::new();
    for (key, value) in styles {
        let k = match key.as_str() {
            "fill" => "fill",
            "stroke" => "stroke",
            "font_color" | "color" => "color",
            "stroke_width" => "stroke-width",
            "stroke_dash" | "stroke_dasharray" => "stroke-dasharray",
            _ => continue,
        };
        let mut value = value.clone();
        if k == "stroke-width" && value.chars().all(|c| c.is_ascii_digit() || c == '.') {
            value.push_str("px");
        }
        parts.push(format!("{k}:{value}"));
    }
    (!parts.is_empty()).then(|| parts.join(","))
}
fn mermaid_edge_style(styles: &BTreeMap<String, String>) -> Option<String> {
    mermaid_style(styles)
}

fn mermaid_direction(direction: &str) -> &str {
    match direction {
        "TD" | "down" => "TD",
        "BT" | "up" => "BT",
        "LR" | "right" => "LR",
        "RL" | "left" => "RL",
        other => other,
    }
}

fn mermaid_id(id: &str) -> String {
    let mut body: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if body.is_empty() || body.chars().next().unwrap().is_ascii_digit() {
        body.insert(0, '_');
    }
    format!("dq_{}_{}", body, fnv1a(id))
}
fn fnv1a(s: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
fn mermaid_node_label(node: &ViewNode) -> String {
    let mut label = mermaid_label(&node.label);
    for (_, value) in node
        .properties
        .iter()
        .filter(|(k, _)| k.starts_with("member."))
    {
        if let Some((name, ty)) = value.split_once('\u{1f}') {
            label.push_str("<br/>");
            label.push_str(&mermaid_label(name));
            if !ty.is_empty() {
                label.push_str(": ");
                label.push_str(&mermaid_label(ty));
            }
        }
    }
    label
}

fn mermaid_edge_label(s: &str) -> String {
    mermaid_label(s).replace('|', "#124;")
}

fn mermaid_label(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\n', "<br/>")
}
