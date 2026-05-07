use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::process::Output;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

const CONFORMANCE_REPORT_SCHEMA: &str = "sley.conformance.report.v0";
const CONFORMANCE_COVERAGE_SCHEMA: &str = "sley.conformance.coverage.v0";

const DEFAULT_CORPUS_TAGS: &[&str] = &[
    "accepted:DatabaseRead",
    "accepted:DatabaseWrite",
    "accepted:Deploy",
    "accepted:FileRead",
    "accepted:FileWrite",
    "accepted:ModelCall",
    "accepted:Network",
    "accepted:SecretRead",
    "accepted:Shell",
    "accepted:Spend",
    "accepted:agent-data-authority",
    "accepted:agent-deploy-pipeline",
    "accepted:agent-spend-authority",
    "accepted:agent-split-authority",
    "authority:transitive-effects",
    "diagnostic:DUPLICATE_EFFECT",
    "diagnostic:DUPLICATE_TASK",
    "diagnostic:DUPLICATE_TYPE",
    "diagnostic:EFFECT_UNAUTHORIZED",
    "diagnostic:MISSING_RETURN",
    "diagnostic:TYPE_MISMATCH",
    "diagnostic:UNKNOWN_IDENTIFIER",
    "formatter:round-trip",
    "language:module-namespace",
    "language:type-alias",
    "language:type-alias-transparent",
    "rejected:DatabaseRead",
    "rejected:DatabaseWrite",
    "rejected:Deploy",
    "rejected:FileRead",
    "rejected:FileWrite",
    "rejected:ModelCall",
    "rejected:Network",
    "rejected:SecretRead",
    "rejected:Shell",
    "rejected:Spend",
    "rejected:agent-transitive-effect",
    "rejected:data-write-transitive-effect",
    "rejected:spend-transitive-effect",
];

const DEFAULT_SMOKE_TAGS: &[&str] = &[
    "agent-bench:unused-private-task-repair",
    "ci:corpus",
    "cli:sley-ci",
    "cli:sley-agent-bench",
    "cli:ast",
    "cli:check",
    "cli:deploy",
    "cli:doctor",
    "cli:fix",
    "cli:format",
    "cli:graft-dry-run",
    "cli:graph",
    "cli:lint",
    "cli:new",
    "cli:parse",
    "cli:plan",
    "cli:query",
    "cli:run",
    "cli:seal",
    "cli:sley-conformance",
    "cli:sley-contract",
    "cli:sley-docgen",
    "cli:sley-lsp",
    "cli:sley-migrate",
    "cli:sley-sandbox-runner",
    "cli:sley-workbench",
    "cli:sley-zjx",
    "cli:trace",
    "cli:verify",
    "cli:zjx",
    "contract:check-fixtures",
    "contract:inspect-deploy-artifacts",
    "contract:inventory",
    "contract:validate",
    "conformance:coverage",
    "diagnostic:RUNTIME_CAPABILITY_SCOPE_DENIED",
    "diagnostic:RETURN_TYPE_MISMATCH",
    "docgen:reference",
    "graft:templates:declaration-surface",
    "graft:templates:module-parent-surface",
    "graft:templates:module-surface",
    "graft:templates:replace-task-body",
    "graph-slice:delete-affordances",
    "graph-slice:inbound-calls",
    "graph-slice:insert-affordances",
    "graph-slice:module-focus",
    "graph-slice:move-affordances",
    "graph-slice:replace-affordances",
    "host:scoped-capability",
    "json:sley.agent_bench.report.v0",
    "json:sley.ci.report.v0",
    "json:sley.conformance.coverage.v0",
    "json:sley.contract.fixture_check.v0",
    "json:sley.contract.inventory.v0",
    "json:sley.contract.validate.v0",
    "json:sley.deploy.artifact_check.v0",
    "json:sley.docgen.report.v0",
    "json:sley.migrate.report.v0",
    "json:sley.sandbox.report.v0",
    "json:sley.workbench.report.v0",
    "json:sley.zjx.tool.report.v0",
    "language:type-alias",
    "language:type-alias-transparent",
    "lsp:help",
    "migrate:raw-host-adapter",
    "migrate:unchecked-result",
    "readiness:agent-bench-repair-loop",
    "readiness:agent-quickstart-path",
    "readiness:ci-corpus-gate",
    "readiness:conformance-coverage",
    "readiness:contract-schema-defaults",
    "readiness:deploy-package-artifact-inspection",
    "readiness:docgen-reference",
    "readiness:lsp-stdio-startup",
    "readiness:migrate-report",
    "readiness:sandbox-replay",
    "readiness:workbench-inspection",
    "readiness:zjx-tool-inspect",
    "runtime:seeded-host-authority",
    "sandbox:replay",
    "workbench:graph-slice",
    "zjx-tool:inspect",
];

const DEFAULT_V1_GATE_TARGETS: &[&str] = &[
    "fmt",
    "diff-check",
    "test",
    "contracts",
    "conformance",
    "corpus",
    "examples",
    "smoke",
    "lsp",
    "editor-shims",
    "workbench",
    "agent-bench",
    "migrate",
    "docgen",
    "sandbox-runner",
    "zjx-tools",
    "syntax",
];

#[derive(Debug, Parser)]
#[command(name = "sley-conformance")]
#[command(about = "Summarize Sley release-readiness contracts and coverage")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build a conformance summary report from the current release fixtures.
    Report {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "docs/schemas")]
        schema_dir: PathBuf,
        #[arg(long, default_value = "fixtures/contracts")]
        fixtures_dir: PathBuf,
        #[arg(long, default_value = "fixtures/corpus/manifest.json")]
        corpus_manifest: PathBuf,
        #[arg(long = "smoke-manifest")]
        smoke_manifest: Vec<PathBuf>,
        #[arg(long, default_value = "examples")]
        examples_root: PathBuf,
        #[arg(long, default_value = "tests/sley_v0.rs")]
        integration_tests: PathBuf,
        #[arg(long, default_value = "SleyGoal.md")]
        goal_doc: PathBuf,
        #[arg(long, default_value = "editors/vscode-sley")]
        editor_shim_root: PathBuf,
        #[arg(long, default_value = "Makefile")]
        makefile: PathBuf,
        #[arg(long, default_value = "Cargo.toml")]
        cargo_manifest: PathBuf,
        #[arg(long, default_value = "LICENSE")]
        license_file: PathBuf,
        #[arg(long, default_value = "tree-sitter-sley/package.json")]
        tree_sitter_package: PathBuf,
        #[arg(long, default_value = "tree-sitter-sley/tree-sitter.json")]
        tree_sitter_config: PathBuf,
        #[arg(long)]
        sley_contract_bin: Option<PathBuf>,
        #[arg(long)]
        require_public_release_ready: bool,
        #[arg(long)]
        markdown: Option<PathBuf>,
        #[arg(long)]
        html: Option<PathBuf>,
    },
    /// Check that corpus or smoke coverage tags are present.
    Coverage {
        #[arg(long)]
        json: bool,
        #[arg(long = "require-tag")]
        require_tag: Vec<String>,
        #[arg(long, default_value = "fixtures/corpus/manifest.json")]
        corpus_manifest: PathBuf,
        #[arg(long = "smoke-manifest")]
        smoke_manifest: Vec<PathBuf>,
    },
}

