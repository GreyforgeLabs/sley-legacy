use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

const CI_REPORT_SCHEMA: &str = "sley.ci.report.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-ci")]
#[command(about = "Local Sley CI and smoke gate wrapper")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run check plus deny-warning lint over one target.
    Check {
        #[arg(long)]
        json: bool,
        target: PathBuf,
    },
    /// Run the Sley verify gate over one target.
    Verify {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        deny_warnings: bool,
        #[arg(long = "cap", value_name = "EFFECT[=ROOT]")]
        cap: Vec<String>,
        #[arg(long = "db-table", value_name = "TABLE=JSON")]
        db_table: Vec<String>,
        #[arg(long = "secret", value_names = ["NAME", "TEXT"], num_args = 2)]
        secret: Vec<String>,
        #[arg(long = "deploy-result", value_names = ["TARGET", "TEXT"], num_args = 2)]
        deploy_result: Vec<String>,
        #[arg(long = "spend-result", value_names = ["REQUEST", "TEXT"], num_args = 2)]
        spend_result: Vec<String>,
        #[arg(long = "http-text", value_names = ["URL", "TEXT"], num_args = 2)]
        http_text: Vec<String>,
        #[arg(long = "shell-output", value_names = ["COMMAND", "TEXT"], num_args = 2)]
        shell_output: Vec<String>,
        #[arg(long = "model-output", value_names = ["PROMPT", "TEXT"], num_args = 2)]
        model_output: Vec<String>,
        target: PathBuf,
    },
    /// Run the Sley local deploy dry-run gate over one target.
    Deploy {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long = "artifacts-dir")]
        artifacts_dir: Option<PathBuf>,
        #[arg(long, default_value = "staging")]
        environment: String,
        #[arg(long = "cap", value_name = "EFFECT[=ROOT]")]
        cap: Vec<String>,
        #[arg(long = "db-table", value_name = "TABLE=JSON")]
        db_table: Vec<String>,
        #[arg(long = "secret", value_names = ["NAME", "TEXT"], num_args = 2)]
        secret: Vec<String>,
        #[arg(long = "deploy-result", value_names = ["TARGET", "TEXT"], num_args = 2)]
        deploy_result: Vec<String>,
        #[arg(long = "spend-result", value_names = ["REQUEST", "TEXT"], num_args = 2)]
        spend_result: Vec<String>,
        #[arg(long = "http-text", value_names = ["URL", "TEXT"], num_args = 2)]
        http_text: Vec<String>,
        #[arg(long = "shell-output", value_names = ["COMMAND", "TEXT"], num_args = 2)]
        shell_output: Vec<String>,
        #[arg(long = "model-output", value_names = ["PROMPT", "TEXT"], num_args = 2)]
        model_output: Vec<String>,
        target: PathBuf,
    },
    /// Run a Sley CLI smoke manifest in an isolated temp directory.
    Smoke {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        repo_root: Option<PathBuf>,
        manifest: PathBuf,
    },
}

#[derive(Debug, Serialize)]
struct CiReport {
    schema: String,
    status: String,
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<String>,
    summary: CiSummary,
    steps: Vec<CiStep>,
    issues: Vec<CiIssue>,
}

#[derive(Debug, Serialize)]
struct CiSummary {
    step_count: usize,
    passed_count: usize,
    failed_count: usize,
}

#[derive(Debug, Serialize)]
struct CiStep {
    name: String,
    status: String,
    command: Vec<String>,
    expected_success: bool,
    actual_success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout_schema: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    covers: Vec<String>,
    issues: Vec<CiIssue>,
}

#[derive(Debug, Serialize)]
struct CiIssue {
    code: String,
    message: String,
}

