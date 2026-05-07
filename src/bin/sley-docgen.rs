use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use sley::Program;
use sley::authority::host_effect_contracts;
use sley::checker::{check_program, has_errors};
use sley::diagnostics::Diagnostic;
use sley::parser::parse_program;
use sley::project::load_project;
use sley::query::{
    QUERY_REPORT_SCHEMA, QueryEffectSummary, QueryKind, QueryOptions, QueryReport,
    QueryTaskSummary, QueryTypeSummary, build_query_report,
};

const DOCGEN_REPORT_SCHEMA: &str = "sley.docgen.report.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-docgen")]
#[command(about = "Generate human Sley reference docs from checked query data")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate module, task, effect, and capability reference docs.
    Reference(ReferenceArgs),
}

#[derive(Debug, Args)]
struct ReferenceArgs {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    markdown: Option<PathBuf>,
    #[arg(long)]
    module: Option<String>,
    #[arg(long)]
    exported_only: bool,
    target: PathBuf,
}

#[derive(Debug, Serialize)]
struct DocgenReport {
    schema: String,
    status: String,
    target: String,
    source_schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    markdown_path: Option<String>,
    summary: DocgenSummary,
    filters: DocgenFilters,
    documents: Vec<DocgenDocument>,
    modules: Vec<DocgenModule>,
    tasks: Vec<QueryTaskSummary>,
    types: Vec<QueryTypeSummary>,
    effects: Vec<QueryEffectSummary>,
    capabilities: Vec<DocgenCapability>,
    diagnostics: Vec<Diagnostic>,
    issues: Vec<DocgenIssue>,
}

#[derive(Debug, Serialize)]
struct DocgenSummary {
    module_count: usize,
    task_count: usize,
    type_count: usize,
    effect_count: usize,
    capability_count: usize,
    document_count: usize,
    diagnostic_count: usize,
    issue_count: usize,
}

#[derive(Debug, Serialize)]
struct DocgenFilters {
    module: Option<String>,
    exported_only: bool,
}

#[derive(Debug, Serialize)]
struct DocgenDocument {
    kind: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    section_count: usize,
}

