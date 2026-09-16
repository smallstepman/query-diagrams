use std::collections::BTreeMap;

use mermaid_rs_parser::{Subgraph, parse_mermaid};

use crate::{DqError, Edge, GraphDoc, Group, Node, Result};

pub fn parse(source: &str) -> Result<GraphDoc> {
    let header = first_payload_line(source).unwrap_or("");
    if header.starts_with("treeView-beta") {
        return parse_indented_tree(source, "treeview", false);
    }
    if header.starts_with("ishikawa-beta") {
        return parse_indented_tree(source, "ishikawa", true);
    }
    if header.starts_with("agentflow-beta") {
        return parse_agentflow(source);
    }
    if header.starts_with("usecase-beta") {
        return parse_usecase(source);
    }

    parse_with_mermaid_rs(source)
}

fn parse_with_mermaid_rs(source: &str) -> Result<GraphDoc> {
    let parsed = parse_mermaid(source).map_err(|e| DqError::MermaidParse(e.to_string()))?;
    let graph = parsed.graph;

    let mut out = GraphDoc {
        source_format: "mermaid".into(),
        family: format!("{:?}", graph.kind).to_ascii_lowercase(),
        direction: Some(direction_name(&format!("{:?}", graph.direction)).to_string()),
        ..GraphDoc::default()
    };

    for node in graph.nodes.values() {
        let shape = format!("{:?}", node.shape).to_ascii_lowercase();
        out.nodes.insert(
            node.id.clone(),
            Node {
                id: node.id.clone(),
                kind: out.family.clone(),
                label: node.label.clone(),
                shape,
                properties: BTreeMap::new(),
            },
        );
    }

    for (index, edge) in graph.edges.iter().enumerate() {
        let id = format!("e{index}");
        let mut properties = BTreeMap::new();
        properties.insert(
            "mermaid.edge_style".into(),
            format!("{:?}", edge.style).to_ascii_lowercase(),
        );
        properties.insert(
            "mermaid.arrow_start".into(),
            format!("{:?}", edge.arrow_start).to_ascii_lowercase(),
        );
        properties.insert(
            "mermaid.arrow_end".into(),
            format!("{:?}", edge.arrow_end).to_ascii_lowercase(),
        );
        out.edges.insert(
            id.clone(),
            Edge {
                id,
                kind: "relation".into(),
                from: edge.from.clone(),
                to: edge.to.clone(),
                label: edge.label.clone().unwrap_or_default(),
                directed: edge.directed,
                properties,
            },
        );
    }

    let subgraph_ids: Vec<_> = graph
        .subgraphs
        .iter()
        .enumerate()
        .map(|(index, subgraph)| {
            subgraph
                .id
                .clone()
                .unwrap_or_else(|| format!("group{index}"))
        })
        .collect();
    let subgraph_parents = infer_subgraph_parents(&graph.subgraphs);
    for (index, subgraph) in graph.subgraphs.iter().enumerate() {
        let id = &subgraph_ids[index];
        let direct_members = direct_subgraph_members(index, &graph.subgraphs, &subgraph_parents);
        let mut properties = BTreeMap::new();
        properties.insert("members".into(), direct_members.join("\u{1f}"));
        if let Some(parent_index) = subgraph_parents[index] {
            properties.insert("parent".into(), subgraph_ids[parent_index].clone());
        }
        if let Some(direction) = subgraph.direction {
            properties.insert(
                "direction".into(),
                direction_name(&format!("{:?}", direction)).to_string(),
            );
        }
        out.groups.insert(
            id.clone(),
            Group {
                id: id.clone(),
                label: subgraph.label.clone(),
                properties,
            },
        );
    }

    out.properties.insert("source".into(), source.to_owned());
    Ok(out)
}

/// Mermaid's parser records each nested group's nodes in its ancestors. A child
/// with no direct sibling nodes therefore has the same set as its parent. Source
/// order disambiguates that equal-set case because a Mermaid parent opens first.
fn infer_subgraph_parents(subgraphs: &[Subgraph]) -> Vec<Option<usize>> {
    (0..subgraphs.len())
        .map(|child_index| {
            let child = &subgraphs[child_index];
            subgraphs
                .iter()
                .enumerate()
                .filter(|(parent_index, parent)| {
                    *parent_index < child_index
                        && (is_strict_subgraph(&parent.nodes, &child.nodes)
                            || same_subgraph_nodes(&parent.nodes, &child.nodes))
                })
                .min_by(|(left_index, left), (right_index, right)| {
                    left.nodes
                        .len()
                        .cmp(&right.nodes.len())
                        .then_with(|| right_index.cmp(left_index))
                })
                .map(|(index, _)| index)
        })
        .collect()
}