#[derive(Debug)]
struct StepRun {
    step: CiStep,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SmokeManifest {
    schema: String,
    cases: Vec<SmokeCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SmokeCase {
    name: String,
    #[serde(default)]
    cwd: SmokeCwd,
    #[serde(default)]
    setup_files: Vec<SmokeSetupFile>,
    args: Vec<String>,
    covers: Vec<String>,
    expect: SmokeExpectation,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SmokeCwd {
    #[default]
    Repo,
    Tmp,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SmokeSetupFile {
    path: String,
    content: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SmokeExpectation {
    success: bool,
    #[serde(default)]
    stdout_contains: Vec<String>,
    #[serde(default)]
    stderr_contains: Vec<String>,
    #[serde(default)]
    stdout_json: Vec<JsonExpectation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonExpectation {
    pointer: String,
    value: JsonValue,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (report, json) = run(cli)?;
    let failed = report.status == "failed";
    emit_report(&report, json)?;
    if failed {
        std::process::exit(1);
    }
    Ok(())
}

fn run(cli: Cli) -> Result<(CiReport, bool)> {
    let sley_bin = find_sley_binary();
    match cli.command {
        Command::Check { json, target } => {
            let report = build_check_report(&sley_bin, &target)?;
            Ok((report, json))
        }
        Command::Verify {
            json,
            deny_warnings,
            cap,
            db_table,
            secret,
            deploy_result,
            spend_result,
            http_text,
            shell_output,
            model_output,
            target,
        } => {
            let runtime = RuntimeArgs {
                cap,
                db_table,
                secret,
                deploy_result,
                spend_result,
                http_text,
                shell_output,
                model_output,
            };
            let report = build_verify_report(&sley_bin, &target, deny_warnings, &runtime)?;
            Ok((report, json))
        }
        Command::Deploy {
            json,
            dry_run: _dry_run,
            artifacts_dir,
            environment,
            cap,
            db_table,
            secret,
            deploy_result,
            spend_result,
            http_text,
            shell_output,
            model_output,
            target,
        } => {
            let runtime = RuntimeArgs {
                cap,
                db_table,
                secret,
                deploy_result,
                spend_result,
                http_text,
                shell_output,
                model_output,
            };
            let report = build_deploy_report(
                &sley_bin,
                &target,
                &environment,
                artifacts_dir.as_deref(),
                &runtime,
            )?;
            Ok((report, json))
        }
        Command::Smoke {
            json,
            repo_root,
            manifest,
        } => {
            let repo_root = repo_root.unwrap_or(env::current_dir()?);
            let report = build_smoke_report(&sley_bin, &repo_root, &manifest);
            Ok((report, json))
        }
    }
}

#[derive(Debug)]
struct RuntimeArgs {
    cap: Vec<String>,
    db_table: Vec<String>,
    secret: Vec<String>,
    deploy_result: Vec<String>,
    spend_result: Vec<String>,
    http_text: Vec<String>,
    shell_output: Vec<String>,
    model_output: Vec<String>,
}

fn build_check_report(sley_bin: &Path, target: &Path) -> Result<CiReport> {
    let cwd = env::current_dir()?;
    let target_arg = path_string(target);
    let mut steps = Vec::new();
    steps.push(
        run_sley_step(
            sley_bin,
            &cwd,
            "check",
            vec!["check".into(), "--json".into(), target_arg.clone()],
            true,
            Vec::new(),
        )
        .step,
    );
    steps.push(
        run_sley_step(
            sley_bin,
            &cwd,
            "lint_deny_warnings",
            vec![
                "lint".into(),
                "--json".into(),
                "--deny-warnings".into(),
                target_arg.clone(),
            ],
            true,
            Vec::new(),
        )
        .step,
    );
    Ok(finalize_report(
        "check",
        Some(path_string(target)),
        None,
        steps,
        Vec::new(),
    ))
}

fn build_verify_report(
    sley_bin: &Path,
    target: &Path,
    deny_warnings: bool,
    runtime: &RuntimeArgs,
) -> Result<CiReport> {
    let cwd = env::current_dir()?;
    let mut args = vec!["verify".into(), "--json".into()];
    if deny_warnings {
        args.push("--deny-warnings".into());
    }
    append_runtime_args(&mut args, runtime);
    args.push(path_string(target));
    let steps = vec![run_sley_step(sley_bin, &cwd, "verify", args, true, Vec::new()).step];
    Ok(finalize_report(
        "verify",
        Some(path_string(target)),
        None,
        steps,
        Vec::new(),
    ))
}

fn build_deploy_report(
    sley_bin: &Path,
    target: &Path,
    environment: &str,
    artifacts_dir: Option<&Path>,
    runtime: &RuntimeArgs,
) -> Result<CiReport> {
    let cwd = env::current_dir()?;
    let mut args = vec![
        "deploy".into(),
        "--json".into(),
        "--dry-run".into(),
        "--environment".into(),
        environment.into(),
    ];
    if let Some(artifacts_dir) = artifacts_dir {
        args.push("--artifacts-dir".into());
        args.push(path_string(artifacts_dir));
    }
    append_runtime_args(&mut args, runtime);
    args.push(path_string(target));
    let steps = vec![run_sley_step(sley_bin, &cwd, "deploy_dry_run", args, true, Vec::new()).step];
    Ok(finalize_report(
        "deploy",
        Some(path_string(target)),
        None,
        steps,
        Vec::new(),
    ))
}

fn build_smoke_report(sley_bin: &Path, repo_root: &Path, manifest_path: &Path) -> CiReport {
    let mut issues = Vec::new();
    let manifest = match read_smoke_manifest(manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            issues.push(issue(
                "manifest_read_failed",
                format!(
                    "failed to read smoke manifest {}: {error}",
                    manifest_path.display()
                ),
            ));
            return finalize_report(
                "smoke",
                None,
                Some(path_string(manifest_path)),
                Vec::new(),
                issues,
            );
        }
    };
    if manifest.schema != "sley.cli_smoke.manifest.v0" {
        issues.push(issue(
            "manifest_schema_mismatch",
            format!(
                "smoke manifest schema {:?} does not match {:?}",
                manifest.schema, "sley.cli_smoke.manifest.v0"
            ),
        ));
    }

    let tmp_root = temp_smoke_dir();
    let mut steps = Vec::new();
    if let Err(error) = fs::create_dir_all(&tmp_root) {
        issues.push(issue(
            "temp_dir_create_failed",
            format!("failed to create {}: {error}", tmp_root.display()),
        ));
    } else {
        for case in &manifest.cases {
            steps.push(run_smoke_case(sley_bin, repo_root, &tmp_root, case).step);
        }
    }
    let _ = fs::remove_dir_all(&tmp_root);

    finalize_report(
        "smoke",
        None,
        Some(path_string(manifest_path)),
        steps,
        issues,
    )
}

fn run_smoke_case(sley_bin: &Path, repo_root: &Path, tmp_root: &Path, case: &SmokeCase) -> StepRun {
    let args = case
        .args
        .iter()
        .map(|arg| expand_smoke_text(arg, repo_root, tmp_root))
        .collect::<Vec<_>>();
    let cwd = match case.cwd {
        SmokeCwd::Repo => repo_root,
        SmokeCwd::Tmp => tmp_root,
    };
    let setup_issues = write_smoke_setup_files(case, repo_root, tmp_root);
    if !setup_issues.is_empty() {
        return StepRun {
            step: CiStep {
                name: case.name.clone(),
                status: "failed".into(),
                command: command_vector(&args),
                expected_success: case.expect.success,
                actual_success: false,
                exit_code: None,
                stdout_schema: None,
                covers: case.covers.clone(),
                issues: setup_issues,
            },
            stdout: String::new(),
            stderr: String::new(),
        };
    }

    let mut run = run_sley_step(
        sley_bin,
        cwd,
        &case.name,
        args,
        case.expect.success,
        case.covers.clone(),
    );
    apply_smoke_expectations(&mut run, &case.expect);
    run.step.status = if run.step.issues.is_empty() {
        "passed".into()
    } else {
        "failed".into()
    };
    run
}

fn apply_smoke_expectations(run: &mut StepRun, expect: &SmokeExpectation) {
    for expected in &expect.stdout_contains {
        if !run.stdout.contains(expected) {
            run.step.issues.push(issue(
                "stdout_missing_text",
                format!("stdout did not contain {expected:?}"),
            ));
        }
    }
    for expected in &expect.stderr_contains {
        if !run.stderr.contains(expected) {
            run.step.issues.push(issue(
                "stderr_missing_text",
                format!("stderr did not contain {expected:?}"),
            ));
        }
    }
    if expect.stdout_json.is_empty() {
        return;
    }
    let json = match serde_json::from_str::<JsonValue>(&run.stdout) {
        Ok(json) => json,
        Err(error) => {
            run.step.issues.push(issue(
                "stdout_json_parse_failed",
                format!("stdout was not valid JSON: {error}"),
            ));
            return;
        }
    };
    for expectation in &expect.stdout_json {
        match json.pointer(&expectation.pointer) {
            Some(actual) if actual == &expectation.value => {}
            Some(actual) => run.step.issues.push(issue(
                "stdout_json_mismatch",
                format!(
                    "JSON pointer {} had value {}, expected {}",
                    expectation.pointer, actual, expectation.value
                ),
            )),
            None => run.step.issues.push(issue(
                "stdout_json_pointer_missing",
                format!("stdout JSON was missing pointer {}", expectation.pointer),
            )),
        }
    }
}

fn run_sley_step(
    sley_bin: &Path,
    cwd: &Path,
    name: &str,
    args: Vec<String>,
    expected_success: bool,
    covers: Vec<String>,
) -> StepRun {
    let output = match ProcessCommand::new(sley_bin)
        .current_dir(cwd)
        .args(&args)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            return StepRun {
                step: CiStep {
                    name: name.into(),
                    status: "failed".into(),
                    command: command_vector(&args),
                    expected_success,
                    actual_success: false,
                    exit_code: None,
                    stdout_schema: None,
                    covers,
                    issues: vec![issue(
                        "command_spawn_failed",
                        format!("failed to run sley command: {error}"),
                    )],
                },
                stdout: String::new(),
                stderr: String::new(),
            };
        }
    };

    let actual_success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout_schema = serde_json::from_str::<JsonValue>(&stdout)
        .ok()
        .and_then(|value| {
            value
                .get("schema")
                .and_then(JsonValue::as_str)
                .map(str::to_string)
        });
    let mut issues = Vec::new();
    if actual_success != expected_success {
        issues.push(issue(
            "exit_status_mismatch",
            format!(
                "expected success={expected_success}, got success={actual_success}; stderr={stderr:?}"
            ),
        ));
    }

    StepRun {
        step: CiStep {
            name: name.into(),
            status: if issues.is_empty() {
                "passed".into()
            } else {
                "failed".into()
            },
            command: command_vector(&args),
            expected_success,
            actual_success,
            exit_code: output.status.code(),
            stdout_schema,
            covers,
            issues,
        },
        stdout,
        stderr,
    }
}

fn finalize_report(
    command: &str,
    target: Option<String>,
    manifest: Option<String>,
    steps: Vec<CiStep>,
    issues: Vec<CiIssue>,
) -> CiReport {
    let failed_count = steps.iter().filter(|step| step.status == "failed").count();
    let step_count = steps.len();
    CiReport {
        schema: CI_REPORT_SCHEMA.into(),
        status: if failed_count == 0 && issues.is_empty() {
            "passed"
        } else {
            "failed"
        }
        .into(),
        command: command.into(),
        target,
        manifest,
        summary: CiSummary {
            step_count,
            passed_count: step_count.saturating_sub(failed_count),
            failed_count,
        },
        steps,
        issues,
    }
}

fn append_runtime_args(args: &mut Vec<String>, runtime: &RuntimeArgs) {
    append_repeated_flag(args, "--cap", &runtime.cap);
    append_repeated_flag(args, "--db-table", &runtime.db_table);
    append_pair_flag(args, "--secret", &runtime.secret);
    append_pair_flag(args, "--deploy-result", &runtime.deploy_result);
    append_pair_flag(args, "--spend-result", &runtime.spend_result);
    append_pair_flag(args, "--http-text", &runtime.http_text);
    append_pair_flag(args, "--shell-output", &runtime.shell_output);
    append_pair_flag(args, "--model-output", &runtime.model_output);
}

fn append_repeated_flag(args: &mut Vec<String>, flag: &str, values: &[String]) {
    for value in values {
        args.push(flag.into());
        args.push(value.clone());
    }
}

fn append_pair_flag(args: &mut Vec<String>, flag: &str, values: &[String]) {
    for pair in values.chunks(2) {
        if let [left, right] = pair {
            args.push(flag.into());
            args.push(left.clone());
            args.push(right.clone());
        }
    }
}

fn read_smoke_manifest(path: &Path) -> Result<SmokeManifest> {
    let source = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&source)?)
}

fn write_smoke_setup_files(case: &SmokeCase, repo_root: &Path, tmp_root: &Path) -> Vec<CiIssue> {
    let mut issues = Vec::new();
    for file in &case.setup_files {
        let path = PathBuf::from(expand_smoke_text(&file.path, repo_root, tmp_root));
        if !path.starts_with(tmp_root) {
            issues.push(issue(
                "setup_file_outside_tmp",
                format!("setup file {} does not live under {{tmp}}", path.display()),
            ));
            continue;
        }
        if let Some(parent) = path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                issues.push(issue(
                    "setup_dir_create_failed",
                    format!("failed to create {}: {error}", parent.display()),
                ));
                continue;
            }
        }
        if let Err(error) = fs::write(&path, expand_smoke_text(&file.content, repo_root, tmp_root))
        {
            issues.push(issue(
                "setup_file_write_failed",
                format!("failed to write {}: {error}", path.display()),
            ));
        }
    }
    issues
}

