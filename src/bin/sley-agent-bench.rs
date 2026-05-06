use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::Value as JsonValue;

const AGENT_BENCH_SCHEMA: &str = "sley.agent_bench.report.v0";
const UNUSED_PRIVATE_TASK_CASE: &str = "unused-private-task-repair";

#[derive(Debug, Parser)]
#[command(name = "sley-agent-bench")]
#[command(about = "Run deterministic Sley agent-edit loop benchmarks")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run one or more deterministic benchmark cases.
    Run(RunArgs),
}

#[derive(Debug, Args)]
struct RunArgs {
    #[arg(long)]
    json: bool,
    #[arg(long = "case", value_name = "NAME")]
    case: Vec<String>,
    #[arg(long)]
    keep_workdir: bool,
    #[arg(long = "sley-bin", value_name = "PATH")]
    sley_bin: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct AgentBenchReport {
    schema: String,
    status: String,
    sley_bin: String,
    case_count: usize,
    passed_count: usize,
    failed_count: usize,
    cases: Vec<AgentBenchCase>,
    issues: Vec<AgentBenchIssue>,
}

#[derive(Debug, Serialize)]
struct AgentBenchCase {
    name: String,
    status: String,
    workdir: String,
    workdir_retained: bool,
    source_path: String,
    trace_path: String,
    trace_receipt_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_repair: Option<SelectedRepair>,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<AgentBenchEvidence>,
    steps: Vec<AgentBenchStep>,
    issues: Vec<AgentBenchIssue>,
}

#[derive(Debug, Serialize)]
struct SelectedRepair {
    kind: String,
    surface: String,
}

#[derive(Debug, Serialize)]
struct AgentBenchEvidence {
    trace_digest: String,
    seal_digest: String,
    graph_digest: String,
    trace_receipt_count: usize,
}

#[derive(Debug, Serialize)]
struct AgentBenchStep {
    name: String,
    command: Vec<String>,
    expected_success: bool,
    actual_success: bool,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout_schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr_excerpt: Option<String>,
    issues: Vec<AgentBenchIssue>,
}

#[derive(Debug, Clone, Serialize)]
struct AgentBenchIssue {
    code: String,
    message: String,
}

struct StepRun {
    step: AgentBenchStep,
    stdout_json: Option<JsonValue>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (report, json) = match cli.command {
        Command::Run(args) => {
            let json = args.json;
            (run_bench(args), json)
        }
    };

    if json {
        print_json(&report)?;
    } else {
        print_human(&report);
    }