fn direct_subgraph_members(
    group_index: usize,
    subgraphs: &[Subgraph],
    parents: &[Option<usize>],
) -> Vec<String> {
    subgraphs[group_index]
        .nodes
        .iter()
        .filter(|node| {
            !subgraphs.iter().enumerate().any(|(child_index, child)| {
                is_subgraph_descendant(child_index, group_index, parents)
                    && child.nodes.contains(*node)
            })
        })
        .cloned()
        .collect()
}

fn is_subgraph_descendant(
    mut child_index: usize,
    ancestor_index: usize,
    parents: &[Option<usize>],
) -> bool {
    while let Some(parent_index) = parents[child_index] {
        if parent_index == ancestor_index {
            return true;
        }
        child_index = parent_index;
    }
    false
}

fn same_subgraph_nodes(left: &[String], right: &[String]) -> bool {
    left.len() == right.len() && left.iter().all(|node| right.contains(node))
}

fn is_strict_subgraph(parent: &[String], child: &[String]) -> bool {
    parent.iter().any(|node| !child.contains(node))
        && child.iter().all(|node| parent.contains(node))
}

/// Mermaid TreeView and Ishikawa are both indentation-defined trees.  The
/// canonical graph only needs hierarchy, role and labels; the native visual
/// grammar remains an emitter concern.
fn parse_indented_tree(source: &str, family: &str, problem_is_root: bool) -> Result<GraphDoc> {
    let mut out = GraphDoc {
        source_format: "mermaid".into(),
        family: family.into(),
        direction: Some(if problem_is_root { "LR" } else { "TD" }.into()),
        ..GraphDoc::default()
    };
    out.properties.insert("source".into(), source.to_owned());

    let mut levels: Vec<(usize, String)> = Vec::new();
    let mut counter = 0usize;
    for raw in source
        .lines()
        .skip_while(|line| {
            !line.trim_start().starts_with(if problem_is_root {
                "ishikawa-beta"
            } else {
                "treeView-beta"
            })
        })
        .skip(1)
    {
        if raw.trim().is_empty() || raw.trim_start().starts_with("%%") {
            continue;
        }
        let expanded = raw.replace('\t', "    ");
        let indent = expanded.len() - expanded.trim_start().len();
        let text = expanded.trim();
        let (label, kind) = tree_label(text, family);
        let id = format!("n{counter}");
        counter += 1;
        let mut properties = BTreeMap::new();
        properties.insert("tree.indent".into(), indent.to_string());
        if problem_is_root && out.nodes.is_empty() {
            properties.insert("ishikawa.role".into(), "problem".into());
        }
        out.nodes.insert(
            id.clone(),
            Node {
                id: id.clone(),
                kind,
                label,
                shape: "rectangle".into(),
                properties,
            },
        );

        while levels.last().is_some_and(|(level, _)| *level >= indent) {
            levels.pop();
        }
        if let Some((_, parent)) = levels.last() {
            let eid = format!("e{}", out.edges.len());
            out.edges.insert(
                eid.clone(),
                Edge {
                    id: eid,
                    kind: if problem_is_root { "cause" } else { "contains" }.into(),
                    from: parent.clone(),
                    to: id.clone(),
                    label: String::new(),
                    directed: true,
                    properties: BTreeMap::new(),
                },
            );
        }
        levels.push((indent, id));
    }

    if out.nodes.is_empty() {
        return Err(DqError::MermaidParse(format!(
            "{family} diagram contains no nodes"
        )));
    }
    Ok(out)
}

fn tree_label(text: &str, family: &str) -> (String, String) {
    let text = text.split(":::").next().unwrap_or(text);
    let text = text.split("##").next().unwrap_or(text).trim();
    let folder = text.ends_with('/');
    let label = text
        .trim_end_matches('/')
        .trim_matches('"')
        .trim()
        .to_string();
    let kind = if family == "treeview" {
        if folder { "folder" } else { "file" }
    } else {
        "cause"
    };
    (label, kind.into())
}

