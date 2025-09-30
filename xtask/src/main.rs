use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use regex::Regex;
use scraper::{Html, Selector};

fn main() -> Result<()> {
    Cli::parse().run()
}

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Utilities for the atcoder-rust-template",
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

impl Cli {
    fn run(self) -> Result<()> {
        match self.command {
            CommandKind::Fetch(args) => fetch_samples(args),
            CommandKind::Run(args) => run_case(args),
            CommandKind::Test(args) => run_tests(args),
        }
    }
}

#[derive(Subcommand)]
enum CommandKind {
    /// Fetch sample test cases from AtCoder
    Fetch(FetchArgs),
    /// Run a binary with a sample input
    Run(RunArgs),
    /// Run tests (optionally scoped to a single task)
    Test(TestArgs),
}

#[derive(Args)]
struct FetchArgs {
    /// Task identifier(s): pass `<task>` or `<contest> <task>`
    #[arg(value_name = "IDENT", num_args = 1..=2)]
    identifiers: Vec<String>,
    /// Override the full problem id (defaults to `<contest>_<task>`)
    #[arg(long)]
    problem_id: Option<String>,
    /// Language query parameter (ja/en)
    #[arg(long)]
    lang: Option<String>,
    /// Output root directory relative to the project (default: tests)
    #[arg(long, default_value = "tests")]
    out_dir: PathBuf,
    /// Overwrite existing sample files
    #[arg(long)]
    overwrite: bool,
}

#[derive(Args)]
struct RunArgs {
    /// Binary name (e.g. a)
    bin: String,
    /// Sample id (e.g. 1 or 001). Defaults to 001 when omitted.
    case: Option<String>,
    /// Root directory for tests (default: tests)
    #[arg(long, default_value = "tests")]
    tests_dir: PathBuf,
    /// Run in release mode
    #[arg(long)]
    release: bool,
}

#[derive(Args)]
struct TestArgs {
    /// Optional task letter; when omitted runs the entire suite
    target: Option<String>,
    /// Test in release mode
    #[arg(long)]
    release: bool,
}

fn fetch_samples(args: FetchArgs) -> Result<()> {
    let FetchArgs {
        identifiers,
        problem_id,
        lang,
        out_dir,
        overwrite,
    } = args;

    let (contest, task) = match identifiers.as_slice() {
        [task] => (default_contest(), task.clone()),
        [contest, task] => (contest.clone(), task.clone()),
        _ => bail!("expected 1 or 2 identifiers, got {}", identifiers.len()),
    };

    let problem_id = problem_id.unwrap_or_else(|| format!("{}_{}", contest, task));
    let mut url = format!(
        "https://atcoder.jp/contests/{}/tasks/{}",
        contest, problem_id
    );
    if let Some(lang) = lang.as_deref() {
        if !lang.is_empty() {
            url.push_str(&format!("?lang={lang}"));
        }
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("atcoder-rust-template xtask")
        .build()
        .context("failed to build HTTP client")?;

    let body = client
        .get(&url)
        .send()
        .with_context(|| format!("failed to download {url}"))?
        .error_for_status()
        .with_context(|| format!("server returned error for {url}"))?
        .text()
        .context("failed to read response body")?;

    let samples = parse_samples(&body)?;
    if samples.is_empty() {
        bail!("no samples found on the page");
    }

    let project_root = project_root();
    let task_dir = task.to_ascii_lowercase();
    let out_dir = project_root.join(out_dir).join(task_dir);
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create directory {}", out_dir.display()))?;

    let mut written = 0usize;
    for (index, sample) in samples {
        let file_stem = format!("{index:03}");
        let input_path = out_dir.join(format!("{file_stem}.in"));
        let output_path = out_dir.join(format!("{file_stem}.out"));

        if !overwrite && input_path.exists() && output_path.exists() {
            println!(
                "skip existing sample {} (use --overwrite to replace)",
                file_stem
            );
            continue;
        }

        fs::write(&input_path, &sample.input)
            .with_context(|| format!("failed to write {}", input_path.display()))?;
        fs::write(&output_path, &sample.output)
            .with_context(|| format!("failed to write {}", output_path.display()))?;
        println!(
            "wrote samples: {} and {}",
            rel_path(&input_path)?,
            rel_path(&output_path)?
        );
        written += 1;
    }

    if written == 0 {
        println!("no new samples were written");
    }

    Ok(())
}

fn default_contest() -> String {
    env::current_dir()
        .ok()
        .and_then(|path| path.file_name().map(|s| s.to_os_string()))
        .and_then(|os_str| os_str.into_string().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "contest".to_string())
}

fn run_case(args: RunArgs) -> Result<()> {
    let project_root = project_root();
    let bin = args.bin;
    let case = args.case.unwrap_or_else(|| "001".to_string());
    let tests_dir = project_root.join(args.tests_dir);
    let dir = tests_dir.join(&bin);

    let candidates = candidate_inputs(&dir, &case);
    let input_path = candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| anyhow!("input for case '{case}' not found in {}", dir.display()))?;

    let input = fs::read(&input_path)
        .with_context(|| format!("failed to read {}", input_path.display()))?;

    println!("cargo run --bin {bin} < {}", rel_path(&input_path)?);
    let mut command = Command::new("cargo");
    command.arg("run").arg("--bin").arg(&bin);
    if args.release {
        command.arg("--release");
    }
    command.current_dir(&project_root);
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    command.stdin(Stdio::piped());

    let mut child = command.spawn().context("failed to spawn cargo run")?;
    child
        .stdin
        .as_mut()
        .context("failed to open stdin of cargo run")?
        .write_all(&input)
        .context("failed to pipe input to cargo run")?;
    let status = child.wait().context("failed to wait on cargo run")?;

    if !status.success() {
        bail!("cargo run exited with status {status}");
    }
    Ok(())
}