    if report.status == "failed" {
        std::process::exit(1);
    }
    Ok(())
}

fn run_bench(args: RunArgs) -> AgentBenchReport {
    let sley_bin = normalize_sley_bin(args.sley_bin.unwrap_or_else(default_sley_bin));
    let requested_cases = if args.case.is_empty() {
        known_cases()
            .iter()
            .map(|case| (*case).to_string())
            .collect::<Vec<_>>()
    } else {
        args.case
    };

    let mut issues = Vec::new();
    let mut cases = Vec::new();
    for case in requested_cases {
        match case.as_str() {
            UNUSED_PRIVATE_TASK_CASE => {
                cases.push(run_unused_private_task_case(&sley_bin, args.keep_workdir));
            }
            unknown => issues.push(issue(
                "AGENT_BENCH_UNKNOWN_CASE",
                format!("unknown benchmark case `{unknown}`"),
            )),
        }
    }

    let passed_count = cases.iter().filter(|case| case.status == "passed").count();
    let failed_count = cases.iter().filter(|case| case.status == "failed").count();
    let status = if issues.is_empty() && failed_count == 0 && !cases.is_empty() {
        "passed"
    } else {
        "failed"
    };

    AgentBenchReport {
        schema: AGENT_BENCH_SCHEMA.to_string(),
        status: status.to_string(),
        sley_bin: path_string(&sley_bin),
        case_count: cases.len(),
        passed_count,
        failed_count,
        cases,
        issues,
    }
}

fn run_unused_private_task_case(sley_bin: &Path, keep_workdir: bool) -> AgentBenchCase {
    let workdir = create_workdir(UNUSED_PRIVATE_TASK_CASE);
    let source_path = workdir.join("main.sley");
    let trace_path = workdir.join("trace.jsonl");
    let mut issues = Vec::new();
    let mut steps = Vec::new();
    let mut selected_repair = None;
    let mut evidence = None;

    if let Err(error) = fs::create_dir_all(&workdir) {
        issues.push(issue(
            "AGENT_BENCH_WORKDIR_CREATE_FAILED",
            format!("failed to create {}: {error}", workdir.display()),
        ));
    } else if let Err(error) = fs::write(&source_path, unused_private_task_source()) {
        issues.push(issue(
            "AGENT_BENCH_SOURCE_WRITE_FAILED",
            format!("failed to write {}: {error}", source_path.display()),
        ));
    } else {
        let source = path_string(&source_path);
        let trace = path_string(&trace_path);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "check_initial",
            vec!["check", "--json", &source],
            true,
            Some("sley.diagnostics.report.v0"),
            Some("ok"),
        );
        steps.push(run.step);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "query_tasks_initial",
            vec!["query", "--json", "--kind", "tasks", &source],
            true,
            Some("sley.query.report.v0"),
            None,
        );
        steps.push(run.step);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "lint_initial_findings",
            vec!["lint", "--json", "--deny-warnings", &source],
            false,
            Some("sley.lint.report.v0"),
            Some("findings"),
        );
        steps.push(run.step);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "plan_repair",
            vec!["plan", "--json", "--graft-templates", &source],
            true,
            Some("sley.edit_plan.report.v0"),
            Some("warnings"),
        );
        if let Some(plan_json) = &run.stdout_json {
            match select_repair(plan_json, "delete_unused_private_task") {
                Ok(repair) => selected_repair = Some(repair),
                Err(issue) => issues.push(issue),
            }
        } else {
            issues.push(issue(
                "AGENT_BENCH_PLAN_JSON_MISSING",
                "plan_repair did not produce parseable JSON for repair selection",
            ));
        }
        steps.push(run.step);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "fix_write",
            vec![
                "fix",
                "--json",
                "--kind",
                "delete_unused_private_task",
                "--write",
                "--trace",
                &trace,
                &source,
            ],
            true,
            Some("sley.graft.outcome.v0"),
            Some("accepted"),
        );
        steps.push(run.step);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "lint_after_fix",
            vec!["lint", "--json", "--deny-warnings", &source],
            true,
            Some("sley.lint.report.v0"),
            Some("ok"),
        );
        steps.push(run.step);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "verify_after_fix",
            vec!["verify", "--json", "--deny-warnings", &source],
            true,
            Some("sley.verify.report.v0"),
            Some("passed"),
        );
        steps.push(run.step);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "seal_after_fix",
            vec!["seal", "--json", "--trace", &trace, &source],
            true,
            Some("sley.trace.seal.v0"),
            None,
        );
        let seal_json = run.stdout_json.clone();
        steps.push(run.step);

        let run = run_sley_step(
            sley_bin,
            &workdir,
            "zjx_after_fix",
            vec!["zjx", "--json", "--trace", &trace, &source],
            true,
            Some("sley.zjx.envelope.v0"),
            None,
        );
        let zjx_json = run.stdout_json.clone();
        steps.push(run.step);

        let trace_receipt_count = count_trace_receipts(&trace_path, &mut issues);
        if trace_receipt_count == 0 {
            issues.push(issue(
                "AGENT_BENCH_TRACE_EMPTY",
                "fix_write did not leave a trace receipt",
            ));
        }
        evidence = build_evidence(seal_json.as_ref(), zjx_json.as_ref(), trace_receipt_count)
            .map_err(|issue| issues.push(issue))
            .ok();
    }

    let trace_receipt_count = count_trace_receipts(&trace_path, &mut Vec::new());
    let mut case = AgentBenchCase {
        name: UNUSED_PRIVATE_TASK_CASE.to_string(),
        status: case_status(&steps, &issues).to_string(),
        workdir: path_string(&workdir),
        workdir_retained: true,
        source_path: path_string(&source_path),
        trace_path: path_string(&trace_path),
        trace_receipt_count,
        selected_repair,
        evidence,
        steps,
        issues,
    };

    if case.status == "passed" && !keep_workdir {
        match fs::remove_dir_all(&workdir) {
            Ok(()) => case.workdir_retained = false,
            Err(error) => {
                case.workdir_retained = true;
                case.status = "failed".to_string();
                case.issues.push(issue(
                    "AGENT_BENCH_WORKDIR_CLEANUP_FAILED",
                    format!("failed to remove {}: {error}", workdir.display()),
                ));
            }
        }
    }

    case
}

