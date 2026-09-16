use std::collections::BTreeMap;

use crate::{DqError, Edge, GraphDoc, Group, Node, Result};

#[derive(Debug, Clone)]
struct ObjectBlock {
    id: String,
    label: String,
    parent: Option<String>,
    attrs: BTreeMap<String, String>,
    children: Vec<String>,
    member_counter: usize,
}

#[derive(Debug, Clone)]
struct EdgeBlock {
    edge: Edge,
}

#[derive(Debug, Clone)]
enum Scope {
    Object(ObjectBlock),
    Edge(EdgeBlock),
}

/// Deterministic D2 architecture reader.
///
/// This deliberately targets the structural D2 subset used by generated
/// architecture documents: objects, containers, nested containers, class / SQL
/// table bodies, attributes, styles, and directed/undirected edges.  It is
/// isolated behind the frontend so an official D2 semantic adapter can replace
/// it later without changing the IR or Datalog contract.
pub fn parse(source: &str) -> Result<GraphDoc> {
    let mut out = GraphDoc {
        source_format: "d2".into(),
        family: "d2".into(),
        ..GraphDoc::default()
    };
    out.properties.insert("source".into(), source.to_owned());

    let mut stack: Vec<Scope> = Vec::new();
    let mut edge_counter = 0usize;

    for (line_no, raw) in source.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }

        if line == "}" {
            let scope = stack
                .pop()
                .ok_or_else(|| DqError::D2Parse(format!("line {}: unmatched }}", line_no + 1)))?;
            finish_scope(&mut out, scope)?;
            continue;
        }

        // Attributes inside an edge block belong to that edge, not an object.
        if let Some(Scope::Edge(edge_scope)) = stack.last_mut() {
            if let Some((key, value)) = split_once_colon(line) {
                edge_scope
                    .edge
                    .properties
                    .insert(key.trim().to_string(), scalar(value));
                continue;
            }
            return Err(DqError::D2Parse(format!(
                "line {}: expected an edge attribute or }} inside edge block",
                line_no + 1
            )));
        }

        let opens_block = line.ends_with('{');
        let core = if opens_block {
            line[..line.len() - 1].trim()
        } else {
            line
        };

        if let Some((lhs, op, rhs, label)) = parse_edge(core) {
            let mut from = qualify_endpoint(&stack, lhs.trim());
            let mut to = qualify_endpoint(&stack, rhs.trim());
            let operator = match op {
                "<-" => "->",
                "<--" => "-->",
                _ => op,
            };
            if matches!(op, "<-" | "<--") {
                std::mem::swap(&mut from, &mut to);
            }
            ensure_placeholder(&mut out, &from);
            ensure_placeholder(&mut out, &to);
            let id = format!("e{edge_counter}");
            edge_counter += 1;
            let mut properties = BTreeMap::new();
            properties.insert("d2.operator".into(), operator.to_string());
            let edge = Edge {
                id: id.clone(),
                kind: "relation".into(),
                from,
                to,
                label: label.map(scalar).unwrap_or_default(),
                directed: operator.contains('>') || operator.contains('<'),
                properties,
            };
            if opens_block {
                stack.push(Scope::Edge(EdgeBlock { edge }));
            } else {
                out.edges.insert(id, edge);
            }
            continue;
        }

        if opens_block {
            let (id, label) = parse_object_head(core)?;
            let full_id = qualify_child(&stack, &id);
            let parent = current_object_id(&stack).map(str::to_owned);
            if let Some(parent_block) = current_object_mut(&mut stack) {
                parent_block.children.push(full_id.clone());
            }
            stack.push(Scope::Object(ObjectBlock {
                id: full_id,
                label,
                parent,
                attrs: BTreeMap::new(),
                children: Vec::new(),
                member_counter: 0,
            }));
            continue;
        }

        if let Some((key, value)) = split_once_colon(line) {
            let key = unquote(key.trim());
            let value = scalar(value);
            if let Some(block) = current_object_mut(&mut stack) {
                if is_attribute_key(&key) {
                    block.attrs.insert(key, value);
                } else if is_structured_shape(block.attrs.get("shape").map(String::as_str)) {
                    let member_key = format!("member.{}", block.member_counter);
                    block.member_counter += 1;
                    block
                        .attrs
                        .insert(member_key, format!("{key}\u{1f}{value}"));
                } else {
                    let parent_id = block.id.clone();
                    let full_id = if key.contains('.') {
                        key
                    } else {
                        format!("{parent_id}.{key}")
                    };
                    ensure_node(&mut out, &full_id, value, "rectangle");
                    block.children.push(full_id);
                }
            } else if is_document_attribute(&key) {
                if key == "direction" {
                    out.direction = Some(value.clone());
                }
                out.properties.insert(key, value);
            } else {
                ensure_node(&mut out, &key, value, "rectangle");
            }
            continue;
        }

        // Bare identifier declaration.
        let id = qualify_child(&stack, line);
        ensure_node(&mut out, &id, scalar(line), "rectangle");
        if let Some(parent) = current_object_mut(&mut stack) {
            parent.children.push(id);
        }
    }

    if let Some(scope) = stack.last() {
        return Err(DqError::D2Parse(format!(
            "unclosed block {:?}",
            scope_name(scope)
        )));
    }

    Ok(out)
}

