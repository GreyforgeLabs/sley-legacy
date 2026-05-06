use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::Value as JsonValue;
use sley::symbols::SymbolGraph;
use sley::trace::content_digest;

const ZJX_TOOL_SCHEMA: &str = "sley.zjx.tool.report.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-zjx")]
#[command(about = "Inspect Sley ZJX preview envelopes without writing source files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate envelope shape, preview scope, and graph digest.
    Validate {
        envelope: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Summarize an envelope and recompute its graph digest.
    Inspect {
        envelope: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Recompute the graph digest and fail if it differs from the envelope.
    VerifyDigest {
        envelope: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Extract the embedded symbol graph to stdout or an output path.
    ExtractGraph {
        envelope: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Compare two envelopes by metadata, graph digest, modules, and task IDs.
    DiffEnvelope {
        left: PathBuf,
        right: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct ZjxToolReport {
    schema: String,
    status: String,
    command: String,
    envelopes: Vec<EnvelopeSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    digest: Option<DigestCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<EnvelopeDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graph: Option<JsonValue>,
    issues: Vec<ZjxToolIssue>,
}

#[derive(Debug, Clone, Serialize)]
struct EnvelopeSummary {
    path: String,
    target: String,
    format: String,
    compression: String,
    graph_digest: String,
    computed_graph_digest: String,
    graph_digest_match: bool,
    module_count: usize,
    import_count: usize,
    type_count: usize,
    effect_count: usize,
    task_count: usize,
    trace_receipt_count: usize,
    slice_present: bool,
}

#[derive(Debug, Serialize)]
struct DigestCheck {
    declared: String,
    computed: String,
    matches: bool,
}

#[derive(Debug, Serialize)]
struct EnvelopeDiff {
    left_path: String,
    right_path: String,
    changed: bool,
    target_changed: bool,
    graph_digest_changed: bool,
    module_count_delta: isize,
    task_count_delta: isize,
    trace_receipt_count_delta: isize,
    modules_added: Vec<String>,
    modules_removed: Vec<String>,
    tasks_added: Vec<String>,
    tasks_removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ZjxToolIssue {
    code: String,
    message: String,
}

#[derive(Debug, Clone)]
struct ParsedEnvelope {
    summary: EnvelopeSummary,
    graph: SymbolGraph,
    graph_value: JsonValue,
    issues: Vec<ZjxToolIssue>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (report, json) = match cli.command {
        Command::Validate { envelope, json } => (validate_report(&envelope), json),
        Command::Inspect { envelope, json } => (inspect_report(&envelope), json),
        Command::VerifyDigest { envelope, json } => (verify_digest_report(&envelope), json),
        Command::ExtractGraph {
            envelope,
            output,
            json,
        } => (extract_graph_report(&envelope, output.as_deref()), json),
        Command::DiffEnvelope { left, right, json } => (diff_envelope_report(&left, &right), json),
    };

    if json {
        print_json(&report)?;
    } else {
        print_human(&report)?;
    }
    if report.status == "failed" {
        std::process::exit(1);
    }
    Ok(())
}

fn validate_report(path: &Path) -> ZjxToolReport {
    envelope_check_report("validate", path)
}

fn inspect_report(path: &Path) -> ZjxToolReport {
    envelope_check_report("inspect", path)
}

fn envelope_check_report(command: &str, path: &Path) -> ZjxToolReport {
    let parsed = read_envelope(path);
    let mut issues = parsed
        .as_ref()
        .map(|parsed| parsed.issues.clone())
        .unwrap_or_else(|issues| issues.clone());
    let envelopes = parsed
        .as_ref()
        .ok()
        .map(|parsed| vec![parsed.summary.clone()])
        .unwrap_or_default();
    let digest = envelopes.first().map(|summary| DigestCheck {
        declared: summary.graph_digest.clone(),
        computed: summary.computed_graph_digest.clone(),
        matches: summary.graph_digest_match,
    });
    if digest.as_ref().is_some_and(|digest| !digest.matches) {
        issues.push(issue(
            "ZJX_GRAPH_DIGEST_MISMATCH",
            "declared graph_digest does not match the recomputed graph digest",
        ));
    }
    tool_report(command, envelopes, digest, None, None, None, issues)
}

fn verify_digest_report(path: &Path) -> ZjxToolReport {
    let parsed = read_envelope(path);
    let mut issues = parsed
        .as_ref()
        .map(|parsed| parsed.issues.clone())
        .unwrap_or_else(|issues| issues.clone());
    let envelopes = parsed
        .as_ref()
        .ok()
        .map(|parsed| vec![parsed.summary.clone()])
        .unwrap_or_default();
    let digest = envelopes.first().map(|summary| DigestCheck {
        declared: summary.graph_digest.clone(),
        computed: summary.computed_graph_digest.clone(),
        matches: summary.graph_digest_match,
    });
    if digest.as_ref().is_some_and(|digest| !digest.matches) {
        issues.push(issue(
            "ZJX_GRAPH_DIGEST_MISMATCH",
            "declared graph_digest does not match the recomputed graph digest",
        ));
    }
    tool_report("verify-digest", envelopes, digest, None, None, None, issues)
}

fn extract_graph_report(path: &Path, output: Option<&Path>) -> ZjxToolReport {
    let parsed = read_envelope(path);
    let mut issues = parsed
        .as_ref()
        .map(|parsed| parsed.issues.clone())
        .unwrap_or_else(|issues| issues.clone());
    let envelopes = parsed
        .as_ref()
        .ok()
        .map(|parsed| vec![parsed.summary.clone()])
        .unwrap_or_default();
    let graph = parsed
        .as_ref()
        .ok()
        .map(|parsed| parsed.graph_value.clone());
    let mut output_path = None;
    if let (Some(graph), Some(output)) = (graph.as_ref(), output) {
        match write_pretty_json(output, graph) {
            Ok(()) => output_path = Some(path_string(output)),
            Err(error) => issues.push(issue(
                "ZJX_GRAPH_WRITE_FAILED",
                format!("failed to write graph output {}: {error}", output.display()),
            )),
        }
    }
    tool_report(
        "extract-graph",
        envelopes,
        None,
        None,
        output_path,
        graph,
        issues,
    )
}

fn diff_envelope_report(left: &Path, right: &Path) -> ZjxToolReport {
    let left_parsed = read_envelope(left);
    let right_parsed = read_envelope(right);
    let mut issues = Vec::new();
    let mut envelopes = Vec::new();
    if let Ok(parsed) = &left_parsed {
        issues.extend(parsed.issues.clone());
        envelopes.push(parsed.summary.clone());
    } else if let Err(left_issues) = left_parsed {
        issues.extend(left_issues);
    }
    if let Ok(parsed) = &right_parsed {
        issues.extend(parsed.issues.clone());
        envelopes.push(parsed.summary.clone());
    } else if let Err(right_issues) = right_parsed {
        issues.extend(right_issues);
    }

    let diff = if envelopes.len() == 2 {
        Some(build_diff(&envelopes[0], &envelopes[1], &left, &right))
    } else {
        None
    };

    tool_report("diff-envelope", envelopes, None, diff, None, None, issues)
}

fn build_diff(
    left_summary: &EnvelopeSummary,
    right_summary: &EnvelopeSummary,
    left_path: &Path,
    right_path: &Path,
) -> EnvelopeDiff {
    let left = read_envelope(left_path).expect("left envelope was already parsed");
    let right = read_envelope(right_path).expect("right envelope was already parsed");
    let left_modules = module_set(&left.graph);
    let right_modules = module_set(&right.graph);
    let left_tasks = task_set(&left.graph);
    let right_tasks = task_set(&right.graph);
    let modules_added = right_modules
        .difference(&left_modules)
        .cloned()
        .collect::<Vec<_>>();
    let modules_removed = left_modules
        .difference(&right_modules)
        .cloned()
        .collect::<Vec<_>>();
    let tasks_added = right_tasks
        .difference(&left_tasks)
        .cloned()
        .collect::<Vec<_>>();
    let tasks_removed = left_tasks
        .difference(&right_tasks)
        .cloned()
        .collect::<Vec<_>>();
    let target_changed = left_summary.target != right_summary.target;
    let graph_digest_changed = left_summary.graph_digest != right_summary.graph_digest;
    let changed = target_changed
        || graph_digest_changed
        || !modules_added.is_empty()
        || !modules_removed.is_empty()
        || !tasks_added.is_empty()
        || !tasks_removed.is_empty()
        || left_summary.trace_receipt_count != right_summary.trace_receipt_count;

    EnvelopeDiff {
        left_path: path_string(left_path),
        right_path: path_string(right_path),
        changed,
        target_changed,
        graph_digest_changed,
        module_count_delta: right_summary.module_count as isize
            - left_summary.module_count as isize,
        task_count_delta: right_summary.task_count as isize - left_summary.task_count as isize,
        trace_receipt_count_delta: right_summary.trace_receipt_count as isize
            - left_summary.trace_receipt_count as isize,
        modules_added,
        modules_removed,
        tasks_added,
        tasks_removed,
    }
}

fn read_envelope(path: &Path) -> std::result::Result<ParsedEnvelope, Vec<ZjxToolIssue>> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            return Err(vec![issue(
                "ZJX_READ_FAILED",
                format!("failed to read {}: {error}", path.display()),
            )]);
        }
    };
    let value = match serde_json::from_str::<JsonValue>(&source) {
        Ok(value) => value,
        Err(error) => {
            return Err(vec![issue(
                "ZJX_PARSE_FAILED",
                format!("failed to parse {} as JSON: {error}", path.display()),
            )]);
        }
    };

    let mut issues = Vec::new();
    expect_string(
        &value,
        "/schema",
        "sley.zjx.envelope.v0",
        "ZJX_SCHEMA_MISMATCH",
        &mut issues,
    );
    expect_string(
        &value,
        "/format",
        "zjx-preview-json",
        "ZJX_FORMAT_MISMATCH",
        &mut issues,
    );
    expect_string(
        &value,
        "/compression",
        "none",
        "ZJX_COMPRESSION_MISMATCH",
        &mut issues,
    );

    let Some(graph_value) = value.get("graph").cloned() else {
        issues.push(issue("ZJX_GRAPH_MISSING", "envelope is missing /graph"));
        return Ok(ParsedEnvelope {
            summary: missing_graph_summary(path, &value),
            graph: empty_graph(),
            graph_value: JsonValue::Null,
            issues,
        });
    };
    let graph = match serde_json::from_value::<SymbolGraph>(graph_value.clone()) {
        Ok(graph) => graph,
        Err(error) => {
            issues.push(issue(
                "ZJX_GRAPH_PARSE_FAILED",
                format!("failed to parse /graph as sley.symbol_graph.v0: {error}"),
            ));
            empty_graph()
        }
    };
    let computed_graph_digest = content_digest(
        &serde_json::to_vec(&graph).expect("serialize parsed symbol graph for digest check"),
    );
    let graph_digest = value
        .get("graph_digest")
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_string();

    Ok(ParsedEnvelope {
        summary: EnvelopeSummary {
            path: path_string(path),
            target: string_value(&value, "target"),
            format: string_value(&value, "format"),
            compression: string_value(&value, "compression"),
            graph_digest: graph_digest.clone(),
            computed_graph_digest: computed_graph_digest.clone(),
            graph_digest_match: graph_digest == computed_graph_digest,
            module_count: graph.modules.len(),
            import_count: graph
                .modules
                .iter()
                .map(|module| module.imports.len())
                .sum(),
            type_count: graph.modules.iter().map(|module| module.types.len()).sum(),
            effect_count: graph
                .modules
                .iter()
                .map(|module| module.effects.len())
                .sum(),
            task_count: graph.modules.iter().map(|module| module.tasks.len()).sum(),
            trace_receipt_count: value
                .get("trace_receipts")
                .and_then(JsonValue::as_array)
                .map_or(0, Vec::len),
            slice_present: value.get("slice").is_some_and(|slice| !slice.is_null()),
        },
        graph,
        graph_value,
        issues,
    })
}

fn tool_report(
    command: &str,
    envelopes: Vec<EnvelopeSummary>,
    digest: Option<DigestCheck>,
    diff: Option<EnvelopeDiff>,
    output_path: Option<String>,
    graph: Option<JsonValue>,
    issues: Vec<ZjxToolIssue>,
) -> ZjxToolReport {
    ZjxToolReport {
        schema: ZJX_TOOL_SCHEMA.to_string(),
        status: if issues.is_empty() {
            "passed"
        } else {
            "failed"
        }
        .to_string(),
        command: command.to_string(),
        envelopes,
        digest,
        diff,
        output_path,
        graph,
        issues,
    }
}

fn missing_graph_summary(path: &Path, value: &JsonValue) -> EnvelopeSummary {
    EnvelopeSummary {
        path: path_string(path),
        target: string_value(value, "target"),
        format: string_value(value, "format"),
        compression: string_value(value, "compression"),
        graph_digest: string_value(value, "graph_digest"),
        computed_graph_digest: String::new(),
        graph_digest_match: false,
        module_count: 0,
        import_count: 0,
        type_count: 0,
        effect_count: 0,
        task_count: 0,
        trace_receipt_count: value
            .get("trace_receipts")
            .and_then(JsonValue::as_array)
            .map_or(0, Vec::len),
        slice_present: value.get("slice").is_some_and(|slice| !slice.is_null()),
    }
}

fn empty_graph() -> SymbolGraph {
    SymbolGraph {
        schema: "sley.symbol_graph.v0".to_string(),
        entry_module: String::new(),
        modules: Vec::new(),
    }
}

fn expect_string(
    value: &JsonValue,
    pointer: &str,
    expected: &str,
    code: &str,
    issues: &mut Vec<ZjxToolIssue>,
) {
    let actual = value.pointer(pointer).and_then(JsonValue::as_str);
    if actual != Some(expected) {
        issues.push(issue(
            code,
            format!("{pointer} was {actual:?}, expected {expected:?}"),
        ));
    }
}

fn string_value(value: &JsonValue, key: &str) -> String {
    value
        .get(key)
        .and_then(JsonValue::as_str)
        .unwrap_or("")
        .to_string()
}

fn module_set(graph: &SymbolGraph) -> BTreeSet<String> {
    graph
        .modules
        .iter()
        .map(|module| module.module.clone())
        .collect()
}

fn task_set(graph: &SymbolGraph) -> BTreeSet<String> {
    graph
        .modules
        .iter()
        .flat_map(|module| module.tasks.iter().map(|task| task.id.clone()))
        .collect()
}

fn issue(code: impl Into<String>, message: impl Into<String>) -> ZjxToolIssue {
    ZjxToolIssue {
        code: code.into(),
        message: message.into(),
    }
}

fn write_pretty_json(path: &Path, value: &JsonValue) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let source = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{source}\n"))
        .with_context(|| format!("failed to write {}", path.display()))
}

fn print_json(report: &ZjxToolReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_human(report: &ZjxToolReport) -> Result<()> {
    if report.command == "extract-graph"
        && report.status == "passed"
        && report.output_path.is_none()
        && let Some(graph) = &report.graph
    {
        println!("{}", serde_json::to_string_pretty(graph)?);
        return Ok(());
    }
    println!(
        "sley-zjx {} status={} envelopes={} issues={}",
        report.command,
        report.status,
        report.envelopes.len(),
        report.issues.len()
    );
    for envelope in &report.envelopes {
        println!(
            "{} target={} digest_match={} modules={} tasks={} traces={}",
            envelope.path,
            envelope.target,
            envelope.graph_digest_match,
            envelope.module_count,
            envelope.task_count,
            envelope.trace_receipt_count
        );
    }
    if let Some(diff) = &report.diff {
        println!(
            "diff changed={} modules_added={} modules_removed={} tasks_added={} tasks_removed={}",
            diff.changed,
            diff.modules_added.len(),
            diff.modules_removed.len(),
            diff.tasks_added.len(),
            diff.tasks_removed.len()
        );
    }
    for issue in &report.issues {
        eprintln!("{}: {}", issue.code, issue.message);
    }
    Ok(())
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