fn run_sley_step(
    sley_bin: &Path,
    workdir: &Path,
    name: &str,
    args: Vec<&str>,
    expected_success: bool,
    expected_schema: Option<&str>,
    expected_status: Option<&str>,
) -> StepRun {
    let command = std::iter::once(path_string(sley_bin))
        .chain(args.iter().map(|arg| (*arg).to_string()))
        .collect::<Vec<_>>();
    let output = match ProcessCommand::new(sley_bin)
        .args(&args)
        .current_dir(workdir)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            return StepRun {
                step: AgentBenchStep {
                    name: name.to_string(),
                    command,
                    expected_success,
                    actual_success: false,
                    status: "failed".to_string(),
                    exit_code: None,
                    expected_schema: expected_schema.map(str::to_string),
                    stdout_schema: None,
                    expected_status: expected_status.map(str::to_string),
                    stdout_status: None,
                    stderr_excerpt: None,
                    issues: vec![issue(
                        "AGENT_BENCH_STEP_SPAWN_FAILED",
                        format!("failed to run step `{name}`: {error}"),
                    )],
                },
                stdout_json: None,
            };
        }
    };

    let actual_success = output.status.success();
    let exit_code = exit_code(output.status);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout_json = serde_json::from_str::<JsonValue>(&stdout).ok();
    let stdout_schema = stdout_json
        .as_ref()
        .and_then(|value| value.pointer("/schema"))
        .and_then(JsonValue::as_str)
        .map(str::to_string);
    let stdout_status = stdout_json
        .as_ref()
        .and_then(|value| value.pointer("/status"))
        .and_then(JsonValue::as_str)
        .map(str::to_string);
    let mut issues = Vec::new();

    if actual_success != expected_success {
        issues.push(issue(
            "AGENT_BENCH_STEP_EXIT_UNEXPECTED",
            format!("step `{name}` success was {actual_success}, expected {expected_success}"),
        ));
    }
    if expected_schema.is_some() && stdout_json.is_none() {
        issues.push(issue(
            "AGENT_BENCH_STEP_STDOUT_JSON_INVALID",
            format!(
                "step `{name}` did not emit parseable JSON stdout: {}",
                excerpt(&stdout).unwrap_or_else(|| "<empty stdout>".to_string())
            ),
        ));
    }
    if let Some(expected) = expected_schema
        && stdout_schema.as_deref() != Some(expected)
    {
        issues.push(issue(
            "AGENT_BENCH_STEP_SCHEMA_UNEXPECTED",
            format!(
                "step `{name}` schema was {:?}, expected {expected:?}",
                stdout_schema.as_deref()
            ),
        ));
    }
    if let Some(expected) = expected_status
        && stdout_status.as_deref() != Some(expected)
    {
        issues.push(issue(
            "AGENT_BENCH_STEP_STATUS_UNEXPECTED",
            format!(
                "step `{name}` report status was {:?}, expected {expected:?}",
                stdout_status.as_deref()
            ),
        ));
    }

    let status = if issues.is_empty() {
        "passed"
    } else {
        "failed"
    };

    StepRun {
        step: AgentBenchStep {
            name: name.to_string(),
            command,
            expected_success,
            actual_success,
            status: status.to_string(),
            exit_code,
            expected_schema: expected_schema.map(str::to_string),
            stdout_schema,
            expected_status: expected_status.map(str::to_string),
            stdout_status,
            stderr_excerpt: excerpt(&stderr),
            issues,
        },
        stdout_json,
    }
}

fn select_repair(
    plan_json: &JsonValue,
    kind: &str,
) -> std::result::Result<SelectedRepair, AgentBenchIssue> {
    let templates = plan_json
        .pointer("/graft_templates")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| {
            issue(
                "AGENT_BENCH_PLAN_TEMPLATES_MISSING",
                "plan JSON is missing /graft_templates",
            )
        })?;
    let mut matches = templates
        .iter()
        .filter(|template| template.pointer("/kind").and_then(JsonValue::as_str) == Some(kind));
    let Some(template) = matches.next() else {
        return Err(issue(
            "AGENT_BENCH_REPAIR_NOT_FOUND",
            format!("plan did not offer graft template `{kind}`"),
        ));
    };
    if matches.next().is_some() {
        return Err(issue(
            "AGENT_BENCH_REPAIR_AMBIGUOUS",
            format!("plan offered multiple graft templates named `{kind}`"),
        ));
    }
    let surface = template
        .pointer("/surface")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            issue(
                "AGENT_BENCH_REPAIR_SURFACE_MISSING",
                format!("graft template `{kind}` is missing /surface"),
            )
        })?;
    Ok(SelectedRepair {
        kind: kind.to_string(),
        surface: surface.to_string(),
    })
}