fn finish_scope(out: &mut GraphDoc, scope: Scope) -> Result<()> {
    match scope {
        Scope::Edge(edge) => {
            out.edges.insert(edge.edge.id.clone(), edge.edge);
            Ok(())
        }
        Scope::Object(block) => finish_object(out, block),
    }
}

fn finish_object(out: &mut GraphDoc, block: ObjectBlock) -> Result<()> {
    let shape = block.attrs.get("shape").map(String::as_str).unwrap_or("");
    let node_like = !shape.is_empty() || block.children.is_empty();

    if node_like {
        let mut props = block.attrs.clone();
        if let Some(parent) = &block.parent {
            props.insert("parent".into(), parent.clone());
        }
        out.nodes.insert(
            block.id.clone(),
            Node {
                id: block.id,
                kind: if shape.is_empty() { "node" } else { shape }.to_string(),
                label: block.label,
                shape: if shape.is_empty() {
                    "rectangle".into()
                } else {
                    shape.into()
                },
                properties: props,
            },
        );
    } else {
        let mut props = block.attrs;
        if let Some(parent) = block.parent {
            props.insert("parent".into(), parent);
        }
        props.insert("members".into(), block.children.join("\u{1f}"));
        out.groups.insert(
            block.id.clone(),
            Group {
                id: block.id,
                label: block.label,
                properties: props,
            },
        );
    }
    Ok(())
}

fn parse_object_head(head: &str) -> Result<(String, String)> {
    if let Some((id, label)) = split_once_colon(head) {
        let id = unquote(id.trim());
        if id.is_empty() {
            return Err(DqError::D2Parse("empty object id".into()));
        }
        Ok((id.clone(), scalar(label)))
    } else {
        let id = unquote(head.trim());
        if id.is_empty() {
            return Err(DqError::D2Parse("empty object id".into()));
        }
        Ok((id.clone(), id))
    }
}

fn parse_edge(line: &str) -> Option<(&str, &str, &str, Option<&str>)> {
    // Longest operators first.
    for op in ["<->", "-->", "<--", "->", "<-", "--"] {
        if let Some(pos) = find_unquoted(line, op) {
            let lhs = &line[..pos];
            let rest = &line[pos + op.len()..];
            let (rhs, label) = match split_once_colon(rest) {
                Some((r, l)) => (r, Some(l)),
                None => (rest, None),
            };
            return Some((lhs, op, rhs, label));
        }
    }
    None
}

fn find_unquoted(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let needle = needle.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    let mut i = 0usize;
    while i + needle.len() <= bytes.len() {
        let b = bytes[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if b == b'\\' {
            escaped = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            quoted = !quoted;
            i += 1;
            continue;
        }
        if !quoted && &bytes[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn split_once_colon(s: &str) -> Option<(&str, &str)> {
    let mut quoted = false;
    let mut escaped = false;
    for (i, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            ':' if !quoted => return Some((&s[..i], &s[i + 1..])),
            _ => {}
        }
    }
    None
}

fn scalar(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        serde_json::from_str::<String>(value).unwrap_or_else(|_| unquote(value))
    } else {
        value.to_string()
    }
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn is_structured_shape(shape: Option<&str>) -> bool {
    matches!(shape, Some("class" | "sql_table" | "table"))
}

fn is_attribute_key(k: &str) -> bool {
    k == "shape"
        || k == "label"
        || k == "link"
        || k == "tooltip"
        || k == "near"
        || k.starts_with("style.")
        || k.starts_with("icon.")
        || k.starts_with("meta.")
        || k.starts_with("dq.")
}

fn is_document_attribute(k: &str) -> bool {
    matches!(k, "direction" | "layout" | "theme") | k.starts_with("vars.")
}

fn current_object_id(stack: &[Scope]) -> Option<&str> {
    stack.iter().rev().find_map(|scope| match scope {
        Scope::Object(block) => Some(block.id.as_str()),
        Scope::Edge(_) => None,
    })
}

fn current_object_mut(stack: &mut [Scope]) -> Option<&mut ObjectBlock> {
    stack.iter_mut().rev().find_map(|scope| match scope {
        Scope::Object(block) => Some(block),
        Scope::Edge(_) => None,
    })
}

fn qualify_child(stack: &[Scope], child: &str) -> String {
    let child = unquote(child);
    if child.contains('.') {
        child
    } else if let Some(parent) = current_object_id(stack) {
        format!("{parent}.{child}")
    } else {
        child
    }
}

fn qualify_endpoint(stack: &[Scope], endpoint: &str) -> String {
    qualify_child(stack, endpoint.trim())
}

fn ensure_placeholder(out: &mut GraphDoc, id: &str) {
    if out.nodes.contains_key(id) || out.groups.contains_key(id) {
        return;
    }
    ensure_node(
        out,
        id,
        id.rsplit('.').next().unwrap_or(id).to_string(),
        "rectangle",
    );
}

fn ensure_node(out: &mut GraphDoc, id: &str, label: String, shape: &str) {
    out.nodes
        .entry(id.to_string())
        .and_modify(|n| {
            if n.label == n.id || n.label.is_empty() {
                n.label = label.clone();
            }
        })
        .or_insert_with(|| Node {
            id: id.to_string(),
            kind: "node".into(),
            label,
            shape: shape.into(),
            properties: BTreeMap::new(),
        });
}

fn scope_name(scope: &Scope) -> &str {
    match scope {
        Scope::Object(block) => &block.id,
        Scope::Edge(block) => &block.edge.id,
    }
}

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (i, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..i],
            _ => {}
        }
    }
    line
}
