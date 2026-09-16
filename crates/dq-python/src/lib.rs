use dq_core::{Document, InputFormat, OutputFormat, TransformOptions};
use pyo3::prelude::*;

#[pyclass(name = "Document")]
struct PyDocument {
    inner: Document,
    input_format: InputFormat,
}

#[pymethods]
impl PyDocument {
    #[new]
    fn new(source: &str, input_format: &str) -> PyResult<Self> {
        let input_format = parse_input(input_format)?;
        let inner = Document::parse(source, input_format)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        Ok(Self {
            inner,
            input_format,
        })
    }

    fn transform(&self, query: &str, output_format: &str) -> PyResult<String> {
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
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

#[pyfunction]
fn transform(
    source: &str,
    query: &str,
    input_format: &str,
    output_format: &str,
) -> PyResult<String> {
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
    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

#[pymodule]
fn dq(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDocument>()?;
    m.add_function(wrap_pyfunction!(transform, m)?)?;
    Ok(())
}

fn parse_input(v: &str) -> PyResult<InputFormat> {
    match v {
        "d2" => Ok(InputFormat::D2),
        "mermaid" => Ok(InputFormat::Mermaid),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "input_format must be d2|mermaid",
        )),
    }
}

fn parse_output(v: &str) -> PyResult<OutputFormat> {
    match v {
        "d2" => Ok(OutputFormat::D2),
        "mermaid" => Ok(OutputFormat::Mermaid),
        _ => Err(pyo3::exceptions::PyValueError::new_err(
            "output_format must be d2|mermaid",
        )),
    }
}