fn run_tests(args: TestArgs) -> Result<()> {
    let project_root = project_root();
    let package = package_name()?;
    let mut command = Command::new("cargo");
    command.arg("test");
    command.arg("-p").arg(&package);

    let mut filter_args: Vec<String> = Vec::new();
    if let Some(target) = args.target {
        let normalized = target.to_ascii_lowercase();
        let (test_name, task_slug) = if normalized.ends_with("_test") {
            (
                normalized.clone(),
                normalized.trim_end_matches("_test").to_string(),
            )
        } else {
            (format!("{normalized}_test"), normalized.clone())
        };
        command.arg("--test").arg(&test_name);
        filter_args.push("--exact".to_string());
        filter_args.push(format!("{task_slug}_all_cases"));
    }

    if args.release {
        command.arg("--release");
    }
    if !filter_args.is_empty() {
        command.arg("--");
        for arg in filter_args {
            command.arg(arg);
        }
    }

    command.current_dir(&project_root);
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    let status = command.status().context("failed to run cargo test")?;
    if !status.success() {
        bail!("cargo test exited with status {status}");
    }
    Ok(())
}

fn candidate_inputs(dir: &Path, case: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    paths.push(dir.join(format!("{case}.in")));
    if case.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(value) = case.parse::<usize>() {
            paths.push(dir.join(format!("{value:03}.in")));
        }
    }
    paths
}

fn parse_samples(html: &str) -> Result<BTreeMap<usize, SamplePair>> {
    let document = Html::parse_document(html);
    let section_selector = Selector::parse("section").unwrap();
    let heading_selector = Selector::parse("h3").unwrap();
    let pre_selector = Selector::parse("pre").unwrap();

    let index_regex = Regex::new(r"(\d+)(?:\s*)$").unwrap();

    let mut inputs: BTreeMap<usize, String> = BTreeMap::new();
    let mut outputs: BTreeMap<usize, String> = BTreeMap::new();

    for section in document.select(&section_selector) {
        let Some(heading) = section.select(&heading_selector).next() else {
            continue;
        };
        let title = heading.text().collect::<String>().trim().to_string();
        let Some(kind) = classify_heading(&title) else {
            continue;
        };
        let Some(pre) = section.select(&pre_selector).next() else {
            continue;
        };
        let content = normalize_pre(&pre.text().collect::<String>());
        let Some(captures) = index_regex.captures(&title) else {
            continue;
        };
        let index: usize = captures[1].parse().unwrap_or(0);
        if index == 0 {
            continue;
        }
        match kind {
            SampleKind::Input => {
                inputs.insert(index, ensure_trailing_newline(content));
            }
            SampleKind::Output => {
                outputs.insert(index, ensure_trailing_newline(content));
            }
        }
    }

    let mut samples = BTreeMap::new();
    for (index, input) in inputs {
        if let Some(output) = outputs.get(&index).cloned() {
            samples.insert(index, SamplePair { input, output });
        }
    }

    Ok(samples)
}

fn classify_heading(title: &str) -> Option<SampleKind> {
    let title = title.trim();
    if title.contains("Sample Input") || title.contains("入力例") {
        Some(SampleKind::Input)
    } else if title.contains("Sample Output") || title.contains("出力例") {
        Some(SampleKind::Output)
    } else {
        None
    }
}

fn normalize_pre(raw: &str) -> String {
    raw.replace("\r\n", "\n")
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rel_path(path: &Path) -> Result<String> {
    let root = project_root();
    let rel = path.strip_prefix(&root).unwrap_or(path);
    Ok(rel.display().to_string())
}

fn package_name() -> Result<String> {
    let manifest_path = project_root().join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;

    let mut in_package = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim();
                if let Some(stripped) = rest.strip_prefix('"') {
                    if let Some(end) = stripped.find('"') {
                        return Ok(stripped[..end].to_string());
                    }
                }
            }
        }
    }

    bail!("package name not found in Cargo.toml");
}

#[derive(Debug, Clone)]
struct SamplePair {
    input: String,
    output: String,
}

#[derive(Debug, Clone, Copy)]
enum SampleKind {
    Input,
    Output,
}
