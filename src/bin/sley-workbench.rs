use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use serde_json::Value as JsonValue;
use sley::Program;
use sley::checker::{check_program, has_errors};
use sley::diagnostics::Diagnostic;
use sley::doctor::{DoctorAction, build_doctor_report};
use sley::lint::{LintFinding, LintOptions, build_lint_report};
use sley::parser::parse_program;
use sley::plan::{
    EditPlanAction, EditPlanOptions, EditPlanTaskSurface, build_edit_plan_report_with_options,
    module_name_from_sley_path,
};
use sley::project::load_project;
use sley::query::{
    QueryKind, QueryOptions, QueryTaskSummary, QueryTypeSummary, build_query_report,
};
use sley::symbols::{
    ModuleSymbolSummary, SymbolGraphSlice, TaskCallSummary, build_symbol_graph, slice_symbol_graph,
};

const WORKBENCH_REPORT_SCHEMA: &str = "sley.workbench.report.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-workbench")]
#[command(about = "Build a local Sley inspection workbench report")]
struct Cli {
    #[arg(long)]
    json: bool,
    #[arg(long)]
    html: Option<PathBuf>,
    #[arg(long)]
    slice: Option<String>,
    #[arg(long)]
    deny_warnings: bool,
    target: PathBuf,
}

#[derive(Debug, Serialize)]
struct WorkbenchReport {
    schema: String,
    status: String,
    target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_path: Option<String>,
    summary: WorkbenchSummary,
    diagnostics: Vec<Diagnostic>,
    doctor: WorkbenchDoctorPanel,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<WorkbenchQueryPanel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lint: Option<WorkbenchLintPanel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<WorkbenchPlanPanel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graph: Option<WorkbenchGraphPanel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graph_slice: Option<SymbolGraphSlice>,
    issues: Vec<WorkbenchIssue>,
}

#[derive(Debug, Serialize)]
struct WorkbenchSummary {
    module_count: usize,
    task_count: usize,
    type_count: usize,
    effect_count: usize,
    call_count: usize,
    lint_finding_count: usize,
    graft_template_count: usize,
    transaction_template_count: usize,
    diagnostic_count: usize,
    issue_count: usize,
}

#[derive(Debug, Serialize)]
struct WorkbenchDoctorPanel {
    status: String,
    error_count: usize,
    warning_count: usize,
    next_actions: Vec<DoctorAction>,
}

#[derive(Debug, Serialize)]
struct WorkbenchQueryPanel {
    source_schema: String,
    entry_module: String,
    modules: Vec<String>,
    tasks: Vec<QueryTaskSummary>,
    types: Vec<QueryTypeSummary>,
    calls: Vec<TaskCallSummary>,
}

#[derive(Debug, Serialize)]
struct WorkbenchLintPanel {
    source_schema: String,
    status: String,
    findings: Vec<LintFinding>,
}

#[derive(Debug, Serialize)]
struct WorkbenchPlanPanel {
    source_schema: String,
    status: String,
    task_surfaces: Vec<EditPlanTaskSurface>,
    graft_templates: Vec<WorkbenchTemplatePreview>,
    transaction_templates: Vec<WorkbenchTemplatePreview>,
    next_actions: Vec<EditPlanAction>,
}

#[derive(Debug, Serialize)]
struct WorkbenchTemplatePreview {
    kind: String,
    surface: String,
    reason: String,
    editable_json_pointers: Vec<String>,
    payload: JsonValue,
    preview_command: Vec<String>,
    write_command: Vec<String>,
    post_fix_gate_commands: Vec<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct WorkbenchGraphPanel {
    source_schema: String,
    entry_module: String,
    module_count: usize,
    modules: Vec<ModuleSymbolSummary>,
}

#[derive(Debug, Serialize)]
struct WorkbenchIssue {
    code: String,
    message: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let target = cli.target.display().to_string();
    let mut report = build_workbench_report(
        target,
        load_target_program(&cli.target),
        cli.deny_warnings,
        cli.slice.as_deref(),
        module_name_hint_for_target(&cli.target),
    );
    if let Some(html_path) = cli.html {
        report.html_path = Some(html_path.display().to_string());
        let html = render_html(&report)?;
        if let Some(parent) = html_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&html_path, html)
            .with_context(|| format!("failed to write {}", html_path.display()))?;
    }

