use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use sley::Program;
use sley::checker::{check_program, has_errors};
use sley::diagnostics::Diagnostic;
use sley::lint::{LINT_REPORT_SCHEMA, LintFinding, LintOptions, build_lint_report};
use sley::parser::parse_program;
use sley::project::load_project;
use sley::query::{QUERY_REPORT_SCHEMA, QueryKind, QueryOptions, QueryReport, build_query_report};

const SHADOW_REPORT_SCHEMA: &str = "sley.shadow.report.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-shadow")]
#[command(about = "Replay non-authoritative Sley helper joins over query and lint reports")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build a shadow report from checked query and lint data.
    Report(ReportArgs),
}

#[derive(Debug, Args)]
struct ReportArgs {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    module: Option<String>,
    target: PathBuf,
}

#[derive(Debug, Serialize)]
struct ShadowReport {
    schema: String,
    status: String,
    target: String,
    source_schemas: ShadowSourceSchemas,
    filters: ShadowFilters,
    summary: ShadowSummary,
    authority_seeds: Vec<AuthoritySeed>,
    lint_links: Vec<LintLink>,
    diagnostics: Vec<Diagnostic>,
    issues: Vec<ShadowIssue>,
}

#[derive(Debug, Serialize)]
struct ShadowSourceSchemas {
    query: String,
    lint: String,
}

#[derive(Debug, Serialize)]
struct ShadowFilters {
    module: Option<String>,
}

#[derive(Debug, Serialize)]
struct ShadowSummary {
    module_count: usize,
    task_count: usize,
    effectful_task_count: usize,
    call_count: usize,
    lint_finding_count: usize,
    linked_lint_finding_count: usize,
    unlinked_lint_finding_count: usize,
    authority_seed_count: usize,
    diagnostic_count: usize,
    issue_count: usize,
}

#[derive(Debug, Serialize)]
struct AuthoritySeed {
    task_id: String,
    qualified_name: String,
    effects: Vec<String>,
    command_args: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LintLink {
    finding_id: String,
    rule: String,
    node: String,
    module: String,
    linked: bool,
    target_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    qualified_name: Option<String>,
    hint: String,
}

#[derive(Debug, Serialize)]
struct ShadowIssue {
    code: String,
    message: String,
}

#[derive(Debug, Clone)]
struct LinkTarget {
    kind: &'static str,
    qualified_name: String,
}

#[derive(Debug, Default)]
struct LinkIndex {
    tasks: BTreeMap<String, String>,
    types: BTreeMap<String, String>,
    effects: BTreeMap<String, String>,
    modules: BTreeSet<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (report, json) = match cli.command {
        Command::Report(args) => {
            let json = args.json;
            (build_shadow_report(args), json)
        }
    };

    if json {
        print_json(&report)?;
    } else {
        print_human(&report);
    }