fn expand_smoke_text(text: &str, repo_root: &Path, tmp_root: &Path) -> String {
    text.replace("{repo}", &path_string(repo_root))
        .replace("{tmp}", &path_string(tmp_root))
}

fn temp_smoke_dir() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    env::temp_dir().join(format!("sley-ci-smoke-{}-{timestamp}", std::process::id()))
}

fn find_sley_binary() -> PathBuf {
    if let Some(path) = env::var_os("SLEY_BIN") {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("CARGO_BIN_EXE_sley") {
        return PathBuf::from(path);
    }
    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let candidate = parent.join(if cfg!(windows) { "sley.exe" } else { "sley" });
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("sley")
}

fn command_vector(args: &[String]) -> Vec<String> {
    let mut command = Vec::with_capacity(args.len() + 1);
    command.push("sley".into());
    command.extend(args.iter().cloned());
    command
}

fn issue(code: impl Into<String>, message: impl Into<String>) -> CiIssue {
    CiIssue {
        code: code.into(),
        message: message.into(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn emit_report(report: &CiReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    println!(
        "sley-ci {} status={} passed={} failed={} steps={}",
        report.command,
        report.status,
        report.summary.passed_count,
        report.summary.failed_count,
        report.summary.step_count
    );
    for issue in &report.issues {
        println!("{} {}", issue.code, issue.message);
    }
    for step in &report.steps {
        if step.status == "failed" {
            println!("failed {} {}", step.name, step.command.join(" "));
            for issue in &step.issues {
                println!("  {} {}", issue.code, issue.message);
            }
        }
    }
    Ok(())
}
