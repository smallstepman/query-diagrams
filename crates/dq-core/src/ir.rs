use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Props = BTreeMap<String, String>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphDoc {
    pub source_format: String,
    pub family: String,
    pub direction: Option<String>,
    pub nodes: BTreeMap<String, Node>,
    pub edges: BTreeMap<String, Edge>,
    pub groups: BTreeMap<String, Group>,
    pub notes: BTreeMap<String, Note>,
    pub properties: Props,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub shape: String,
    #[serde(default)]
    pub properties: Props,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub directed: bool,
    #[serde(default)]
    pub properties: Props,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub properties: Props,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub target_kind: String,
    pub target_id: String,
    pub text: String,
    pub position: String,
    #[serde(default)]
    pub properties: Props,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewGraph {
    pub family: String,
    pub direction: Option<String>,
    pub nodes: BTreeMap<String, ViewNode>,
    pub edges: BTreeMap<String, ViewEdge>,
    pub groups: BTreeMap<String, ViewGroup>,
    pub notes: BTreeMap<String, Note>,
    #[serde(default)]
    pub options: Props,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewNode {
    pub id: String,
    pub entity_id: Option<String>,
    pub kind: String,
    pub label: String,
    pub shape: String,
    #[serde(default)]
    pub properties: Props,
    #[serde(default)]
    pub styles: Props,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewEdge {
    pub id: String,
    pub relation_id: Option<String>,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub directed: bool,
    #[serde(default)]
    pub properties: Props,
    #[serde(default)]
    pub styles: Props,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewGroup {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub properties: Props,
    #[serde(default)]
    pub styles: Props,
    #[serde(default)]
    pub members: BTreeMap<String, String>,
}
