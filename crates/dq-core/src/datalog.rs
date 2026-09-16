#[cfg(not(target_arch = "wasm32"))]
use oxirs_rule::datalog::parser::parse_program;
#[cfg(not(target_arch = "wasm32"))]
use oxirs_rule::datalog::{DatalogFact, SemiNaiveEvaluator};

#[cfg(not(target_arch = "wasm32"))]
use std::borrow::Cow;

#[cfg(any(target_arch = "wasm32", test))]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use oxirs_rule::datalog::{DatalogValue, FactDatabase};
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::{DatalogValue, FactDatabase};

use crate::{DqError, GraphDoc, Result};

pub fn evaluate(graph: &GraphDoc, query: &str) -> Result<FactDatabase> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let query = encode_native_quoted_strings(query)?;
        let mut program = parse_program(&query).map_err(|e| DqError::Datalog(e.to_string()))?;
        for (predicate, mut args) in facts(graph) {
            for value in &mut args {
                encode_native_value(value);
            }
            program.add_fact(DatalogFact { predicate, args });
        }
        SemiNaiveEvaluator::new()
            .evaluate(&program)
            .map_err(|e| DqError::Datalog(e.to_string()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm::evaluate_with_facts(query, facts(graph))
    }
}

pub fn facts(graph: &GraphDoc) -> Vec<(String, Vec<DatalogValue>)> {
    let mut out = Vec::new();
    push_fact(
        &mut out,
        "document",
        &[s(&graph.source_format), s(&graph.family)],
    );
    if let Some(direction) = &graph.direction {
        push_fact(&mut out, "document_direction", &[s(direction)]);
    }

    for node in graph.nodes.values() {
        push_fact(
            &mut out,
            "node",
            &[s(&node.id), s(&node.kind), s(&node.label), s(&node.shape)],
        );
        for (key, value) in &node.properties {
            push_fact(
                &mut out,
                "prop",
                &[s("node"), s(&node.id), s(key), s(value)],
            );
            if let Some(position) = key.strip_prefix("member.")
                && let Some((name, ty)) = value.split_once('\u{1f}')
            {
                let position = position.parse::<i64>().unwrap_or(0);
                push_fact(
                    &mut out,
                    "member",
                    &[s(&node.id), DatalogValue::Int(position), s(name), s(ty)],
                );
            }
        }
    }

    for edge in graph.edges.values() {
        push_fact(
            &mut out,
            "edge",
            &[
                s(&edge.id),
                s(&edge.kind),
                s(&edge.from),
                s(&edge.to),
                s(&edge.label),
                DatalogValue::Bool(edge.directed),
            ],
        );
        for (key, value) in &edge.properties {
            push_fact(
                &mut out,
                "prop",
                &[s("edge"), s(&edge.id), s(key), s(value)],
            );
        }
    }

    for group in graph.groups.values() {
        push_fact(&mut out, "group", &[s(&group.id), s(&group.label)]);
        for (key, value) in &group.properties {
            push_fact(
                &mut out,
                "prop",
                &[s("group"), s(&group.id), s(key), s(value)],
            );
        }
    }

    for group in graph.groups.values() {
        if let Some(parent) = group.properties.get("parent") {
            push_fact(&mut out, "contains", &[s(parent), s("group"), s(&group.id)]);
        }
        if let Some(members) = group.properties.get("members") {
            for member in members.split('\u{1f}') {
                if !member.is_empty() {
                    let kind = if graph.groups.contains_key(member) {
                        "group"
                    } else {
                        "node"
                    };
                    push_fact(&mut out, "contains", &[s(&group.id), s(kind), s(member)]);
                }
            }
        }
    }

    for note in graph.notes.values() {
        push_fact(
            &mut out,
            "note",
            &[
                s(&note.id),
                s(&note.target_kind),
                s(&note.target_id),
                s(&note.text),
                s(&note.position),
            ],
        );
    }

    for (key, value) in &graph.properties {
        push_fact(&mut out, "document_prop", &[s(key), s(value)]);
    }

    out
}

fn s(value: &str) -> DatalogValue {
    DatalogValue::Str(value.to_owned())
}

