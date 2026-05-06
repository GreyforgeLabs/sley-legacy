use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sley::Program;
use sley::diagnostics::Diagnostic;
use sley::parser::parse_program;
use sley::project::load_project;
use sley::runtime::{RuntimeGate, RuntimeGates, Value};
use sley::verify::{VerifyReport, build_verify_report};

const SANDBOX_REPORT_SCHEMA: &str = "sley.sandbox.report.v0";
const SANDBOX_MANIFEST_SCHEMA: &str = "sley.sandbox.manifest.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-sandbox-runner")]
#[command(about = "Replay Sley runtime targets from deterministic sandbox manifests")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a deterministic sandbox manifest.
    Run(RunArgs),
}

#[derive(Debug, Args)]
struct RunArgs {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    keep_workdir: bool,
    manifest: PathBuf,
}

#[derive(Debug, Deserialize)]
struct SandboxManifest {
    schema: String,
    target: String,
    #[serde(default)]
    deny_warnings: bool,
    #[serde(default)]
    capabilities: Vec<SandboxCapability>,
    #[serde(default)]
    files: Vec<SandboxFile>,
    #[serde(default)]
    db_tables: Vec<SandboxDbTable>,
    #[serde(default)]
    secrets: Vec<SandboxNameText>,
    #[serde(default)]
    http_text: Vec<SandboxUrlText>,
    #[serde(default)]
    shell_outputs: Vec<SandboxCommandText>,
    #[serde(default)]
    model_outputs: Vec<SandboxPromptText>,
    #[serde(default)]
    deploy_results: Vec<SandboxTargetText>,
    #[serde(default)]
    spend_results: Vec<SandboxRequestText>,
}