/// AgentFlow v12 is still beta and is newer than the Rust Mermaid parser.
/// Parse its graph-relevant surface: flow/global containers, connector/nodes,
/// node metadata and chained edges. Everything else stays in provenance.
fn parse_agentflow(source: &str) -> Result<GraphDoc> {
    let header = first_payload_line(source).unwrap_or("agentflow-beta");
    let mut out = GraphDoc {
        source_format: "mermaid".into(),
        family: "agentflow".into(),
        direction: header
            .split_whitespace()
            .nth(1)
            .map(str::to_owned)
            .or(Some("TD".into())),
        ..GraphDoc::default()
    };
    out.properties.insert("source".into(), source.to_owned());

    let mut groups: Vec<String> = Vec::new();
    for raw in payload_lines(source).skip(1) {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if line == "end" {
            groups.pop();
            continue;
        }
        if let Some(rest) = line
            .strip_prefix("flow ")
            .or_else(|| line.strip_prefix("global "))
        {
            let (id, label, _) = parse_flow_node(rest);
            let mut props = BTreeMap::new();
            props.insert("members".into(), String::new());
            if let Some(parent) = groups.last() {
                props.insert("parent".into(), parent.clone());
            }
            out.groups.insert(
                id.clone(),
                Group {
                    id: id.clone(),
                    label,
                    properties: props,
                },
            );
            groups.push(id);
            continue;
        }
        if line.starts_with("direction ") {
            out.direction = line.split_whitespace().nth(1).map(str::to_owned);
            continue;
        }
        if line.contains("-->") || line.contains("-.-") || line.contains("--x") {
            parse_agentflow_edges(&mut out, line);
            continue;
        }
        let rest = line.strip_prefix("connector ").unwrap_or(line);
        let (id, label, kind) = parse_flow_node(rest);
        if id.is_empty() {
            continue;
        }
        let mut props = BTreeMap::new();
        if let Some(group) = groups.last() {
            add_group_member(&mut out, group, &id);
            props.insert("agentflow.flow".into(), group.clone());
        }
        out.nodes.entry(id.clone()).or_insert(Node {
            id,
            kind,
            label,
            shape: "rectangle".into(),
            properties: props,
        });
    }
    Ok(out)
}

fn parse_agentflow_edges(out: &mut GraphDoc, line: &str) {
    let normalized = line.replace("-.-", "-->").replace("--x", "-->");
    let parts: Vec<_> = normalized.split("-->").map(str::trim).collect();
    for pair in parts.windows(2) {
        let (from, _, _) = parse_flow_node(pair[0]);
        let (to, _, _) = parse_flow_node(pair[1]);
        if from.is_empty() || to.is_empty() {
            continue;
        }
        ensure_simple_node(out, &from);
        ensure_simple_node(out, &to);
        let id = format!("e{}", out.edges.len());
        out.edges.insert(
            id.clone(),
            Edge {
                id,
                kind: "sequence".into(),
                from,
                to,
                label: String::new(),
                directed: true,
                properties: BTreeMap::new(),
            },
        );
    }
}

