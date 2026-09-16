use std::collections::{BTreeMap, BTreeSet};

use crate::datalog::{DatalogValue, FactDatabase, as_i64, as_string};
use crate::{
    DqError, Edge, GraphDoc, Note, Props, Result, ViewEdge, ViewGraph, ViewGroup, ViewNode,
};

#[derive(Debug, Clone)]
struct Winner {
    priority: i64,
    value: String,
}

pub fn materialize(src: &GraphDoc, db: &FactDatabase) -> Result<ViewGraph> {
    let mut out = ViewGraph {
        family: src.family.clone(),
        direction: src.direction.clone(),
        ..ViewGraph::default()
    };

    // Existing entity instances: view_node(Instance, Entity).
    for tuple in db.tuples_for("view_node") {
        require_arity("view_node", tuple, 2)?;
        let instance = as_string(&tuple[0])?;
        let entity = as_string(&tuple[1])?;
        let n = src.nodes.get(&entity).ok_or_else(|| {
            DqError::InvalidView(format!("view_node references unknown entity {entity:?}"))
        })?;
        out.nodes.insert(
            instance.clone(),
            ViewNode {
                id: instance,
                entity_id: Some(entity),
                kind: n.kind.clone(),
                label: n.label.clone(),
                shape: n.shape.clone(),
                properties: n.properties.clone(),
                styles: base_styles(&n.properties),
            },
        );
    }

    // Synthetic nodes: view_new_node(Instance, Kind, Label, Shape).
    for tuple in db.tuples_for("view_new_node") {
        require_arity("view_new_node", tuple, 4)?;
        let id = as_string(&tuple[0])?;
        out.nodes.insert(
            id.clone(),
            ViewNode {
                id,
                entity_id: None,
                kind: as_string(&tuple[1])?,
                label: as_string(&tuple[2])?,
                shape: as_string(&tuple[3])?,
                properties: BTreeMap::new(),
                styles: BTreeMap::new(),
            },
        );
    }

    // Groups come before edges because groups are valid edge endpoints.
    // view_group(Instance, Label).
    for tuple in db.tuples_for("view_group") {
        require_arity("view_group", tuple, 2)?;
        let id = as_string(&tuple[0])?;
        let label = as_string(&tuple[1])?;
        let props = src
            .groups
            .get(&id)
            .map(|g| g.properties.clone())
            .unwrap_or_default();
        out.groups.insert(
            id.clone(),
            ViewGroup {
                id,
                label,
                styles: base_styles(&props),
                properties: props,
                members: BTreeMap::new(),
            },
        );
    }

    // Existing relations: view_edge(Instance, Relation, FromInstance, ToInstance).
    for tuple in db.tuples_for("view_edge") {
        require_arity("view_edge", tuple, 4)?;
        let instance = as_string(&tuple[0])?;
        let relation = as_string(&tuple[1])?;
        let from = as_string(&tuple[2])?;
        let to = as_string(&tuple[3])?;
        require_endpoint(&out, &instance, &from)?;
        require_endpoint(&out, &instance, &to)?;
        let e = src.edges.get(&relation).ok_or_else(|| {
            DqError::InvalidView(format!(
                "view_edge references unknown relation {relation:?}"
            ))
        })?;
        out.edges
            .insert(instance.clone(), copy_edge(instance, relation, from, to, e));
    }

    // Synthetic edges: view_new_edge(Instance, From, To, Kind, Label).
    for tuple in db.tuples_for("view_new_edge") {
        require_arity("view_new_edge", tuple, 5)?;
        let id = as_string(&tuple[0])?;
        let from = as_string(&tuple[1])?;
        let to = as_string(&tuple[2])?;
        require_endpoint(&out, &id, &from)?;
        require_endpoint(&out, &id, &to)?;
        out.edges.insert(
            id.clone(),
            ViewEdge {
                id,
                relation_id: None,
                kind: as_string(&tuple[3])?,
                from,
                to,
                label: as_string(&tuple[4])?,
                directed: true,
                properties: BTreeMap::new(),
                styles: BTreeMap::new(),
            },
        );
    }

    // Placement: view_contains(Group, ItemKind, ItemInstance, Priority).
    let mut parent_winners: BTreeMap<(String, String), Winner> = BTreeMap::new();
    for tuple in db.tuples_for("view_contains") {
        require_arity("view_contains", tuple, 4)?;
        let group = as_string(&tuple[0])?;
        let kind = as_string(&tuple[1])?;
        let item = as_string(&tuple[2])?;
        let priority = as_i64(&tuple[3])?;
        if !out.groups.contains_key(&group) {
            return Err(DqError::InvalidView(format!(
                "view_contains references unknown group {group:?}"
            )));
        }
        match kind.as_str() {
            "node" if !out.nodes.contains_key(&item) => {
                return Err(DqError::InvalidView(format!(
                    "unknown node instance {item:?}"
                )));
            }
            "group" if !out.groups.contains_key(&item) => {
                return Err(DqError::InvalidView(format!(
                    "unknown group instance {item:?}"
                )));
            }
            "node" | "group" => {}
            _ => {
                return Err(DqError::InvalidView(format!(
                    "view_contains item kind must be node/group, got {kind:?}"
                )));
            }
        }
        choose(
            &mut parent_winners,
            (kind.clone(), item.clone()),
            group,
            priority,
            "containment",
        )?;
    }
    for ((kind, item), winner) in parent_winners {
        out.groups
            .get_mut(&winner.value)
            .expect("group was validated")
            .members
            .insert(item, kind);
    }
    validate_group_cycles(&out)?;

    // Styles: view_style(TargetKind, TargetId, Key, Value, Priority).
    let mut style_winners: BTreeMap<(String, String, String), Winner> = BTreeMap::new();
    for tuple in db.tuples_for("view_style") {
        require_arity("view_style", tuple, 5)?;
        let kind = as_string(&tuple[0])?;
        let id = as_string(&tuple[1])?;
        let key = canonicalize_style_key(as_string(&tuple[2])?);
        let value = as_string(&tuple[3])?;
        let priority = as_i64(&tuple[4])?;
        choose(
            &mut style_winners,
            (kind, id, key),
            value,
            priority,
            "style",
        )?;
    }
    for ((kind, id, key), winner) in style_winners {
        match kind.as_str() {
            "node" => {
                out.nodes
                    .get_mut(&id)
                    .ok_or_else(|| {
                        DqError::InvalidView(format!("style target node {id:?} does not exist"))
                    })?
                    .styles
                    .insert(key, winner.value);
            }
            "edge" => {
                out.edges
                    .get_mut(&id)
                    .ok_or_else(|| {
                        DqError::InvalidView(format!("style target edge {id:?} does not exist"))
                    })?
                    .styles
                    .insert(key, winner.value);
            }
            "group" => {
                out.groups
                    .get_mut(&id)
                    .ok_or_else(|| {
                        DqError::InvalidView(format!("style target group {id:?} does not exist"))
                    })?
                    .styles
                    .insert(key, winner.value);
            }
            "graph" => {
                out.options.insert(key, winner.value);
            }
            other => {
                return Err(DqError::InvalidView(format!(
                    "unsupported style target kind {other:?}"
                )));
            }
        }
    }

    // Output-format-specific escape hatch:
    // view_native(Format, TargetKind, TargetId, Key, Value, Priority).
    let mut native_winners: BTreeMap<(String, String, String, String), Winner> = BTreeMap::new();
    for tuple in db.tuples_for("view_native") {
        require_arity("view_native", tuple, 6)?;
        let renderer = as_string(&tuple[0])?;
        let kind = as_string(&tuple[1])?;
        let id = as_string(&tuple[2])?;
        let key = as_string(&tuple[3])?;
        let value = as_string(&tuple[4])?;
        let priority = as_i64(&tuple[5])?;
        choose(
            &mut native_winners,
            (renderer, kind, id, key),
            value,
            priority,
            "native property",
        )?;
    }
    for ((renderer, kind, id, key), winner) in native_winners {
        let property_key = format!("native.{renderer}.{key}");
        match kind.as_str() {
            "node" => {
                out.nodes
                    .get_mut(&id)
                    .ok_or_else(|| {
                        DqError::InvalidView(format!("native target node {id:?} does not exist"))
                    })?
                    .properties
                    .insert(property_key, winner.value);
            }
            "edge" => {
                out.edges
                    .get_mut(&id)
                    .ok_or_else(|| {
                        DqError::InvalidView(format!("native target edge {id:?} does not exist"))
                    })?
                    .properties
                    .insert(property_key, winner.value);
            }
            "group" => {
                out.groups
                    .get_mut(&id)
                    .ok_or_else(|| {
                        DqError::InvalidView(format!("native target group {id:?} does not exist"))
                    })?
                    .properties
                    .insert(property_key, winner.value);
            }
            "graph" => {
                out.options.insert(property_key, winner.value);
            }
            other => {
                return Err(DqError::InvalidView(format!(
                    "unsupported native target kind {other:?}"
                )));
            }
        }
    }

    // Notes: view_note(NoteId, TargetKind, TargetId, Text, Position, Priority).
    let mut note_winners: BTreeMap<String, (i64, Note)> = BTreeMap::new();
    for tuple in db.tuples_for("view_note") {
        require_arity("view_note", tuple, 6)?;
        let note = Note {
            id: as_string(&tuple[0])?,
            target_kind: as_string(&tuple[1])?,
            target_id: as_string(&tuple[2])?,
            text: as_string(&tuple[3])?,
            position: as_string(&tuple[4])?,
            properties: BTreeMap::new(),
        };
        validate_note_target(&out, &note)?;
        let priority = as_i64(&tuple[5])?;
        match note_winners.get(&note.id) {
            Some((p, old))
                if *p == priority
                    && (old.text != note.text
                        || old.target_id != note.target_id
                        || old.target_kind != note.target_kind) =>
            {
                return Err(DqError::InvalidView(format!(
                    "conflicting note {:?} at priority {priority}",
                    note.id
                )));
            }
            Some((p, _)) if *p > priority => {}
            _ => {
                note_winners.insert(note.id.clone(), (priority, note));
            }
        }
    }
    out.notes = note_winners
        .into_iter()
        .map(|(id, (_, note))| (id, note))
        .collect();

    // Options: view_option(Key, Value, Priority).
    let mut option_winners: BTreeMap<String, Winner> = BTreeMap::new();
    for tuple in db.tuples_for("view_option") {
        require_arity("view_option", tuple, 3)?;
        choose(
            &mut option_winners,
            as_string(&tuple[0])?,
            as_string(&tuple[1])?,
            as_i64(&tuple[2])?,
            "option",
        )?;
    }
    for (key, winner) in option_winners {
        out.options.insert(key, winner.value);
    }
    if let Some(direction) = out.options.get("direction") {
        out.direction = Some(direction.clone());
    }

    Ok(out)
}