    if cli.json {
        print_json(&report)?;
    } else {
        print_human_report(&report);
    }

    if report.status == "blocked" {
        anyhow::bail!("workbench target is blocked");
    }
    Ok(())
}

fn build_workbench_report(
    target: String,
    program_result: Result<Program, Vec<Diagnostic>>,
    deny_warnings: bool,
    slice_target: Option<&str>,
    module_name_hint: Option<String>,
) -> WorkbenchReport {
    let doctor = build_doctor_report(target.clone(), program_result.clone(), deny_warnings);
    let mut issues = Vec::new();
    let mut status = doctor.status.clone();
    let mut diagnostics = doctor.diagnostics.clone();
    let mut summary = WorkbenchSummary {
        module_count: doctor.summary.module_count,
        task_count: doctor.summary.task_count,
        type_count: 0,
        effect_count: 0,
        call_count: doctor.summary.call_count,
        lint_finding_count: doctor.summary.lint_finding_count,
        graft_template_count: 0,
        transaction_template_count: 0,
        diagnostic_count: diagnostics.len(),
        issue_count: 0,
    };
    let doctor_panel = WorkbenchDoctorPanel {
        status: doctor.status,
        error_count: doctor.summary.error_count,
        warning_count: doctor.summary.warning_count,
        next_actions: doctor.next_actions,
    };

    let Ok(program) = program_result else {
        summary.issue_count = issues.len();
        return WorkbenchReport {
            schema: WORKBENCH_REPORT_SCHEMA.to_string(),
            status,
            target,
            html_path: None,
            summary,
            diagnostics,
            doctor: doctor_panel,
            query: None,
            lint: None,
            plan: None,
            graph: None,
            graph_slice: None,
            issues,
        };
    };

    let compiler_diagnostics = check_program(&program);
    if has_errors(&compiler_diagnostics) {
        summary.diagnostic_count = diagnostics.len();
        summary.issue_count = issues.len();
        return WorkbenchReport {
            schema: WORKBENCH_REPORT_SCHEMA.to_string(),
            status,
            target,
            html_path: None,
            summary,
            diagnostics,
            doctor: doctor_panel,
            query: None,
            lint: None,
            plan: None,
            graph: None,
            graph_slice: None,
            issues,
        };
    }
    diagnostics = compiler_diagnostics;

    let query_report = build_query_report(
        &program,
        QueryOptions {
            kind: QueryKind::All,
            module: None,
            exported_only: false,
        },
    );
    let lint_report = build_lint_report(&program, LintOptions::default());
    let plan_report = build_edit_plan_report_with_options(
        target.clone(),
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint,
        },
    );
    let graph = build_symbol_graph(&program);
    let graph_slice = match slice_target {
        Some(slice_target) => match slice_symbol_graph(&program, slice_target) {
            Some(slice) => Some(slice),
            None => {
                issues.push(WorkbenchIssue {
                    code: "GRAPH_SLICE_NOT_FOUND".to_string(),
                    message: format!("graph slice target `{slice_target}` was not found"),
                });
                None
            }
        },
        None => None,
    };

    if !issues.is_empty() && status == "ready" {
        status = "warnings".to_string();
    }
    summary.module_count = query_report.modules.len();
    summary.task_count = query_report.tasks.len();
    summary.type_count = query_report.types.len();
    summary.effect_count = query_report.effects.len();
    summary.call_count = query_report.calls.len();
    summary.lint_finding_count = lint_report.findings.len();
    summary.graft_template_count = plan_report.graft_templates.len();
    summary.transaction_template_count = plan_report.transaction_templates.len();
    summary.diagnostic_count = diagnostics.len();
    summary.issue_count = issues.len();

    let query = WorkbenchQueryPanel {
        source_schema: query_report.schema,
        entry_module: query_report.entry_module,
        modules: query_report
            .modules
            .iter()
            .map(|module| module.module.clone())
            .collect(),
        tasks: query_report.tasks,
        types: query_report.types,
        calls: query_report.calls,
    };
    let lint = WorkbenchLintPanel {
        source_schema: lint_report.schema,
        status: lint_report.status,
        findings: lint_report.findings,
    };
    let plan = WorkbenchPlanPanel {
        source_schema: plan_report.schema,
        status: plan_report.status,
        task_surfaces: plan_report.task_surfaces,
        graft_templates: plan_report
            .graft_templates
            .into_iter()
            .map(|template| WorkbenchTemplatePreview {
                preview_command: fix_template_command(
                    &target,
                    &template.kind,
                    &template.surface,
                    "--dry-run",
                ),
                write_command: fix_template_command(
                    &target,
                    &template.kind,
                    &template.surface,
                    "--write",
                ),
                post_fix_gate_commands: post_fix_gate_commands(&target),
                kind: template.kind,
                surface: template.surface,
                reason: template.reason,
                editable_json_pointers: template.editable_json_pointers,
                payload: template.operation,
            })
            .collect(),
        transaction_templates: plan_report
            .transaction_templates
            .into_iter()
            .map(|template| WorkbenchTemplatePreview {
                preview_command: fix_template_command(
                    &target,
                    &template.kind,
                    &template.surface,
                    "--dry-run",
                ),
                write_command: fix_template_command(
                    &target,
                    &template.kind,
                    &template.surface,
                    "--write",
                ),
                post_fix_gate_commands: post_fix_gate_commands(&target),
                kind: template.kind,
                surface: template.surface,
                reason: template.reason,
                editable_json_pointers: template.editable_json_pointers,
                payload: template.transaction,
            })
            .collect(),
        next_actions: plan_report.next_actions,
    };
    let graph_panel = WorkbenchGraphPanel {
        source_schema: graph.schema,
        entry_module: graph.entry_module,
        module_count: graph.modules.len(),
        modules: graph.modules,
    };

    WorkbenchReport {
        schema: WORKBENCH_REPORT_SCHEMA.to_string(),
        status,
        target,
        html_path: None,
        summary,
        diagnostics,
        doctor: doctor_panel,
        query: Some(query),
        lint: Some(lint),
        plan: Some(plan),
        graph: Some(graph_panel),
        graph_slice,
        issues,
    }
}