fn push_fact(out: &mut Vec<(String, Vec<DatalogValue>)>, predicate: &str, values: &[DatalogValue]) {
    out.push((predicate.to_owned(), values.to_vec()));
}

pub fn as_string(value: &DatalogValue) -> Result<String> {
    match value {
        DatalogValue::Str(v) => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                Ok(decode_native_string(v).into_owned())
            }
            #[cfg(target_arch = "wasm32")]
            {
                Ok(v.clone())
            }
        }
        DatalogValue::Int(v) => Ok(v.to_string()),
        DatalogValue::Bool(v) => Ok(v.to_string()),
    }
}

pub fn as_i64(value: &DatalogValue) -> Result<i64> {
    match value {
        DatalogValue::Int(v) => Ok(*v),
        DatalogValue::Str(v) => v
            .parse()
            .map_err(|_| DqError::InvalidView(format!("expected integer, got {v:?}"))),
        DatalogValue::Bool(v) => Err(DqError::InvalidView(format!("expected integer, got {v}"))),
    }
}

// oxirs-rule preserves quoted string escapes literally. Encode strings before
// native evaluation so its parser sees no escaped delimiters, then decode at
// materialization.
#[cfg(not(target_arch = "wasm32"))]
const NATIVE_ESCAPE: char = '\u{e000}';

#[cfg(not(target_arch = "wasm32"))]
fn encode_native_quoted_strings(source: &str) -> Result<Cow<'_, str>> {
    if !source.contains('"') {
        return Ok(Cow::Borrowed(source));
    }

    let mut out = String::with_capacity(source.len());
    let mut copied_until = 0;
    let mut in_comment = false;
    let mut chars = source.char_indices();

    while let Some((start, ch)) = chars.next() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if ch == '%' {
            in_comment = true;
            continue;
        }
        if ch != '"' {
            continue;
        }

        let mut escaped = false;
        let end = loop {
            let Some((offset, ch)) = chars.next() else {
                return Err(DqError::Datalog(
                    "unterminated quoted string in query".into(),
                ));
            };
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => break offset,
                _ => {}
            }
        };

        let literal = &source[start..=end];
        let decoded = serde_json::from_str::<String>(literal).map_err(|error| {
            DqError::Datalog(format!(
                "invalid quoted Datalog string {literal:?}: {error}"
            ))
        })?;
        out.push_str(&source[copied_until..start]);
        out.push('"');
        out.push_str(&encode_native_string(&decoded));
        out.push('"');
        copied_until = end + 1;
    }
    out.push_str(&source[copied_until..]);
    Ok(Cow::Owned(out))
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_native_value(value: &mut DatalogValue) {
    if let DatalogValue::Str(value) = value
        && let Cow::Owned(encoded) = encode_native_string(value)
    {
        *value = encoded;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_native_string(value: &str) -> Cow<'_, str> {
    if !value
        .chars()
        .any(|ch| matches!(ch, NATIVE_ESCAPE | '"' | '\\' | '\n' | '\r' | '\t' | '%'))
    {
        return Cow::Borrowed(value);
    }

    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            NATIVE_ESCAPE => out.push_str("\u{e000}0"),
            '"' => out.push_str("\u{e000}q"),
            '\\' => out.push_str("\u{e000}b"),
            '\n' => out.push_str("\u{e000}n"),
            '\r' => out.push_str("\u{e000}r"),
            '\t' => out.push_str("\u{e000}t"),
            '%' => out.push_str("\u{e000}p"),
            _ => out.push(ch),
        }
    }
    Cow::Owned(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_native_string(value: &str) -> Cow<'_, str> {
    if !value.contains(NATIVE_ESCAPE) {
        return Cow::Borrowed(value);
    }

    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != NATIVE_ESCAPE {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('0') => out.push(NATIVE_ESCAPE),
            Some('q') => out.push('"'),
            Some('b') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('p') => out.push('%'),
            Some(other) => {
                out.push(NATIVE_ESCAPE);
                out.push(other);
            }
            None => out.push(NATIVE_ESCAPE),
        }
    }
    Cow::Owned(out)
}
