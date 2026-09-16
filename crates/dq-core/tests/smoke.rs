use dq_core::{Document, InputFormat, OutputFormat, TransformOptions};

const QUERY: &str = r##"
view_node(N, N) :- node(N, K, L, S).
view_group(G, L) :- group(G, L).
view_contains(G, K, I, 10) :- contains(G, K, I).
view_edge(E, E, A, B) :- edge(E, K, A, B, L, D), view_node(A, A), view_node(B, B).
view_style("node", N, "fill", "#dbeafe", 10) :- view_node(N, E).
view_option("direction", "LR", 100).
"##;

const ADVERSARIAL_D2: &str = include_str!("../../../tests/assets/source/adversarial-d2.d2");
const ADVERSARIAL_ALL: &str = include_str!("../../../tests/assets/perspectives/adversarial-all.dl");
const CONTRACT_STRESS: &str = include_str!("../../../tests/assets/perspectives/contract-stress.dl");

fn transform_source(doc: &Document, query: &str, output_format: OutputFormat) -> String {
    doc.transform(
        query,
        &TransformOptions {
            input_format: InputFormat::D2,
            output_format,
            direction: None,
        },
    )
    .expect("transform source")
}

#[test]
fn d2_to_mermaid_materializes_groups_and_styles() {
    let src = r#"
backend: Backend {
  api: API { 
    shape: rectangle
  }
  db: Database {
    shape: cylinder
  }
  api -> db: query
}
"#;
    let doc = Document::parse(src, InputFormat::D2).expect("parse d2");
    let result = doc
        .transform(
            QUERY,
            &TransformOptions {
                input_format: InputFormat::D2,
                output_format: OutputFormat::Mermaid,
                direction: None,
            },
        )
        .expect("transform");
    assert!(result.contains("flowchart LR"));
    assert!(result.contains("subgraph"));
    assert!(result.contains("fill:#dbeafe"));
}

#[test]
fn d2_output_uses_hierarchical_paths_for_nested_nodes() {
    let src = r#"
backend: Backend {
  api: API {
    shape: rectangle
  }
  db: Database {
    shape: cylinder
  }
  api -> db
}
"#;
    let doc = Document::parse(src, InputFormat::D2).expect("parse d2");
    let result = doc
        .transform(
            QUERY,
            &TransformOptions {
                input_format: InputFormat::D2,
                output_format: OutputFormat::D2,
                direction: None,
            },
        )
        .expect("transform");
    assert!(result.starts_with("direction: \"right\"\n"));
    assert!(result.contains(r#""backend"."api" -> "backend"."db""#));
    assert!(result.contains("->"));
}

#[test]
fn generated_adversarial_sources_reparse_in_both_formats() {
    let doc = Document::parse(ADVERSARIAL_D2, InputFormat::D2).expect("parse adversarial D2");

    for query in [ADVERSARIAL_ALL, CONTRACT_STRESS] {
        let d2 = transform_source(&doc, query, OutputFormat::D2);
        Document::parse(&d2, InputFormat::D2).expect("reparse emitted D2");

        let mermaid = transform_source(&doc, query, OutputFormat::Mermaid);
        Document::parse(&mermaid, InputFormat::Mermaid).expect("reparse emitted Mermaid");
    }
}

#[test]
fn escaped_datalog_strings_match_source_facts_and_emit_literals() {
    let source = r#"
api: "API \"quoted\" value" {
  shape: hexagon
}
"#;
    let query = r#"
view_node(N, N) :- node(N, K, "API \"quoted\" value", S).
view_new_node("annotation", "annotation", "Output \"quoted\" value", "rounded").
view_new_node("multiline", "annotation", "Line 1\nLine 2", "rounded").
"#;
    let doc = Document::parse(source, InputFormat::D2).expect("parse quoted D2");

    let d2 = transform_source(&doc, query, OutputFormat::D2);
    assert!(d2.contains(r#""api": "API \"quoted\" value""#), "{d2}");
    assert!(
        d2.contains(r#""annotation": "Output \"quoted\" value""#),
        "{d2}"
    );
    assert!(d2.contains(r#""multiline": "Line 1\nLine 2""#), "{d2}");

    let mermaid = transform_source(&doc, query, OutputFormat::Mermaid);
    assert!(
        mermaid.contains("API &quot;quoted&quot; value"),
        "{mermaid}"
    );
    assert!(
        mermaid.contains("Output &quot;quoted&quot; value"),
        "{mermaid}"
    );
    assert!(mermaid.contains("Line 1<br/>Line 2"), "{mermaid}");
}
