use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "dq-external-validation-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create external validation temporary directory");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn valid_fixtures(extension: &str) -> Vec<PathBuf> {
    let root = repo();
    let mut paths = [
        root.join("examples"),
        root.join("tests/assets/source"),
        root.join("tests/assets/expect"),
    ]
    .into_iter()
    .flat_map(|directory| {
        fs::read_dir(directory)
            .expect("read diagram fixture directory")
            .map(|entry| entry.expect("read diagram fixture entry").path())
    })
    .filter(|path| path.extension().is_some_and(|actual| actual == extension))
    .filter(|path| {
        !path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("invalid-"))
    })
    .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn valid_mermaid_fixtures() -> Vec<PathBuf> {
    let mut paths = valid_fixtures("mmd");
    paths.push(repo().join("tests/assets/source/adversarial-mermaid.txt"));
    paths.sort();
    paths
}

fn validator(command: &str, override_name: &str) -> Option<OsString> {
    let executable = env::var_os(override_name).unwrap_or_else(|| OsString::from(command));
    match Command::new(&executable).arg("--version").output() {
        Ok(output) if output.status.success() => Some(executable),
        Ok(output) => panic!(
            "{command} was found but `--version` failed with {}; stderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "skipping external {command} validation: install {command} or set {override_name}"
            );
            None
        }
        Err(error) => panic!("could not execute {command}: {error}"),
    }
}

fn assert_success(tool: &str, input: &Path, output: std::process::Output) {
    assert!(
        output.status.success(),
        "{tool} rejected {} with {}; stdout:\n{}\nstderr:\n{}",
        input.display(),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn valid_d2_sources_and_snapshots_compile_with_official_d2() {
    let Some(d2) = validator("d2", "DQ_D2") else {
        return;
    };
    let temporary = TempDir::new();

    for (index, input) in valid_fixtures("d2").iter().enumerate() {
        let svg = temporary.path.join(format!("{index}.svg"));
        let output = Command::new(&d2)
            .arg(input)
            .arg(&svg)
            .output()
            .expect("run d2");
        assert_success("d2", input, output);
        assert!(
            fs::metadata(&svg)
                .expect("read generated D2 SVG metadata")
                .len()
                > 0,
            "d2 generated an empty SVG for {}",
            input.display(),
        );
    }
}

#[test]
fn valid_mermaid_sources_and_snapshots_parse_with_merman_cli() {
    let Some(merman) = validator("merman-cli", "DQ_MERMAN_CLI") else {
        return;
    };

    for input in valid_mermaid_fixtures() {
        let output = Command::new(&merman)
            .arg("parse")
            .arg(&input)
            .output()
            .expect("run merman-cli parse");
        assert_success("merman-cli parse", &input, output);
    }
}
