use dq_core::{Document as CoreDocument, InputFormat, OutputFormat, TransformOptions};
use napi_derive::napi;

#[napi]
pub struct Document {
    inner: CoreDocument,
    input_format: InputFormat,
}

#[napi]
impl Document {
    #[napi(constructor)]
    pub fn new(source: String, input_format: String) -> napi::Result<Self> {
        let input_format = parse_input(&input_format)?;
        let inner = CoreDocument::parse(&source, input_format)
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner,
            input_format,
        })
    }

    #[napi]
    pub fn transform(&self, query: String, output_format: String) -> napi::Result<String> {
        let output_format = parse_output(&output_format)?;
        self.inner
            .transform(
                &query,
                &TransformOptions {
                    input_format: self.input_format,
                    output_format,
                    direction: None,
                },
            )
            .map_err(|e| napi::Error::from_reason(e.to_string()))
    }
}

#[napi]
pub fn transform(
    source: String,
    query: String,
    input_format: String,
    output_format: String,
) -> napi::Result<String> {
    let input_format = parse_input(&input_format)?;
    let output_format = parse_output(&output_format)?;
    dq_core::transform(
        &source,
        &query,
        &TransformOptions {
            input_format,
            output_format,
            direction: None,
        },
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

fn parse_input(value: &str) -> napi::Result<InputFormat> {
    match value {
        "d2" => Ok(InputFormat::D2),
        "mermaid" => Ok(InputFormat::Mermaid),
        _ => Err(napi::Error::from_reason("input_format must be d2|mermaid")),
    }
}

fn parse_output(value: &str) -> napi::Result<OutputFormat> {
    match value {
        "d2" => Ok(OutputFormat::D2),
        "mermaid" => Ok(OutputFormat::Mermaid),
        _ => Err(napi::Error::from_reason("output_format must be d2|mermaid")),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_output;

    #[test]
    fn output_formats_are_limited_to_source_formats() {
        assert!(parse_output("d2").is_ok());
        assert!(parse_output("mermaid").is_ok());

        for output_format in ["json", "mermaid_svg", "d2_svg", "png"] {
            assert!(
                parse_output(output_format).is_err(),
                "unexpectedly accepted {output_format:?}"
            );
        }
    }
}
