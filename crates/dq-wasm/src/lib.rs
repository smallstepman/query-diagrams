use dq_core::{Document as CoreDocument, InputFormat, OutputFormat, TransformOptions};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Document {
    inner: CoreDocument,
    input_format: InputFormat,
}

#[wasm_bindgen]
impl Document {
    #[wasm_bindgen(constructor)]
    pub fn new(source: &str, input_format: &str) -> Result<Document, JsValue> {
        let input_format = parse_input(input_format)?;
        let inner = CoreDocument::parse(source, input_format)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(Self {
            inner,
            input_format,
        })
    }

    pub fn transform(&self, query: &str, output_format: &str) -> Result<String, JsValue> {
        let output_format = parse_output(output_format)?;
        self.inner
            .transform(
                query,
                &TransformOptions {
                    input_format: self.input_format,
                    output_format,
                    direction: None,
                },
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[wasm_bindgen]
pub fn transform(
    source: &str,
    query: &str,
    input_format: &str,
    output_format: &str,
) -> Result<String, JsValue> {
    let input_format = parse_input(input_format)?;
    let output_format = parse_output(output_format)?;
    dq_core::transform(
        source,
        query,
        &TransformOptions {
            input_format,
            output_format,
            direction: None,
        },
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn parse_input(value: &str) -> Result<InputFormat, JsValue> {
    match value {
        "d2" => Ok(InputFormat::D2),
        "mermaid" => Ok(InputFormat::Mermaid),
        _ => Err(JsValue::from_str("input_format must be d2|mermaid")),
    }
}

fn parse_output(value: &str) -> Result<OutputFormat, JsValue> {
    match value {
        "d2" => Ok(OutputFormat::D2),
        "mermaid" => Ok(OutputFormat::Mermaid),
        _ => Err(JsValue::from_str("output_format must be d2|mermaid")),
    }
}