#[derive(Debug, Serialize)]
struct DocgenModule {
    module: String,
    import_count: usize,
    task_count: usize,
    type_count: usize,
    effect_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct DocgenCapability {
    effect: String,
    aliases: Vec<String>,
    seed_capability_args: Vec<String>,
    host_calls: Vec<String>,
    source_needles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DocgenIssue {
    code: String,
    message: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (mut report, json, markdown_path) = match cli.command {
        Command::Reference(args) => {
            let json = args.json;
            let markdown_path = args.markdown.clone();
            (build_reference_report(args), json, markdown_path)
        }
    };

    if report.status != "blocked" {
        if let Some(path) = markdown_path {
            match write_markdown(&path, &report) {
                Ok(()) => {
                    let path = path_string(&path);
                    report.markdown_path = Some(path.clone());
                    for document in &mut report.documents {
                        document.path = Some(path.clone());
                    }
                }
                Err(error) => {
                    report.status = "blocked".to_string();
                    report.issues.push(issue(
                        "DOCGEN_MARKDOWN_WRITE_FAILED",
                        format!("failed to write markdown reference: {error}"),
                    ));
                    report.summary.issue_count = report.issues.len();
                }
            }
        }
    }

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

fn build_reference_report(args: ReferenceArgs) -> DocgenReport {
    let target = path_string(&args.target);
    let filters = DocgenFilters {
        module: args.module.clone(),
        exported_only: args.exported_only,
    };
    let capabilities = build_capability_docs();
    let program = match load_target_program(&args.target) {
        Ok(program) => program,
        Err(diagnostics) => {
            return blocked_report(target, filters, capabilities, diagnostics, Vec::new());
        }
    };
    let diagnostics = check_program(&program);
    if has_errors(&diagnostics) {
        return blocked_report(target, filters, capabilities, diagnostics, Vec::new());
    }

    let query = build_query_report(
        &program,
        QueryOptions {
            kind: QueryKind::All,
            module: args.module,
            exported_only: args.exported_only,
        },
    );
    let modules = summarize_modules(&query);
    let title_module = filters.module.as_deref().unwrap_or(&query.entry_module);
    let documents = vec![DocgenDocument {
        kind: "reference".to_string(),
        title: format!("Sley Reference: {title_module}"),
        path: None,
        section_count: 5,
    }];
    let summary = DocgenSummary {
        module_count: modules.len(),
        task_count: query.tasks.len(),
        type_count: query.types.len(),
        effect_count: query.effects.len(),
        capability_count: capabilities.len(),
        document_count: documents.len(),
        diagnostic_count: diagnostics.len(),
        issue_count: 0,
    };

    DocgenReport {
        schema: DOCGEN_REPORT_SCHEMA.to_string(),
        status: "generated".to_string(),
        target,
        source_schema: query.schema,
        markdown_path: None,
        summary,
        filters,
        documents,
        modules,
        tasks: query.tasks,
        types: query.types,
        effects: query.effects,
        capabilities,
        diagnostics,
        issues: Vec::new(),
    }
}

fn blocked_report(
    target: String,
    filters: DocgenFilters,
    capabilities: Vec<DocgenCapability>,
    diagnostics: Vec<Diagnostic>,
    issues: Vec<DocgenIssue>,
) -> DocgenReport {
    DocgenReport {
        schema: DOCGEN_REPORT_SCHEMA.to_string(),
        status: "blocked".to_string(),
        target,
        source_schema: QUERY_REPORT_SCHEMA.to_string(),
        markdown_path: None,
        summary: DocgenSummary {
            module_count: 0,
            task_count: 0,
            type_count: 0,
            effect_count: 0,
            capability_count: capabilities.len(),
            document_count: 0,
            diagnostic_count: diagnostics.len(),
            issue_count: issues.len(),
        },
        filters,
        documents: Vec::new(),
        modules: Vec::new(),
        tasks: Vec::new(),
        types: Vec::new(),
        effects: Vec::new(),
        capabilities,
        diagnostics,
        issues,
    }
}

fn summarize_modules(query: &QueryReport) -> Vec<DocgenModule> {
    query
        .modules
        .iter()
        .map(|module| DocgenModule {
            module: module.module.clone(),
            import_count: module.imports.len(),
            task_count: module.tasks.len(),
            type_count: module.types.len(),
            effect_count: module.effects.len(),
        })
        .collect()
}

fn build_capability_docs() -> Vec<DocgenCapability> {
    #[derive(Default)]
    struct CapabilityBuilder {
        aliases: BTreeSet<String>,
        host_calls: BTreeSet<String>,
        source_needles: BTreeSet<String>,
    }

    let mut capabilities = BTreeMap::<String, CapabilityBuilder>::new();
    for contract in host_effect_contracts() {
        let Some((effect, aliases)) = contract.effects.split_first() else {
            continue;
        };
        let entry = capabilities.entry((*effect).to_string()).or_default();
        for alias in aliases {
            entry.aliases.insert((*alias).to_string());
        }
        entry.host_calls.insert(contract.callee.to_string());
        entry
            .source_needles
            .insert(contract.source_needle.to_string());
    }
    capabilities
        .into_iter()
        .map(|(effect, builder)| DocgenCapability {
            seed_capability_args: vec!["--cap".to_string(), effect.clone()],
            effect,
            aliases: builder.aliases.into_iter().collect(),
            host_calls: builder.host_calls.into_iter().collect(),
            source_needles: builder.source_needles.into_iter().collect(),
        })
        .collect()
}

fn write_markdown(path: &Path, report: &DocgenReport) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, render_markdown(report))
        .with_context(|| format!("failed to write {}", path.display()))
}

fn render_markdown(report: &DocgenReport) -> String {
    let mut output = String::new();
    let title = report
        .documents
        .first()
        .map(|document| document.title.as_str())
        .unwrap_or("Sley Reference");
    output.push_str(&format!("# {}\n\n", markdown_text(title)));
    output.push_str("## Summary\n\n");
    output.push_str(&format!("- Modules: `{}`\n", report.summary.module_count));
    output.push_str(&format!("- Tasks: `{}`\n", report.summary.task_count));
    output.push_str(&format!("- Types: `{}`\n", report.summary.type_count));
    output.push_str(&format!("- Effects: `{}`\n", report.summary.effect_count));
    output.push_str(&format!(
        "- Capabilities: `{}`\n\n",
        report.summary.capability_count
    ));

    output.push_str("## Modules\n\n");
    if report.modules.is_empty() {
        output.push_str("_No modules matched._\n\n");
    } else {
        for module in &report.modules {
            output.push_str(&format!(
                "- `{}`: imports `{}`, tasks `{}`, types `{}`, effects `{}`\n",
                code_text(&module.module),
                module.import_count,
                module.task_count,
                module.type_count,
                module.effect_count
            ));
        }
        output.push('\n');
    }

    output.push_str("## Tasks\n\n");
    if report.tasks.is_empty() {
        output.push_str("_No tasks matched._\n\n");
    } else {
        for task in &report.tasks {
            output.push_str(&format!("### `{}`\n\n", code_text(&task.qualified_name)));
            output.push_str(&format!("- Module: `{}`\n", code_text(&task.module)));
            output.push_str(&format!("- Exported: `{}`\n", task.exported));
            output.push_str(&format!("- Return: `{}`\n", code_text(&task.return_type)));
            output.push_str(&format!(
                "- Effects: `{}`\n",
                code_text(&join_or_none(&task.effects))
            ));
            output.push_str(&format!(
                "- Calls: outbound `{}`, inbound `{}`\n",
                task.outbound_call_count, task.inbound_call_count
            ));
            if task.takes.is_empty() {
                output.push_str("- Takes: `none`\n\n");
            } else {
                output.push_str("- Takes:\n");
                for take in &task.takes {
                    output.push_str(&format!(
                        "  - `{}` `{}`: `{}`\n",
                        code_text(&take.binding_kind),
                        code_text(&take.name),
                        code_text(&take.ty)
                    ));
                }
                output.push('\n');
            }
        }
    }

    output.push_str("## Types\n\n");
    if report.types.is_empty() {
        output.push_str("_No types matched._\n\n");
    } else {
        for ty in &report.types {
            output.push_str(&format!("### `{}`\n\n", code_text(&ty.qualified_name)));
            output.push_str(&format!("- Exported: `{}`\n", ty.exported));
            output.push_str(&format!("- Value: `{}`\n", code_text(&ty.value)));
            if !ty.fields.is_empty() {
                output.push_str("- Fields:\n");
                for field in &ty.fields {
                    output.push_str(&format!(
                        "  - `{}`: `{}`\n",
                        code_text(&field.name),
                        code_text(&field.ty)
                    ));
                }
            }
            output.push('\n');
        }
    }

    output.push_str("## Effects\n\n");
    if report.effects.is_empty() {
        output.push_str("_No declared effects matched._\n\n");
    } else {
        for effect in &report.effects {
            output.push_str(&format!(
                "- `{}` exported `{}`\n",
                code_text(&effect.qualified_name),
                effect.exported
            ));
        }
        output.push('\n');
    }

    output.push_str("## Capabilities\n\n");
    for capability in &report.capabilities {
        output.push_str(&format!("### `{}`\n\n", code_text(&capability.effect)));
        output.push_str(&format!(
            "- Seed capability args: `{}`\n",
            code_text(&capability.seed_capability_args.join(" "))
        ));
        output.push_str(&format!(
            "- Aliases: `{}`\n",
            code_text(&join_or_none(&capability.aliases))
        ));
        output.push_str(&format!(
            "- Host calls: `{}`\n\n",
            code_text(&join_or_none(&capability.host_calls))
        ));
    }
    output
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn markdown_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('#', "\\#")
}

fn code_text(text: &str) -> String {
    text.replace('`', "\\`")
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

fn print_json(report: &DocgenReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_human(report: &DocgenReport) {
    println!(
        "sley-docgen status={} target={} module={} exported_only={} modules={} tasks={} effects={} capabilities={} documents={} issues={}",
        report.status,
        report.target,
        report.filters.module.as_deref().unwrap_or("*"),
        report.filters.exported_only,
        report.summary.module_count,
        report.summary.task_count,
        report.summary.effect_count,
        report.summary.capability_count,
        report.summary.document_count,
        report.summary.issue_count
    );
    for document in &report.documents {
        println!(
            "document {} sections={} path={}",
            document.kind,
            document.section_count,
            document.path.as_deref().unwrap_or("stdout")
        );
    }
    for issue in &report.issues {
        eprintln!("{}: {}", issue.code, issue.message);
    }
}

fn issue(code: impl Into<String>, message: impl Into<String>) -> DocgenIssue {
    DocgenIssue {
        code: code.into(),
        message: message.into(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