fn base_styles(properties: &Props) -> Props {
    properties
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix("style.")
                .map(|style_key| (canonical_style_key(style_key).to_owned(), value.clone()))
        })
        .collect()
}

fn canonicalize_style_key(key: String) -> String {
    match key.as_str() {
        "font-color" | "font_color" => "font_color".to_owned(),
        "stroke-width" | "stroke_width" => "stroke_width".to_owned(),
        "stroke-dash" | "stroke_dash" | "stroke_dasharray" => "stroke_dash".to_owned(),
        _ => key,
    }
}

fn canonical_style_key(key: &str) -> &str {
    match key {
        "font-color" | "font_color" => "font_color",
        "stroke-width" | "stroke_width" => "stroke_width",
        "stroke-dash" | "stroke_dash" | "stroke_dasharray" => "stroke_dash",
        _ => key,
    }
}

fn copy_edge(id: String, relation: String, from: String, to: String, edge: &Edge) -> ViewEdge {
    ViewEdge {
        id,
        relation_id: Some(relation),
        kind: edge.kind.clone(),
        from,
        to,
        label: edge.label.clone(),
        directed: edge.directed,
        styles: base_styles(&edge.properties),
        properties: edge.properties.clone(),
    }
}

fn require_endpoint(out: &ViewGraph, edge: &str, endpoint: &str) -> Result<()> {
    if out.nodes.contains_key(endpoint) || out.groups.contains_key(endpoint) {
        Ok(())
    } else {
        Err(DqError::InvalidView(format!(
            "edge {edge:?} references missing endpoint instance {endpoint:?}"
        )))
    }
}

