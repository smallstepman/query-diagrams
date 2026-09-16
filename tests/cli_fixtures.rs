use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn asset(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/assets")
        .join(path)
}

fn run_cli(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_dq"))
        .args(args)
        .output()
        .expect("run dq CLI");
    assert_success(output)
}

fn assert_success(output: Output) -> String {
    assert!(
        output.status.success(),
        "dq exited with {status}; stderr:\n{stderr}",
        status = output.status,
        stderr = String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout).expect("dq stdout is UTF-8")
}

fn assert_perspective(source: &str, perspective: &str, emit: &str, expectation: &str) {
    let source = asset(source);
    let perspective = asset(perspective);
    let expected = fs::read_to_string(asset(expectation)).expect("read expected perspective");
    let actual = run_cli(&[
        source.to_str().expect("UTF-8 source path"),
        "--query",
        perspective.to_str().expect("UTF-8 perspective path"),
        "--emit",
        emit,
    ]);

    assert_eq!(
        actual, expected,
        "perspective output differed from {}",
        expectation
    );
}

fn assert_rejected(args: &[&str], expected_error: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_dq"))
        .args(args)
        .output()
        .expect("run dq CLI");
    assert!(
        !output.status.success(),
        "dq unexpectedly succeeded; stdout:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected_error),
        "stderr did not contain {expected_error:?}:\n{stderr}",
    );
}

#[test]
fn adversarial_d2_view_matches_d2_snapshot() {
    assert_perspective(
        "source/adversarial-d2.d2",
        "perspectives/adversarial-all.dl",
        "d2",
        "expect/adversarial-d2-all.d2",
    );
}

#[test]
fn adversarial_d2_view_matches_mermaid_snapshot() {
    assert_perspective(
        "source/adversarial-d2.d2",
        "perspectives/adversarial-all.dl",
        "mermaid",
        "expect/adversarial-d2-all.mmd",
    );
}

#[test]
fn adversarial_mermaid_view_matches_d2_snapshot() {
    assert_perspective(
        "source/adversarial-mermaid.txt",
        "perspectives/adversarial-all.dl",
        "d2",
        "expect/adversarial-mermaid-all.d2",
    );
}

#[test]
fn contract_stress_view_matches_d2_snapshot() {
    assert_perspective(
        "source/adversarial-d2.d2",
        "perspectives/contract-stress.dl",
        "d2",
        "expect/contract-stress.d2",
    );
}

#[test]
fn contract_stress_view_matches_mermaid_snapshot() {
    assert_perspective(
        "source/adversarial-d2.d2",
        "perspectives/contract-stress.dl",
        "mermaid",
        "expect/contract-stress.mmd",
    );
}

#[test]
fn direction_flag_overrides_the_perspective_option() {
    let source = asset("source/adversarial-d2.d2");
    let perspective = asset("perspectives/adversarial-all.dl");
    let actual = run_cli(&[
        source.to_str().expect("UTF-8 source path"),
        "--query",
        perspective.to_str().expect("UTF-8 perspective path"),
        "--emit",
        "mermaid",
        "--direction",
        "BT",
    ]);

    assert!(actual.starts_with("flowchart BT\n"), "{actual}");
}

#[test]
fn d2_complete_view_matches_mermaid_snapshot() {
    assert_perspective(
        "source/commerce-platform.d2",
        "perspectives/all.dl",
        "mermaid",
        "expect/d2-all.mmd",
    );
}

#[test]
fn d2_data_path_matches_d2_snapshot() {
    assert_perspective(
        "source/commerce-platform.d2",
        "perspectives/d2-data-path.dl",
        "d2",
        "expect/d2-data-path.d2",
    );
}

#[test]
fn mermaid_complete_view_matches_d2_snapshot() {
    // The .mmd extension exercises automatic Mermaid format detection.
    assert_perspective(
        "source/commerce-platform.mmd",
        "perspectives/all.dl",
        "d2",
        "expect/mermaid-all.d2",
    );
}

#[test]
fn mermaid_perimeter_view_matches_mermaid_snapshot() {
    assert_perspective(
        "source/commerce-platform.mmd",
        "perspectives/mermaid-perimeter.dl",
        "mermaid",
        "expect/mermaid-perimeter.mmd",
    );
}

#[test]
fn cli_rejects_removed_render_and_json_outputs() {
    let input = asset("source/commerce-platform.d2");
    let query = asset("perspectives/all.dl");
    let input = input.to_str().expect("UTF-8 fixture path");
    let query = query.to_str().expect("UTF-8 fixture path");

    let render = Command::new(env!("CARGO_BIN_EXE_dq"))
        .args([input, "--query", query, "--render", "d2-svg"])
        .output()
        .expect("run dq CLI with removed render option");
    assert!(!render.status.success());
    assert!(
        String::from_utf8_lossy(&render.stderr).contains("unexpected argument '--render'"),
        "stderr:\n{}",
        String::from_utf8_lossy(&render.stderr),
    );

    let json = Command::new(env!("CARGO_BIN_EXE_dq"))
        .args([input, "--query", query, "--emit", "json"])
        .output()
        .expect("run dq CLI with removed JSON output");
    assert!(!json.status.success());
    let stderr = String::from_utf8_lossy(&json.stderr);
    assert!(stderr.contains("invalid value 'json'"), "stderr:\n{stderr}");
    assert!(stderr.contains("d2, mermaid"), "stderr:\n{stderr}");
}

#[test]
fn cli_rejects_invalid_sources() {
    let query = asset("perspectives/all.dl");
    let query = query.to_str().expect("UTF-8 perspective path");

    for (source, expected_error) in [
        ("source/invalid-unclosed.d2", "D2 parse error"),
        ("source/invalid-unmatched-brace.d2", "D2 parse error"),
        ("source/invalid-edge-block.d2", "D2 parse error"),
        ("source/invalid-flowchart.mmd", "Mermaid parse error"),
    ] {
        let source = asset(source);
        assert_rejected(
            &[
                source.to_str().expect("UTF-8 source path"),
                "--query",
                query,
                "--emit",
                "d2",
            ],
            expected_error,
        );
    }
}

#[test]
fn cli_rejects_invalid_materialized_views() {
    let source = asset("source/adversarial-d2.d2");
    let source = source.to_str().expect("UTF-8 source path");

    for perspective in [
        "perspectives/invalid-conflicting-style.dl",
        "perspectives/invalid-style-alias-conflict.dl",
        "perspectives/invalid-containment-cycle.dl",
        "perspectives/invalid-dangling-edge.dl",
        "perspectives/invalid-note-target.dl",
    ] {
        let perspective = asset(perspective);
        assert_rejected(
            &[
                source,
                "--query",
                perspective.to_str().expect("UTF-8 perspective path"),
                "--emit",
                "d2",
            ],
            "invalid view:",
        );
    }
}