#[derive(Debug, Deserialize)]
struct SandboxCapability {
    effect: String,
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SandboxFile {
    path: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct SandboxDbTable {
    table: String,
    rows: Vec<BTreeMap<String, JsonValue>>,
}

#[derive(Debug, Deserialize)]
struct SandboxNameText {
    name: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct SandboxUrlText {
    url: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct SandboxCommandText {
    command: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct SandboxPromptText {
    prompt: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct SandboxTargetText {
    target: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct SandboxRequestText {
    request: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct SandboxReport {
    schema: String,
    status: String,
    manifest: String,
    manifest_schema: String,
    target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sandbox_dir: Option<String>,
    sandbox_retained: bool,
    summary: SandboxSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: Option<VerifyReport>,
    issues: Vec<SandboxIssue>,
}

#[derive(Debug, Serialize)]
struct SandboxSummary {
    capability_count: usize,
    seed_count: usize,
    file_seed_count: usize,
    db_table_count: usize,
    secret_count: usize,
    http_text_count: usize,
    shell_output_count: usize,
    model_output_count: usize,
    deploy_result_count: usize,
    spend_result_count: usize,
    diagnostic_count: usize,
    issue_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct SandboxIssue {
    code: String,
    message: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (mut report, json, keep_workdir) = match cli.command {
        Command::Run(args) => {
            let json = args.json;
            let keep_workdir = args.keep_workdir;
            (run_manifest(&args.manifest), json, keep_workdir)
        }
    };

    cleanup_sandbox(&mut report, keep_workdir);

    if json {
        print_json(&report)?;
    } else {
        print_human(&report);
    }

    if report.status == "blocked" {
        std::process::exit(1);
    }
    Ok(())
}

fn run_manifest(path: &Path) -> SandboxReport {
    let manifest_path = path_string(path);
    let manifest = match read_manifest(path) {
        Ok(manifest) => manifest,
        Err(issue) => {
            return blocked_report(
                manifest_path,
                String::new(),
                String::new(),
                None,
                Vec::new(),
                vec![issue],
            );
        }
    };
    let mut issues = Vec::new();
    if manifest.schema != SANDBOX_MANIFEST_SCHEMA {
        issues.push(issue(
            "SANDBOX_MANIFEST_SCHEMA_UNEXPECTED",
            format!(
                "manifest schema was `{}`, expected `{SANDBOX_MANIFEST_SCHEMA}`",
                manifest.schema
            ),
        ));
    }
    let manifest_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let target_path = resolve_manifest_path(manifest_dir, &manifest.target);
    let target = path_string(&target_path);
    let sandbox_dir = create_sandbox_dir(path);
    if let Err(error) = write_file_seeds(&sandbox_dir, &manifest.files) {
        issues.push(error);
    }
    let gates = match build_gates(&manifest, &sandbox_dir) {
        Ok(gates) => gates,
        Err(error) => {
            issues.push(error);
            RuntimeGates::new()
        }
    };
    let previous_dir = enter_sandbox_dir(&manifest, &sandbox_dir, &mut issues);
    let verify = build_verify_report(
        target.clone(),
        load_target_program(&target_path),
        gates,
        manifest.deny_warnings,
    );
    restore_current_dir(previous_dir, &mut issues);
    let status = if !issues.is_empty() || verify.status == "blocked" {
        "blocked"
    } else if verify.status == "warnings" {
        "warnings"
    } else {
        "passed"
    };
    let diagnostic_count = verify.summary.error_count
        + verify.summary.warning_count
        + verify
            .runtime
            .as_ref()
            .map(|runtime| runtime.diagnostics.len())
            .unwrap_or(0);
    let summary = build_summary(&manifest, diagnostic_count, issues.len());

    SandboxReport {
        schema: SANDBOX_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        manifest: manifest_path,
        manifest_schema: manifest.schema,
        target,
        sandbox_dir: Some(path_string(&sandbox_dir)),
        sandbox_retained: true,
        summary,
        verify: Some(verify),
        issues,
    }
}

fn blocked_report(
    manifest: String,
    manifest_schema: String,
    target: String,
    sandbox_dir: Option<String>,
    diagnostics: Vec<Diagnostic>,
    issues: Vec<SandboxIssue>,
) -> SandboxReport {
    let diagnostic_count = diagnostics.len();
    SandboxReport {
        schema: SANDBOX_REPORT_SCHEMA.to_string(),
        status: "blocked".to_string(),
        manifest,
        manifest_schema,
        target,
        sandbox_dir,
        sandbox_retained: false,
        summary: SandboxSummary {
            capability_count: 0,
            seed_count: 0,
            file_seed_count: 0,
            db_table_count: 0,
            secret_count: 0,
            http_text_count: 0,
            shell_output_count: 0,
            model_output_count: 0,
            deploy_result_count: 0,
            spend_result_count: 0,
            diagnostic_count,
            issue_count: issues.len(),
        },
        verify: None,
        issues,
    }
}

fn build_summary(
    manifest: &SandboxManifest,
    diagnostic_count: usize,
    issue_count: usize,
) -> SandboxSummary {
    let seed_count = manifest.files.len()
        + manifest.db_tables.len()
        + manifest.secrets.len()
        + manifest.http_text.len()
        + manifest.shell_outputs.len()
        + manifest.model_outputs.len()
        + manifest.deploy_results.len()
        + manifest.spend_results.len();
    SandboxSummary {
        capability_count: manifest.capabilities.len(),
        seed_count,
        file_seed_count: manifest.files.len(),
        db_table_count: manifest.db_tables.len(),
        secret_count: manifest.secrets.len(),
        http_text_count: manifest.http_text.len(),
        shell_output_count: manifest.shell_outputs.len(),
        model_output_count: manifest.model_outputs.len(),
        deploy_result_count: manifest.deploy_results.len(),
        spend_result_count: manifest.spend_results.len(),
        diagnostic_count,
        issue_count,
    }
}

fn build_gates(
    manifest: &SandboxManifest,
    sandbox_dir: &Path,
) -> std::result::Result<RuntimeGates, SandboxIssue> {
    let mut gates = RuntimeGates::new();
    for capability in &manifest.capabilities {
        let effect = capability.effect.trim();
        if effect.is_empty() {
            return Err(issue(
                "SANDBOX_CAPABILITY_EFFECT_EMPTY",
                "capability effect cannot be empty",
            ));
        }
        match (capability.root.as_deref(), capability.scope.as_deref()) {
            (Some(_), Some(_)) => {
                return Err(issue(
                    "SANDBOX_CAPABILITY_AUTHORITY_AMBIGUOUS",
                    format!("capability `{effect}` cannot set both root and scope"),
                ));
            }
            (Some("$sandbox"), None) => gates.grant(RuntimeGate::with_root(effect, sandbox_dir)),
            (Some(root), None) => gates.grant(RuntimeGate::with_root(effect, root)),
            (None, Some(scope)) => gates.grant(RuntimeGate::with_scope(effect, scope)),
            (None, None) => gates.grant_effect(effect),
        }
    }
    for table in &manifest.db_tables {
        let mut rows = Vec::new();
        for row in &table.rows {
            let mut converted = BTreeMap::new();
            for (name, value) in row {
                let value = Value::try_from(value.clone()).map_err(|error| {
                    issue(
                        "SANDBOX_DB_VALUE_INVALID",
                        format!(
                            "db table `{}` field `{name}` is invalid: {error}",
                            table.table
                        ),
                    )
                })?;
                converted.insert(name.clone(), value);
            }
            rows.push(converted);
        }
        gates.grant_db_rows(table.table.clone(), rows);
    }
    for seed in &manifest.secrets {
        gates.grant_secret(seed.name.clone(), seed.text.clone());
    }
    for seed in &manifest.http_text {
        gates.grant_http_text(seed.url.clone(), seed.text.clone());
    }
    for seed in &manifest.shell_outputs {
        gates.grant_shell_output(seed.command.clone(), seed.text.clone());
    }
    for seed in &manifest.model_outputs {
        gates.grant_model_output(seed.prompt.clone(), seed.text.clone());
    }
    for seed in &manifest.deploy_results {
        gates.grant_deploy_result(seed.target.clone(), seed.text.clone());
    }
    for seed in &manifest.spend_results {
        gates.grant_spend_result(seed.request.clone(), seed.text.clone());
    }
    Ok(gates)
}

fn enter_sandbox_dir(
    manifest: &SandboxManifest,
    sandbox_dir: &Path,
    issues: &mut Vec<SandboxIssue>,
) -> Option<PathBuf> {
    if !should_enter_sandbox(manifest) {
        return None;
    }
    let previous_dir = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            issues.push(issue(
                "SANDBOX_CURRENT_DIR_READ_FAILED",
                format!("failed to read current directory before sandbox run: {error}"),
            ));
            return None;
        }
    };
    if let Err(error) = fs::create_dir_all(sandbox_dir) {
        issues.push(issue(
            "SANDBOX_DIR_CREATE_FAILED",
            format!(
                "failed to create sandbox dir {}: {error}",
                sandbox_dir.display()
            ),
        ));
        return None;
    }
    if let Err(error) = std::env::set_current_dir(sandbox_dir) {
        issues.push(issue(
            "SANDBOX_CURRENT_DIR_SET_FAILED",
            format!(
                "failed to enter sandbox dir {}: {error}",
                sandbox_dir.display()
            ),
        ));
        return None;
    }
    Some(previous_dir)
}

fn should_enter_sandbox(manifest: &SandboxManifest) -> bool {
    !manifest.files.is_empty()
        || manifest
            .capabilities
            .iter()
            .any(|capability| capability.root.as_deref() == Some("$sandbox"))
}

fn restore_current_dir(previous_dir: Option<PathBuf>, issues: &mut Vec<SandboxIssue>) {
    let Some(previous_dir) = previous_dir else {
        return;
    };
    if let Err(error) = std::env::set_current_dir(&previous_dir) {
        issues.push(issue(
            "SANDBOX_CURRENT_DIR_RESTORE_FAILED",
            format!(
                "failed to restore current directory {} after sandbox run: {error}",
                previous_dir.display()
            ),
        ));
    }
}

fn write_file_seeds(
    sandbox_dir: &Path,
    files: &[SandboxFile],
) -> std::result::Result<(), SandboxIssue> {
    if files.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(sandbox_dir).map_err(|error| {
        issue(
            "SANDBOX_DIR_CREATE_FAILED",
            format!(
                "failed to create sandbox dir {}: {error}",
                sandbox_dir.display()
            ),
        )
    })?;
    for file in files {
        let path = sandbox_child_path(sandbox_dir, &file.path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                issue(
                    "SANDBOX_FILE_PARENT_CREATE_FAILED",
                    format!("failed to create {}: {error}", parent.display()),
                )
            })?;
        }
        fs::write(&path, &file.text).map_err(|error| {
            issue(
                "SANDBOX_FILE_WRITE_FAILED",
                format!("failed to write {}: {error}", path.display()),
            )
        })?;
    }
    Ok(())
}

fn sandbox_child_path(
    sandbox_dir: &Path,
    relative: &str,
) -> std::result::Result<PathBuf, SandboxIssue> {
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err(issue(
            "SANDBOX_FILE_PATH_UNSAFE",
            format!("file seed path `{relative}` must be relative"),
        ));
    }
    let mut child = sandbox_dir.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) => child.push(part),
            _ => {
                return Err(issue(
                    "SANDBOX_FILE_PATH_UNSAFE",
                    format!("file seed path `{relative}` cannot escape the sandbox"),
                ));
            }
        }
    }
    Ok(child)
}

fn cleanup_sandbox(report: &mut SandboxReport, keep_workdir: bool) {
    if keep_workdir || report.status == "blocked" {
        return;
    }
    let Some(path) = report.sandbox_dir.as_ref().map(PathBuf::from) else {
        report.sandbox_retained = false;
        return;
    };
    match fs::remove_dir_all(&path) {
        Ok(()) => report.sandbox_retained = false,
        Err(error) if path.exists() => {
            report.status = "blocked".to_string();
            report.issues.push(issue(
                "SANDBOX_DIR_CLEANUP_FAILED",
                format!("failed to remove {}: {error}", path.display()),
            ));
            report.summary.issue_count = report.issues.len();
            report.sandbox_retained = true;
        }
        Err(_) => report.sandbox_retained = false,
    }
}

fn read_manifest(path: &Path) -> std::result::Result<SandboxManifest, SandboxIssue> {
    let source = fs::read_to_string(path).map_err(|error| {
        issue(
            "SANDBOX_MANIFEST_READ_FAILED",
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    serde_json::from_str(&source).map_err(|error| {
        issue(
            "SANDBOX_MANIFEST_PARSE_FAILED",
            format!("failed to parse {}: {error}", path.display()),
        )
    })
}

fn load_target_program(path: &Path) -> std::result::Result<Program, Vec<Diagnostic>> {
    if path.is_dir() || path.file_name().and_then(|name| name.to_str()) == Some("sley.toml") {
        return load_project(path).map(|project| project.program);
    }
    let source = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SOURCE_READ_FAILED",
            format!("failed to read {}: {error}", path.display()),
        )]
    })?;
    parse_program(&source)
}

fn resolve_manifest_path(manifest_dir: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    let candidate = if path.is_absolute() {
        path
    } else {
        manifest_dir.join(path)
    };
    absolute_path(&candidate)
}

fn absolute_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current_dir| current_dir.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn create_sandbox_dir(manifest_path: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let name = manifest_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("manifest");
    std::env::temp_dir().join(format!(
        "sley-sandbox-{name}-{}-{timestamp}",
        std::process::id()
    ))
}

fn print_json(report: &SandboxReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_human(report: &SandboxReport) {
    println!(
        "sley-sandbox-runner status={} target={} seeds={} diagnostics={} issues={}",
        report.status,
        report.target,
        report.summary.seed_count,
        report.summary.diagnostic_count,
        report.summary.issue_count
    );
    for issue in &report.issues {
        eprintln!("{}: {}", issue.code, issue.message);
    }
}

fn issue(code: impl Into<String>, message: impl Into<String>) -> SandboxIssue {
    SandboxIssue {
        code: code.into(),
        message: message.into(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
