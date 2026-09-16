use anyhow::{Context, Result, bail};
use clap::{ArgGroup, Parser, ValueEnum};
use dq_core::{Document, InputFormat, OutputFormat, TransformOptions};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(name = "dq", about = "Datalog-query D2/Mermaid architecture diagrams")]
#[command(group(ArgGroup::new("query_source").required(true).args(["query", "expr"])))]
struct Cli {
    /// Input file, or '-' for stdin.
    #[arg(default_value = "-")]
    input: PathBuf,

    /// Datalog query file.
    #[arg(short = 'q', long)]
    query: Option<PathBuf>,

    /// Inline Datalog query.
    #[arg(short = 'e', long)]
    expr: Option<String>,

    /// Input syntax. Auto uses the filename or source header.
    #[arg(long, value_enum, default_value = "auto")]
    input_format: InputKind,

    /// Output source format. Defaults to d2.
    #[arg(long, value_enum)]
    emit: Option<EmitKind>,

    /// Override output direction (TD/LR/RL/BT).
    #[arg(long)]
    direction: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InputKind {
    Auto,
    D2,
    Mermaid,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
enum EmitKind {
    D2,
    Mermaid,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let source = read_input(&cli.input)?;
    let query = match (&cli.query, &cli.expr) {
        (Some(path), None) => {
            fs::read_to_string(path).with_context(|| format!("reading query {}", path.display()))?
        }
        (None, Some(expr)) => expr.clone(),
        _ => unreachable!("clap enforces exactly one query source"),
    };
    let input_format = detect_input_format(&cli, &source)?;
    let output_format = match cli.emit.unwrap_or(EmitKind::D2) {
        EmitKind::D2 => OutputFormat::D2,
        EmitKind::Mermaid => OutputFormat::Mermaid,
    };
    let doc = Document::parse(&source, input_format)?;
    let result = doc.transform(
        &query,
        &TransformOptions {
            input_format,
            output_format,
            direction: cli.direction,
        },
    )?;
    print!("{result}");
    if !result.ends_with('\n') {
        println!();
    }
    Ok(())
}

fn read_input(path: &Path) -> Result<String> {
    if path == Path::new("-") {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .context("reading stdin")?;
        Ok(source)
    } else {
        fs::read_to_string(path).with_context(|| format!("reading input {}", path.display()))
    }
}

fn detect_input_format(cli: &Cli, source: &str) -> Result<InputFormat> {
    match cli.input_format {
        InputKind::D2 => Ok(InputFormat::D2),
        InputKind::Mermaid => Ok(InputFormat::Mermaid),
        InputKind::Auto => {
            match cli
                .input
                .extension()
                .and_then(|s| s.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("d2") => return Ok(InputFormat::D2),
                Some("mmd" | "mermaid") => return Ok(InputFormat::Mermaid),
                _ => {}
            }
            let first = source
                .lines()
                .map(str::trim_start)
                .find(|line| !line.is_empty() && !line.starts_with("%%"))
                .unwrap_or("");
            let mermaid = [
                "flowchart",
                "graph ",
                "classDiagram",
                "stateDiagram",
                "erDiagram",
                "mindmap",
                "requirementDiagram",
                "C4",
                "block",
                "architecture",
                "sankey",
                "agentflow-beta",
                "usecase-beta",
                "treeView-beta",
                "ishikawa-beta",
            ];
            if mermaid.iter().any(|prefix| first.starts_with(prefix)) {
                Ok(InputFormat::Mermaid)
            } else if first.is_empty() {
                bail!("cannot detect format of empty input; pass --input-format")
            } else {
                Ok(InputFormat::D2)
            }
        }
    }
}
