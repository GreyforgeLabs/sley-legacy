use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
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
    /// Run the Sley lint gate over one target.
    Lint {
        #[arg(long)]
        json: bool,
        #[arg(long = "rule", value_name = "RULE")]
        rule: Vec<String>,
        #[arg(long)]
        module: Option<String>,
        #[arg(long)]
        deny_warnings: bool,
        target: PathBuf,
    },
    /// Run the Sley doctor readiness gate over one target.
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        deny_warnings: bool,
        target: PathBuf,
    },
    /// Run the Sley edit-plan readiness helper over one target.
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        deny_warnings: bool,
        #[arg(long)]
        graft_templates: bool,
        #[arg(long, requires = "graft_templates")]
        template_surface: Option<String>,
        target: PathBuf,
    },
    /// Run the Sley deterministic runtime gate over one target.
    Run {
        #[arg(long)]
        json: bool,
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
        /// Smoke manifest path, or a smoke directory containing manifest.json.
        manifest: PathBuf,
    },
    /// Run the accepted/rejected compiler conformance corpus.
    Corpus {
        #[arg(long)]
        json: bool,
        /// Corpus manifest path, or a corpus directory containing manifest.json.
        manifest: PathBuf,
    },
    /// Run packaged example conformance checks.
    Examples {
        #[arg(long)]
        json: bool,
        root: PathBuf,
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
    diagnostics: Vec<CiDiagnostic>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    findings: Vec<CiLintFinding>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    covers: Vec<String>,
    issues: Vec<CiIssue>,
}

#[derive(Debug, Serialize)]
struct CiDiagnostic {
    id: String,
    severity: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<String>,
}

