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
const VALIDATION_LEVEL: &str = "json_schema_draft_2020_12";

#[derive(Debug, Parser)]
#[command(name = "sley-contract")]
#[command(about = "Inspect Sley JSON schemas and validate contract fixtures")]
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
    /// Validate every fixture against its matching top-level schema ID.
    CheckFixtures {
        fixtures_dir: PathBuf,
        #[arg(long, default_value = "docs/schemas")]
        schemas: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Validate one report against the requested schema.
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
    issues: Vec<ContractIssue>,
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

#[derive(Debug)]
struct SchemaDocument {
    info: SchemaInfo,
    value: JsonValue,
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
    let schemas: Vec<SchemaInfo> = load_schema_documents(schema_dir)?
        .into_iter()
        .map(|schema| schema.info)
        .collect();
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
    let schemas = load_schema_documents(schema_dir)?;
    let registry = build_schema_registry(&schemas)?;
    let fixture_paths = list_json_files(fixtures_dir, false)?;
    let fixtures = fixture_paths
        .iter()
        .map(|path| check_fixture(path, &schemas, &registry))
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
        validation_level: VALIDATION_LEVEL.to_string(),
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
    let schemas = load_schema_documents(schema_dir)?;
    let registry = build_schema_registry(&schemas)?;
    let mut issues = Vec::new();
    let requested_document = find_schema(&schemas, requested_schema);

    if requested_document.is_none() {
        issues.push(issue(
            "unknown_requested_schema",
            format!(
                "schema {requested_schema:?} was not found in {}",
                schema_dir.display()
            ),
        ));
    }

    let (report_value, parse_issue) = read_json_report(report_path);
    if let Some(issue) = parse_issue {
        issues.push(issue);
    }

    let report_schema = report_value
        .as_ref()
        .and_then(|value| string_field(value, "schema"));

    match report_schema.as_deref() {
        Some(actual_schema) if actual_schema == requested_schema => {
            if let (Some(document), Some(value)) = (requested_document, report_value.as_ref()) {
                issues.extend(validate_instance(document, &registry, value));
            }
        }
        Some(actual_schema) => issues.push(issue(
            "schema_mismatch",
            format!(
                "report schema {actual_schema:?} does not match requested schema {requested_schema:?}"
            ),
        )),
        None if report_value.is_some() => issues.push(issue(
            "missing_schema",
            "report does not have a top-level string schema field",
        )),
        None => {}
    }

    Ok(ValidateReport {
        schema: VALIDATE_SCHEMA.to_string(),
        status: if issues.is_empty() {
            "passed"
        } else {
            "failed"
        }
        .to_string(),
        validation_level: VALIDATION_LEVEL.to_string(),
        requested_schema: requested_schema.to_string(),
        report_path: path_string(report_path),
        schema_dir: path_string(schema_dir),
        report_schema,
        issues,
    })
}

fn load_schema_documents(schema_dir: &Path) -> Result<Vec<SchemaDocument>> {
    let mut seen = BTreeSet::new();
    let mut schemas = Vec::new();
    for path in list_json_files(schema_dir, true)? {
        let value = read_json_file(&path)?;
        let id = string_field(&value, "$id")
            .ok_or_else(|| anyhow!("schema file {} is missing $id", path.display()))?;
        if !seen.insert(id.clone()) {
            return Err(anyhow!("duplicate schema id {id:?} in {}", path.display()));
        }
        schemas.push(SchemaDocument {
            info: SchemaInfo {
                id,
                path: path_string(&path),
                title: string_field(&value, "title"),
                draft: string_field(&value, "$schema"),
                root_type: string_field(&value, "type"),
            },
            value,
        });
    }
    if schemas.is_empty() {
        return Err(anyhow!("no schema files found in {}", schema_dir.display()));
    }
    schemas.sort_by(|left, right| left.info.id.cmp(&right.info.id));
    Ok(schemas)
}

fn build_schema_registry<'a>(schemas: &'a [SchemaDocument]) -> Result<jsonschema::Registry<'a>> {
    let mut builder = jsonschema::Registry::new();
    for schema in schemas {
        builder = builder
            .add(schema.info.id.as_str(), &schema.value)
            .map_err(|error| {
                anyhow!(
                    "failed to register schema {} from {}: {error}",
                    schema.info.id,
                    schema.info.path
                )
            })?;
    }
    builder
        .prepare()
        .map_err(|error| anyhow!("failed to prepare schema registry: {error}"))
}

fn find_schema<'a>(schemas: &'a [SchemaDocument], schema_id: &str) -> Option<&'a SchemaDocument> {
    schemas.iter().find(|schema| schema.info.id == schema_id)
}

fn check_fixture(
    path: &Path,
    schemas: &[SchemaDocument],
    registry: &jsonschema::Registry<'_>,
) -> FixtureCheck {
    let (value, parse_issue) = read_json_report(path);
    if let Some(issue) = parse_issue {
        return FixtureCheck {
            path: path_string(path),
            status: "failed".to_string(),
            schema_id: None,
            issues: vec![issue],
        };
    }

    let Some(value) = value else {
        return FixtureCheck {
            path: path_string(path),
            status: "failed".to_string(),
            schema_id: None,
            issues: vec![issue(
                "invalid_json",
                "fixture could not be read as a JSON report",
            )],
        };
    };

    let schema_id = string_field(&value, "schema");
    let Some(schema_id) = schema_id else {
        return FixtureCheck {
            path: path_string(path),
            status: "failed".to_string(),
            schema_id: None,
            issues: vec![issue(
                "missing_schema",
                "fixture does not have a top-level string schema field",
            )],
        };
    };

    let Some(schema) = find_schema(schemas, &schema_id) else {
        return FixtureCheck {
            path: path_string(path),
            status: "failed".to_string(),
            schema_id: Some(schema_id.clone()),
            issues: vec![issue(
                "unknown_schema",
                format!("fixture references unknown schema {schema_id:?}"),
            )],
        };
    };

    let issues = validate_instance(schema, registry, &value);
    FixtureCheck {
        path: path_string(path),
        status: if issues.is_empty() {
            "passed"
        } else {
            "failed"
        }
        .to_string(),
        schema_id: Some(schema_id),
        issues,
    }
}

fn validate_instance(
    schema: &SchemaDocument,
    registry: &jsonschema::Registry<'_>,
    instance: &JsonValue,
) -> Vec<ContractIssue> {
    let validator = match jsonschema::options()
        .with_registry(registry)
        .with_base_uri(schema.info.id.clone())
        .build(&schema.value)
    {
        Ok(validator) => validator,
        Err(error) => {
            return vec![issue(
                "invalid_schema",
                format!(
                    "schema {} from {} could not be compiled: {error}",
                    schema.info.id, schema.info.path
                ),
            )];
        }
    };

    validator
        .iter_errors(instance)
        .map(|error| {
            issue(
                "schema_validation_error",
                format!(
                    "{} at instance {} against schema {}",
                    error,
                    error.instance_path(),
                    error.schema_path()
                ),
            )
        })
        .collect()
}

fn read_json_report(path: &Path) -> (Option<JsonValue>, Option<ContractIssue>) {
    match read_json_file(path) {
        Ok(value) => (Some(value), None),
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
            if fixture.issues.is_empty() {
                println!("failed {} unknown issue", fixture.path);
                continue;
            }
            for issue in &fixture.issues {
                println!("failed {} {} {}", fixture.path, issue.code, issue.message);
            }
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
