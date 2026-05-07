use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::Value as JsonValue;
use sley::Program;
use sley::diagnostics::Diagnostic;
use sley::parser::parse_program;
use sley::plan::{
    EditPlanOptions, build_edit_plan_report_with_options, module_name_from_sley_path,
};
use sley::project::load_project;

const MIGRATE_REPORT_SCHEMA: &str = "sley.migrate.report.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-migrate")]
#[command(about = "Report checked Sley source and contract migrations")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build a migration report for one source/project target.
    Report(ReportArgs),
}

#[derive(Debug, Args)]
struct ReportArgs {
    #[arg(long)]
    json: bool,
    #[arg(long = "schemas", value_name = "DIR")]
    schemas: Option<PathBuf>,
    #[arg(long = "fixtures", value_name = "DIR")]
    fixtures: Option<PathBuf>,
    target: PathBuf,
}

#[derive(Debug, Serialize)]
struct MigrateReport {
    schema: String,
    status: String,
    target: String,
    source_schema: String,
    summary: MigrateSummary,
    migrations: Vec<MigrationAction>,
    schema_drift: Vec<SchemaDrift>,
    diagnostics: Vec<Diagnostic>,
    issues: Vec<MigrateIssue>,
}

#[derive(Debug, Serialize)]
struct MigrateSummary {
    diagnostic_count: usize,
    migration_count: usize,
    raw_host_adapter_count: usize,
    module_declaration_count: usize,
    naming_cleanup_count: usize,
    result_propagation_count: usize,
    schema_drift_count: usize,
    issue_count: usize,
}

#[derive(Debug, Serialize)]
struct MigrationAction {
    kind: String,
    category: String,
    surface: String,
    reason: String,
    dry_run_command: Vec<String>,
    write_command: Vec<String>,
    editable_json_pointers: Vec<String>,
    operation: JsonValue,
}

#[derive(Debug, Serialize)]
struct SchemaDrift {
    kind: String,
    schema_id: String,
    path: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct MigrateIssue {
    code: String,
    message: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (report, json) = match cli.command {
        Command::Report(args) => {
            let json = args.json;
            (build_migrate_report(args), json)
        }
    };

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

fn build_migrate_report(args: ReportArgs) -> MigrateReport {
    let target = path_string(&args.target);
    let mut issues = Vec::new();
    let plan = build_edit_plan_report_with_options(
        target.clone(),
        load_target_program(&args.target),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: module_name_hint(&args.target),
        },
    );
    let migrations = plan
        .graft_templates
        .iter()
        .filter_map(|template| {
            let category = migration_category(&template.kind)?;
            Some(MigrationAction {
                kind: template.kind.clone(),
                category: category.to_string(),
                surface: template.surface.clone(),
                reason: template.reason.clone(),
                dry_run_command: fix_command(&template.kind, &template.surface, &target, false),
                write_command: fix_command(&template.kind, &template.surface, &target, true),
                editable_json_pointers: template.editable_json_pointers.clone(),
                operation: template.operation.clone(),
            })
        })
        .collect::<Vec<_>>();
    let schema_drift = detect_schema_drift(
        args.schemas.as_deref(),
        args.fixtures.as_deref(),
        &mut issues,
    );
    let summary = build_summary(&plan.diagnostics, &migrations, &schema_drift, &issues);
    let status = if !issues.is_empty() || plan.status == "blocked" {
        "blocked"
    } else if migrations.is_empty() && schema_drift.is_empty() {
        "ready"
    } else {
        "migrations"
    };

    MigrateReport {
        schema: MIGRATE_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        target,
        source_schema: plan.schema,
        summary,
        migrations,
        schema_drift,
        diagnostics: plan.diagnostics,
        issues,
    }
}

fn build_summary(
    diagnostics: &[Diagnostic],
    migrations: &[MigrationAction],
    schema_drift: &[SchemaDrift],
    issues: &[MigrateIssue],
) -> MigrateSummary {
    MigrateSummary {
        diagnostic_count: diagnostics.len(),
        migration_count: migrations.len(),
        raw_host_adapter_count: migrations
            .iter()
            .filter(|migration| migration.category == "raw_host_adapter")
            .count(),
        module_declaration_count: migrations
            .iter()
            .filter(|migration| migration.category == "module_declaration")
            .count(),
        naming_cleanup_count: migrations
            .iter()
            .filter(|migration| migration.category == "naming_cleanup")
            .count(),
        result_propagation_count: migrations
            .iter()
            .filter(|migration| migration.category == "result_propagation")
            .count(),
        schema_drift_count: schema_drift.len(),
        issue_count: issues.len(),
    }
}

fn migration_category(kind: &str) -> Option<&'static str> {
    match kind {
        "add_module_declaration" => Some("module_declaration"),
        "migrate_raw_host_adapter" => Some("raw_host_adapter"),
        "qualify_imported_call" => Some("naming_cleanup"),
        "propagate_unchecked_result" => Some("result_propagation"),
        "propagate_unchecked_result_binding" => Some("result_propagation"),
        _ => None,
    }
}

fn fix_command(kind: &str, surface: &str, target: &str, write: bool) -> Vec<String> {
    let mut command = vec![
        "sley".to_string(),
        "fix".to_string(),
        "--json".to_string(),
        "--kind".to_string(),
        kind.to_string(),
        "--template-surface".to_string(),
        surface.to_string(),
    ];
    command.push(if write { "--write" } else { "--dry-run" }.to_string());
    command.push(target.to_string());
    command
}

