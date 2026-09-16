#[derive(Debug, thiserror::Error)]
pub enum DqError {
    #[error("D2 parse error: {0}")]
    D2Parse(String),
    #[error("Mermaid parse error: {0}")]
    MermaidParse(String),
    #[error("Datalog error: {0}")]
    Datalog(String),
    #[error("invalid view: {0}")]
    InvalidView(String),
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DqError>;