fn fix_template_command(target: &str, kind: &str, surface: &str, mode: &str) -> Vec<String> {
    vec![
        "sley".to_string(),
        "fix".to_string(),
        "--json".to_string(),
        "--kind".to_string(),
        kind.to_string(),
        "--template-surface".to_string(),
        surface.to_string(),
        mode.to_string(),
        target.to_string(),
    ]
}

fn post_fix_gate_commands(target: &str) -> Vec<Vec<String>> {
    vec![
        vec![
            "sley".to_string(),
            "check".to_string(),
            "--json".to_string(),
            target.to_string(),
        ],
        vec![
            "sley".to_string(),
            "lint".to_string(),
            "--json".to_string(),
            "--deny-warnings".to_string(),
            target.to_string(),
        ],
        vec![
            "sley".to_string(),
            "verify".to_string(),
            "--json".to_string(),
            "--deny-warnings".to_string(),
            target.to_string(),
        ],
    ]
}

fn render_html(report: &WorkbenchReport) -> Result<String> {
    let summary_json = serde_json::to_string_pretty(&report.summary)?;
    let diagnostics = if report.diagnostics.is_empty() {
        "<p class=\"muted\">No compiler diagnostics.</p>".to_string()
    } else {
        let rows = report
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                    escape_html(&diagnostic.id),
                    escape_html(&format!("{:?}", diagnostic.severity)),
                    escape_html(&diagnostic.message)
                )
            })
            .collect::<String>();
        format!(
            "<table><thead><tr><th>ID</th><th>Severity</th><th>Message</th></tr></thead><tbody>{rows}</tbody></table>"
        )
    };
    let lint = report
        .lint
        .as_ref()
        .map(render_lint_panel)
        .unwrap_or_else(|| "<p class=\"muted\">Lint is unavailable.</p>".to_string());
    let repair_focus = report
        .lint
        .as_ref()
        .map(render_repair_focus_panel)
        .unwrap_or_else(|| "<p class=\"muted\">Repair focus is unavailable.</p>".to_string());
    let tasks = report
        .query
        .as_ref()
        .map(render_tasks_panel)
        .unwrap_or_else(|| "<p class=\"muted\">Query is unavailable.</p>".to_string());
    let templates = report
        .plan
        .as_ref()
        .map(render_templates_panel)
        .unwrap_or_else(|| "<p class=\"muted\">Edit plan is unavailable.</p>".to_string());
    let graph = report
        .graph
        .as_ref()
        .map(render_graph_panel)
        .unwrap_or_else(|| "<p class=\"muted\">Graph is unavailable.</p>".to_string());
    let graph_slice = report
        .graph_slice
        .as_ref()
        .map(render_graph_slice_panel)
        .unwrap_or_else(|| "<p class=\"muted\">Run with <code>--slice &lt;node-id&gt;</code> to inspect a focused graph slice.</p>".to_string());

    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Sley Workbench - {target}</title>
