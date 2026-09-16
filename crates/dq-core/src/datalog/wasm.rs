use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use crate::{DqError, Result};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DatalogValue {
    Str(String),
    Int(i64),
    Bool(bool),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FactDatabase {
    inner: BTreeMap<String, BTreeSet<Vec<DatalogValue>>>,
}

impl FactDatabase {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, predicate: &str, args: Vec<DatalogValue>) -> bool {
        self.inner
            .entry(predicate.to_owned())
            .or_default()
            .insert(args)
    }

    fn contains(&self, predicate: &str, args: &[DatalogValue]) -> bool {
        self.inner
            .get(predicate)
            .is_some_and(|tuples| tuples.contains(args))
    }

    pub(crate) fn tuples_for(&self, predicate: &str) -> impl Iterator<Item = &Vec<DatalogValue>> {
        static EMPTY: OnceLock<BTreeSet<Vec<DatalogValue>>> = OnceLock::new();
        self.inner
            .get(predicate)
            .unwrap_or_else(|| EMPTY.get_or_init(BTreeSet::new))
            .iter()
    }

    fn is_empty(&self) -> bool {
        self.inner.values().all(BTreeSet::is_empty)
    }

    fn merge(&mut self, other: &Self) {
        for (predicate, tuples) in &other.inner {
            for tuple in tuples {
                self.insert(predicate, tuple.clone());
            }
        }
    }
}

#[derive(Debug)]
struct Program {
    facts: Vec<Fact>,
    rules: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Fact {
    predicate: String,
    args: Vec<DatalogValue>,
}

#[derive(Debug)]
struct Rule {
    head: Atom,
    body: Vec<Atom>,
}

#[derive(Debug, Clone)]
struct Atom {
    predicate: String,
    terms: Vec<Term>,
}

#[derive(Debug, Clone)]
enum Term {
    Variable(String),
    Constant(DatalogValue),
}

type Substitution = BTreeMap<String, DatalogValue>;

#[cfg(test)]
pub(crate) fn evaluate(source: &str) -> Result<FactDatabase> {
    evaluate_with_facts(source, Vec::new())
}

pub(crate) fn evaluate_with_facts(
    source: &str,
    facts: Vec<(String, Vec<DatalogValue>)>,
) -> Result<FactDatabase> {
    let mut program = parse_program(source)?;
    program.facts.extend(
        facts
            .into_iter()
            .map(|(predicate, args)| Fact { predicate, args }),
    );
    evaluate_program(program)
}

fn evaluate_program(program: Program) -> Result<FactDatabase> {
    let mut database = FactDatabase::new();
    let mut delta = FactDatabase::new();

    for fact in program.facts {
        if database.insert(&fact.predicate, fact.args.clone()) {
            delta.insert(&fact.predicate, fact.args);
        }
    }

    // Rules with no premises are facts. Apply them before the semi-naive loop so
    // their output participates in the first delta round.
    for rule in &program.rules {
        if rule.body.is_empty() {
            let fact = ground(&rule.head, &Substitution::new())?;
            if database.insert(&fact.predicate, fact.args.clone()) {
                delta.insert(&fact.predicate, fact.args);
            }
        }
    }

    while !delta.is_empty() {
        let mut next_delta = FactDatabase::new();
        for rule in &program.rules {
            for anchor in 0..rule.body.len() {
                for fact in derive(rule, &database, &delta, anchor)? {
                    if !database.contains(&fact.predicate, &fact.args) {
                        next_delta.insert(&fact.predicate, fact.args);
                    }
                }
            }
        }
        if next_delta.is_empty() {
            break;
        }
        database.merge(&next_delta);
        delta = next_delta;
    }

    Ok(database)
}

fn derive(
    rule: &Rule,
    database: &FactDatabase,
    delta: &FactDatabase,
    anchor: usize,
) -> Result<Vec<Fact>> {
    let mut substitutions = vec![Substitution::new()];
    for (index, atom) in rule.body.iter().enumerate() {
        let candidates = if index == anchor { delta } else { database };
        substitutions = extend(substitutions, atom, candidates);
        if substitutions.is_empty() {
            return Ok(Vec::new());
        }
    }
    substitutions
        .into_iter()
        .map(|substitution| ground(&rule.head, &substitution))
        .collect()
}

fn extend(
    substitutions: Vec<Substitution>,
    atom: &Atom,
    database: &FactDatabase,
) -> Vec<Substitution> {
    let mut next = Vec::new();
    for substitution in substitutions {
        for tuple in database.tuples_for(&atom.predicate) {
            if tuple.len() != atom.terms.len() {
                continue;
            }
            let mut candidate = substitution.clone();
            if atom
                .terms
                .iter()
                .zip(tuple)
                .all(|(term, value)| unify(term, value, &mut candidate))
            {
                next.push(candidate);
            }
        }
    }
    next
}

fn unify(term: &Term, value: &DatalogValue, substitution: &mut Substitution) -> bool {
    match term {
        Term::Constant(constant) => constant == value,
        Term::Variable(name) => match substitution.get(name) {
            Some(existing) => existing == value,
            None => {
                substitution.insert(name.clone(), value.clone());
                true
            }
        },
    }
}

fn ground(atom: &Atom, substitution: &Substitution) -> Result<Fact> {
    let mut args = Vec::with_capacity(atom.terms.len());
    for term in &atom.terms {
        match term {
            Term::Constant(value) => args.push(value.clone()),
            Term::Variable(name) => {
                let value = substitution.get(name).ok_or_else(|| {
                    DqError::Datalog(format!(
                        "rule head variable {name:?} is not bound by its body"
                    ))
                })?;
                args.push(value.clone());
            }
        }
    }
    Ok(Fact {
        predicate: atom.predicate.clone(),
        args,
    })
}

fn parse_program(source: &str) -> Result<Program> {
    let mut facts = Vec::new();
    let mut rules = Vec::new();
    for clause in split_top_level(&strip_comments(source), '.')? {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }
        if let Some((head, body)) = split_once_unquoted(clause, ":-") {
            rules.push(Rule {
                head: parse_atom(head)?,
                body: split_top_level(body, ',')
                    .map_err(|error| DqError::Datalog(error.to_string()))?
                    .into_iter()
                    .map(|atom| parse_atom(&atom))
                    .collect::<Result<Vec<_>>>()?,
            });
        } else {
            let atom = parse_atom(clause)?;
            let args = atom
                .terms
                .into_iter()
                .map(|term| match term {
                    Term::Constant(value) => Ok(value),
                    Term::Variable(name) => Err(DqError::Datalog(format!(
                        "facts cannot contain variable {name:?}"
                    ))),
                })
                .collect::<Result<Vec<_>>>()?;
            facts.push(Fact {
                predicate: atom.predicate,
                args,
            });
        }
    }
    Ok(Program { facts, rules })
}