#[derive(Debug, Serialize)]
struct ConformanceReport {
    schema: String,
    status: String,
    summary: ConformanceSummary,
    validation: ValidationSection,
    schemas: SchemaSection,
    corpus: CorpusSection,
    smoke: SmokeSection,
    examples: ExamplesSection,
    tests: TestSection,
    editor_shims: EditorShimsSection,
    v1_gate: V1GateSection,
    release: ReleaseSection,
    issues: Vec<ConformanceIssue>,
}

#[derive(Debug, Serialize)]
struct ConformanceSummary {
    schema_count: usize,
    schema_instance_count: usize,
    schema_without_instance_count: usize,
    contract_fixture_count: usize,
    contract_fixture_failed_count: usize,
    corpus_accepted_count: usize,
    corpus_rejected_count: usize,
    smoke_case_count: usize,
    example_project_count: usize,
    example_source_count: usize,
    integration_test_count: usize,
    declared_integration_test_count: Option<usize>,
    test_count_matches_declared: bool,
    editor_shim_count: usize,
    v1_gate_target_count: usize,
    missing_v1_gate_target_count: usize,
    public_release_blocker_count: usize,
    issue_count: usize,
}

#[derive(Debug, Serialize)]
struct ValidationSection {
    contract_fixtures: ValidationRun,
    manifests: Vec<ValidationRun>,
}

#[derive(Debug, Serialize)]
struct ValidationRun {
    name: String,
    status: String,
    command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    report_schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixture_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    passed_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failed_count: Option<usize>,
    issue_count: usize,
    issues: Vec<ConformanceIssue>,
}