fn validate_note_target(out: &ViewGraph, note: &Note) -> Result<()> {
    let exists = match note.target_kind.as_str() {
        "node" => out.nodes.contains_key(&note.target_id),
        "group" => out.groups.contains_key(&note.target_id),
        "edge" => out.edges.contains_key(&note.target_id),
        _ => false,
    };
    if exists {
        Ok(())
    } else {
        Err(DqError::InvalidView(format!(
            "note {:?} references missing {} target {:?}",
            note.id, note.target_kind, note.target_id
        )))
    }
}

fn require_arity(name: &str, tuple: &[DatalogValue], expected: usize) -> Result<()> {
    if tuple.len() == expected {
        Ok(())
    } else {
        Err(DqError::InvalidView(format!(
            "{name} expects {expected} arguments, got {}",
            tuple.len()
        )))
    }
}

fn choose<K: Ord + Clone + std::fmt::Debug>(
    map: &mut BTreeMap<K, Winner>,
    key: K,
    value: String,
    priority: i64,
    label: &str,
) -> Result<()> {
    match map.get(&key) {
        Some(old) if old.priority == priority && old.value != value => {
            Err(DqError::InvalidView(format!(
                "conflicting {label} for {key:?} at priority {priority}: {:?} vs {:?}",
                old.value, value
            )))
        }
        Some(old) if old.priority > priority => Ok(()),
        _ => {
            map.insert(key, Winner { priority, value });
            Ok(())
        }
    }
}

fn validate_group_cycles(out: &ViewGraph) -> Result<()> {
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    for (group_id, group) in &out.groups {
        for (member, kind) in &group.members {
            if kind == "group" {
                parent.insert(member.as_str(), group_id.as_str());
            }
        }
    }
    for start in out.groups.keys() {
        let mut seen = BTreeSet::new();
        let mut cur = start.as_str();
        while let Some(next) = parent.get(cur).copied() {
            if !seen.insert(cur) {
                return Err(DqError::InvalidView(format!(
                    "group containment cycle involving {start:?}"
                )));
            }
            cur = next;
        }
    }
    Ok(())
}