fn parse_atom(source: &str) -> Result<Atom> {
    let source = source.trim();
    if source.is_empty() {
        return Err(DqError::Datalog("empty atom".into()));
    }
    let Some(open) = source.find('(') else {
        validate_identifier(source, "predicate")?;
        return Ok(Atom {
            predicate: source.to_owned(),
            terms: Vec::new(),
        });
    };
    let predicate = source[..open].trim();
    validate_identifier(predicate, "predicate")?;
    let close = source
        .rfind(')')
        .filter(|close| source[*close + 1..].trim().is_empty())
        .ok_or_else(|| DqError::Datalog(format!("missing closing ')' in atom {source:?}")))?;
    let terms = split_top_level(&source[open + 1..close], ',')?
        .into_iter()
        .filter(|term| !term.trim().is_empty())
        .map(|term| parse_term(&term))
        .collect::<Result<Vec<_>>>()?;
    Ok(Atom {
        predicate: predicate.to_owned(),
        terms,
    })
}

fn parse_term(source: &str) -> Result<Term> {
    let source = source.trim();
    let first = source
        .chars()
        .next()
        .ok_or_else(|| DqError::Datalog("empty term".into()))?;
    if source.starts_with('"') {
        let value = serde_json::from_str(source).map_err(|error| {
            DqError::Datalog(format!("invalid quoted Datalog string {source:?}: {error}"))
        })?;
        return Ok(Term::Constant(DatalogValue::Str(value)));
    }
    if first.is_uppercase() || first == '_' {
        validate_identifier(source, "variable")?;
        return Ok(Term::Variable(source.to_owned()));
    }
    let value = match source {
        "true" => DatalogValue::Bool(true),
        "false" => DatalogValue::Bool(false),
        _ => match source.parse() {
            Ok(integer) => DatalogValue::Int(integer),
            Err(_) if first.is_lowercase() => {
                validate_identifier(source, "string constant")?;
                DatalogValue::Str(source.to_owned())
            }
            Err(_) => {
                return Err(DqError::Datalog(format!(
                    "cannot parse Datalog term {source:?}"
                )));
            }
        },
    };
    Ok(Term::Constant(value))
}

fn strip_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let mut quoted = false;
            let mut escaped = false;
            for (index, character) in line.char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }
                match character {
                    '\\' if quoted => escaped = true,
                    '"' => quoted = !quoted,
                    '%' if !quoted => return &line[..index],
                    _ => {}
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn split_top_level(source: &str, separator: char) -> Result<Vec<String>> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| DqError::Datalog("unmatched ')'".into()))?;
            }
            _ if !quoted && depth == 0 && character == separator => {
                items.push(source[start..index].trim().to_owned());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if quoted {
        return Err(DqError::Datalog("unterminated quoted string".into()));
    }
    if depth != 0 {
        return Err(DqError::Datalog("unclosed '('".into()));
    }
    items.push(source[start..].trim().to_owned());
    Ok(items)
}

fn split_once_unquoted<'a>(source: &'a str, separator: &str) -> Option<(&'a str, &'a str)> {
    let bytes = source.as_bytes();
    let separator = separator.as_bytes();
    let mut quoted = false;
    let mut escaped = false;
    let mut index = 0;
    while index + separator.len() <= bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if byte == b'\\' && quoted {
            escaped = true;
            index += 1;
            continue;
        }
        if byte == b'"' {
            quoted = !quoted;
            index += 1;
            continue;
        }
        if !quoted && &bytes[index..index + separator.len()] == separator {
            return Some((&source[..index], &source[index + separator.len()..]));
        }
        index += 1;
    }
    None
}

fn validate_identifier(value: &str, kind: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
    {
        return Err(DqError::Datalog(format!("invalid {kind} {value:?}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DatalogValue, evaluate};

    #[test]
    fn evaluates_recursive_rules_and_quoted_strings() {
        let database = evaluate(
            r#"
            parent("alice", "bob").
            parent("bob", "carol").
            ancestor(X, Y) :- parent(X, Y).
            ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z).
            label("a\"b").
            "#,
        )
        .expect("evaluate program");

        assert!(database.contains(
            "ancestor",
            &[
                DatalogValue::Str("alice".into()),
                DatalogValue::Str("carol".into()),
            ],
        ));
        assert!(database.contains("label", &[DatalogValue::Str("a\"b".into())]));
    }
}
