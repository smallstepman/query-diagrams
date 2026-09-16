use std::collections::{BTreeMap, BTreeSet};

use crate::{DqError, Result, ViewGraph, ViewGroup, ViewNode};

pub fn emit(graph: &ViewGraph) -> Result<String> {
    let mut out = String::new();
    if let Some(direction) = &graph.direction {
        out.push_str(&format!(
            "direction: {}\n\n",
            d2_scalar(d2_direction(direction))
        ));
    }

    for (key, value) in &graph.options {
        if let Some(key) = key.strip_prefix("native.d2.") {
            out.push_str(&format!("{}: {}\n", key, d2_scalar(value)));
        }
    }
    if graph.options.keys().any(|k| k.starts_with("native.d2.")) {
        out.push('\n');
    }

    let parent = parent_map(graph);
    let paths = build_paths(graph, &parent)?;
    let grouped_nodes = grouped_node_set(graph);
    let nested_groups = nested_group_set(graph);

    for group in graph
        .groups
        .values()
        .filter(|g| !nested_groups.contains(g.id.as_str()))
    {
        emit_group(&mut out, graph, group, 0, None);
        out.push('\n');
    }
    for node in graph
        .nodes
        .values()
        .filter(|n| !grouped_nodes.contains(n.id.as_str()))
    {
        emit_node(&mut out, node, 0, None);
    }
    if !graph.nodes.is_empty() {
        out.push('\n');
    }

    for edge in graph.edges.values() {
        let op = edge
            .properties
            .get("native.d2.operator")
            .or_else(|| edge.properties.get("d2.operator"))
            .map(String::as_str)
            .unwrap_or(if edge.directed { "->" } else { "--" });
        let from = paths.get(&edge.from).ok_or_else(|| {
            DqError::InvalidView(format!("missing D2 path for edge endpoint {:?}", edge.from))
        })?;
        let to = paths.get(&edge.to).ok_or_else(|| {
            DqError::InvalidView(format!("missing D2 path for edge endpoint {:?}", edge.to))
        })?;
        out.push_str(&format!("{from} {op} {to}"));
        if !edge.label.is_empty() {
            out.push_str(&format!(": {}", d2_scalar(&edge.label)));
        }

        let native = native_d2_properties(&edge.properties);
        if edge.styles.is_empty() && native.is_empty() {
            out.push('\n');
        } else {
            out.push_str(" {\n");
            for (k, v) in &edge.styles {
                if !native_overrides_style(&native, k) {
                    out.push_str(&format!("  style.{}: {}\n", d2_style_key(k), d2_scalar(v)));
                }
            }
            for (k, v) in native {
                if k != "operator" {
                    out.push_str(&format!("  {k}: {}\n", d2_scalar(v)));
                }
            }
            out.push_str("}\n");
        }
    }

    for note in graph.notes.values() {
        let note_id = format!("__dq_note_{}", safe_suffix(&note.id));
        out.push_str(&format!(
            "{}: {} {{\n  shape: text\n  style.fill: \"#fff7cc\"\n}}\n",
            d2_id(&note_id),
            d2_scalar(&note.text)
        ));
        if let Some(target) = paths.get(&note.target_id) {
            out.push_str(&format!(
                "{} -- {} {{\n  style.stroke-dash: 3\n}}\n",
                d2_id(&note_id),
                target
            ));
        }
    }
    Ok(out)
}

fn emit_group(
    out: &mut String,
    graph: &ViewGraph,
    group: &ViewGroup,
    depth: usize,
    parent: Option<&str>,
) {
    let native = native_d2_properties(&group.properties);
    let i = "  ".repeat(depth);
    out.push_str(&format!(
        "{i}{}: {} {{\n",
        d2_id(d2_local_id(&group.id, parent)),
        d2_scalar(&group.label)
    ));
    for (k, v) in &group.styles {
        if !native_overrides_style(&native, k) {
            out.push_str(&format!(
                "{i}  style.{}: {}\n",
                d2_style_key(k),
                d2_scalar(v)
            ));
        }
    }
    for (k, v) in native {
        out.push_str(&format!("{i}  {k}: {}\n", d2_scalar(v)));
    }
    for (member, kind) in &group.members {
        match kind.as_str() {
            "node" => {
                if let Some(node) = graph.nodes.get(member) {
                    emit_node(out, node, depth + 1, Some(&group.id));
                }
            }
            "group" => {
                if let Some(child) = graph.groups.get(member) {
                    emit_group(out, graph, child, depth + 1, Some(&group.id));
                }
            }
            _ => {}
        }
    }
    out.push_str(&format!("{i}}}\n"));
}

