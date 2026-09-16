mod datalog;
mod emit_d2;
mod emit_mermaid;
mod error;
mod frontend_d2;
mod frontend_mermaid;
mod ir;
mod materialize;

pub use error::{DqError, Result};
pub use ir::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputFormat {
    D2,
    Mermaid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    D2,
    Mermaid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformOptions {
    pub input_format: InputFormat,
    pub output_format: OutputFormat,
    pub direction: Option<String>,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            input_format: InputFormat::D2,
            output_format: OutputFormat::D2,
            direction: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    graph: GraphDoc,
}

impl Document {
    pub fn parse(source: &str, format: InputFormat) -> Result<Self> {
        let graph = match format {
            InputFormat::D2 => frontend_d2::parse(source)?,
            InputFormat::Mermaid => frontend_mermaid::parse(source)?,
        };
        Ok(Self { graph })
    }

    pub fn graph(&self) -> &GraphDoc {
        &self.graph
    }

    pub fn query(&self, datalog_source: &str) -> Result<ViewGraph> {
        let db = datalog::evaluate(&self.graph, datalog_source)?;
        materialize::materialize(&self.graph, &db)
    }

    pub fn transform(&self, datalog_source: &str, options: &TransformOptions) -> Result<String> {
        let mut view = self.query(datalog_source)?;
        if let Some(direction) = &options.direction {
            view.direction = Some(direction.clone());
            view.options.insert("direction".into(), direction.clone());
        }
        match options.output_format {
            OutputFormat::D2 => emit_d2::emit(&view),
            OutputFormat::Mermaid => emit_mermaid::emit(&view),
        }
    }
}

pub fn transform(source: &str, query: &str, options: &TransformOptions) -> Result<String> {
    Document::parse(source, options.input_format)?.transform(query, options)
}