<style>
:root {{
  color-scheme: light;
  --ink: #182026;
  --muted: #60707d;
  --line: #cdd7df;
  --surface: #f6f8fa;
  --accent: #0f766e;
  --warn: #a16207;
  --bad: #b42318;
}}
* {{ box-sizing: border-box; }}
body {{
  margin: 0;
  color: var(--ink);
  background: #ffffff;
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  line-height: 1.45;
}}
header {{
  padding: 28px 32px 20px;
  border-bottom: 1px solid var(--line);
  background: var(--surface);
}}
main {{ padding: 24px 32px 40px; }}
h1 {{ margin: 0 0 6px; font-size: 26px; font-weight: 700; }}
h2 {{ margin: 30px 0 12px; font-size: 18px; }}
.target {{ color: var(--muted); font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}
.status {{
  display: inline-flex;
  margin-top: 14px;
  padding: 4px 8px;
  border: 1px solid var(--line);
  background: #fff;
  font-weight: 700;
  text-transform: uppercase;
  font-size: 12px;
}}
.status.ready {{ color: var(--accent); }}
.status.warnings {{ color: var(--warn); }}
.status.blocked {{ color: var(--bad); }}
.metrics {{
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 1px;
  border: 1px solid var(--line);
  background: var(--line);
}}
.metric {{ background: #fff; padding: 12px; min-height: 74px; }}
.metric strong {{ display: block; font-size: 24px; }}
.metric span {{ color: var(--muted); font-size: 12px; text-transform: uppercase; }}
table {{ width: 100%; border-collapse: collapse; border: 1px solid var(--line); }}
th, td {{ padding: 8px 10px; text-align: left; border-bottom: 1px solid var(--line); vertical-align: top; }}
th {{ background: var(--surface); font-size: 12px; text-transform: uppercase; color: var(--muted); }}
td {{ font-size: 14px; }}
code, pre {{ font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}
pre {{ overflow: auto; padding: 12px; border: 1px solid var(--line); background: var(--surface); }}
select {{ min-width: min(100%, 420px); padding: 8px 10px; border: 1px solid var(--line); background: #fff; color: var(--ink); }}
tr.selected {{ outline: 2px solid var(--accent); outline-offset: -2px; }}
tr[hidden] {{ display: none; }}
.slice-focus {{ margin-bottom: 12px; }}
.slice-focus code {{ font-weight: 700; }}
.muted {{ color: var(--muted); }}
</style>
</head>
<body>
<header>
  <h1>Sley Workbench</h1>
  <div class="target">{target}</div>
  <div class="status {status_class}">{status}</div>
</header>
<main>
  <section class="metrics">
    {metrics}
  </section>
  <section>
    <h2>Repair Focus</h2>
    {repair_focus}
  </section>
  <section>
    <h2>Tasks</h2>
    {tasks}
  </section>
  <section>
    <h2>Lint</h2>
    {lint}
  </section>
  <section>
    <h2>Edit Plan</h2>
    {templates}
  </section>
  <section>
    <h2>Diagnostics</h2>
    {diagnostics}
  </section>
  <section>
    <h2>Graph Slice</h2>
    {graph_slice}
  </section>
  <section>
    <h2>Graph</h2>
    {graph}
  </section>
  <section>
    <h2>Summary JSON</h2>
    <pre>{summary_json}</pre>
  </section>
</main>
<script>
const focusSelect = document.querySelector("[data-workbench-focus]");
function applyWorkbenchFocus(value) {{
  document.querySelectorAll("[data-lint-row]").forEach((row) => {{
    row.classList.toggle("selected", Boolean(value) && row.dataset.node === value);
  }});
  document.querySelectorAll("[data-template-row]").forEach((row) => {{
    row.hidden = Boolean(value) && row.dataset.surface !== value;
  }});
}}
if (focusSelect) {{
  focusSelect.addEventListener("change", (event) => applyWorkbenchFocus(event.target.value));
  applyWorkbenchFocus(focusSelect.value);
}}
</script>
</body>
</html>
"#,
        target = escape_html(&report.target),
        status = escape_html(&report.status),
        status_class = escape_html(&report.status),
        metrics = render_metrics(&report.summary),
        repair_focus = repair_focus,
        tasks = tasks,
        lint = lint,
        templates = templates,
        diagnostics = diagnostics,
        graph_slice = graph_slice,
        graph = graph,
        summary_json = escape_html(&summary_json)
    ))
}

fn render_repair_focus_panel(lint: &WorkbenchLintPanel) -> String {
    if lint.findings.is_empty() {
        return "<p class=\"muted\">No repair focus.</p>".to_string();
    }
    let options = lint
        .findings
        .iter()
        .map(|finding| {
            format!(
                "<option value=\"{}\">{} - {}</option>",
                escape_html(&finding.node),
                escape_html(&finding.rule),
                escape_html(&finding.node)
            )
        })
        .collect::<String>();
    format!(
        "<select data-workbench-focus><option value=\"\">All findings</option>{options}</select>"
    )
}

fn render_metrics(summary: &WorkbenchSummary) -> String {
    [
        ("Modules", summary.module_count),
        ("Tasks", summary.task_count),
        ("Types", summary.type_count),
        ("Effects", summary.effect_count),
        ("Calls", summary.call_count),
        ("Lint", summary.lint_finding_count),
        ("Grafts", summary.graft_template_count),
        ("Transactions", summary.transaction_template_count),
    ]
    .into_iter()
    .map(|(label, value)| {
        format!(
            "<div class=\"metric\"><strong>{value}</strong><span>{}</span></div>",
            escape_html(label)
        )
    })
    .collect()
}

fn render_tasks_panel(query: &WorkbenchQueryPanel) -> String {
    if query.tasks.is_empty() {
        return "<p class=\"muted\">No tasks.</p>".to_string();
    }
    let rows = query
        .tasks
        .iter()
        .map(|task| {
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}/{}</td></tr>",
                escape_html(&task.qualified_name),
                escape_html(&task.return_type),
                escape_html(&task.effects.join(", ")),
                task.inbound_call_count,
                task.outbound_call_count
            )
        })
        .collect::<String>();
    format!(
        "<table><thead><tr><th>Task</th><th>Return</th><th>Effects</th><th>In/Out Calls</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_lint_panel(lint: &WorkbenchLintPanel) -> String {
    if lint.findings.is_empty() {
        return "<p class=\"muted\">No lint findings.</p>".to_string();
    }
    let rows = lint
        .findings
        .iter()
        .map(|finding| {
            format!(
                "<tr data-lint-row data-node=\"{}\"><td>{}</td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>",
                escape_html(&finding.node),
                escape_html(&finding.rule),
                escape_html(&finding.node),
                escape_html(&finding.message),
                escape_html(&finding.hint)
            )
        })
        .collect::<String>();
    format!(
        "<table><thead><tr><th>Rule</th><th>Node</th><th>Message</th><th>Hint</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_templates_panel(plan: &WorkbenchPlanPanel) -> String {
    if plan.graft_templates.is_empty() && plan.transaction_templates.is_empty() {
        return "<p class=\"muted\">No edit templates.</p>".to_string();
    }
    let rows = plan
        .graft_templates
        .iter()
        .chain(plan.transaction_templates.iter())
        .map(|template| {
            format!(
                "<tr data-template-row data-surface=\"{}\"><td>{}</td><td><code>{}</code></td><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td></tr>",
                escape_html(&template.surface),
                escape_html(&template.kind),
                escape_html(&template.surface),
                escape_html(&template.reason),
                escape_html(&command_text(&template.preview_command)),
                escape_html(&command_text(&template.write_command)),
                command_list_html(&template.post_fix_gate_commands)
            )
        })
        .collect::<String>();
    format!(
        "<table><thead><tr><th>Template</th><th>Surface</th><th>Reason</th><th>Preview Command</th><th>Write Command</th><th>Post-Fix Gates</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_graph_panel(graph: &WorkbenchGraphPanel) -> String {
    if graph.modules.is_empty() {
        return "<p class=\"muted\">No modules.</p>".to_string();
    }
    let rows = graph
        .modules
        .iter()
        .map(|module| {
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&module.module),
                module.imports.len(),
                module.types.len(),
                module.effects.len(),
                module.tasks.len()
            )
        })
        .collect::<String>();
    format!(
        "<table><thead><tr><th>Module</th><th>Imports</th><th>Types</th><th>Effects</th><th>Tasks</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_graph_slice_panel(slice: &SymbolGraphSlice) -> String {
    let focus = format!(
        "<p class=\"slice-focus\"><span class=\"muted\">Focused {}</span> <code>{}</code> in <code>{}</code></p>",
        escape_html(&slice.focus.kind),
        escape_html(&slice.focus.id),
        escape_html(&slice.focus.module)
    );
    let metrics = [
        ("Imports", slice.imports.len()),
        ("Types", slice.types.len()),
        ("Effects", slice.effects.len()),
        ("Tasks", slice.tasks.len()),
        ("Outbound Calls", slice.outbound_calls.len()),
        ("Inbound Calls", slice.inbound_calls.len()),
        ("Insert Affordances", slice.insert_affordances.len()),
        ("Move Affordances", slice.move_affordances.len()),
        ("Delete Affordances", slice.delete_affordances.len()),
        ("Replace Affordances", slice.replace_affordances.len()),
    ]
    .into_iter()
    .map(|(label, value)| {
        format!(
            "<div class=\"metric\"><strong>{value}</strong><span>{}</span></div>",
            escape_html(label)
        )
    })
    .collect::<String>();
    let calls = render_slice_calls(slice);
    let affordances = render_slice_affordances(slice);
    format!(
        "{focus}<section class=\"metrics\">{metrics}</section><h3>Calls</h3>{calls}<h3>Affordances</h3>{affordances}"
    )
}

fn render_slice_calls(slice: &SymbolGraphSlice) -> String {
    if slice.outbound_calls.is_empty() && slice.inbound_calls.is_empty() {
        return "<p class=\"muted\">No task calls in this slice.</p>".to_string();
    }
    let rows = slice
        .outbound_calls
        .iter()
        .map(|call| ("outbound", call))
        .chain(slice.inbound_calls.iter().map(|call| ("inbound", call)))
        .map(|(direction, call)| {
            format!(
                "<tr><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td><code>{}</code></td></tr>",
                escape_html(direction),
                escape_html(&call.from),
                escape_html(&call.callee),
                escape_html(&call.status),
                escape_html(call.target.as_deref().unwrap_or(""))
            )
        })
        .collect::<String>();
    format!(
        "<table><thead><tr><th>Direction</th><th>From</th><th>Callee</th><th>Status</th><th>Target</th></tr></thead><tbody>{rows}</tbody></table>"
    )
}

fn render_slice_affordances(slice: &SymbolGraphSlice) -> String {
    let mut rows = Vec::new();
    rows.extend(slice.insert_affordances.iter().map(|affordance| {
        format!(
            "<tr><td>insert</td><td>{}</td><td><code>{}</code></td><td>max position {}</td></tr>",
            escape_html(&affordance.target_kind),
            escape_html(&affordance.target),
            affordance.max_position
        )
    }));
    rows.extend(slice.move_affordances.iter().map(|affordance| {
        format!(
            "<tr><td>move</td><td>{}</td><td><code>{}</code></td><td>parent <code>{}</code>, {} destinations</td></tr>",
            escape_html(&affordance.target_kind),
            escape_html(&affordance.target),
            escape_html(&affordance.parent),
            affordance.destinations.len()
        )
    }));
    rows.extend(slice.delete_affordances.iter().map(|affordance| {
        format!(
            "<tr><td>delete</td><td>{}</td><td><code>{}</code></td><td>parent <code>{}</code>, position {}</td></tr>",
            escape_html(&affordance.target_kind),
            escape_html(&affordance.target),
            escape_html(&affordance.parent),
            affordance.position
        )
    }));
    rows.extend(slice.replace_affordances.iter().map(|affordance| {
        format!(
            "<tr><td>replace</td><td>{}</td><td><code>{}</code></td><td>parent <code>{}</code></td></tr>",
            escape_html(&affordance.target_kind),
            escape_html(&affordance.target),
            escape_html(&affordance.parent)
        )
    }));
    if rows.is_empty() {
        return "<p class=\"muted\">No graft affordances in this slice.</p>".to_string();
    }
    format!(
        "<table><thead><tr><th>Kind</th><th>Target Kind</th><th>Target</th><th>Detail</th></tr></thead><tbody>{}</tbody></table>",
        rows.join("")
    )
}

fn print_human_report(report: &WorkbenchReport) {
    println!(
        "schema={} status={} target={} modules={} tasks={} lint_findings={} graft_templates={} transactions={}",
        report.schema,
        report.status,
        report.target,
        report.summary.module_count,
        report.summary.task_count,
        report.summary.lint_finding_count,
        report.summary.graft_template_count,
        report.summary.transaction_template_count
    );
    if let Some(html_path) = &report.html_path {
        println!("html={html_path}");
    }
}

fn load_target_program(path: &PathBuf) -> Result<Program, Vec<Diagnostic>> {
    if is_project_target(path) {
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

fn is_project_target(path: &Path) -> bool {
    path.is_dir() || path.file_name().and_then(|name| name.to_str()) == Some("sley.toml")
}

fn module_name_hint_for_target(path: &Path) -> Option<String> {
    if is_project_target(path) {
        None
    } else {
        module_name_from_sley_path(path)
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn command_text(command: &[String]) -> String {
    command.join(" ")
}

fn command_list_html(commands: &[Vec<String>]) -> String {
    if commands.is_empty() {
        return "<span class=\"muted\">none</span>".to_string();
    }
    let items = commands
        .iter()
        .map(|command| {
            format!(
                "<li><code>{}</code></li>",
                escape_html(&command_text(command))
            )
        })
        .collect::<String>();
    format!("<ol>{items}</ol>")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