fn emit_node(out: &mut String, node: &ViewNode, depth: usize, parent: Option<&str>) {
    let i = "  ".repeat(depth);
    let native = native_d2_properties(&node.properties);
    let members: Vec<_> = node
        .properties
        .iter()
        .filter(|(k, _)| k.starts_with("member."))
        .collect();
    let (shape, implicit_border_radius) = d2_shape(&node.shape);
    let has_body = shape != "rectangle"
        || implicit_border_radius.is_some()
        || !node.styles.is_empty()
        || !native.is_empty()
        || !members.is_empty()
        || node.properties.contains_key("link")
        || node.properties.contains_key("tooltip")
        || node.properties.contains_key("near");

    if !has_body {
        out.push_str(&format!(
            "{i}{}: {}\n",
            d2_id(d2_local_id(&node.id, parent)),
            d2_scalar(&node.label)
        ));
        return;
    }

    out.push_str(&format!(
        "{i}{}: {} {{\n",
        d2_id(d2_local_id(&node.id, parent)),
        d2_scalar(&node.label)
    ));
    if shape != "rectangle" {
        out.push_str(&format!("{i}  shape: {}\n", d2_scalar(shape)));
    }
    if let Some(radius) = implicit_border_radius
        && !node
            .styles
            .keys()
            .any(|key| d2_style_key(key) == "border-radius")
        && !native_overrides_style(&native, "border-radius")
    {
        out.push_str(&format!("{i}  style.border-radius: {radius}\n"));
    }
    for (k, v) in &node.styles {
        if !native_overrides_style(&native, k) {
            out.push_str(&format!(
                "{i}  style.{}: {}\n",
                d2_style_key(k),
                d2_scalar(v)
            ));
        }
    }
    for key in ["link", "tooltip", "near"] {
        if !native_overrides(&native, key)
            && let Some(value) = node.properties.get(key)
        {
            out.push_str(&format!("{i}  {key}: {}\n", d2_scalar(value)));
        }
    }
    for (k, v) in native {
        out.push_str(&format!("{i}  {k}: {}\n", d2_scalar(v)));
    }
    for (_, value) in members {
        if let Some((name, ty)) = value.split_once('\u{1f}') {
            out.push_str(&format!("{i}  {}: {}\n", d2_id(name), d2_scalar(ty)));
        }
    }
    out.push_str(&format!("{i}}}\n"));
}

fn native_d2_properties(properties: &BTreeMap<String, String>) -> Vec<(&str, &str)> {
    properties
        .iter()
        .filter_map(|(k, v)| {
            k.strip_prefix("native.d2.")
                .or_else(|| k.strip_prefix("d2."))
                .map(|key| (key, v.as_str()))
        })
        .collect()
}

fn native_overrides(native: &[(&str, &str)], key: &str) -> bool {
    native.iter().any(|(native_key, _)| *native_key == key)
}

fn native_overrides_style(native: &[(&str, &str)], style_key: &str) -> bool {
    native
        .iter()
        .any(|(native_key, _)| native_key.strip_prefix("style.") == Some(d2_style_key(style_key)))
}

fn parent_map(graph: &ViewGraph) -> BTreeMap<String, String> {
    let mut parent = BTreeMap::new();
    for group in graph.groups.values() {
        for (member, kind) in &group.members {
            if kind == "node" || kind == "group" {
                parent.insert(member.clone(), group.id.clone());
            }
        }
    }
    parent
}

fn build_paths(
    graph: &ViewGraph,
    parent: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut paths = BTreeMap::new();
    for id in graph.nodes.keys().chain(graph.groups.keys()) {
        let mut parts = Vec::new();
        let mut current = id.as_str();
        let mut seen = BTreeSet::new();
        loop {
            let parent_id = parent.get(current).map(String::as_str);
            parts.push(d2_id(d2_local_id(current, parent_id)));
            let Some(next) = parent_id else {
                break;
            };
            if !seen.insert(current.to_owned()) {
                return Err(DqError::InvalidView(format!(
                    "containment cycle while creating D2 path for {id:?}"
                )));
            }
            current = next;
        }
        parts.reverse();
        paths.insert(id.clone(), parts.join("."));
    }
    Ok(paths)
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

fn d2_local_id<'a>(id: &'a str, parent: Option<&str>) -> &'a str {
    parent
        .and_then(|parent| id.strip_prefix(parent))
        .and_then(|suffix| suffix.strip_prefix('.'))
        .unwrap_or(id)
}

fn d2_id(id: &str) -> String {
    // Quoted keys are valid in every D2 object/path position. Keeping identifiers
    // quoted avoids collisions with D2's context-sensitive reserved fields such
    // as `left` and `style`, including when they occur in an edge endpoint.
    d2_scalar(id)
}

fn d2_shape(shape: &str) -> (&str, Option<&'static str>) {
    match shape {
        "rounded" | "roundrect" => ("rectangle", Some("8")),
        "stadium" => ("rectangle", Some("20")),
        "actorbox" => ("person", None),
        "parallelogramalt" => ("parallelogram", None),
        "forkjoin" | "subroutine" | "doublecircle" | "trapezoid" | "trapezoidalt"
        | "asymmetric" | "mindmapdefault" | "node" => ("rectangle", None),
        "rectangle" | "square" | "page" | "parallelogram" | "document" | "cylinder" | "queue"
        | "package" | "step" | "callout" | "stored_data" | "person" | "c4-person" | "diamond"
        | "oval" | "circle" | "hexagon" | "cloud" | "text" | "code" | "image" | "sql_table"
        | "class" | "sequence_diagram" => (shape, None),
        _ => ("rectangle", None),
    }
}

fn d2_scalar(value: &str) -> String {
    serde_json::to_string(value).expect("serializing string cannot fail")
}

fn d2_direction(direction: &str) -> &str {
    match direction {
        "TD" | "TB" | "down" => "down",
        "BT" | "up" => "up",
        "LR" | "right" => "right",
        "RL" | "left" => "left",
        other => other,
    }
}

fn d2_style_key(key: &str) -> &str {
    match key {
        "font_color" => "font-color",
        "stroke_width" => "stroke-width",
        "stroke_dash" | "stroke_dasharray" => "stroke-dash",

        other => other,
    }
}

fn safe_suffix(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