    if report.status == "blocked" {
        bail!("shadow report target is blocked");
    }
    Ok(())
}

fn build_shadow_report(args: ReportArgs) -> ShadowReport {
    let target = path_string(&args.target);
    let filters = ShadowFilters {
        module: args.module.clone(),
    };
    let program = match load_target_program(&args.target) {
        Ok(program) => program,
        Err(diagnostics) => return blocked_report(target, filters, diagnostics, Vec::new()),
    };

    let diagnostics = check_program(&program);
    if has_errors(&diagnostics) {
        return blocked_report(target, filters, diagnostics, Vec::new());
    }

    let query = build_query_report(
        &program,
        QueryOptions {
            kind: QueryKind::All,
            module: args.module.clone(),
            exported_only: false,
        },
    );
    let lint = build_lint_report(
        &program,
        LintOptions {
            module: args.module,
            ..LintOptions::default()
        },
    );
    let authority_seeds = build_authority_seeds(&query);
    let index = build_link_index(&query);
    let lint_links = lint
        .findings
        .iter()
        .map(|finding| link_lint_finding(finding, &index))
        .collect::<Vec<_>>();
    let linked_lint_finding_count = lint_links.iter().filter(|link| link.linked).count();
    let unlinked_lint_finding_count = lint_links.len() - linked_lint_finding_count;
    let effectful_task_count = query
        .tasks
        .iter()
        .filter(|task| !task.effects.is_empty())
        .count();
    let summary = ShadowSummary {
        module_count: query.modules.len(),
        task_count: query.tasks.len(),
        effectful_task_count,
        call_count: query.calls.len(),
        lint_finding_count: lint.findings.len(),
        linked_lint_finding_count,
        unlinked_lint_finding_count,
        authority_seed_count: authority_seeds.len(),
        diagnostic_count: diagnostics.len(),
        issue_count: 0,
    };
    let status = if lint.findings.is_empty() {
        "clean"
    } else {
        "findings"
    };

    ShadowReport {
        schema: SHADOW_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        target,
        source_schemas: ShadowSourceSchemas {
            query: query.schema,
            lint: lint.schema,
        },
        filters,
        summary,
        authority_seeds,
        lint_links,
        diagnostics,
        issues: Vec::new(),
    }
}

fn blocked_report(
    target: String,
    filters: ShadowFilters,
    diagnostics: Vec<Diagnostic>,
    issues: Vec<ShadowIssue>,
) -> ShadowReport {
    ShadowReport {
        schema: SHADOW_REPORT_SCHEMA.to_string(),
        status: "blocked".to_string(),
        target,
        source_schemas: ShadowSourceSchemas {
            query: QUERY_REPORT_SCHEMA.to_string(),
            lint: LINT_REPORT_SCHEMA.to_string(),
        },
        filters,
        summary: ShadowSummary {
            module_count: 0,
            task_count: 0,
            effectful_task_count: 0,
            call_count: 0,
            lint_finding_count: 0,
            linked_lint_finding_count: 0,
            unlinked_lint_finding_count: 0,
            authority_seed_count: 0,
            diagnostic_count: diagnostics.len(),
            issue_count: issues.len(),
        },
        authority_seeds: Vec::new(),
        lint_links: Vec::new(),
        diagnostics,
        issues,
    }
}

fn build_authority_seeds(query: &QueryReport) -> Vec<AuthoritySeed> {
    query
        .tasks
        .iter()
        .filter(|task| !task.effects.is_empty())
        .map(|task| {
            let mut command_args = Vec::with_capacity(task.effects.len() * 2);
            for effect in &task.effects {
                command_args.push("--cap".to_string());
                command_args.push(effect.clone());
            }
            AuthoritySeed {
                task_id: task.id.clone(),
                qualified_name: task.qualified_name.clone(),
                effects: task.effects.clone(),
                command_args,
            }
        })
        .collect()
}

fn build_link_index(query: &QueryReport) -> LinkIndex {
    let mut index = LinkIndex::default();
    for module in &query.modules {
        index.modules.insert(module.module.clone());
    }
    for task in &query.tasks {
        index
            .tasks
            .insert(task.id.clone(), task.qualified_name.clone());
    }
    for ty in &query.types {
        index.types.insert(ty.id.clone(), ty.qualified_name.clone());
    }
    for effect in &query.effects {
        index
            .effects
            .insert(effect.id.clone(), effect.qualified_name.clone());
    }
    index
}

fn link_lint_finding(finding: &LintFinding, index: &LinkIndex) -> LintLink {
    let target = find_decl_target(&finding.node, index)
        .or_else(|| {
            parse_import_owner_module(&finding.node)
                .filter(|module| index.modules.contains(module.as_str()))
                .map(|module| LinkTarget {
                    kind: "module",
                    qualified_name: module,
                })
        })
        .or_else(|| {
            index
                .modules
                .contains(finding.module.as_str())
                .then(|| LinkTarget {
                    kind: "module",
                    qualified_name: finding.module.clone(),
                })
        });

    let linked = target.is_some();
    let target_kind = target
        .as_ref()
        .map(|target| target.kind)
        .unwrap_or("unknown")
        .to_string();
    let qualified_name = target.map(|target| target.qualified_name);
    LintLink {
        finding_id: finding.id.clone(),
        rule: finding.rule.clone(),
        node: finding.node.clone(),
        module: finding.module.clone(),
        linked,
        target_kind,
        qualified_name,
        hint: finding.hint.clone(),
    }
}

fn find_decl_target(node: &str, index: &LinkIndex) -> Option<LinkTarget> {
    find_indexed_id(node, "task", &index.tasks)
        .map(|qualified_name| LinkTarget {
            kind: "task",
            qualified_name,
        })
        .or_else(|| {
            find_indexed_id(node, "type", &index.types).map(|qualified_name| LinkTarget {
                kind: "type",
                qualified_name,
            })
        })
        .or_else(|| {
            find_indexed_id(node, "effect", &index.effects).map(|qualified_name| LinkTarget {
                kind: "effect",
                qualified_name,
            })
        })
}

fn find_indexed_id(node: &str, kind: &str, entries: &BTreeMap<String, String>) -> Option<String> {
    let prefix = format!("{kind}:");
    if let Some(value) = candidate_from_marker(node, &prefix, entries) {
        return Some(value);
    }
    let marker = format!(":{prefix}");
    node.find(&marker)
        .and_then(|start| candidate_from_marker(&node[start + 1..], &prefix, entries))
}

fn candidate_from_marker(
    text: &str,
    prefix: &str,
    entries: &BTreeMap<String, String>,
) -> Option<String> {
    if !text.starts_with(prefix) {
        return None;
    }
    if let Some(value) = entries.get(text) {
        return Some(value.clone());
    }
    let rest = &text[prefix.len()..];
    for (index, _) in rest.match_indices(':') {
        let candidate = format!("{prefix}{}", &rest[..index]);
        if let Some(value) = entries.get(&candidate) {
            return Some(value.clone());
        }
    }
    None
}

fn parse_import_owner_module(node: &str) -> Option<String> {
    let rest = node.strip_prefix("import:")?;
    rest.split(':')
        .next()
        .filter(|module| !module.is_empty())
        .map(str::to_string)
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

fn print_json(report: &ShadowReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_human(report: &ShadowReport) {
    println!(
        "sley-shadow report status={} target={} module={} modules={} tasks={} effectful_tasks={} calls={} lint_findings={} linked={} unlinked={} authority_seeds={} issues={}",
        report.status,
        report.target,
        report.filters.module.as_deref().unwrap_or("*"),
        report.summary.module_count,
        report.summary.task_count,
        report.summary.effectful_task_count,
        report.summary.call_count,
        report.summary.lint_finding_count,
        report.summary.linked_lint_finding_count,
        report.summary.unlinked_lint_finding_count,
        report.summary.authority_seed_count,
        report.summary.issue_count
    );
    for issue in &report.issues {
        eprintln!("{}: {}", issue.code, issue.message);
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