fn build_evidence(
    seal_json: Option<&JsonValue>,
    zjx_json: Option<&JsonValue>,
    trace_receipt_count: usize,
) -> std::result::Result<AgentBenchEvidence, AgentBenchIssue> {
    let seal = seal_json.ok_or_else(|| {
        issue(
            "AGENT_BENCH_SEAL_JSON_MISSING",
            "seal_after_fix did not produce parseable JSON",
        )
    })?;
    let zjx = zjx_json.ok_or_else(|| {
        issue(
            "AGENT_BENCH_ZJX_JSON_MISSING",
            "zjx_after_fix did not produce parseable JSON",
        )
    })?;
    Ok(AgentBenchEvidence {
        trace_digest: json_string(seal, "/trace_digest")?,
        seal_digest: json_string(seal, "/seal_digest")?,
        graph_digest: json_string(zjx, "/graph_digest")?,
        trace_receipt_count,
    })
}

fn json_string(value: &JsonValue, pointer: &str) -> std::result::Result<String, AgentBenchIssue> {
    value
        .pointer(pointer)
        .and_then(JsonValue::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            issue(
                "AGENT_BENCH_EVIDENCE_FIELD_MISSING",
                format!("evidence JSON is missing string field {pointer}"),
            )
        })
}

fn count_trace_receipts(path: &Path, issues: &mut Vec<AgentBenchIssue>) -> usize {
    match fs::read_to_string(path) {
        Ok(source) => source
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        Err(error) if path.exists() => {
            issues.push(issue(
                "AGENT_BENCH_TRACE_READ_FAILED",
                format!("failed to read {}: {error}", path.display()),
            ));
            0
        }
        Err(_) => 0,
    }
}

fn case_status(steps: &[AgentBenchStep], issues: &[AgentBenchIssue]) -> &'static str {
    if issues.is_empty() && !steps.is_empty() && steps.iter().all(|step| step.status == "passed") {
        "passed"
    } else {
        "failed"
    }
}

fn create_workdir(case_name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!(
        "sley-agent-bench-{case_name}-{}-{timestamp}",
        std::process::id()
    ))
}

fn unused_private_task_source() -> &'static str {
    r#"module app.bench

task main -> Text {
  return "ready"
}

task orphan -> Text {
  return "unused"
}
"#
}

fn known_cases() -> &'static [&'static str] {
    &[UNUSED_PRIVATE_TASK_CASE]
}

fn default_sley_bin() -> PathBuf {
    let exe_name = if cfg!(windows) { "sley.exe" } else { "sley" };
    if let Ok(current_exe) = env::current_exe()
        && let Some(parent) = current_exe.parent()
    {
        let sibling = parent.join(exe_name);
        if sibling.exists() {
            return sibling;
        }
    }
    PathBuf::from(exe_name)
}

fn normalize_sley_bin(path: PathBuf) -> PathBuf {
    if path.is_absolute() || path.components().count() == 1 {
        return path;
    }
    fs::canonicalize(&path).unwrap_or(path)
}

fn exit_code(status: ExitStatus) -> Option<i32> {
    status.code()
}

fn excerpt(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut excerpt = trimmed.chars().take(400).collect::<String>();
    if trimmed.chars().count() > 400 {
        excerpt.push_str("...");
    }
    Some(excerpt)
}

fn issue(code: impl Into<String>, message: impl Into<String>) -> AgentBenchIssue {
    AgentBenchIssue {
        code: code.into(),
        message: message.into(),
    }
}

fn print_json(report: &AgentBenchReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_human(report: &AgentBenchReport) {
    println!(
        "sley-agent-bench status={} cases={} passed={} failed={}",
        report.status, report.case_count, report.passed_count, report.failed_count
    );
    for case in &report.cases {
        let repair = case
            .selected_repair
            .as_ref()
            .map(|repair| format!(" repair={}", repair.kind))
            .unwrap_or_default();
        println!(
            "{} status={} steps={} traces={} retained={}{}",
            case.name,
            case.status,
            case.steps.len(),
            case.trace_receipt_count,
            case.workdir_retained,
            repair
        );
        for issue in &case.issues {
            eprintln!("{}: {}", issue.code, issue.message);
        }
        for step in &case.steps {
            for issue in &step.issues {
                eprintln!("{} {}: {}", step.name, issue.code, issue.message);
            }
        }
    }
    for issue in &report.issues {
        eprintln!("{}: {}", issue.code, issue.message);
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