fn detect_schema_drift(
    schemas_dir: Option<&Path>,
    fixtures_dir: Option<&Path>,
    issues: &mut Vec<MigrateIssue>,
) -> Vec<SchemaDrift> {
    match (schemas_dir, fixtures_dir) {
        (None, None) => Vec::new(),
        (Some(_), None) | (None, Some(_)) => {
            issues.push(issue(
                "MIGRATE_SCHEMA_DRIFT_INPUT_INCOMPLETE",
                "schema drift checks require both --schemas and --fixtures",
            ));
            Vec::new()
        }
        (Some(schemas_dir), Some(fixtures_dir)) => {
            let schemas = collect_schema_ids(schemas_dir, issues);
            let fixtures = collect_fixture_schema_ids(fixtures_dir, issues);
            let mut drift = Vec::new();
            for fixture in &fixtures.entries {
                if !schemas.ids.contains(&fixture.schema_id) {
                    drift.push(SchemaDrift {
                        kind: "fixture_without_schema".to_string(),
                        schema_id: fixture.schema_id.clone(),
                        path: fixture.path.clone(),
                        message: format!(
                            "fixture {} declares schema `{}` but no matching schema file was found",
                            fixture.path, fixture.schema_id
                        ),
                    });
                }
            }
            for schema_id in schemas.ids.difference(&fixtures.ids) {
                let path = schemas
                    .paths
                    .iter()
                    .find(|entry| entry.schema_id == *schema_id)
                    .map(|entry| entry.path.clone())
                    .unwrap_or_default();
                drift.push(SchemaDrift {
                    kind: "schema_without_fixture".to_string(),
                    schema_id: schema_id.clone(),
                    path,
                    message: format!("schema `{schema_id}` has no fixture instance"),
                });
            }
            drift
        }
    }
}

#[derive(Debug, Default)]
struct SchemaIndex {
    ids: BTreeSet<String>,
    paths: Vec<SchemaPath>,
}

#[derive(Debug)]
struct SchemaPath {
    schema_id: String,
    path: String,
}

#[derive(Debug, Default)]
struct FixtureIndex {
    ids: BTreeSet<String>,
    entries: Vec<FixturePath>,
}

#[derive(Debug)]
struct FixturePath {
    schema_id: String,
    path: String,
}

fn collect_schema_ids(dir: &Path, issues: &mut Vec<MigrateIssue>) -> SchemaIndex {
    let mut index = SchemaIndex::default();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            issues.push(issue(
                "MIGRATE_SCHEMA_DIR_READ_FAILED",
                format!("failed to read schema dir {}: {error}", dir.display()),
            ));
            return index;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".schema.json") {
            continue;
        }
        match read_json(&path) {
            Ok(value) => {
                if let Some(schema_id) = value.get("$id").and_then(JsonValue::as_str) {
                    index.ids.insert(schema_id.to_string());
                    index.paths.push(SchemaPath {
                        schema_id: schema_id.to_string(),
                        path: path_string(&path),
                    });
                } else {
                    issues.push(issue(
                        "MIGRATE_SCHEMA_ID_MISSING",
                        format!("schema {} is missing $id", path.display()),
                    ));
                }
            }
            Err(issue) => issues.push(issue),
        }
    }
    index
}

fn collect_fixture_schema_ids(dir: &Path, issues: &mut Vec<MigrateIssue>) -> FixtureIndex {
    let mut index = FixtureIndex::default();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            issues.push(issue(
                "MIGRATE_FIXTURE_DIR_READ_FAILED",
                format!("failed to read fixture dir {}: {error}", dir.display()),
            ));
            return index;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match read_json(&path) {
            Ok(value) => {
                if let Some(schema_id) = value.get("schema").and_then(JsonValue::as_str) {
                    index.ids.insert(schema_id.to_string());
                    index.entries.push(FixturePath {
                        schema_id: schema_id.to_string(),
                        path: path_string(&path),
                    });
                } else {
                    issues.push(issue(
                        "MIGRATE_FIXTURE_SCHEMA_MISSING",
                        format!("fixture {} is missing /schema", path.display()),
                    ));
                }
            }
            Err(issue) => issues.push(issue),
        }
    }
    index
}

fn read_json(path: &Path) -> std::result::Result<JsonValue, MigrateIssue> {
    let source = fs::read_to_string(path).map_err(|error| {
        issue(
            "MIGRATE_JSON_READ_FAILED",
            format!("failed to read {}: {error}", path.display()),
        )
    })?;
    serde_json::from_str(&source).map_err(|error| {
        issue(
            "MIGRATE_JSON_PARSE_FAILED",
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

fn module_name_hint(path: &Path) -> Option<String> {
    if path.is_file() {
        module_name_from_sley_path(path)
    } else {
        None
    }
}

fn print_json(report: &MigrateReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_human(report: &MigrateReport) {
    println!(
        "sley-migrate status={} migrations={} schema_drift={} issues={}",
        report.status,
        report.summary.migration_count,
        report.summary.schema_drift_count,
        report.summary.issue_count
    );
    for migration in &report.migrations {
        println!(
            "{} category={} surface={}",
            migration.kind, migration.category, migration.surface
        );
    }
    for drift in &report.schema_drift {
        eprintln!("{} {}: {}", drift.kind, drift.schema_id, drift.message);
    }
    for issue in &report.issues {
        eprintln!("{}: {}", issue.code, issue.message);
    }
}

fn issue(code: impl Into<String>, message: impl Into<String>) -> MigrateIssue {
    MigrateIssue {
        code: code.into(),
        message: message.into(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