#[derive(Debug, Serialize)]
struct CiLintFinding {
    id: String,
    rule: String,
    severity: String,
    message: String,
    node: String,
    module: String,
    hint: String,
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
    bin: Option<String>,
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
    #[serde(default)]
    stdout_json_absent: Vec<String>,
    #[serde(default)]
    files: Vec<FileExpectation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonExpectation {
    pointer: String,
    value: JsonValue,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileExpectation {
    path: String,
    #[serde(default)]
    contains: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusManifest {
    schema: String,
    accepted: Vec<CorpusCase>,
    rejected: Vec<CorpusCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusCase {
    path: String,
    covers: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusExpectation {
    diagnostics: Vec<String>,
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
        Command::Lint {
            json,
            rule,
            module,
            deny_warnings,
            target,
        } => {
            let report =
                build_lint_report(&sley_bin, &target, &rule, module.as_deref(), deny_warnings)?;
            Ok((report, json))
        }
        Command::Doctor {
            json,
            deny_warnings,
            target,
        } => {
            let report = build_doctor_report(&sley_bin, &target, deny_warnings)?;
            Ok((report, json))
        }
        Command::Plan {
            json,
            deny_warnings,
            graft_templates,
            template_surface,
            target,
        } => {
            let report = build_plan_report(
                &sley_bin,
                &target,
                deny_warnings,
                graft_templates,
                template_surface.as_deref(),
            )?;
            Ok((report, json))
        }
        Command::Run {
            json,
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
            let report = build_run_report(&sley_bin, &target, &runtime)?;
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
            let repo_root = repo_root.canonicalize().with_context(|| {
                format!("failed to resolve smoke repo root {}", repo_root.display())
            })?;
            let report = build_smoke_report(&sley_bin, &repo_root, &manifest);
            Ok((report, json))
        }
        Command::Corpus { json, manifest } => {
            let report = build_corpus_report(&sley_bin, &manifest);
            Ok((report, json))
        }
        Command::Examples { json, root } => {
            let report = build_examples_report(&sley_bin, &root);
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

fn build_lint_report(
    sley_bin: &Path,
    target: &Path,
    rules: &[String],
    module: Option<&str>,
    deny_warnings: bool,
) -> Result<CiReport> {
    let cwd = env::current_dir()?;
    let mut args = vec!["lint".into(), "--json".into()];
    for rule in rules {
        args.push("--rule".into());
        args.push(rule.clone());
    }
    if let Some(module) = module {
        args.push("--module".into());
        args.push(module.into());
    }
    if deny_warnings {
        args.push("--deny-warnings".into());
    }
    args.push(path_string(target));
    let steps = vec![run_sley_step(sley_bin, &cwd, "lint", args, true, Vec::new()).step];
    Ok(finalize_report(
        "lint",
        Some(path_string(target)),
        None,
        steps,
        Vec::new(),
    ))
}

fn build_doctor_report(sley_bin: &Path, target: &Path, deny_warnings: bool) -> Result<CiReport> {
    let cwd = env::current_dir()?;
    let mut args = vec!["doctor".into(), "--json".into()];
    if deny_warnings {
        args.push("--deny-warnings".into());
    }
    args.push(path_string(target));
    let steps = vec![run_sley_step(sley_bin, &cwd, "doctor", args, true, Vec::new()).step];
    Ok(finalize_report(
        "doctor",
        Some(path_string(target)),
        None,
        steps,
        Vec::new(),
    ))
}

fn build_plan_report(
    sley_bin: &Path,
    target: &Path,
    deny_warnings: bool,
    graft_templates: bool,
    template_surface: Option<&str>,
) -> Result<CiReport> {
    let cwd = env::current_dir()?;
    let mut args = vec!["plan".into(), "--json".into()];
    if deny_warnings {
        args.push("--deny-warnings".into());
    }
    if graft_templates {
        args.push("--graft-templates".into());
    }
    if let Some(template_surface) = template_surface {
        args.push("--template-surface".into());
        args.push(template_surface.into());
    }
    args.push(path_string(target));
    let steps = vec![run_sley_step(sley_bin, &cwd, "plan", args, true, Vec::new()).step];
    Ok(finalize_report(
        "plan",
        Some(path_string(target)),
        None,
        steps,
        Vec::new(),
    ))
}

fn build_run_report(sley_bin: &Path, target: &Path, runtime: &RuntimeArgs) -> Result<CiReport> {
    let cwd = env::current_dir()?;
    let mut args = vec!["run".into(), "--json".into()];
    append_runtime_args(&mut args, runtime);
    args.push(path_string(target));
    let steps = vec![run_sley_step(sley_bin, &cwd, "run", args, true, Vec::new()).step];
    Ok(finalize_report(
        "run",
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
    let manifest_path = resolve_manifest_path(manifest_path);
    let mut issues = Vec::new();
    let manifest = match read_smoke_manifest(&manifest_path) {
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
                Some(path_string(&manifest_path)),
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
        Some(path_string(&manifest_path)),
        steps,
        issues,
    )
}

fn build_corpus_report(sley_bin: &Path, manifest_path: &Path) -> CiReport {
    let manifest_path = resolve_manifest_path(manifest_path);
    let mut issues = Vec::new();
    let manifest = match read_corpus_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            issues.push(issue(
                "corpus_manifest_read_failed",
                format!(
                    "failed to read corpus manifest {}: {error}",
                    manifest_path.display()
                ),
            ));
            return finalize_report(
                "corpus",
                None,
                Some(path_string(&manifest_path)),
                Vec::new(),
                issues,
            );
        }
    };
    if manifest.schema != "sley.conformance.manifest.v0" {
        issues.push(issue(
            "corpus_manifest_schema_mismatch",
            format!(
                "corpus manifest schema {:?} does not match {:?}",
                manifest.schema, "sley.conformance.manifest.v0"
            ),
        ));
    }

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let corpus_root = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_root = temp_corpus_dir();
    let mut steps = Vec::new();
    if let Err(error) = fs::create_dir_all(&tmp_root) {
        issues.push(issue(
            "temp_dir_create_failed",
            format!("failed to create {}: {error}", tmp_root.display()),
        ));
    } else {
        for (index, case) in manifest.accepted.iter().enumerate() {
            let file = corpus_root.join(&case.path);
            steps.push(
                run_sley_step(
                    sley_bin,
                    &cwd,
                    &format!("accepted_check:{}", case.path),
                    vec!["check".into(), "--json".into(), path_string(&file)],
                    true,
                    case.covers.clone(),
                )
                .step,
            );
            steps.push(run_format_round_trip_step(
                sley_bin,
                &cwd,
                &tmp_root,
                index,
                &file,
                &format!("accepted_format_round_trip:{}", case.path),
                case.covers.clone(),
            ));
        }
        for case in &manifest.rejected {
            let file = corpus_root.join(&case.path);
            steps.push(run_corpus_rejected_check(sley_bin, &cwd, &file, case));
        }
    }
    let _ = fs::remove_dir_all(&tmp_root);

    finalize_report(
        "corpus",
        None,
        Some(path_string(&manifest_path)),
        steps,
        issues,
    )
}

fn build_examples_report(sley_bin: &Path, root: &Path) -> CiReport {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut issues = Vec::new();
    let mut project_roots = match collect_project_roots(root) {
        Ok(project_roots) => project_roots,
        Err(error) => {
            issues.push(issue(
                "examples_project_collect_failed",
                format!(
                    "failed to collect project roots under {}: {error}",
                    root.display()
                ),
            ));
            Vec::new()
        }
    };
    let mut source_files = match collect_sley_sources(root) {
        Ok(source_files) => source_files,
        Err(error) => {
            issues.push(issue(
                "examples_source_collect_failed",
                format!(
                    "failed to collect Sley sources under {}: {error}",
                    root.display()
                ),
            ));
            Vec::new()
        }
    };
    project_roots.sort();
    source_files.sort();
    if project_roots.is_empty() && source_files.is_empty() && issues.is_empty() {
        issues.push(issue(
            "examples_empty",
            format!("no Sley examples found under {}", root.display()),
        ));
    }

    let tmp_root = temp_examples_dir();
    let mut steps = Vec::new();
    if let Err(error) = fs::create_dir_all(&tmp_root) {
        issues.push(issue(
            "temp_dir_create_failed",
            format!("failed to create {}: {error}", tmp_root.display()),
        ));
    } else {
        for project_root in &project_roots {
            steps.push(
                run_sley_step(
                    sley_bin,
                    &cwd,
                    &format!("project_check:{}", path_string(project_root)),
                    vec!["check".into(), "--json".into(), path_string(project_root)],
                    true,
                    vec![
                        "examples:project".into(),
                        "json:sley.diagnostics.report.v0".into(),
                    ],
                )
                .step,
            );
        }
        for file in source_files.iter().filter(|file| {
            !project_roots
                .iter()
                .any(|project_root| file.starts_with(project_root))
        }) {
            steps.push(
                run_sley_step(
                    sley_bin,
                    &cwd,
                    &format!("file_check:{}", path_string(file)),
                    vec!["check".into(), "--json".into(), path_string(file)],
                    true,
                    vec![
                        "examples:standalone".into(),
                        "json:sley.diagnostics.report.v0".into(),
                    ],
                )
                .step,
            );
        }
        for (index, file) in source_files.iter().enumerate() {
            steps.push(run_format_round_trip_step(
                sley_bin,
                &cwd,
                &tmp_root,
                index,
                file,
                &format!("format_round_trip:{}", path_string(file)),
                vec!["examples:format-round-trip".into()],
            ));
        }
    }
    let _ = fs::remove_dir_all(&tmp_root);

    finalize_report("examples", Some(path_string(root)), None, steps, issues)
}

fn run_format_round_trip_step(
    sley_bin: &Path,
    cwd: &Path,
    tmp_root: &Path,
    index: usize,
    file: &Path,
    name: &str,
    covers: Vec<String>,
) -> CiStep {
    let file_arg = path_string(file);
    let args = vec!["format".into(), file_arg];
    let mut issues = Vec::new();
    let mut actual_success = false;
    let mut exit_code = None;
    match ProcessCommand::new(sley_bin)
        .current_dir(cwd)
        .args(&args)
        .output()
    {
        Ok(output) => {
            actual_success = output.status.success();
            exit_code = output.status.code();
            let formatted = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            if !actual_success {
                issues.push(issue(
                    "exit_status_mismatch",
                    format!("expected success=true, got success=false; stderr={stderr:?}"),
                ));
            } else {
                let round_trip_file = tmp_root.join(format!("format-round-trip-{index}.sley"));
                if let Err(error) = fs::write(&round_trip_file, &formatted) {
                    issues.push(issue(
                        "format_round_trip_write_failed",
                        format!("failed to write {}: {error}", round_trip_file.display()),
                    ));
                } else {
                    match ProcessCommand::new(sley_bin)
                        .current_dir(cwd)
                        .args(["format", &path_string(&round_trip_file)])
                        .output()
                    {
                        Ok(second) => {
                            if !second.status.success() {
                                issues.push(issue(
                                    "format_round_trip_failed",
                                    format!(
                                        "formatted source failed to format again; stderr={:?}",
                                        String::from_utf8_lossy(&second.stderr)
                                    ),
                                ));
                            } else if formatted != String::from_utf8_lossy(&second.stdout) {
                                issues.push(issue(
                                    "format_round_trip_unstable",
                                    "formatter output changed after a second format pass",
                                ));
                            }
                        }
                        Err(error) => issues.push(issue(
                            "command_spawn_failed",
                            format!("failed to run sley format round trip: {error}"),
                        )),
                    }
                }
            }
        }
        Err(error) => issues.push(issue(
            "command_spawn_failed",
            format!("failed to run sley format: {error}"),
        )),
    }
    CiStep {
        name: name.into(),
        status: if issues.is_empty() {
            "passed".into()
        } else {
            "failed".into()
        },
        command: command_vector(&args),
        expected_success: true,
        actual_success,
        exit_code,
        stdout_schema: None,
        diagnostics: Vec::new(),
        findings: Vec::new(),
        covers,
        issues,
    }
}

fn run_corpus_rejected_check(
    sley_bin: &Path,
    cwd: &Path,
    file: &Path,
    case: &CorpusCase,
) -> CiStep {
    let mut run = run_sley_step(
        sley_bin,
        cwd,
        &format!("rejected_check:{}", case.path),
        vec!["check".into(), "--json".into(), path_string(file)],
        false,
        case.covers.clone(),
    );
    let expectation_path = file.with_extension("json");
    match read_corpus_expectation(&expectation_path) {
        Ok(expectation) => apply_corpus_diagnostic_expectations(&mut run, &expectation),
        Err(error) => run.step.issues.push(issue(
            "corpus_expectation_read_failed",
            format!("failed to read {}: {error}", expectation_path.display()),
        )),
    }
    run.step.status = if run.step.issues.is_empty() {
        "passed".into()
    } else {
        "failed".into()
    };
    run.step
}

fn apply_corpus_diagnostic_expectations(run: &mut StepRun, expectation: &CorpusExpectation) {
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
    let actual_ids = json
        .pointer("/diagnostics")
        .and_then(JsonValue::as_array)
        .map(|diagnostics| {
            diagnostics
                .iter()
                .filter_map(|diagnostic| diagnostic.pointer("/id").and_then(JsonValue::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for expected in &expectation.diagnostics {
        if !actual_ids.iter().any(|actual| actual == expected) {
            run.step.issues.push(issue(
                "expected_diagnostic_missing",
                format!("expected diagnostic {expected}, got {:?}", actual_ids),
            ));
        }
    }
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
        let binary = case.bin.as_deref().unwrap_or("sley");
        return StepRun {
            step: CiStep {
                name: case.name.clone(),
                status: "failed".into(),
                command: command_vector_for(binary, &args),
                expected_success: case.expect.success,
                actual_success: false,
                exit_code: None,
                stdout_schema: None,
                diagnostics: Vec::new(),
                findings: Vec::new(),
                covers: case.covers.clone(),
                issues: setup_issues,
            },
            stdout: String::new(),
            stderr: String::new(),
        };
    }

    let binary = case.bin.as_deref().unwrap_or("sley");
    let tool = match find_smoke_binary(sley_bin, binary) {
        Ok(tool) => tool,
        Err(message) => {
            return StepRun {
                step: CiStep {
                    name: case.name.clone(),
                    status: "failed".into(),
                    command: command_vector_for(binary, &args),
                    expected_success: case.expect.success,
                    actual_success: false,
                    exit_code: None,
                    stdout_schema: None,
                    diagnostics: Vec::new(),
                    findings: Vec::new(),
                    covers: case.covers.clone(),
                    issues: vec![issue("unsupported_smoke_binary", message)],
                },
                stdout: String::new(),
                stderr: String::new(),
            };
        }
    };

    let mut run = run_binary_step(
        binary,
        &tool,
        cwd,
        &case.name,
        args,
        case.expect.success,
        case.covers.clone(),
    );
    apply_smoke_expectations(&mut run, &case.expect, repo_root, tmp_root);
    run.step.status = if run.step.issues.is_empty() {
        "passed".into()
    } else {
        "failed".into()
    };
    run
}

fn apply_smoke_expectations(
    run: &mut StepRun,
    expect: &SmokeExpectation,
    repo_root: &Path,
    tmp_root: &Path,
) {
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
    if expect.stdout_json.is_empty() && expect.stdout_json_absent.is_empty() {
        apply_smoke_file_expectations(run, expect, repo_root, tmp_root);
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
    for pointer in &expect.stdout_json_absent {
        if json.pointer(pointer).is_some() {
            run.step.issues.push(issue(
                "stdout_json_pointer_present",
                format!("stdout JSON unexpectedly contained pointer {pointer}"),
            ));
        }
    }
    apply_smoke_file_expectations(run, expect, repo_root, tmp_root);
}

fn apply_smoke_file_expectations(
    run: &mut StepRun,
    expect: &SmokeExpectation,
    repo_root: &Path,
    tmp_root: &Path,
) {
    for expected in &expect.files {
        let path = PathBuf::from(expand_smoke_text(&expected.path, repo_root, tmp_root));
        if !path.starts_with(repo_root) && !path.starts_with(tmp_root) {
            run.step.issues.push(issue(
                "expected_file_outside_allowed_roots",
                format!(
                    "expected file {} does not live under {{repo}} or {{tmp}}",
                    path.display()
                ),
            ));
            continue;
        }
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                run.step.issues.push(issue(
                    "expected_file_read_failed",
                    format!("failed to read expected file {}: {error}", path.display()),
                ));
                continue;
            }
        };
        for needle in &expected.contains {
            if !source.contains(needle) {
                run.step.issues.push(issue(
                    "expected_file_missing_text",
                    format!("{} did not contain {needle:?}", path.display()),
                ));
            }
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
    run_binary_step("sley", sley_bin, cwd, name, args, expected_success, covers)
}

fn run_binary_step(
    binary: &str,
    binary_path: &Path,
    cwd: &Path,
    name: &str,
    args: Vec<String>,
    expected_success: bool,
    covers: Vec<String>,
) -> StepRun {
    let output = match ProcessCommand::new(binary_path)
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
                    command: command_vector_for(binary, &args),
                    expected_success,
                    actual_success: false,
                    exit_code: None,
                    stdout_schema: None,
                    diagnostics: Vec::new(),
                    findings: Vec::new(),
                    covers,
                    issues: vec![issue(
                        "command_spawn_failed",
                        format!("failed to run {binary} command: {error}"),
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
    let stdout_json = serde_json::from_str::<JsonValue>(&stdout).ok();
    let stdout_schema = stdout_json.as_ref().and_then(|value| {
        value
            .get("schema")
            .and_then(JsonValue::as_str)
            .map(str::to_string)
    });
    let diagnostics = stdout_json
        .as_ref()
        .map(|json| extract_step_diagnostics(json, stdout_schema.as_deref()))
        .unwrap_or_default();
    let findings = stdout_json
        .as_ref()
        .map(|json| extract_step_findings(json, stdout_schema.as_deref()))
        .unwrap_or_default();
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
            command: command_vector_for(binary, &args),
            expected_success,
            actual_success,
            exit_code: output.status.code(),
            stdout_schema,
            diagnostics,
            findings,
            covers,
            issues,
        },
        stdout,
        stderr,
    }
}

fn extract_step_diagnostics(json: &JsonValue, stdout_schema: Option<&str>) -> Vec<CiDiagnostic> {
    let pointers: &[&str] = match stdout_schema {
        Some("sley.diagnostics.report.v0" | "sley.run.report.v0") => &["/diagnostics"],
        Some("sley.verify.report.v0") => &["/diagnostics", "/runtime/diagnostics"],
        _ => return Vec::new(),
    };
    pointers
        .iter()
        .flat_map(|pointer| extract_diagnostics_at(json, pointer))
        .collect()
}

fn extract_diagnostics_at(json: &JsonValue, pointer: &str) -> Vec<CiDiagnostic> {
    json.pointer(pointer)
        .and_then(JsonValue::as_array)
        .map(|diagnostics| {
            diagnostics
                .iter()
                .filter_map(|diagnostic| {
                    let id = diagnostic.get("id").and_then(JsonValue::as_str)?;
                    let severity = diagnostic.get("severity").and_then(JsonValue::as_str)?;
                    let message = diagnostic.get("message").and_then(JsonValue::as_str)?;
                    Some(CiDiagnostic {
                        id: id.into(),
                        severity: severity.into(),
                        message: message.into(),
                        node: diagnostic
                            .get("node")
                            .and_then(JsonValue::as_str)
                            .map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_step_findings(json: &JsonValue, stdout_schema: Option<&str>) -> Vec<CiLintFinding> {
    let pointer = match stdout_schema {
        Some("sley.lint.report.v0") => "/findings",
        Some("sley.doctor.report.v0" | "sley.edit_plan.report.v0" | "sley.verify.report.v0") => {
            "/lint/findings"
        }
        _ => return Vec::new(),
    };
    json.pointer(pointer)
        .and_then(JsonValue::as_array)
        .map(|findings| {
            findings
                .iter()
                .filter_map(|finding| {
                    Some(CiLintFinding {
                        id: finding.get("id").and_then(JsonValue::as_str)?.into(),
                        rule: finding.get("rule").and_then(JsonValue::as_str)?.into(),
                        severity: finding.get("severity").and_then(JsonValue::as_str)?.into(),
                        message: finding.get("message").and_then(JsonValue::as_str)?.into(),
                        node: finding.get("node").and_then(JsonValue::as_str)?.into(),
                        module: finding.get("module").and_then(JsonValue::as_str)?.into(),
                        hint: finding.get("hint").and_then(JsonValue::as_str)?.into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
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

fn read_corpus_manifest(path: &Path) -> Result<CorpusManifest> {
    let source = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&source)?)
}

fn resolve_manifest_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.join("manifest.json")
    } else {
        path.to_path_buf()
    }
}

fn read_corpus_expectation(path: &Path) -> Result<CorpusExpectation> {
    let source = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&source)?)
}

fn collect_project_roots(root: &Path) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    collect_project_roots_into(root, &mut roots)?;
    roots.sort();
    Ok(roots)
}

fn collect_project_roots_into(path: &Path, roots: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        return Ok(());
    }
    let mut entries = fs::read_dir(path)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_project_roots_into(&entry_path, roots)?;
        } else if entry_path.file_name().and_then(|name| name.to_str()) == Some("sley.toml") {
            if let Some(parent) = entry_path.parent() {
                roots.push(parent.to_path_buf());
            }
        }
    }
    Ok(())
}

fn collect_sley_sources(root: &Path) -> Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    collect_sley_sources_into(root, &mut sources)?;
    sources.sort();
    Ok(sources)
}

fn collect_sley_sources_into(path: &Path, sources: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|extension| extension.to_str()) == Some("sley") {
            sources.push(path.to_path_buf());
        }
        return Ok(());
    }
    let mut entries = fs::read_dir(path)?.collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        collect_sley_sources_into(&entry.path(), sources)?;
    }
    Ok(())
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

fn temp_corpus_dir() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    env::temp_dir().join(format!("sley-ci-corpus-{}-{timestamp}", std::process::id()))
}

fn temp_examples_dir() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    env::temp_dir().join(format!(
        "sley-ci-examples-{}-{timestamp}",
        std::process::id()
    ))
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

fn find_smoke_binary(sley_bin: &Path, binary: &str) -> std::result::Result<PathBuf, String> {
    if !allowed_smoke_binary(binary) {
        return Err(format!(
            "smoke binary {binary:?} is not in the Sley binary allowlist"
        ));
    }
    if binary == "sley" {
        return Ok(sley_bin.to_path_buf());
    }
    let cargo_env_key = format!("CARGO_BIN_EXE_{binary}");
    if let Some(path) = env::var_os(&cargo_env_key) {
        return Ok(PathBuf::from(path));
    }
    let override_env_key = format!("{}_BIN", binary.replace('-', "_").to_uppercase());
    if let Some(path) = env::var_os(&override_env_key) {
        return Ok(PathBuf::from(path));
    }
    if let Some(parent) = sley_bin.parent() {
        let candidate = parent.join(exe_name(binary));
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Ok(PathBuf::from(exe_name(binary)))
}

fn allowed_smoke_binary(binary: &str) -> bool {
    matches!(
        binary,
        "sley"
            | "sley-agent-bench"
            | "sley-ci"
            | "sley-conformance"
            | "sley-contract"
            | "sley-docgen"
            | "sley-lsp"
            | "sley-migrate"
            | "sley-sandbox-runner"
            | "sley-shadow"
            | "sley-workbench"
            | "sley-zjx"
    )
}

fn exe_name(binary: &str) -> String {
    if cfg!(windows) {
        format!("{binary}.exe")
    } else {
        binary.to_string()
    }
}

fn command_vector(args: &[String]) -> Vec<String> {
    command_vector_for("sley", args)
}

fn command_vector_for(binary: &str, args: &[String]) -> Vec<String> {
    let mut command = Vec::with_capacity(args.len() + 1);
    command.push(binary.into());
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