#[derive(Debug, Serialize)]
struct SchemaSection {
    schema_dir: String,
    fixture_dir: String,
    schemas: Vec<SchemaInfo>,
    instances: Vec<SchemaInstanceCount>,
    schemas_without_instances: Vec<String>,
    unknown_instance_schemas: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SchemaInfo {
    id: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[derive(Debug, Serialize)]
struct SchemaInstanceCount {
    schema_id: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct CorpusSection {
    manifest: String,
    accepted_count: usize,
    rejected_count: usize,
    tag_inventory: Vec<TagCount>,
    feature_counts: Vec<TagCount>,
    required_tags: Vec<String>,
    missing_required_tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SmokeSection {
    manifests: Vec<SmokeManifestSummary>,
    case_count: usize,
    tag_inventory: Vec<TagCount>,
    required_tags: Vec<String>,
    missing_required_tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SmokeManifestSummary {
    path: String,
    case_count: usize,
}

#[derive(Debug, Serialize)]
struct ExamplesSection {
    root: String,
    project_count: usize,
    source_count: usize,
}

#[derive(Debug, Serialize)]
struct TestSection {
    integration_test_file: String,
    integration_test_count: usize,
    goal_doc: String,
    declared_integration_test_count: Option<usize>,
    declared_matches_actual: bool,
}

#[derive(Debug, Serialize)]
struct EditorShimsSection {
    root: String,
    package_count: usize,
    packages: Vec<EditorShimPackage>,
    validation: ValidationRun,
    issue_count: usize,
    issues: Vec<ConformanceIssue>,
}

#[derive(Debug, Serialize)]
struct EditorShimPackage {
    path: String,
    name: Option<String>,
    display_name: Option<String>,
    version: Option<String>,
    private: Option<bool>,
    language_ids: Vec<String>,
    grammar_count: usize,
    command_count: usize,
    dependency_count: usize,
}

#[derive(Debug, Serialize)]
struct V1GateSection {
    makefile: String,
    target: String,
    required_targets: Vec<String>,
    actual_targets: Vec<String>,
    missing_required_targets: Vec<String>,
    undefined_targets: Vec<String>,
    issue_count: usize,
    issues: Vec<ConformanceIssue>,
}

#[derive(Debug, Serialize)]
struct ReleaseSection {
    public_release_ready: bool,
    blocker_count: usize,
    blockers: Vec<ConformanceIssue>,
    cargo_package: CargoPackageSection,
    tree_sitter_package: TreeSitterPackageSection,
    license: LicenseSection,
}

#[derive(Debug, Serialize)]
struct CargoPackageSection {
    manifest: String,
    name: Option<String>,
    version: Option<String>,
    edition: Option<String>,
    rust_version: Option<String>,
    description_present: bool,
    readme_present: bool,
    repository_present: bool,
    license: Option<String>,
    license_file: Option<String>,
    publish: Option<String>,
    publish_restricted: bool,
}

#[derive(Debug, Serialize)]
struct TreeSitterPackageSection {
    package_json: String,
    config: String,
    name: Option<String>,
    version: Option<String>,
    private: Option<bool>,
    package_license: Option<String>,
    package_description_present: bool,
    config_license: Option<String>,
    config_description_present: bool,
}

#[derive(Debug, Serialize)]
struct LicenseSection {
    file: String,
    present: bool,
    operator_decision_required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct TagCount {
    tag: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ConformanceIssue {
    code: String,
    message: String,
}

#[derive(Debug, Clone)]
struct ContractCommand {
    program: PathBuf,
    prefix_args: Vec<String>,
    display: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CoverageReport {
    schema: String,
    status: String,
    required_tags: Vec<String>,
    missing_tags: Vec<String>,
    tag_inventory: Vec<TagCount>,
    issues: Vec<ConformanceIssue>,
}

#[derive(Debug, Deserialize)]
struct CorpusManifest {
    accepted: Vec<ManifestCase>,
    rejected: Vec<ManifestCase>,
}

#[derive(Debug, Deserialize)]
struct ManifestCase {
    covers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SmokeManifest {
    cases: Vec<SmokeCase>,
}

#[derive(Debug, Deserialize)]
struct SmokeCase {
    covers: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Report {
            json,
            schema_dir,
            fixtures_dir,
            corpus_manifest,
            smoke_manifest,
            examples_root,
            integration_tests,
            goal_doc,
            editor_shim_root,
            makefile,
            cargo_manifest,
            license_file,
            tree_sitter_package,
            tree_sitter_config,
            sley_contract_bin,
            require_public_release_ready,
            markdown,
            html,
        } => {
            let corpus_manifest = normalize_manifest_path(corpus_manifest);
            let smoke_manifests = normalize_smoke_manifests(smoke_manifest);
            let mut report = build_report(
                &schema_dir,
                &fixtures_dir,
                &corpus_manifest,
                &smoke_manifests,
                &examples_root,
                &integration_tests,
                &goal_doc,
                &editor_shim_root,
                &makefile,
                &cargo_manifest,
                &license_file,
                &tree_sitter_package,
                &tree_sitter_config,
                sley_contract_bin.as_deref(),
            );
            if require_public_release_ready {
                apply_public_release_gate(&mut report);
            }
            if let Some(path) = markdown {
                fs::write(&path, render_markdown(&report))
                    .with_context(|| format!("failed to write {}", path.display()))?;
            }
            if let Some(path) = html {
                fs::write(&path, render_html(&report))
                    .with_context(|| format!("failed to write {}", path.display()))?;
            }
            emit_report(&report, json)?;
            if report.status == "failed" {
                std::process::exit(1);
            }
        }
        Command::Coverage {
            json,
            require_tag,
            corpus_manifest,
            smoke_manifest,
        } => {
            let corpus_manifest = normalize_manifest_path(corpus_manifest);
            let smoke_manifests = normalize_smoke_manifests(smoke_manifest);
            let report = build_coverage_report(&corpus_manifest, &smoke_manifests, require_tag);
            emit_coverage(&report, json)?;
            if report.status == "failed" {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn apply_public_release_gate(report: &mut ConformanceReport) {
    if report.release.public_release_ready {
        return;
    }
    report.issues.push(issue(
        "public_release_not_ready",
        format!(
            "`--require-public-release-ready` was set but public release has {} blockers",
            report.release.blocker_count
        ),
    ));
    report.summary.issue_count = report.issues.len();
    report.status = "failed".to_string();
}

fn build_report(
    schema_dir: &Path,
    fixtures_dir: &Path,
    corpus_manifest: &Path,
    smoke_manifests: &[PathBuf],
    examples_root: &Path,
    integration_tests: &Path,
    goal_doc: &Path,
    editor_shim_root: &Path,
    makefile: &Path,
    cargo_manifest: &Path,
    license_file: &Path,
    tree_sitter_package: &Path,
    tree_sitter_config: &Path,
    sley_contract_bin: Option<&Path>,
) -> ConformanceReport {
    let mut issues = Vec::new();
    let schemas = match load_schemas(schema_dir) {
        Ok(schemas) => schemas,
        Err(error) => {
            issues.push(issue(
                "schema_inventory_failed",
                format!(
                    "failed to load schemas from {}: {error}",
                    schema_dir.display()
                ),
            ));
            Vec::new()
        }
    };
    let fixture_instances = match load_fixture_schema_ids(fixtures_dir) {
        Ok(ids) => ids,
        Err(error) => {
            issues.push(issue(
                "fixture_inventory_failed",
                format!(
                    "failed to load fixture schema IDs from {}: {error}",
                    fixtures_dir.display()
                ),
            ));
            Vec::new()
        }
    };
    let manifest_instances =
        load_manifest_schema_ids(corpus_manifest, smoke_manifests, &mut issues);
    let mut all_instances = fixture_instances;
    all_instances.extend(manifest_instances);

    let schema_ids = schemas
        .iter()
        .map(|schema| schema.id.clone())
        .collect::<BTreeSet<_>>();
    let instance_counts = count_strings(all_instances.iter().cloned());
    let schemas_without_instances = schema_ids
        .iter()
        .filter(|schema_id| !instance_counts.contains_key(*schema_id))
        .cloned()
        .collect::<Vec<_>>();
    let unknown_instance_schemas = instance_counts
        .keys()
        .filter(|schema_id| !schema_ids.contains(*schema_id))
        .cloned()
        .collect::<Vec<_>>();
    for schema_id in &schemas_without_instances {
        issues.push(issue(
            "schema_without_instance",
            format!("schema {schema_id:?} has no contract fixture or release manifest instance"),
        ));
    }
    for schema_id in &unknown_instance_schemas {
        issues.push(issue(
            "unknown_instance_schema",
            format!("fixture or manifest references unknown schema {schema_id:?}"),
        ));
    }

    let corpus = build_corpus_section(corpus_manifest, &mut issues);
    let smoke = build_smoke_section(smoke_manifests, &mut issues);
    for tag in &corpus.missing_required_tags {
        issues.push(issue(
            "missing_corpus_tag",
            format!("corpus manifest does not cover required tag {tag:?}"),
        ));
    }
    for tag in &smoke.missing_required_tags {
        issues.push(issue(
            "missing_smoke_tag",
            format!("smoke manifest does not cover required tag {tag:?}"),
        ));
    }
    let examples = build_examples_section(examples_root, &mut issues);
    let tests = build_test_section(integration_tests, goal_doc, &mut issues);
    let editor_shims = build_editor_shims_section(editor_shim_root);
    collect_editor_shim_issues(&editor_shims, &mut issues);
    let v1_gate = build_v1_gate_section(makefile);
    collect_v1_gate_issues(&v1_gate, &mut issues);
    let release = build_release_section(
        cargo_manifest,
        license_file,
        tree_sitter_package,
        tree_sitter_config,
    );
    let validation = build_validation_section(
        sley_contract_bin,
        schema_dir,
        fixtures_dir,
        corpus_manifest,
        smoke_manifests,
    );
    collect_validation_issues(&validation, &mut issues);

    let schema_instance_count = instance_counts.values().sum::<usize>();
    let contract_fixture_count = validation.contract_fixtures.fixture_count.unwrap_or(0);
    let contract_fixture_failed_count = validation.contract_fixtures.failed_count.unwrap_or(0);
    let issue_count = issues.len();
    let summary = ConformanceSummary {
        schema_count: schemas.len(),
        schema_instance_count,
        schema_without_instance_count: schemas_without_instances.len(),
        contract_fixture_count,
        contract_fixture_failed_count,
        corpus_accepted_count: corpus.accepted_count,
        corpus_rejected_count: corpus.rejected_count,
        smoke_case_count: smoke.case_count,
        example_project_count: examples.project_count,
        example_source_count: examples.source_count,
        integration_test_count: tests.integration_test_count,
        declared_integration_test_count: tests.declared_integration_test_count,
        test_count_matches_declared: tests.declared_matches_actual,
        editor_shim_count: editor_shims.package_count,
        v1_gate_target_count: v1_gate.actual_targets.len(),
        missing_v1_gate_target_count: v1_gate.missing_required_targets.len(),
        public_release_blocker_count: release.blocker_count,
        issue_count,
    };
    let status = if issues.is_empty() {
        "passed"
    } else {
        "failed"
    }
    .to_string();
    ConformanceReport {
        schema: CONFORMANCE_REPORT_SCHEMA.to_string(),
        status,
        summary,
        validation,
        schemas: SchemaSection {
            schema_dir: path_string(schema_dir),
            fixture_dir: path_string(fixtures_dir),
            schemas,
            instances: instance_counts
                .into_iter()
                .map(|(schema_id, count)| SchemaInstanceCount { schema_id, count })
                .collect(),
            schemas_without_instances,
            unknown_instance_schemas,
        },
        corpus,
        smoke,
        examples,
        tests,
        editor_shims,
        v1_gate,
        release,
        issues,
    }
}

fn build_coverage_report(
    corpus_manifest: &Path,
    smoke_manifests: &[PathBuf],
    required_tags: Vec<String>,
) -> CoverageReport {
    let mut issues = Vec::new();
    let mut tags = Vec::new();
    match read_json_file::<CorpusManifest>(corpus_manifest) {
        Ok(manifest) => {
            for case in manifest.accepted.iter().chain(manifest.rejected.iter()) {
                tags.extend(case.covers.clone());
            }
        }
        Err(error) => issues.push(issue(
            "corpus_manifest_read_failed",
            format!("failed to read {}: {error}", corpus_manifest.display()),
        )),
    }
    for path in smoke_manifests {
        match read_json_file::<SmokeManifest>(path) {
            Ok(manifest) => {
                for case in manifest.cases {
                    tags.extend(case.covers);
                }
            }
            Err(error) => issues.push(issue(
                "smoke_manifest_read_failed",
                format!("failed to read {}: {error}", path.display()),
            )),
        }
    }
    let inventory = count_strings(tags).into_iter().collect::<BTreeMap<_, _>>();
    let missing_tags = required_tags
        .iter()
        .filter(|tag| !inventory.contains_key(*tag))
        .cloned()
        .collect::<Vec<_>>();
    for tag in &missing_tags {
        issues.push(issue(
            "missing_required_tag",
            format!("required coverage tag {tag:?} was not found"),
        ));
    }
    let status = if issues.is_empty() {
        "passed"
    } else {
        "failed"
    }
    .to_string();
    CoverageReport {
        schema: CONFORMANCE_COVERAGE_SCHEMA.to_string(),
        status,
        required_tags,
        missing_tags,
        tag_inventory: inventory
            .into_iter()
            .map(|(tag, count)| TagCount { tag, count })
            .collect(),
        issues,
    }
}

fn build_validation_section(
    sley_contract_bin: Option<&Path>,
    schema_dir: &Path,
    fixtures_dir: &Path,
    corpus_manifest: &Path,
    smoke_manifests: &[PathBuf],
) -> ValidationSection {
    let command = sley_contract_bin
        .map(ContractCommand::from_binary)
        .unwrap_or_else(find_sley_contract_command);
    let contract_fixtures = run_contract_fixtures(&command, schema_dir, fixtures_dir);
    let mut manifests = Vec::new();
    manifests.push(run_contract_validate(
        &command,
        "corpus_manifest",
        "sley.conformance.manifest.v0",
        corpus_manifest,
        schema_dir,
    ));
    for (index, path) in smoke_manifests.iter().enumerate() {
        manifests.push(run_contract_validate(
            &command,
            &format!("smoke_manifest_{index}"),
            "sley.cli_smoke.manifest.v0",
            path,
            schema_dir,
        ));
    }
    ValidationSection {
        contract_fixtures,
        manifests,
    }
}

fn run_contract_fixtures(
    command: &ContractCommand,
    schema_dir: &Path,
    fixtures_dir: &Path,
) -> ValidationRun {
    let args = vec![
        "check-fixtures".to_string(),
        path_string(fixtures_dir),
        "--schemas".to_string(),
        path_string(schema_dir),
        "--json".to_string(),
    ];
    let mut run = run_contract_command(command, "contract_fixtures", args);
    if let Some(value) = run_contract_json(command, &run.command) {
        run.report_schema = string_field(&value, "schema");
        run.fixture_count = value
            .pointer("/fixture_count")
            .and_then(JsonValue::as_u64)
            .map(|value| value as usize);
        run.passed_count = value
            .pointer("/passed_count")
            .and_then(JsonValue::as_u64)
            .map(|value| value as usize);
        run.failed_count = value
            .pointer("/failed_count")
            .and_then(JsonValue::as_u64)
            .map(|value| value as usize);
        run.issue_count = value
            .pointer("/fixtures")
            .and_then(JsonValue::as_array)
            .map(|fixtures| {
                fixtures
                    .iter()
                    .map(|fixture| {
                        fixture
                            .pointer("/issues")
                            .and_then(JsonValue::as_array)
                            .map(Vec::len)
                            .unwrap_or(0)
                    })
                    .sum()
            })
            .unwrap_or(run.issue_count);
    }
    run
}

fn run_contract_validate(
    command: &ContractCommand,
    name: &str,
    schema: &str,
    report_path: &Path,
    schema_dir: &Path,
) -> ValidationRun {
    let args = vec![
        "validate".to_string(),
        "--schema".to_string(),
        schema.to_string(),
        path_string(report_path),
        "--schemas".to_string(),
        path_string(schema_dir),
        "--json".to_string(),
    ];
    let mut run = run_contract_command(command, name, args);
    if let Some(value) = run_contract_json(command, &run.command) {
        run.report_schema = string_field(&value, "schema");
        run.issue_count = value
            .pointer("/issues")
            .and_then(JsonValue::as_array)
            .map(Vec::len)
            .unwrap_or(run.issue_count);
    }
    run
}

fn run_contract_command(
    contract_command: &ContractCommand,
    name: &str,
    args: Vec<String>,
) -> ValidationRun {
    let command = contract_command.with_args(&args);
    match contract_command.output(&args) {
        Ok(output) => {
            let mut issues = Vec::new();
            if !output.status.success() {
                issues.push(issue(
                    "contract_command_failed",
                    format!(
                        "{} exited unsuccessfully; stderr={:?}",
                        name,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
            ValidationRun {
                name: name.to_string(),
                status: if output.status.success() {
                    "passed"
                } else {
                    "failed"
                }
                .to_string(),
                command,
                report_schema: None,
                fixture_count: None,
                passed_count: None,
                failed_count: None,
                issue_count: issues.len(),
                issues,
            }
        }
        Err(error) => ValidationRun {
            name: name.to_string(),
            status: "failed".to_string(),
            command,
            report_schema: None,
            fixture_count: None,
            passed_count: None,
            failed_count: None,
            issue_count: 1,
            issues: vec![issue(
                "contract_command_spawn_failed",
                format!(
                    "failed to run {}: {error}",
                    contract_command.display.join(" ")
                ),
            )],
        },
    }
}

fn run_contract_json(contract_command: &ContractCommand, command: &[String]) -> Option<JsonValue> {
    let args = command
        .iter()
        .skip(contract_command.display.len())
        .cloned()
        .collect::<Vec<_>>();
    let output = contract_command.output(&args).ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn collect_validation_issues(validation: &ValidationSection, issues: &mut Vec<ConformanceIssue>) {
    if validation.contract_fixtures.status != "passed" {
        issues.push(issue(
            "contract_fixtures_failed",
            "contract fixture validation did not pass",
        ));
    }
    for issue in &validation.contract_fixtures.issues {
        issues.push(issue.clone());
    }
    for manifest in &validation.manifests {
        if manifest.status != "passed" {
            issues.push(issue(
                "manifest_validation_failed",
                format!("{} validation did not pass", manifest.name),
            ));
        }
        for issue in &manifest.issues {
            issues.push(issue.clone());
        }
    }
}

fn collect_editor_shim_issues(
    editor_shims: &EditorShimsSection,
    issues: &mut Vec<ConformanceIssue>,
) {
    if editor_shims.validation.status != "passed" {
        issues.push(issue(
            "editor_shims_failed",
            "editor shim validation did not pass",
        ));
    }
    for issue in &editor_shims.validation.issues {
        issues.push(issue.clone());
    }
    for issue in &editor_shims.issues {
        issues.push(issue.clone());
    }
}

fn collect_v1_gate_issues(v1_gate: &V1GateSection, issues: &mut Vec<ConformanceIssue>) {
    for issue in &v1_gate.issues {
        issues.push(issue.clone());
    }
}

fn build_corpus_section(path: &Path, issues: &mut Vec<ConformanceIssue>) -> CorpusSection {
    let required_tags = DEFAULT_CORPUS_TAGS
        .iter()
        .map(|tag| (*tag).to_string())
        .collect::<Vec<_>>();
    match read_json_file::<CorpusManifest>(path) {
        Ok(manifest) => {
            let mut tags = Vec::new();
            for case in manifest.accepted.iter().chain(manifest.rejected.iter()) {
                tags.extend(case.covers.clone());
            }
            let tag_set = tags.iter().cloned().collect::<BTreeSet<_>>();
            let missing_required_tags = required_tags
                .iter()
                .filter(|tag| !tag_set.contains(*tag))
                .cloned()
                .collect::<Vec<_>>();
            CorpusSection {
                manifest: path_string(path),
                accepted_count: manifest.accepted.len(),
                rejected_count: manifest.rejected.len(),
                feature_counts: tag_counts(tags.iter().map(|tag| feature_prefix(tag))),
                tag_inventory: tag_counts(tags.into_iter()),
                required_tags,
                missing_required_tags,
            }
        }
        Err(error) => {
            issues.push(issue(
                "corpus_manifest_read_failed",
                format!("failed to read {}: {error}", path.display()),
            ));
            CorpusSection {
                manifest: path_string(path),
                accepted_count: 0,
                rejected_count: 0,
                tag_inventory: Vec::new(),
                feature_counts: Vec::new(),
                required_tags,
                missing_required_tags: Vec::new(),
            }
        }
    }
}

fn build_smoke_section(paths: &[PathBuf], issues: &mut Vec<ConformanceIssue>) -> SmokeSection {
    let mut manifests = Vec::new();
    let mut tags = Vec::new();
    for path in paths {
        match read_json_file::<SmokeManifest>(path) {
            Ok(manifest) => {
                let case_count = manifest.cases.len();
                for case in manifest.cases {
                    tags.extend(case.covers);
                }
                manifests.push(SmokeManifestSummary {
                    path: path_string(path),
                    case_count,
                });
            }
            Err(error) => issues.push(issue(
                "smoke_manifest_read_failed",
                format!("failed to read {}: {error}", path.display()),
            )),
        }
    }
    let tag_set = tags.iter().cloned().collect::<BTreeSet<_>>();
    let required_tags = DEFAULT_SMOKE_TAGS
        .iter()
        .map(|tag| (*tag).to_string())
        .collect::<Vec<_>>();
    let missing_required_tags = required_tags
        .iter()
        .filter(|tag| !tag_set.contains(*tag))
        .cloned()
        .collect::<Vec<_>>();
    SmokeSection {
        case_count: manifests.iter().map(|manifest| manifest.case_count).sum(),
        manifests,
        tag_inventory: tag_counts(tags),
        required_tags,
        missing_required_tags,
    }
}

fn build_examples_section(root: &Path, issues: &mut Vec<ConformanceIssue>) -> ExamplesSection {
    let mut project_count = 0;
    let mut source_count = 0;
    if let Err(error) = collect_examples(root, &mut project_count, &mut source_count) {
        issues.push(issue(
            "examples_inventory_failed",
            format!(
                "failed to inspect examples under {}: {error}",
                root.display()
            ),
        ));
    }
    ExamplesSection {
        root: path_string(root),
        project_count,
        source_count,
    }
}

fn build_test_section(
    integration_tests: &Path,
    goal_doc: &Path,
    issues: &mut Vec<ConformanceIssue>,
) -> TestSection {
    let integration_test_count = match fs::read_to_string(integration_tests) {
        Ok(source) => source
            .lines()
            .filter(|line| line.trim() == "#[test]")
            .count(),
        Err(error) => {
            issues.push(issue(
                "integration_test_inventory_failed",
                format!("failed to read {}: {error}", integration_tests.display()),
            ));
            0
        }
    };
    let declared_integration_test_count = match fs::read_to_string(goal_doc) {
        Ok(source) => parse_declared_integration_test_count(&source),
        Err(error) => {
            issues.push(issue(
                "integration_test_declaration_read_failed",
                format!("failed to read {}: {error}", goal_doc.display()),
            ));
            None
        }
    };
    if declared_integration_test_count.is_none() {
        issues.push(issue(
            "integration_test_declaration_missing",
            format!(
                "{} does not declare the current integration coverage count",
                goal_doc.display()
            ),
        ));
    }
    let declared_matches_actual = declared_integration_test_count == Some(integration_test_count);
    if !declared_matches_actual {
        issues.push(issue(
            "integration_test_count_mismatch",
            format!(
                "integration test count is {}, but {} declares {:?}",
                integration_test_count,
                goal_doc.display(),
                declared_integration_test_count
            ),
        ));
    }
    TestSection {
        integration_test_file: path_string(integration_tests),
        integration_test_count,
        goal_doc: path_string(goal_doc),
        declared_integration_test_count,
        declared_matches_actual,
    }
}

fn build_editor_shims_section(root: &Path) -> EditorShimsSection {
    let mut issues = Vec::new();
    let mut packages = Vec::new();
    let package_json = root.join("package.json");
    if !root.is_dir() {
        issues.push(issue(
            "editor_shim_root_missing",
            format!("{} is not an editor shim directory", root.display()),
        ));
    } else if !package_json.is_file() {
        issues.push(issue(
            "editor_shim_package_missing",
            format!("{} is not present", package_json.display()),
        ));
    } else if let Some(package) = build_editor_shim_package(&package_json, &mut issues) {
        packages.push(package);
    }
    let validation = run_editor_shim_validation(root);
    let issue_count = issues.len();
    EditorShimsSection {
        root: path_string(root),
        package_count: packages.len(),
        packages,
        validation,
        issue_count,
        issues,
    }
}

fn build_editor_shim_package(
    package_json: &Path,
    issues: &mut Vec<ConformanceIssue>,
) -> Option<EditorShimPackage> {
    let value = match read_json_value(package_json) {
        Ok(value) => value,
        Err(error) => {
            issues.push(issue(
                "editor_shim_package_read_failed",
                format!("failed to read {}: {error}", package_json.display()),
            ));
            return None;
        }
    };
    let contributes = value.get("contributes");
    let language_ids = contributes
        .and_then(|value| value.get("languages"))
        .and_then(JsonValue::as_array)
        .map(|languages| {
            languages
                .iter()
                .filter_map(|language| string_field(language, "id"))
                .collect()
        })
        .unwrap_or_default();
    let grammar_count = contributes
        .and_then(|value| value.get("grammars"))
        .and_then(JsonValue::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let command_count = contributes
        .and_then(|value| value.get("commands"))
        .and_then(JsonValue::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let dependency_count = value
        .get("dependencies")
        .and_then(JsonValue::as_object)
        .map(serde_json::Map::len)
        .unwrap_or(0);
    Some(EditorShimPackage {
        path: path_string(package_json),
        name: string_field(&value, "name"),
        display_name: string_field(&value, "displayName"),
        version: string_field(&value, "version"),
        private: value.get("private").and_then(JsonValue::as_bool),
        language_ids,
        grammar_count,
        command_count,
        dependency_count,
    })
}

fn run_editor_shim_validation(root: &Path) -> ValidationRun {
    let root_arg = path_string(root);
    let command = vec![
        "node".to_string(),
        "scripts/validate-editor-shims.mjs".to_string(),
        root_arg,
    ];
    match ProcessCommand::new(&command[0])
        .args(&command[1..])
        .output()
    {
        Ok(output) => {
            let mut issues = Vec::new();
            if !output.status.success() {
                issues.push(issue(
                    "editor_shim_validation_failed",
                    format!(
                        "editor shim validator exited unsuccessfully; stderr={:?}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
            ValidationRun {
                name: "editor_shims".to_string(),
                status: if output.status.success() {
                    "passed"
                } else {
                    "failed"
                }
                .to_string(),
                command,
                report_schema: None,
                fixture_count: None,
                passed_count: None,
                failed_count: None,
                issue_count: issues.len(),
                issues,
            }
        }
        Err(error) => ValidationRun {
            name: "editor_shims".to_string(),
            status: "failed".to_string(),
            command,
            report_schema: None,
            fixture_count: None,
            passed_count: None,
            failed_count: None,
            issue_count: 1,
            issues: vec![issue(
                "editor_shim_validation_spawn_failed",
                format!("failed to run editor shim validator: {error}"),
            )],
        },
    }
}

fn build_v1_gate_section(makefile: &Path) -> V1GateSection {
    let required_targets = DEFAULT_V1_GATE_TARGETS
        .iter()
        .map(|target| (*target).to_string())
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    let (actual_targets, defined_targets) = match fs::read_to_string(makefile) {
        Ok(source) => (
            parse_make_target_dependencies(&source, "v1").unwrap_or_default(),
            parse_make_defined_targets(&source),
        ),
        Err(error) => {
            issues.push(issue(
                "v1_gate_makefile_read_failed",
                format!("failed to read {}: {error}", makefile.display()),
            ));
            (Vec::new(), BTreeSet::new())
        }
    };
    if actual_targets.is_empty() && issues.is_empty() {
        issues.push(issue(
            "v1_gate_target_missing",
            format!("{} does not define a v1 target", makefile.display()),
        ));
    }
    let actual_set = actual_targets.iter().cloned().collect::<BTreeSet<_>>();
    let missing_required_targets = required_targets
        .iter()
        .filter(|target| !actual_set.contains(*target))
        .cloned()
        .collect::<Vec<_>>();
    for target in &missing_required_targets {
        issues.push(issue(
            "v1_gate_required_target_missing",
            format!("make v1 does not depend on required target {target:?}"),
        ));
    }
    let undefined_targets = actual_targets
        .iter()
        .filter(|target| !defined_targets.contains(*target))
        .cloned()
        .collect::<Vec<_>>();
    for target in &undefined_targets {
        issues.push(issue(
            "v1_gate_target_undefined",
            format!("make v1 depends on undefined target {target:?}"),
        ));
    }
    let issue_count = issues.len();
    V1GateSection {
        makefile: path_string(makefile),
        target: "v1".to_string(),
        required_targets,
        actual_targets,
        missing_required_targets,
        undefined_targets,
        issue_count,
        issues,
    }
}

fn parse_make_target_dependencies(source: &str, target: &str) -> Option<Vec<String>> {
    let logical_lines = make_logical_lines(source);
    logical_lines.iter().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with('\t') {
            return None;
        }
        let (left, right) = trimmed.split_once(':')?;
        if !left.split_whitespace().any(|candidate| candidate == target) {
            return None;
        }
        Some(make_words(right))
    })
}

fn parse_make_defined_targets(source: &str) -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    for line in make_logical_lines(source) {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with('\t') {
            continue;
        }
        let Some((left, _)) = trimmed.split_once(':') else {
            continue;
        };
        for target in left.split_whitespace() {
            if !target.contains('=') {
                targets.insert(target.to_string());
            }
        }
    }
    targets
}

fn make_logical_lines(source: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for line in source.lines() {
        let trimmed_end = line.trim_end();
        if let Some(prefix) = trimmed_end.strip_suffix('\\') {
            current.push_str(prefix);
            current.push(' ');
            continue;
        }
        current.push_str(trimmed_end);
        lines.push(std::mem::take(&mut current));
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn make_words(value: &str) -> Vec<String> {
    value
        .split('#')
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_declared_integration_test_count(source: &str) -> Option<usize> {
    let marker = "Current integration coverage is ";
    source.lines().find_map(|line| {
        let start = line.find(marker)? + marker.len();
        let digits = line[start..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        digits.parse().ok()
    })
}

fn build_release_section(
    cargo_manifest: &Path,
    license_file: &Path,
    tree_sitter_package: &Path,
    tree_sitter_config: &Path,
) -> ReleaseSection {
    let mut blockers = Vec::new();
    let cargo_package = build_cargo_package_section(cargo_manifest, &mut blockers);
    let tree_sitter_package =
        build_tree_sitter_package_section(tree_sitter_package, tree_sitter_config, &mut blockers);
    let license_present = license_file.is_file();
    if !license_present {
        blockers.push(issue(
            "missing_license_file",
            format!(
                "{} is not present; choose the repository license before public release",
                license_file.display()
            ),
        ));
    }
    if cargo_package.license.is_none() && cargo_package.license_file.is_none() {
        blockers.push(issue(
            "cargo_license_unresolved",
            format!(
                "{} does not declare `package.license` or `package.license-file`",
                cargo_manifest.display()
            ),
        ));
    }
    if !cargo_package.repository_present {
        blockers.push(issue(
            "cargo_repository_unset",
            format!(
                "{} does not declare `package.repository` for public release consumers",
                cargo_manifest.display()
            ),
        ));
    }
    if unresolved_license(tree_sitter_package.package_license.as_deref()) {
        blockers.push(issue(
            "tree_sitter_package_license_unresolved",
            format!(
                "{} has no public release license",
                tree_sitter_package.package_json
            ),
        ));
    }
    if unresolved_license(tree_sitter_package.config_license.as_deref()) {
        blockers.push(issue(
            "tree_sitter_config_license_unresolved",
            format!(
                "{} has no public release license",
                tree_sitter_package.config
            ),
        ));
    }
    let operator_decision_required = blockers
        .iter()
        .any(|blocker| blocker.code.contains("license"));
    let blocker_count = blockers.len();
    ReleaseSection {
        public_release_ready: blocker_count == 0,
        blocker_count,
        blockers,
        cargo_package,
        tree_sitter_package,
        license: LicenseSection {
            file: path_string(license_file),
            present: license_present,
            operator_decision_required,
        },
    }
}

fn build_cargo_package_section(
    manifest: &Path,
    blockers: &mut Vec<ConformanceIssue>,
) -> CargoPackageSection {
    let mut section = CargoPackageSection {
        manifest: path_string(manifest),
        name: None,
        version: None,
        edition: None,
        rust_version: None,
        description_present: false,
        readme_present: false,
        repository_present: false,
        license: None,
        license_file: None,
        publish: None,
        publish_restricted: false,
    };
    let source = match fs::read_to_string(manifest) {
        Ok(source) => source,
        Err(error) => {
            blockers.push(issue(
                "cargo_manifest_read_failed",
                format!("failed to read {}: {error}", manifest.display()),
            ));
            return section;
        }
    };
    let value = match toml::from_str::<toml::Value>(&source) {
        Ok(value) => value,
        Err(error) => {
            blockers.push(issue(
                "cargo_manifest_parse_failed",
                format!("failed to parse {}: {error}", manifest.display()),
            ));
            return section;
        }
    };
    let package = value.get("package");
    section.name = toml_string_field(package, "name");
    section.version = toml_string_field(package, "version");
    section.edition = toml_string_field(package, "edition");
    section.rust_version = toml_string_field(package, "rust-version");
    section.description_present = toml_string_field(package, "description").is_some();
    section.readme_present = toml_string_field(package, "readme").is_some();
    section.repository_present = toml_string_field(package, "repository").is_some();
    section.license = toml_string_field(package, "license");
    section.license_file = toml_string_field(package, "license-file");
    let (publish, restricted) = cargo_publish_state(package.and_then(|value| value.get("publish")));
    section.publish = publish;
    section.publish_restricted = restricted;
    section
}

fn build_tree_sitter_package_section(
    package_json: &Path,
    config: &Path,
    blockers: &mut Vec<ConformanceIssue>,
) -> TreeSitterPackageSection {
    let mut section = TreeSitterPackageSection {
        package_json: path_string(package_json),
        config: path_string(config),
        name: None,
        version: None,
        private: None,
        package_license: None,
        package_description_present: false,
        config_license: None,
        config_description_present: false,
    };
    match read_json_value(package_json) {
        Ok(value) => {
            section.name = string_field(&value, "name");
            section.version = string_field(&value, "version");
            section.private = value.get("private").and_then(JsonValue::as_bool);
            section.package_license = string_field(&value, "license");
            section.package_description_present = string_field(&value, "description").is_some();
        }
        Err(error) => blockers.push(issue(
            "tree_sitter_package_read_failed",
            format!("failed to read {}: {error}", package_json.display()),
        )),
    }
    match read_json_value(config) {
        Ok(value) => {
            let metadata = value.get("metadata");
            section.config_license = metadata.and_then(|value| string_field(value, "license"));
            section.config_description_present =
                metadata.is_some_and(|value| string_field(value, "description").is_some());
        }
        Err(error) => blockers.push(issue(
            "tree_sitter_config_read_failed",
            format!("failed to read {}: {error}", config.display()),
        )),
    }
    section
}

fn toml_string_field(value: Option<&toml::Value>, field: &str) -> Option<String> {
    value
        .and_then(|value| value.get(field))
        .and_then(toml::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn cargo_publish_state(value: Option<&toml::Value>) -> (Option<String>, bool) {
    match value {
        Some(toml::Value::Boolean(false)) => (Some("false".to_string()), true),
        Some(toml::Value::Boolean(true)) => (Some("true".to_string()), false),
        Some(toml::Value::Array(registries)) => {
            (Some(format!("registries:{}", registries.len())), true)
        }
        Some(_) => (Some("custom".to_string()), true),
        None => (None, false),
    }
}

fn unresolved_license(value: Option<&str>) -> bool {
    value
        .map(|license| license.eq_ignore_ascii_case("UNLICENSED"))
        .unwrap_or(true)
}

fn collect_examples(
    path: &Path,
    project_count: &mut usize,
    source_count: &mut usize,
) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|extension| extension.to_str()) == Some("sley") {
            *source_count += 1;
        }
        return Ok(());
    }
    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("sley.toml") {
            *project_count += 1;
        }
        collect_examples(&path, project_count, source_count)?;
    }
    Ok(())
}

fn load_schemas(schema_dir: &Path) -> Result<Vec<SchemaInfo>> {
    let mut schemas = Vec::new();
    for path in list_json_files(schema_dir)? {
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".schema.json"))
        {
            continue;
        }
        let value = read_json_value(&path)?;
        let Some(id) = string_field(&value, "$id") else {
            continue;
        };
        schemas.push(SchemaInfo {
            id,
            path: path_string(&path),
            title: string_field(&value, "title"),
        });
    }
    schemas.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(schemas)
}

fn load_fixture_schema_ids(fixtures_dir: &Path) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    for path in list_json_files(fixtures_dir)? {
        let value = read_json_value(&path)?;
        if let Some(schema) = string_field(&value, "schema") {
            ids.push(schema);
        }
    }
    Ok(ids)
}

fn load_manifest_schema_ids(
    corpus_manifest: &Path,
    smoke_manifests: &[PathBuf],
    issues: &mut Vec<ConformanceIssue>,
) -> Vec<String> {
    let mut ids = Vec::new();
    for path in std::iter::once(corpus_manifest).chain(smoke_manifests.iter().map(PathBuf::as_path))
    {
        match read_json_value(path) {
            Ok(value) => {
                if let Some(schema) = string_field(&value, "schema") {
                    ids.push(schema);
                }
            }
            Err(error) => issues.push(issue(
                "manifest_schema_read_failed",
                format!(
                    "failed to read manifest schema from {}: {error}",
                    path.display()
                ),
            )),
        }
    }
    ids
}

fn tag_counts(tags: impl IntoIterator<Item = String>) -> Vec<TagCount> {
    count_strings(tags)
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect()
}

fn count_strings(tags: impl IntoIterator<Item = String>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for tag in tags {
        *counts.entry(tag).or_insert(0) += 1;
    }
    counts
}

fn feature_prefix(tag: &str) -> String {
    tag.split_once(':')
        .map(|(prefix, _)| prefix.to_string())
        .unwrap_or_else(|| tag.to_string())
}

fn normalize_smoke_manifests(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let paths = if paths.is_empty() {
        vec![
            PathBuf::from("fixtures/cli_smokes/manifest.json"),
            PathBuf::from("fixtures/ci_smoke_probe/manifest.json"),
        ]
    } else {
        paths
    };
    paths.into_iter().map(normalize_manifest_path).collect()
}

fn normalize_manifest_path(path: PathBuf) -> PathBuf {
    if path.is_dir() {
        path.join("manifest.json")
    } else {
        path
    }
}

impl ContractCommand {
    fn from_binary(path: impl Into<PathBuf>) -> Self {
        let program = path.into();
        Self {
            display: vec![path_string(&program)],
            program,
            prefix_args: Vec::new(),
        }
    }

    fn cargo_run() -> Self {
        Self {
            program: PathBuf::from("cargo"),
            prefix_args: vec![
                "run".to_string(),
                "--quiet".to_string(),
                "--bin".to_string(),
                "sley-contract".to_string(),
                "--".to_string(),
            ],
            display: vec![
                "cargo".to_string(),
                "run".to_string(),
                "--quiet".to_string(),
                "--bin".to_string(),
                "sley-contract".to_string(),
                "--".to_string(),
            ],
        }
    }

    fn with_args(&self, args: &[String]) -> Vec<String> {
        let mut command = self.display.clone();
        command.extend(args.iter().cloned());
        command
    }

    fn output(&self, args: &[String]) -> std::io::Result<Output> {
        ProcessCommand::new(&self.program)
            .args(&self.prefix_args)
            .args(args)
            .output()
    }
}

fn find_sley_contract_command() -> ContractCommand {
    if let Some(path) = env::var_os("SLEY_CONTRACT_BIN") {
        return ContractCommand::from_binary(PathBuf::from(path));
    }
    if let Ok(current_exe) = env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let candidate = parent.join("sley-contract");
            if candidate.exists() {
                return ContractCommand::from_binary(candidate);
            }
        }
    }
    if Path::new("Cargo.toml").exists() && Path::new("src/bin/sley-contract.rs").exists() {
        return ContractCommand::cargo_run();
    }
    ContractCommand::from_binary("sley-contract")
}

fn list_json_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read JSON file {}", path.display()))?;
    serde_json::from_str(&source).with_context(|| format!("failed to parse {}", path.display()))
}

fn read_json_value(path: &Path) -> Result<JsonValue> {
    read_json_file(path)
}

fn string_field(value: &JsonValue, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn issue(code: impl Into<String>, message: impl Into<String>) -> ConformanceIssue {
    ConformanceIssue {
        code: code.into(),
        message: message.into(),
    }
}

fn emit_report(report: &ConformanceReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    println!(
        "sley-conformance report status={} schemas={} fixtures={} corpus={}/{} smoke={} examples={} tests={} editor_shims={} v1_gate={} v1_gate_missing={} corpus_required={} corpus_missing={} smoke_required={} smoke_missing={} public_release_blockers={}",
        report.status,
        report.summary.schema_count,
        report.summary.contract_fixture_count,
        report.summary.corpus_accepted_count,
        report.summary.corpus_rejected_count,
        report.summary.smoke_case_count,
        report.summary.example_source_count,
        report.summary.integration_test_count,
        report.summary.editor_shim_count,
        report.summary.v1_gate_target_count,
        report.summary.missing_v1_gate_target_count,
        report.corpus.required_tags.len(),
        report.corpus.missing_required_tags.len(),
        report.smoke.required_tags.len(),
        report.smoke.missing_required_tags.len(),
        report.summary.public_release_blocker_count
    );
    for issue in &report.issues {
        println!("{} {}", issue.code, issue.message);
    }
    for blocker in &report.release.blockers {
        println!("release:{} {}", blocker.code, blocker.message);
    }
    Ok(())
}

fn emit_coverage(report: &CoverageReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }
    println!(
        "sley-conformance coverage status={} required={} missing={}",
        report.status,
        report.required_tags.len(),
        report.missing_tags.len()
    );
    for issue in &report.issues {
        println!("{} {}", issue.code, issue.message);
    }
    Ok(())
}

fn render_markdown(report: &ConformanceReport) -> String {
    let mut output = String::new();
    output.push_str("# Sley Conformance Report\n\n");
    output.push_str(&format!("- Status: `{}`\n", report.status));
    output.push_str(&format!("- Schemas: `{}`\n", report.summary.schema_count));
    output.push_str(&format!(
        "- Contract fixtures: `{}` passed, `{}` failed\n",
        report
            .validation
            .contract_fixtures
            .passed_count
            .unwrap_or_default(),
        report.summary.contract_fixture_failed_count
    ));
    output.push_str(&format!(
        "- Corpus: `{}` accepted, `{}` rejected\n",
        report.summary.corpus_accepted_count, report.summary.corpus_rejected_count
    ));
    output.push_str(&format!(
        "- Required corpus tags: `{}` required, `{}` missing\n",
        report.corpus.required_tags.len(),
        report.corpus.missing_required_tags.len()
    ));
    output.push_str(&format!(
        "- Smoke cases: `{}`\n",
        report.summary.smoke_case_count
    ));
    output.push_str(&format!(
        "- Required smoke tags: `{}` required, `{}` missing\n",
        report.smoke.required_tags.len(),
        report.smoke.missing_required_tags.len()
    ));
    output.push_str(&format!(
        "- Examples: `{}` projects, `{}` sources\n",
        report.summary.example_project_count, report.summary.example_source_count
    ));
    output.push_str(&format!(
        "- Integration tests: `{}` declared, `{}` counted\n",
        report
            .tests
            .declared_integration_test_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "none".to_string()),
        report.tests.integration_test_count
    ));
    output.push_str(&format!(
        "- Editor shims: `{}` packages, validation `{}`\n",
        report.summary.editor_shim_count, report.editor_shims.validation.status
    ));
    output.push_str(&format!(
        "- `make v1` gate: `{}` targets, `{}` missing required targets\n",
        report.summary.v1_gate_target_count, report.summary.missing_v1_gate_target_count
    ));
    output.push_str(&format!(
        "- Public release ready: `{}` with `{}` blockers\n",
        report.release.public_release_ready, report.release.blocker_count
    ));
    if !report.release.blockers.is_empty() {
        output.push_str("\n## Public Release Blockers\n\n");
        for blocker in &report.release.blockers {
            output.push_str(&format!("- `{}`: {}\n", blocker.code, blocker.message));
        }
    }
    if !report.issues.is_empty() {
        output.push_str("\n## Issues\n\n");
        for issue in &report.issues {
            output.push_str(&format!("- `{}`: {}\n", issue.code, issue.message));
        }
    }
    output
}

fn render_html(report: &ConformanceReport) -> String {
    let markdown = render_markdown(report);
    let body = markdown
        .lines()
        .map(|line| {
            if let Some(title) = line.strip_prefix("# ") {
                format!("<h1>{}</h1>", escape_html(title))
            } else if let Some(title) = line.strip_prefix("## ") {
                format!("<h2>{}</h2>", escape_html(title))
            } else if let Some(item) = line.strip_prefix("- ") {
                format!("<p>{}</p>", escape_html(item))
            } else {
                String::new()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Sley Conformance Report</title></head><body>{body}</body></html>\n"
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
