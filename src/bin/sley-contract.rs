use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::Value as JsonValue;

const INVENTORY_SCHEMA: &str = "sley.contract.inventory.v0";
const FIXTURE_CHECK_SCHEMA: &str = "sley.contract.fixture_check.v0";
const VALIDATE_SCHEMA: &str = "sley.contract.validate.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-contract")]
#[command(about = "Inspect Sley JSON schemas and contract fixture roots")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List schema IDs available under a schema directory.
    Inventory {
        schema_dir: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Check that every fixture has a known top-level schema ID.
    CheckFixtures {
        fixtures_dir: PathBuf,
        #[arg(long, default_value = "docs/schemas")]
        schemas: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Check one report's top-level schema ID against the requested schema.
    Validate {
        #[arg(long)]
        schema: String,
        report: PathBuf,
        #[arg(long, default_value = "docs/schemas")]
        schemas: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct InventoryReport {
    schema: String,
    status: String,
    schema_dir: String,
    schema_count: usize,
    schemas: Vec<SchemaInfo>,
}

#[derive(Debug, Clone, Serialize)]
struct SchemaInfo {
    id: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    draft: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct FixtureCheckReport {
    schema: String,
    status: String,
    validation_level: String,
    schema_dir: String,
    fixtures_dir: String,
    fixture_count: usize,
    passed_count: usize,
    failed_count: usize,
    fixtures: Vec<FixtureCheck>,
}

#[derive(Debug, Serialize)]
struct FixtureCheck {
    path: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<ContractIssue>,
}

#[derive(Debug, Serialize)]
struct ValidateReport {
    schema: String,
    status: String,
    validation_level: String,
    requested_schema: String,
    report_path: String,
    schema_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    report_schema: Option<String>,
    issues: Vec<ContractIssue>,
}

#[derive(Debug, Serialize)]
struct ContractIssue {
    code: String,
    message: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inventory { schema_dir, json } => {
            let report = build_inventory_report(&schema_dir)?;
            emit_inventory(&report, json)?;
        }
        Command::CheckFixtures {
            fixtures_dir,
            schemas,
            json,
        } => {
            let report = build_fixture_check_report(&schemas, &fixtures_dir)?;
            let failed = report.status == "failed";
            emit_fixture_check(&report, json)?;
            if failed {
                std::process::exit(1);
            }
        }
        Command::Validate {
            schema,
            report,
            schemas,
            json,
        } => {
            let report = build_validate_report(&schemas, &schema, &report)?;
            let failed = report.status == "failed";
            emit_validate(&report, json)?;
            if failed {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn build_inventory_report(schema_dir: &Path) -> Result<InventoryReport> {
    let schemas = discover_schemas(schema_dir)?;
    Ok(InventoryReport {
        schema: INVENTORY_SCHEMA.to_string(),
        status: "passed".to_string(),
        schema_dir: path_string(schema_dir),
        schema_count: schemas.len(),
        schemas,
    })
}

fn build_fixture_check_report(
    schema_dir: &Path,
    fixtures_dir: &Path,
) -> Result<FixtureCheckReport> {
    let schema_ids = discover_schema_ids(schema_dir)?;
    let fixture_paths = list_json_files(fixtures_dir, false)?;
    let fixtures = fixture_paths
        .iter()
        .map(|path| check_fixture(path, &schema_ids))
        .collect::<Vec<_>>();
    let failed_count = fixtures
        .iter()
        .filter(|fixture| fixture.status == "failed")
        .count();
    let fixture_count = fixtures.len();

    Ok(FixtureCheckReport {
        schema: FIXTURE_CHECK_SCHEMA.to_string(),
        status: if failed_count == 0 {
            "passed"
        } else {
            "failed"
        }
        .to_string(),
        validation_level: "schema_root_match".to_string(),
        schema_dir: path_string(schema_dir),
        fixtures_dir: path_string(fixtures_dir),
        fixture_count,
        passed_count: fixture_count.saturating_sub(failed_count),
        failed_count,
        fixtures,
    })
}

fn build_validate_report(
    schema_dir: &Path,
    requested_schema: &str,
    report_path: &Path,
) -> Result<ValidateReport> {
    let schema_ids = discover_schema_ids(schema_dir)?;
    let mut issues = Vec::new();

    if !schema_ids.contains(requested_schema) {
        issues.push(issue(
            "unknown_requested_schema",
            format!(
                "schema {requested_schema:?} was not found in {}",
                schema_dir.display()
            ),
        ));
    }

    let (report_schema, parse_issue) = read_report_schema(report_path);
    if let Some(issue) = parse_issue {
        issues.push(issue);
    }
    if let Some(actual_schema) = report_schema.as_deref() {
        if actual_schema != requested_schema {
            issues.push(issue(
                "schema_mismatch",
                format!(
                    "report schema {actual_schema:?} does not match requested schema {requested_schema:?}"
                ),
            ));
        }
    }

    Ok(ValidateReport {
        schema: VALIDATE_SCHEMA.to_string(),
        status: if issues.is_empty() {
            "passed"
        } else {
            "failed"
        }
        .to_string(),
        validation_level: "schema_root_match".to_string(),
        requested_schema: requested_schema.to_string(),
        report_path: path_string(report_path),
        schema_dir: path_string(schema_dir),
        report_schema,
        issues,
    })
}

fn discover_schemas(schema_dir: &Path) -> Result<Vec<SchemaInfo>> {
    let mut seen = BTreeSet::new();
    let mut schemas = Vec::new();
    for path in list_json_files(schema_dir, true)? {
        let value = read_json_file(&path)?;
        let id = string_field(&value, "$id")
            .ok_or_else(|| anyhow!("schema file {} is missing $id", path.display()))?;
        if !seen.insert(id.clone()) {
            return Err(anyhow!("duplicate schema id {id:?} in {}", path.display()));
        }
        schemas.push(SchemaInfo {
            id,
            path: path_string(&path),
            title: string_field(&value, "title"),
            draft: string_field(&value, "$schema"),
            root_type: string_field(&value, "type"),
        });
    }
    if schemas.is_empty() {
        return Err(anyhow!("no schema files found in {}", schema_dir.display()));
    }
    schemas.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(schemas)
}

fn discover_schema_ids(schema_dir: &Path) -> Result<BTreeSet<String>> {
    Ok(discover_schemas(schema_dir)?
        .into_iter()
        .map(|schema| schema.id)
        .collect())
}

fn check_fixture(path: &Path, schema_ids: &BTreeSet<String>) -> FixtureCheck {
    let (schema_id, parse_issue) = read_report_schema(path);
    if let Some(issue) = parse_issue {
        return FixtureCheck {
            path: path_string(path),
            status: "failed".to_string(),
            schema_id,
            issue: Some(issue),
        };
    }

    let Some(schema_id) = schema_id else {
        return FixtureCheck {
            path: path_string(path),
            status: "failed".to_string(),
            schema_id: None,
            issue: Some(issue(
                "missing_schema",
                "fixture does not have a top-level string schema field",
            )),
        };
    };

    if !schema_ids.contains(&schema_id) {
        return FixtureCheck {
            path: path_string(path),
            status: "failed".to_string(),
            schema_id: Some(schema_id.clone()),
            issue: Some(issue(
                "unknown_schema",
                format!("fixture references unknown schema {schema_id:?}"),
            )),
        };
    }

    FixtureCheck {
        path: path_string(path),
        status: "passed".to_string(),
        schema_id: Some(schema_id),
        issue: None,
    }
}

fn read_report_schema(path: &Path) -> (Option<String>, Option<ContractIssue>) {
    match read_json_file(path) {
        Ok(value) => (string_field(&value, "schema"), None),
        Err(error) => (
            None,
            Some(issue(
                "invalid_json",
                format!("failed to parse {}: {error}", path.display()),
            )),
        ),
    }
}

fn list_json_files(dir: &Path, schema_files_only: bool) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry =
            entry.with_context(|| format!("failed to read entry under {}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if schema_files_only {
            if file_name.ends_with(".schema.json") {
                paths.push(path);
            }
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_json_file(path: &Path) -> Result<JsonValue> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON file {}", path.display()))?;
    serde_json::from_str(&source).with_context(|| format!("failed to parse {}", path.display()))
}

fn string_field(value: &JsonValue, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn issue(code: impl Into<String>, message: impl Into<String>) -> ContractIssue {
    ContractIssue {
        code: code.into(),
        message: message.into(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn emit_inventory(report: &InventoryReport, json: bool) -> Result<()> {
    if json {
        return print_json(report);
    }
    println!(
        "sley-contract inventory status={} schemas={}",
        report.status, report.schema_count
    );
    for schema in &report.schemas {
        println!("{} {}", schema.id, schema.path);
    }
    Ok(())
}

fn emit_fixture_check(report: &FixtureCheckReport, json: bool) -> Result<()> {
    if json {
        return print_json(report);
    }
    println!(
        "sley-contract check-fixtures status={} passed={} failed={} fixtures={}",
        report.status, report.passed_count, report.failed_count, report.fixture_count
    );
    for fixture in &report.fixtures {
        if fixture.status == "failed" {
            let message = fixture
                .issue
                .as_ref()
                .map(|issue| issue.message.as_str())
                .unwrap_or("unknown issue");
            println!("failed {} {}", fixture.path, message);
        }
    }
    Ok(())
}

fn emit_validate(report: &ValidateReport, json: bool) -> Result<()> {
    if json {
        return print_json(report);
    }
    println!(
        "sley-contract validate status={} schema={} report={}",
        report.status, report.requested_schema, report.report_path
    );
    for issue in &report.issues {
        println!("{} {}", issue.code, issue.message);
    }
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