/// Mermaid 12 use-case diagrams are newer than mermaid-rs-parser.  This parser
/// covers their architecture-relevant semantics: actors, use cases, system
/// boundaries, notes and relationships. Styling is intentionally normalized to
/// the generic view/style layer.
fn parse_usecase(source: &str) -> Result<GraphDoc> {
    let mut out = GraphDoc {
        source_format: "mermaid".into(),
        family: "usecase".into(),
        direction: Some("LR".into()),
        ..GraphDoc::default()
    };
    out.properties.insert("source".into(), source.to_owned());
    let mut boundary: Option<String> = None;

    for raw in payload_lines(source).skip(1) {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if line == "}" || line == "end" {
            boundary = None;
            continue;
        }
        if let Some(dir) = line.strip_prefix("direction ") {
            out.direction = Some(dir.trim().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("systemBoundary ") {
            let rest = rest.trim_end_matches('{').trim();
            let (id, label) = parse_decl_id_label(rest, "boundary");
            out.groups.insert(
                id.clone(),
                Group {
                    id: id.clone(),
                    label,
                    properties: BTreeMap::new(),
                },
            );
            boundary = Some(id);
            continue;
        }
        if let Some(rest) = line.strip_prefix("actor ") {
            let (id, label) = parse_decl_id_label(rest, "actor");
            insert_usecase_node(
                &mut out,
                &id,
                &label,
                "actor",
                "person",
                boundary.as_deref(),
            );
            continue;
        }
        if let Some(rest) = line.strip_prefix("usecase ") {
            let (id, label) = parse_decl_id_label(rest, "usecase");
            insert_usecase_node(
                &mut out,
                &id,
                &label,
                "usecase",
                "rounded",
                boundary.as_deref(),
            );
            continue;
        }
        if let Some(rest) = line.strip_prefix("note ") {
            let mut parts = rest.splitn(2, char::is_whitespace);
            let target = parts.next().unwrap_or("").trim();
            let text = parts.next().unwrap_or("").trim().trim_matches('"');
            if !target.is_empty() {
                let id = format!("note{}", out.notes.len());
                out.notes.insert(
                    id.clone(),
                    crate::Note {
                        id,
                        target_kind: "node".into(),
                        target_id: target.into(),
                        text: text.into(),
                        position: "auto".into(),
                        properties: BTreeMap::new(),
                    },
                );
            }
            continue;
        }
        if let Some((from, op, to, label)) = parse_usecase_edge(line) {
            ensure_simple_node(&mut out, &from);
            ensure_simple_node(&mut out, &to);
            let id = format!("e{}", out.edges.len());
            let mut props = BTreeMap::new();
            props.insert("mermaid.operator".into(), op);
            out.edges.insert(
                id.clone(),
                Edge {
                    id,
                    kind: "usecase_relation".into(),
                    from,
                    to,
                    label,
                    directed: true,
                    properties: props,
                },
            );
            continue;
        }
        // Mermaid also allows direct `Login("Sign in")` declarations.
        if line.contains('(') || line.contains('[') {
            let (id, label) = parse_decl_id_label(line, "usecase");
            if !id.is_empty() {
                let shape = if line.contains('[') {
                    "rectangle"
                } else {
                    "rounded"
                };
                insert_usecase_node(&mut out, &id, &label, "usecase", shape, boundary.as_deref());
            }
        }
    }
    Ok(out)
}

fn parse_usecase_edge(line: &str) -> Option<(String, String, String, String)> {
    for op in ["--|>", "..>", "-->", "<--", "--o", "--x", "--"] {
        if let Some(pos) = line.find(op) {
            let from = line[..pos].trim().to_string();
            let rest = line[pos + op.len()..].trim();
            let (to, label) = if let Some((a, b)) = rest.split_once(':') {
                (a.trim(), b.trim().trim_matches('"'))
            } else {
                (rest, "")
            };
            return Some((from, op.into(), to.to_string(), label.to_string()));
        }
    }
    None
}

fn parse_decl_id_label(text: &str, fallback_prefix: &str) -> (String, String) {
    let text = text
        .split("@{")
        .next()
        .unwrap_or(text)
        .split(":::")
        .next()
        .unwrap_or(text)
        .trim()
        .trim_end_matches('{')
        .trim();
    for (open, close) in [('(', ')'), ('[', ']')] {
        if let Some(pos) = text.find(open) {
            let id = text[..pos].trim().trim_matches('"');
            let label = text[pos + 1..]
                .trim_end_matches(close)
                .trim()
                .trim_matches('"');
            let id = if id.is_empty() {
                stable_id(fallback_prefix, label)
            } else {
                id.to_string()
            };
            return (id, label.to_string());
        }
    }
    let raw = text.trim_matches('"');
    if raw.contains(char::is_whitespace) {
        (stable_id(fallback_prefix, raw), raw.to_string())
    } else {
        (raw.to_string(), raw.to_string())
    }
}

fn parse_flow_node(text: &str) -> (String, String, String) {
    let metadata = text
        .split("@{")
        .nth(1)
        .and_then(|m| m.split('}').next())
        .unwrap_or("");
    let kind = metadata
        .split(',')
        .find_map(|kv| kv.split_once(':'))
        .filter(|(k, _)| k.trim() == "shape")
        .map(|(_, v)| v.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| "node".into());
    let base = text.split("@{").next().unwrap_or(text).trim();
    let (id, label) = parse_decl_id_label(base, "node");
    (id, label, kind)
}

fn insert_usecase_node(
    out: &mut GraphDoc,
    id: &str,
    label: &str,
    kind: &str,
    shape: &str,
    boundary: Option<&str>,
) {
    out.nodes.insert(
        id.to_string(),
        Node {
            id: id.to_string(),
            kind: kind.into(),
            label: label.into(),
            shape: shape.into(),
            properties: BTreeMap::new(),
        },
    );
    if let Some(group) = boundary {
        add_group_member(out, group, id);
    }
}

fn add_group_member(out: &mut GraphDoc, group: &str, member: &str) {
    if let Some(group) = out.groups.get_mut(group) {
        let members = group.properties.entry("members".into()).or_default();
        if !members.is_empty() {
            members.push('\u{1f}');
        }
        members.push_str(member);
    }
}

fn ensure_simple_node(out: &mut GraphDoc, id: &str) {
    out.nodes.entry(id.to_string()).or_insert(Node {
        id: id.into(),
        kind: "node".into(),
        label: id.into(),
        shape: "rectangle".into(),
        properties: BTreeMap::new(),
    });
}

fn stable_id(prefix: &str, label: &str) -> String {
    let body: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{prefix}_{}", body.trim_matches('_'))
}

fn first_payload_line(source: &str) -> Option<&str> {
    payload_lines(source)
        .find(|line| !line.trim().is_empty())
        .map(str::trim)
}

fn payload_lines(source: &str) -> impl Iterator<Item = &str> {
    source.lines().filter(|line| {
        let trimmed = line.trim();
        !trimmed.starts_with("---")
            && !trimmed.starts_with("config:")
            && !trimmed.starts_with("theme:")
    })
}

fn direction_name(debug: &str) -> &'static str {
    match debug {
        "LeftRight" | "LeftToRight" => "LR",
        "RightLeft" | "RightToLeft" => "RL",
        "BottomTop" | "BottomToTop" => "BT",
        _ => "TD",
    }
}
