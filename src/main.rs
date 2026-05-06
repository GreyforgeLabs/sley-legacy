use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::Value as JsonValue;
use sley::Program;
use sley::checker::{check_program, has_errors};
use sley::deploy::{
    DeployArtifacts, DeployReport, build_deploy_artifact_manifest, build_deploy_report,
};
use sley::diagnostics::{Diagnostic, DiagnosticReport, RepairHint};
use sley::doctor::{DoctorReport, build_doctor_report};
use sley::formatter::format_program;
use sley::graft::{GRAFT_OUTCOME_SCHEMA, GraftInput, GraftOutcome, apply_graft_program};
use sley::lint::{LintOptions, LintReport, LintRule, build_lint_report};
use sley::parser::parse_program;
use sley::plan::{
    EditPlanOptions, EditPlanReport, build_edit_plan_report_with_options,
    module_name_from_project_relative_sley_path, module_name_from_sley_path,
};
use sley::project::{ProjectGraph, ProjectManifest, load_project};
use sley::query::{QueryKind, QueryOptions, QueryReport, build_query_report};
use sley::runtime::{RuntimeGate, RuntimeGates, build_run_report, run_main, run_main_with_gates};
use sley::scaffold::{ScaffoldOptions, ScaffoldTemplate, scaffold_project};
use sley::symbols::{
    SymbolGraphSlice, build_symbol_graph, effect_module, import_owner_module, slice_symbol_graph,
    task_module, type_module,
};
use sley::trace::{
    TraceSeal, append_trace_receipt, build_trace_receipt, build_trace_report, build_trace_seal,
    content_digest, default_trace_path, read_trace_receipts,
};
use sley::verify::{VerifyReport, build_verify_report};
use sley::zjx::build_zjx_envelope;

#[derive(Debug, Parser)]
#[command(name = "sley")]
#[command(about = "Sley Loom v0 compiler, runtime, and graft tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    New {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "app.main")]
        module: String,
        #[arg(long, value_enum, default_value = "hello")]
        template: CliScaffoldTemplate,
        path: PathBuf,
    },
    Parse {
        #[arg(long)]
        json: bool,
        file: PathBuf,
    },
    Format {
        #[arg(long)]
        write: bool,
        file: PathBuf,
    },
    Check {
        #[arg(long)]
        json: bool,
        file: PathBuf,
    },
    Run {
        #[arg(long)]
        json: bool,
        #[arg(long = "cap", value_name = "EFFECT[=SCOPE]")]
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
        file: PathBuf,
    },
    Ast {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        node: Option<String>,
        file: PathBuf,
    },
    Graph {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        slice: Option<String>,
        file: PathBuf,
    },
    Query {
        #[arg(long)]
        json: bool,
        #[arg(long, value_enum, default_value = "all")]
        kind: CliQueryKind,
        #[arg(long)]
        module: Option<String>,
        #[arg(long)]
        exported: bool,
        file: PathBuf,
    },
    Lint {
        #[arg(long)]
        json: bool,
        #[arg(long, value_enum)]
        rule: Vec<CliLintRule>,
        #[arg(long)]
        module: Option<String>,
        #[arg(long)]
        deny_warnings: bool,
        file: PathBuf,
    },
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        deny_warnings: bool,
        file: PathBuf,
    },
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        deny_warnings: bool,
        #[arg(long)]
        graft_templates: bool,
        #[arg(long, requires = "graft_templates")]
        template_surface: Option<String>,
        #[arg(long = "emit-graft", requires = "graft_templates", value_name = "KIND")]
        emit_graft: Option<String>,
        file: PathBuf,
    },
    Fix {
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "KIND")]
        kind: String,
        #[arg(long)]
        template_surface: Option<String>,
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        #[arg(long = "type", value_name = "TYPE")]
        ty: Option<String>,
        #[arg(long, value_name = "MODULE")]
        module: Option<String>,
        #[arg(long, value_name = "SOURCE")]
        source: Option<String>,
        #[arg(long, value_name = "PATH", conflicts_with = "source")]
        source_file: Option<PathBuf>,
        #[arg(long, value_name = "POSITION")]
        position: Option<usize>,
        #[arg(long)]
        write: bool,
        #[arg(long, conflicts_with = "write")]
        dry_run: bool,
        #[arg(long)]
        actor: Option<String>,
        #[arg(long)]
        trace: Option<PathBuf>,
        file: PathBuf,
    },
    Verify {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        deny_warnings: bool,
        #[arg(long = "cap", value_name = "EFFECT[=SCOPE]")]
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
        file: PathBuf,
    },
    Deploy {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long = "artifacts-dir")]
        artifacts_dir: Option<PathBuf>,
        #[arg(long, default_value = "staging")]
        environment: String,
        #[arg(long = "cap", value_name = "EFFECT[=SCOPE]")]
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
        file: PathBuf,
    },
    Trace {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        trace: Option<PathBuf>,
        file: PathBuf,
    },
    Seal {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        trace: Option<PathBuf>,
        file: PathBuf,
    },
    Zjx {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        slice: Option<String>,
        #[arg(long)]
        trace: Option<PathBuf>,
        file: PathBuf,
    },
    Graft {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        write: bool,
        #[arg(long, conflicts_with = "write")]
        dry_run: bool,
        #[arg(long)]
        actor: Option<String>,
        #[arg(long)]
        trace: Option<PathBuf>,
        file: PathBuf,
        graft: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let exit_code = match run(cli) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error:#}");
            1
        }
    };
    std::process::exit(exit_code);
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::New {
            json,
            name,
            module,
            template,
            path,
        } => match scaffold_project(
            &path,
            ScaffoldOptions {
                name,
                module,
                template: template.into(),
            },
        ) {
            Ok(report) => {
                if json {
                    print_json(&report)?;
                } else {
                    println!(
                        "created project {} at {}",
                        report.project.name, report.project.root
                    );
                    for file in &report.files {
                        println!("{} {}", file.kind, file.path);
                    }
                    println!("next:");
                    for command in &report.next_commands {
                        println!("  {}", command.join(" "));
                    }
                }
                Ok(())
            }
            Err(diagnostics) => emit_diagnostics_and_fail(diagnostics, json),
        },
        Command::Parse { json, file } => match load_target_program(&file) {
            Ok(program) => {
                if json {
                    print_json(&program)?;
                } else {
                    println!(
                        "parsed module={} imports={} types={} effects={} tasks={}",
                        program.module_name(),
                        program.imports.len(),
                        program.types.len(),
                        program.effects.len(),
                        program.tasks.len()
                    );
                }
                Ok(())
            }
            Err(diagnostics) => emit_diagnostics_and_fail(diagnostics, json),
        },
        Command::Format { write, file } => {
            let source = read_source(&file)?;
            let program = parse_program_or_fail(&source)?;
            let formatted = format_program(&program);
            if write {
                fs::write(&file, formatted)
                    .with_context(|| format!("failed to write {}", file.display()))?;
            } else {
                print!("{formatted}");
            }
            Ok(())
        }
        Command::Check { json, file } => {
            let program = match load_target_program(&file) {
                Ok(program) => program,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            let diagnostics = check_program(&program);
            if json {
                print_json(&DiagnosticReport::from_diagnostics(diagnostics.clone()))?;
            } else if diagnostics.is_empty() {
                println!("ok");
            } else {
                print_human_diagnostics(&diagnostics);
            }
            if has_errors(&diagnostics) {
                anyhow::bail!("check failed");
            }
            Ok(())
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
            file,
        } => {
            let program = match load_target_program(&file) {
                Ok(program) => program,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            let diagnostics = check_program(&program);
            if has_errors(&diagnostics) {
                return emit_diagnostics_and_fail(diagnostics, json);
            }
            let runtime_gates = build_runtime_gates(
                &cap,
                &db_table,
                &secret,
                &deploy_result,
                &spend_result,
                &http_text,
                &shell_output,
                &model_output,
            )?;
            let result = if runtime_gates.is_empty() {
                run_main(&program)
            } else {
                run_main_with_gates(&program, &runtime_gates)
            };
            match result {
                Ok(value) => {
                    if json {
                        print_json(&build_run_report(file.display().to_string(), value))?;
                    } else {
                        println!("{value:?}");
                    }
                    Ok(())
                }
                Err(diagnostics) => emit_diagnostics_and_fail(diagnostics, json),
            }
        }
        Command::Ast { json, node, file } => {
            let program = load_target_program_or_fail(&file)?;
            if let Some(node) = node {
                if let Some(report) = program.ast_node_report(&node) {
                    if json {
                        print_json(&report)?;
                    } else {
                        println!(
                            "schema={} id={} kind={} module={} parent={}",
                            report.schema,
                            report.id,
                            report.node_kind,
                            report.module,
                            report.parent.as_deref().unwrap_or("-")
                        );
                    }
                    return Ok(());
                }
                return emit_diagnostics_and_fail(
                    vec![
                        Diagnostic::error(
                            "AST_NODE_NOT_FOUND",
                            format!("AST node `{node}` was not found in {}", file.display()),
                        )
                        .with_node(node),
                    ],
                    json,
                );
            }
            if json {
                print_json(&program)?;
            } else {
                println!(
                    "schema={} module={} imports={} types={} effects={} tasks={}",
                    program.schema,
                    program.module_name(),
                    program.imports.len(),
                    program.types.len(),
                    program.effects.len(),
                    program.tasks.len()
                );
            }
            Ok(())
        }
        Command::Graph { json, slice, file } => {
            let program = match load_target_program(&file) {
                Ok(program) => program,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            if let Some(target) = slice {
                let graph_slice = slice_symbol_graph(&program, &target).ok_or_else(|| {
                    anyhow::anyhow!("graph slice target `{target}` was not found")
                })?;
                if json {
                    print_json(&graph_slice)?;
                } else {
                    print_human_graph_slice(&graph_slice);
                }
                return Ok(());
            }
            let graph = build_symbol_graph(&program);
            if json {
                print_json(&graph)?;
            } else {
                println!(
                    "entry={} modules={}",
                    graph.entry_module,
                    graph.modules.len()
                );
                for module in graph.modules {
                    println!(
                        "module {} imports={} types={} effects={} tasks={}",
                        module.module,
                        module.imports.len(),
                        module.types.len(),
                        module.effects.len(),
                        module.tasks.len()
                    );
                }
            }
            Ok(())
        }
        Command::Query {
            json,
            kind,
            module,
            exported,
            file,
        } => {
            let program = match load_target_program(&file) {
                Ok(program) => program,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            let diagnostics = check_program(&program);
            if has_errors(&diagnostics) {
                return emit_diagnostics_and_fail(diagnostics, json);
            }
            let report = build_query_report(
                &program,
                QueryOptions {
                    kind: kind.into(),
                    module,
                    exported_only: exported,
                },
            );
            if json {
                print_json(&report)?;
            } else {
                print_human_query_report(&report);
            }
            Ok(())
        }
        Command::Lint {
            json,
            rule,
            module,
            deny_warnings,
            file,
        } => {
            let program = match load_target_program(&file) {
                Ok(program) => program,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            let diagnostics = check_program(&program);
            if has_errors(&diagnostics) {
                return emit_diagnostics_and_fail(diagnostics, json);
            }
            let report = build_lint_report(
                &program,
                LintOptions {
                    rules: rule.into_iter().map(Into::into).collect(),
                    module,
                },
            );
            let has_findings = !report.findings.is_empty();
            if json {
                print_json(&report)?;
            } else {
                print_human_lint_report(&report);
            }
            if deny_warnings && has_findings {
                anyhow::bail!("lint findings found");
            }
            Ok(())
        }
        Command::Doctor {
            json,
            deny_warnings,
            file,
        } => {
            let target = file.display().to_string();
            let report = build_doctor_report(target, load_target_program(&file), deny_warnings);
            if json {
                print_json(&report)?;
            } else {
                print_human_doctor_report(&report);
            }
            if report.status == "blocked" {
                anyhow::bail!("doctor blocked");
            }
            Ok(())
        }
        Command::Plan {
            json,
            deny_warnings,
            graft_templates,
            template_surface,
            emit_graft,
            file,
        } => {
            let target = file.display().to_string();
            let report = build_edit_plan_report_with_options(
                target,
                load_target_program(&file),
                edit_plan_options_for_target(
                    &file,
                    deny_warnings,
                    graft_templates,
                    template_surface,
                ),
            );
            if let Some(kind) = emit_graft {
                if report.status == "blocked" {
                    anyhow::bail!("plan blocked");
                }
                emit_planned_graft(&report, &kind)?;
                return Ok(());
            }
            if json {
                print_json(&report)?;
            } else {
                print_human_edit_plan_report(&report);
            }
            if report.status == "blocked" {
                anyhow::bail!("plan blocked");
            }
            Ok(())
        }
        Command::Fix {
            json,
            kind,
            template_surface,
            name,
            ty,
            module,
            source,
            source_file,
            position,
            write,
            dry_run,
            actor,
            trace,
            file,
        } => {
            let target = file.display().to_string();
            let program = match load_target_program(&file) {
                Ok(program) => program,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            let report = build_edit_plan_report_with_options(
                target,
                Ok(program),
                edit_plan_options_for_target(&file, false, true, template_surface),
            );
            if report.status == "blocked" {
                let outcome = rejected_graft_outcome(report.diagnostics);
                emit_graft_outcome(&outcome, json)?;
                anyhow::bail!("fix plan blocked");
            }
            let source = match load_fix_source_override(source, source_file) {
                Ok(source) => source,
                Err(diagnostic) => {
                    let outcome = rejected_graft_outcome(vec![diagnostic]);
                    emit_graft_outcome(&outcome, json)?;
                    anyhow::bail!("fix source override unavailable");
                }
            };
            let graft_value = match planned_graft_value(&report, &kind).and_then(|value| {
                apply_fix_template_overrides(value, &kind, name, ty, module, source, position)
            }) {
                Ok(value) => value,
                Err(diagnostic) => {
                    let outcome = rejected_graft_outcome(vec![diagnostic]);
                    emit_graft_outcome(&outcome, json)?;
                    anyhow::bail!("fix graft not found");
                }
            };
            let graft_input: GraftInput = match serde_json::from_value(graft_value) {
                Ok(input) => input,
                Err(error) => {
                    let outcome = rejected_graft_outcome(vec![Diagnostic::error(
                        "FIX_GRAFT_TEMPLATE_INVALID",
                        format!("planned graft kind `{kind}` did not parse as a graft: {error}"),
                    )]);
                    emit_graft_outcome(&outcome, json)?;
                    anyhow::bail!("fix graft invalid");
                }
            };
            let outcome = match apply_graft_to_target(
                &file,
                graft_input,
                actor,
                trace.as_deref(),
                write,
                dry_run,
            ) {
                Ok(outcome) => outcome,
                Err(diagnostics) => rejected_graft_outcome(diagnostics),
            };
            emit_graft_outcome(&outcome, json)?;
            if outcome.status != "accepted" {
                anyhow::bail!("fix rejected");
            }
            Ok(())
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
            file,
        } => {
            let runtime_gates = build_runtime_gates(
                &cap,
                &db_table,
                &secret,
                &deploy_result,
                &spend_result,
                &http_text,
                &shell_output,
                &model_output,
            )?;
            let target = file.display().to_string();
            let report = build_verify_report(
                target,
                load_target_program(&file),
                runtime_gates,
                deny_warnings,
            );
            if json {
                print_json(&report)?;
            } else {
                print_human_verify_report(&report);
            }
            if report.status == "blocked" {
                anyhow::bail!("verify blocked");
            }
            Ok(())
        }
        Command::Deploy {
            json,
            dry_run,
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
            file,
        } => {
            if !dry_run {
                anyhow::bail!("sley deploy is report-only in v0; rerun with --dry-run");
            }
            let runtime_gates = build_runtime_gates(
                &cap,
                &db_table,
                &secret,
                &deploy_result,
                &spend_result,
                &http_text,
                &shell_output,
                &model_output,
            )?;
            let target = file.display().to_string();
            let (verify, seal, package) = match load_target_program_and_source_bytes(&file) {
                Ok((program, source_bytes)) => {
                    let verify = build_verify_report(
                        target.clone(),
                        Ok(program.clone()),
                        runtime_gates,
                        true,
                    );
                    if verify.status == "passed" {
                        let trace_path = default_trace_path(&file);
                        let trace_receipts = read_trace_receipts(&trace_path)?;
                        let seal = build_trace_seal(
                            target.clone(),
                            &source_bytes,
                            &program,
                            &trace_receipts,
                        )?;
                        let package = build_zjx_envelope(
                            target.clone(),
                            build_symbol_graph(&program),
                            None,
                            trace_receipts,
                        );
                        (verify, Some(seal), Some(package))
                    } else {
                        (verify, None, None)
                    }
                }
                Err(diagnostics) => {
                    let verify =
                        build_verify_report(target.clone(), Err(diagnostics), runtime_gates, true);
                    (verify, None, None)
                }
            };
            let artifacts = if verify.status == "passed" {
                artifacts_dir
                    .as_ref()
                    .map(|directory| deploy_artifacts(directory))
            } else {
                None
            };
            let report = build_deploy_report(
                target,
                environment,
                verify,
                seal.clone(),
                package.clone(),
                artifacts,
            );
            if let Some(directory) = &artifacts_dir
                && report.status == "ready"
            {
                write_deploy_artifacts(directory, &report, seal.as_ref(), package.as_ref())?;
            }
            if json {
                print_json(&report)?;
            } else {
                print_human_deploy_report(&report);
            }
            if report.status == "blocked" {
                anyhow::bail!("deploy dry-run blocked");
            }
            Ok(())
        }
        Command::Trace { json, trace, file } => {
            let trace_path = trace.unwrap_or_else(|| default_trace_path(&file));
            let receipts = read_trace_receipts(&trace_path)?;
            if json {
                print_json(&build_trace_report(
                    file.display().to_string(),
                    &trace_path,
                    receipts,
                ))?;
            } else {
                print_human_trace(&trace_path, &receipts);
            }
            Ok(())
        }
        Command::Seal { json, trace, file } => {
            let (program, source_bytes) = match load_target_program_and_source_bytes(&file) {
                Ok(target) => target,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            let trace_path = trace.unwrap_or_else(|| default_trace_path(&file));
            let receipts = read_trace_receipts(&trace_path)?;
            let seal = build_trace_seal(
                file.display().to_string(),
                &source_bytes,
                &program,
                &receipts,
            )?;
            if json {
                print_json(&seal)?;
            } else {
                print_human_seal(&seal);
            }
            Ok(())
        }
        Command::Zjx {
            json,
            slice,
            trace,
            file,
        } => {
            let program = match load_target_program(&file) {
                Ok(program) => program,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            let graph = build_symbol_graph(&program);
            let graph_slice = if let Some(target) = slice {
                Some(slice_symbol_graph(&program, &target).ok_or_else(|| {
                    anyhow::anyhow!("graph slice target `{target}` was not found")
                })?)
            } else {
                None
            };
            let trace_path = trace.unwrap_or_else(|| default_trace_path(&file));
            let trace_receipts = read_trace_receipts(&trace_path)?;
            let envelope = build_zjx_envelope(
                file.display().to_string(),
                graph,
                graph_slice,
                trace_receipts,
            );
            if json {
                print_json(&envelope)?;
            } else {
                println!(
                    "zjx envelope schema={} format={} compression={} graph_digest={} modules={} trace_receipts={}",
                    envelope.schema,
                    envelope.format,
                    envelope.compression,
                    envelope.graph_digest,
                    envelope.graph.modules.len(),
                    envelope.trace_receipts.len()
                );
            }
            Ok(())
        }
        Command::Graft {
            json,
            write,
            dry_run,
            actor,
            trace,
            file,
            graft,
        } => {
            let graft_source = fs::read_to_string(&graft)
                .with_context(|| format!("failed to read {}", graft.display()))?;
            let graft_input: GraftInput = serde_json::from_str(&graft_source)
                .with_context(|| format!("failed to parse {}", graft.display()))?;
            let outcome = match apply_graft_to_target(
                &file,
                graft_input,
                actor,
                trace.as_deref(),
                write,
                dry_run,
            ) {
                Ok(outcome) => outcome,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            emit_graft_outcome(&outcome, json)?;
            if outcome.status != "accepted" {
                anyhow::bail!("graft rejected");
            }
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliQueryKind {
    All,
    Modules,
    Tasks,
    Types,
    Effects,
    Calls,
}

impl From<CliQueryKind> for QueryKind {
    fn from(kind: CliQueryKind) -> Self {
        match kind {
            CliQueryKind::All => Self::All,
            CliQueryKind::Modules => Self::Modules,
            CliQueryKind::Tasks => Self::Tasks,
            CliQueryKind::Types => Self::Types,
            CliQueryKind::Effects => Self::Effects,
            CliQueryKind::Calls => Self::Calls,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliLintRule {
    UnusedPrivateTask,
    UnreachablePrivateTask,
    UnusedDeclaredEffect,
    UnusedImport,
    UnusedTake,
    UnusedPrivateType,
    UnusedPrivateEffect,
    RawHostAdapter,
    MissingModuleDeclaration,
    UncheckedResult,
    UnqualifiedImportedCall,
    UnusedPureBinding,
    UnusedPureExpressionStatement,
    MutableBindingNeverSet,
    ConstantIfExpression,
    ConstantIfStatement,
    ConstantFalseIfStatement,
    ConstantFalseWhileStatement,
    ConstantComparisonExpression,
    ConstantArithmeticExpression,
    ConstantTextConcatenationExpression,
    ConstantListIndexExpression,
    ConstantMapIndexExpression,
    ConstantRecordFieldAccessExpression,
    ConstantLenExpression,
    ConstantNotExpression,
    EmptyIfStatement,
    EmptyForStatement,
    EmptyForgeStatement,
    IdentityBinaryExpression,
    RedundantBooleanComparison,
    AbsorbingBooleanExpression,
    SelfComparisonExpression,
    DoubleNegationExpression,
    NegatedComparisonExpression,
    RedundantBooleanIfExpression,
    RedundantBooleanIfStatement,
    SameBranchIfExpression,
    SameBranchIfStatement,
    UnreachableStatement,
    AbsorbingArithmeticExpression,
    SelfAssignmentStatement,
    OverwrittenSetStatement,
    RedundantInitialSetStatement,
}

impl From<CliLintRule> for LintRule {
    fn from(rule: CliLintRule) -> Self {
        match rule {
            CliLintRule::UnusedPrivateTask => Self::UnusedPrivateTask,
            CliLintRule::UnreachablePrivateTask => Self::UnreachablePrivateTask,
            CliLintRule::UnusedDeclaredEffect => Self::UnusedDeclaredEffect,
            CliLintRule::UnusedImport => Self::UnusedImport,
            CliLintRule::UnusedTake => Self::UnusedTake,
            CliLintRule::UnusedPrivateType => Self::UnusedPrivateType,
            CliLintRule::UnusedPrivateEffect => Self::UnusedPrivateEffect,
            CliLintRule::RawHostAdapter => Self::RawHostAdapter,
            CliLintRule::MissingModuleDeclaration => Self::MissingModuleDeclaration,
            CliLintRule::UncheckedResult => Self::UncheckedResult,
            CliLintRule::UnqualifiedImportedCall => Self::UnqualifiedImportedCall,
            CliLintRule::UnusedPureBinding => Self::UnusedPureBinding,
            CliLintRule::UnusedPureExpressionStatement => Self::UnusedPureExpressionStatement,
            CliLintRule::MutableBindingNeverSet => Self::MutableBindingNeverSet,
            CliLintRule::ConstantIfExpression => Self::ConstantIfExpression,
            CliLintRule::ConstantIfStatement => Self::ConstantIfStatement,
            CliLintRule::ConstantFalseIfStatement => Self::ConstantFalseIfStatement,
            CliLintRule::ConstantFalseWhileStatement => Self::ConstantFalseWhileStatement,
            CliLintRule::ConstantComparisonExpression => Self::ConstantComparisonExpression,
            CliLintRule::ConstantArithmeticExpression => Self::ConstantArithmeticExpression,
            CliLintRule::ConstantTextConcatenationExpression => {
                Self::ConstantTextConcatenationExpression
            }
            CliLintRule::ConstantListIndexExpression => Self::ConstantListIndexExpression,
            CliLintRule::ConstantMapIndexExpression => Self::ConstantMapIndexExpression,
            CliLintRule::ConstantRecordFieldAccessExpression => {
                Self::ConstantRecordFieldAccessExpression
            }
            CliLintRule::ConstantLenExpression => Self::ConstantLenExpression,
            CliLintRule::ConstantNotExpression => Self::ConstantNotExpression,
            CliLintRule::EmptyIfStatement => Self::EmptyIfStatement,
            CliLintRule::EmptyForStatement => Self::EmptyForStatement,
            CliLintRule::EmptyForgeStatement => Self::EmptyForgeStatement,
            CliLintRule::IdentityBinaryExpression => Self::IdentityBinaryExpression,
            CliLintRule::RedundantBooleanComparison => Self::RedundantBooleanComparison,
            CliLintRule::AbsorbingBooleanExpression => Self::AbsorbingBooleanExpression,
            CliLintRule::SelfComparisonExpression => Self::SelfComparisonExpression,
            CliLintRule::DoubleNegationExpression => Self::DoubleNegationExpression,
            CliLintRule::NegatedComparisonExpression => Self::NegatedComparisonExpression,
            CliLintRule::RedundantBooleanIfExpression => Self::RedundantBooleanIfExpression,
            CliLintRule::RedundantBooleanIfStatement => Self::RedundantBooleanIfStatement,
            CliLintRule::SameBranchIfExpression => Self::SameBranchIfExpression,
            CliLintRule::SameBranchIfStatement => Self::SameBranchIfStatement,
            CliLintRule::UnreachableStatement => Self::UnreachableStatement,
            CliLintRule::AbsorbingArithmeticExpression => Self::AbsorbingArithmeticExpression,
            CliLintRule::SelfAssignmentStatement => Self::SelfAssignmentStatement,
            CliLintRule::OverwrittenSetStatement => Self::OverwrittenSetStatement,
            CliLintRule::RedundantInitialSetStatement => Self::RedundantInitialSetStatement,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliScaffoldTemplate {
    Hello,
    Deploy,
    Agent,
}

impl From<CliScaffoldTemplate> for ScaffoldTemplate {
    fn from(template: CliScaffoldTemplate) -> Self {
        match template {
            CliScaffoldTemplate::Hello => Self::Hello,
            CliScaffoldTemplate::Deploy => Self::Deploy,
            CliScaffoldTemplate::Agent => Self::Agent,
        }
    }
}

fn write_project_graft(
    target: &Path,
    project: &ProjectGraph,
    candidate: &Program,
    provenance: Vec<sley::ast::ProvenanceRecord>,
    trace: Option<&Path>,
) -> Result<(), Vec<Diagnostic>> {
    let plan = plan_project_writeback(project, candidate)?;
    for write in plan.module_writes {
        if write.source.is_some()
            && let Some(parent) = write.path.parent()
        {
            fs::create_dir_all(parent).map_err(|error| {
                vec![
                    Diagnostic::error(
                        "PROJECT_WRITEBACK_FAILED",
                        format!(
                            "failed to create parent directory for module `{}` at {}: {error}",
                            write.module,
                            parent.display()
                        ),
                    )
                    .with_node(format!("module:{}", write.module)),
                ]
            })?;
        }
        if write.create {
            let Some(source) = write.source.as_deref() else {
                return Err(vec![
                    Diagnostic::error(
                        "PROJECT_WRITEBACK_FAILED",
                        format!("new module `{}` has no source to write", write.module),
                    )
                    .with_node(format!("module:{}", write.module)),
                ]);
            };
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&write.path)
                .map_err(|error| {
                    vec![
                        Diagnostic::error(
                            "PROJECT_WRITEBACK_FAILED",
                            format!(
                                "failed to create module `{}` at {}: {error}",
                                write.module,
                                write.path.display()
                            ),
                        )
                        .with_node(format!("module:{}", write.module)),
                    ]
                })?;
            file.write_all(source.as_bytes()).map_err(|error| {
                vec![
                    Diagnostic::error(
                        "PROJECT_WRITEBACK_FAILED",
                        format!(
                            "failed to write module `{}` at {}: {error}",
                            write.module,
                            write.path.display()
                        ),
                    )
                    .with_node(format!("module:{}", write.module)),
                ]
            })?;
        } else if let Some(source) = write.source.as_deref() {
            fs::write(&write.path, source).map_err(|error| {
                vec![
                    Diagnostic::error(
                        "PROJECT_WRITEBACK_FAILED",
                        format!(
                            "failed to write module `{}` at {}: {error}",
                            write.module,
                            write.path.display()
                        ),
                    )
                    .with_node(format!("module:{}", write.module)),
                ]
            })?;
        } else {
            fs::remove_file(&write.path).map_err(|error| {
                vec![
                    Diagnostic::error(
                        "PROJECT_WRITEBACK_FAILED",
                        format!(
                            "failed to delete module `{}` at {}: {error}",
                            write.module,
                            write.path.display()
                        ),
                    )
                    .with_node(format!("module:{}", write.module)),
                ]
            })?;
        }
    }
    if let Some(manifest_source) = plan.manifest_source {
        fs::write(&project.manifest_path, manifest_source).map_err(|error| {
            vec![Diagnostic::error(
                "PROJECT_MANIFEST_WRITE_FAILED",
                format!(
                    "failed to write project manifest {}: {error}",
                    project.manifest_path.display()
                ),
            )]
        })?;
    }

    if !provenance.is_empty() {
        let trace_path = trace
            .map(Path::to_path_buf)
            .unwrap_or_else(|| default_trace_path(target));
        let receipt = build_trace_receipt(target, provenance);
        append_trace_receipt(&trace_path, &receipt).map_err(|error| {
            vec![Diagnostic::error(
                "PROJECT_TRACE_WRITE_FAILED",
                format!("{error:#}"),
            )]
        })?;
    }
    Ok(())
}

fn apply_graft_to_target(
    target: &PathBuf,
    graft_input: GraftInput,
    actor: Option<String>,
    trace: Option<&Path>,
    write: bool,
    dry_run: bool,
) -> Result<GraftOutcome, Vec<Diagnostic>> {
    if is_project_target(target) {
        let project = load_project(target)?;
        let applied = apply_graft_program(&project.program, graft_input, actor);
        let mut outcome = applied.outcome;
        if outcome.status == "accepted" && write && !dry_run {
            let candidate = applied
                .program
                .as_ref()
                .expect("accepted graft should carry candidate program");
            if let Err(diagnostics) = write_project_graft(
                target,
                &project,
                candidate,
                outcome.provenance.clone(),
                trace,
            ) {
                outcome = rejected_graft_outcome(diagnostics);
            }
        }
        return Ok(outcome);
    }

    let source = fs::read_to_string(target).map_err(|error| {
        vec![Diagnostic::error(
            "SOURCE_READ_FAILED",
            format!("failed to read {}: {error}", target.display()),
        )]
    })?;
    let program = parse_program(&source)?;
    let mut outcome = apply_graft_program(&program, graft_input, actor).outcome;
    if outcome.status == "accepted"
        && write
        && !dry_run
        && let Some(source) = outcome.source.as_deref()
        && let Err(diagnostics) =
            write_file_graft(target, source, outcome.provenance.clone(), trace)
    {
        outcome = rejected_graft_outcome(diagnostics);
    }
    Ok(outcome)
}

fn write_file_graft(
    target: &Path,
    source: &str,
    provenance: Vec<sley::ast::ProvenanceRecord>,
    trace: Option<&Path>,
) -> Result<(), Vec<Diagnostic>> {
    fs::write(target, source).map_err(|error| {
        vec![Diagnostic::error(
            "SOURCE_WRITE_FAILED",
            format!("failed to write {}: {error}", target.display()),
        )]
    })?;
    if !provenance.is_empty() {
        let trace_path = trace
            .map(Path::to_path_buf)
            .unwrap_or_else(|| default_trace_path(target));
        let receipt = build_trace_receipt(target, provenance);
        append_trace_receipt(&trace_path, &receipt).map_err(|error| {
            vec![Diagnostic::error(
                "TRACE_WRITE_FAILED",
                format!("{error:#}"),
            )]
        })?;
    }
    Ok(())
}

struct ProjectWritebackPlan {
    manifest_source: Option<String>,
    module_writes: Vec<ProjectWrite>,
}

struct ProjectWrite {
    module: String,
    path: PathBuf,
    source: Option<String>,
    create: bool,
}

fn plan_project_writeback(
    project: &ProjectGraph,
    candidate: &Program,
) -> Result<ProjectWritebackPlan, Vec<Diagnostic>> {
    let known_modules = project
        .modules
        .iter()
        .map(|module| module.module.clone())
        .collect::<BTreeSet<_>>();
    let declared_modules = candidate_declared_modules(candidate);
    let new_modules = declared_modules
        .iter()
        .filter(|module| !known_modules.contains(*module))
        .cloned()
        .collect::<BTreeSet<_>>();
    let imported_modules = candidate
        .imports
        .iter()
        .map(|import| import.module.clone())
        .collect::<BTreeSet<_>>();
    let removed_modules = known_modules
        .iter()
        .filter(|module| !declared_modules.contains(*module) && !imported_modules.contains(*module))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();

    for module in &new_modules {
        if let Some(diagnostic) = validate_project_writeback_module(module) {
            diagnostics.push(diagnostic);
            continue;
        }
        let path = module_source_path(&project.module_root, module);
        if path.exists() {
            diagnostics.push(
                Diagnostic::error(
                    "PROJECT_WRITEBACK_FILE_EXISTS",
                    format!(
                        "refusing to create module `{module}` because {} already exists",
                        path.display()
                    ),
                )
                .with_node(format!("module:{module}")),
            );
        }
    }

    for import in &candidate.imports {
        if known_modules.contains(&import.module) || new_modules.contains(&import.module) {
            continue;
        }
        if let Some(diagnostic) = validate_project_writeback_module(&import.module) {
            diagnostics.push(diagnostic.with_node(import.id.clone()));
            continue;
        }
        match validate_existing_project_import(project, &import.module, &import.id) {
            Ok(true) => {}
            Ok(false) => diagnostics.push(
                Diagnostic::error(
                    "PROJECT_WRITEBACK_UNKNOWN_IMPORT",
                    format!(
                        "graft candidate imports `{}`, but project writeback cannot create missing module files",
                        import.module
                    ),
                )
                .with_node(import.id.clone()),
            ),
            Err(mut import_diagnostics) => {
                diagnostics.append(&mut import_diagnostics);
            }
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let manifest_source = if candidate.module_name() != project.entry {
        Some(render_project_manifest_with_entry(
            project,
            candidate.module_name(),
        )?)
    } else {
        None
    };

    let mut module_writes = Vec::new();
    for module in &project.modules {
        if removed_modules.contains(&module.module) {
            module_writes.push(ProjectWrite {
                module: module.module.clone(),
                path: module.path.clone(),
                source: None,
                create: false,
            });
            continue;
        }
        let next_program = project_module_program(candidate, &module.module);
        let current_source = format_program(&module.program);
        let next_source = format_program(&next_program);
        if current_source != next_source {
            module_writes.push(ProjectWrite {
                module: module.module.clone(),
                path: module.path.clone(),
                source: Some(next_source),
                create: false,
            });
        }
    }
    for module in new_modules {
        let next_program = project_module_program(candidate, &module);
        let next_source = format_program(&next_program);
        module_writes.push(ProjectWrite {
            module: module.clone(),
            path: module_source_path(&project.module_root, &module),
            source: Some(next_source),
            create: true,
        });
    }
    Ok(ProjectWritebackPlan {
        manifest_source,
        module_writes,
    })
}

fn validate_existing_project_import(
    project: &ProjectGraph,
    module: &str,
    import_node: &str,
) -> Result<bool, Vec<Diagnostic>> {
    let path = module_source_path(&project.module_root, module);
    if !path.exists() {
        return Ok(false);
    }
    let source = fs::read_to_string(&path).map_err(|error| {
        vec![
            Diagnostic::error(
                "PROJECT_WRITEBACK_IMPORT_READ_FAILED",
                format!(
                    "failed to read existing imported module `{module}` at {}: {error}",
                    path.display()
                ),
            )
            .with_node(import_node.to_string()),
        ]
    })?;
    let parsed = parse_program(&source).map_err(|mut diagnostics| {
        for diagnostic in &mut diagnostics {
            diagnostic.message = format!(
                "{} in existing imported module `{module}` at {}",
                diagnostic.message,
                path.display()
            );
            if diagnostic.node.is_none() {
                diagnostic.node = Some(import_node.to_string());
            }
        }
        diagnostics
    })?;
    if parsed.module.as_deref() != Some(module) {
        return Err(vec![
            Diagnostic::error(
                "PROJECT_WRITEBACK_IMPORT_MISMATCH",
                format!(
                    "existing imported module file {} declares `{}` but graft imports `{module}`",
                    path.display(),
                    parsed.module_name()
                ),
            )
            .with_node(import_node.to_string()),
        ]);
    }
    Ok(true)
}

fn render_project_manifest_with_entry(
    project: &ProjectGraph,
    entry: &str,
) -> Result<String, Vec<Diagnostic>> {
    let mut manifest: toml::Value = toml::from_str(&project.manifest_source).map_err(|error| {
        vec![Diagnostic::error(
            "PROJECT_MANIFEST_WRITEBACK_FAILED",
            format!(
                "failed to parse project manifest {} for writeback: {error}",
                project.manifest_path.display()
            ),
        )]
    })?;
    let Some(project_table) = manifest
        .get_mut("project")
        .and_then(toml::Value::as_table_mut)
    else {
        return Err(vec![Diagnostic::error(
            "PROJECT_MANIFEST_WRITEBACK_FAILED",
            format!(
                "project manifest {} does not contain a `[project]` table",
                project.manifest_path.display()
            ),
        )]);
    };
    project_table.insert("entry".to_string(), toml::Value::String(entry.to_string()));
    let mut source = toml::to_string_pretty(&manifest).map_err(|error| {
        vec![Diagnostic::error(
            "PROJECT_MANIFEST_WRITEBACK_FAILED",
            format!(
                "failed to render project manifest {} for writeback: {error}",
                project.manifest_path.display()
            ),
        )]
    })?;
    if !source.ends_with('\n') {
        source.push('\n');
    }
    Ok(source)
}

fn module_source_path(root: &Path, module: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in module.split('.') {
        path.push(segment);
    }
    path.set_extension("sley");
    path
}

fn validate_project_writeback_module(module: &str) -> Option<Diagnostic> {
    if module.trim().is_empty() {
        return Some(Diagnostic::error(
            "PROJECT_WRITEBACK_INVALID_MODULE",
            "new module path cannot be empty",
        ));
    }
    for segment in module.split('.') {
        if !is_module_segment(segment) {
            return Some(
                Diagnostic::error(
                    "PROJECT_WRITEBACK_INVALID_MODULE",
                    format!("new module path `{module}` contains invalid segment `{segment}`"),
                )
                .with_node(format!("module:{module}")),
            );
        }
    }
    None
}

fn is_module_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn candidate_declared_modules(program: &Program) -> BTreeSet<String> {
    let mut modules = BTreeSet::new();
    modules.insert(program.module_name().to_string());
    for import in &program.imports {
        modules.insert(import_owner_module(import));
    }
    for ty in &program.types {
        modules.insert(type_module(ty));
    }
    for effect in &program.effects {
        modules.insert(effect_module(effect));
    }
    for task in &program.tasks {
        modules.insert(task_module(task));
    }
    modules
}

fn project_module_program(candidate: &Program, module: &str) -> Program {
    let mut program = Program::new();
    program.module = Some(module.to_string());
    program.imports = candidate
        .imports
        .iter()
        .filter(|import| import_owner_module(import) == module)
        .cloned()
        .collect();
    program.types = candidate
        .types
        .iter()
        .filter(|ty| type_module(ty) == module)
        .cloned()
        .collect();
    program.effects = candidate
        .effects
        .iter()
        .filter(|effect| effect_module(effect) == module)
        .cloned()
        .collect();
    program.tasks = candidate
        .tasks
        .iter()
        .filter(|task| task_module(task) == module)
        .cloned()
        .collect();
    program.assign_ids();
    program
}

fn emit_graft_outcome(outcome: &GraftOutcome, json: bool) -> Result<()> {
    if json {
        print_json(outcome)?;
    } else if let Some(source) = &outcome.source {
        print!("{source}");
    } else {
        print_human_diagnostics(&outcome.diagnostics);
    }
    Ok(())
}

fn rejected_graft_outcome(diagnostics: Vec<Diagnostic>) -> GraftOutcome {
    GraftOutcome {
        schema: GRAFT_OUTCOME_SCHEMA.to_string(),
        status: "rejected".to_string(),
        diagnostics,
        source: None,
        provenance: Vec::new(),
    }
}

fn parse_runtime_gates(values: &[String]) -> Result<RuntimeGates> {
    let mut gates = RuntimeGates::new();
    for value in values {
        let (effect, root) = value
            .split_once('=')
            .map_or((value.as_str(), None), |(effect, root)| {
                (effect, Some(root))
            });
        let effect = effect.trim();
        if effect.is_empty() {
            anyhow::bail!("runtime capability effect cannot be empty");
        }
        match root {
            Some(root) if !root.trim().is_empty() => {
                if matches!(effect, "FileRead" | "FileWrite") {
                    gates.grant(RuntimeGate::with_root(effect, PathBuf::from(root.trim())));
                } else {
                    gates.grant(RuntimeGate::with_scope(effect, root.trim()));
                }
            }
            Some(_) => anyhow::bail!("runtime capability `{effect}` has an empty scope"),
            None => gates.grant_effect(effect),
        }
    }
    Ok(gates)
}

fn build_runtime_gates(
    cap: &[String],
    db_table: &[String],
    secret: &[String],
    deploy_result: &[String],
    spend_result: &[String],
    http_text: &[String],
    shell_output: &[String],
    model_output: &[String],
) -> Result<RuntimeGates> {
    let mut gates = parse_runtime_gates(cap)?;
    load_runtime_db_tables(&mut gates, db_table)?;
    load_runtime_secrets(&mut gates, secret)?;
    load_runtime_deploy_results(&mut gates, deploy_result)?;
    load_runtime_spend_results(&mut gates, spend_result)?;
    load_runtime_http_texts(&mut gates, http_text)?;
    load_runtime_shell_outputs(&mut gates, shell_output)?;
    load_runtime_model_outputs(&mut gates, model_output)?;
    Ok(gates)
}

fn load_runtime_db_tables(gates: &mut RuntimeGates, values: &[String]) -> Result<()> {
    for value in values {
        let (table, path) = value
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("database table seed must use TABLE=JSON"))?;
        let table = table.trim();
        if table.is_empty() {
            anyhow::bail!("database table seed name cannot be empty");
        }
        let path = PathBuf::from(path.trim());
        if path.as_os_str().is_empty() {
            anyhow::bail!("database table `{table}` seed path cannot be empty");
        }
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read database table seed {}", path.display()))?;
        let json: serde_json::Value = serde_json::from_str(&source)
            .with_context(|| format!("failed to parse database table seed {}", path.display()))?;
        let serde_json::Value::Array(items) = json else {
            anyhow::bail!("database table `{table}` seed must be a JSON array of row objects");
        };
        let mut rows = Vec::new();
        for (index, item) in items.into_iter().enumerate() {
            let serde_json::Value::Object(fields) = item else {
                anyhow::bail!("database table `{table}` row {index} must be a JSON object");
            };
            let mut row = std::collections::BTreeMap::new();
            for (name, value) in fields {
                row.insert(
                    name,
                    sley::runtime::Value::try_from(value).map_err(anyhow::Error::msg)?,
                );
            }
            rows.push(row);
        }
        gates.grant_db_rows(table, rows);
    }
    Ok(())
}

fn load_runtime_secrets(gates: &mut RuntimeGates, values: &[String]) -> Result<()> {
    let mut chunks = values.chunks_exact(2);
    for pair in &mut chunks {
        let name = pair[0].trim();
        if name.is_empty() {
            anyhow::bail!("secret seed name cannot be empty");
        }
        gates.grant_secret(name, pair[1].clone());
    }
    if !chunks.remainder().is_empty() {
        anyhow::bail!("secret seed must use NAME TEXT pairs");
    }
    Ok(())
}

fn load_runtime_deploy_results(gates: &mut RuntimeGates, values: &[String]) -> Result<()> {
    let mut chunks = values.chunks_exact(2);
    for pair in &mut chunks {
        let target = pair[0].trim();
        if target.is_empty() {
            anyhow::bail!("deploy result seed target cannot be empty");
        }
        gates.grant_deploy_result(target, pair[1].clone());
    }
    if !chunks.remainder().is_empty() {
        anyhow::bail!("deploy result seed must use TARGET TEXT pairs");
    }
    Ok(())
}

fn load_runtime_spend_results(gates: &mut RuntimeGates, values: &[String]) -> Result<()> {
    let mut chunks = values.chunks_exact(2);
    for pair in &mut chunks {
        let request = pair[0].trim();
        if request.is_empty() {
            anyhow::bail!("spend result seed request cannot be empty");
        }
        gates.grant_spend_result(request, pair[1].clone());
    }
    if !chunks.remainder().is_empty() {
        anyhow::bail!("spend result seed must use REQUEST TEXT pairs");
    }
    Ok(())
}

fn load_runtime_http_texts(gates: &mut RuntimeGates, values: &[String]) -> Result<()> {
    let mut chunks = values.chunks_exact(2);
    for pair in &mut chunks {
        let url = pair[0].trim();
        if url.is_empty() {
            anyhow::bail!("HTTP text seed URL cannot be empty");
        }
        gates.grant_http_text(url, pair[1].clone());
    }
    if !chunks.remainder().is_empty() {
        anyhow::bail!("HTTP text seed must use URL TEXT pairs");
    }
    Ok(())
}

fn load_runtime_shell_outputs(gates: &mut RuntimeGates, values: &[String]) -> Result<()> {
    let mut chunks = values.chunks_exact(2);
    for pair in &mut chunks {
        let command = pair[0].trim();
        if command.is_empty() {
            anyhow::bail!("shell output seed command cannot be empty");
        }
        gates.grant_shell_output(command, pair[1].clone());
    }
    if !chunks.remainder().is_empty() {
        anyhow::bail!("shell output seed must use COMMAND TEXT pairs");
    }
    Ok(())
}

fn load_runtime_model_outputs(gates: &mut RuntimeGates, values: &[String]) -> Result<()> {
    let mut chunks = values.chunks_exact(2);
    for pair in &mut chunks {
        let prompt = pair[0].trim();
        if prompt.is_empty() {
            anyhow::bail!("model output seed prompt cannot be empty");
        }
        gates.grant_model_output(prompt, pair[1].clone());
    }
    if !chunks.remainder().is_empty() {
        anyhow::bail!("model output seed must use PROMPT TEXT pairs");
    }
    Ok(())
}

fn read_source(file: &PathBuf) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))
}

fn emit_planned_graft(report: &EditPlanReport, kind: &str) -> Result<()> {
    print_json(
        &planned_graft_value(report, kind)
            .map_err(|diagnostic| anyhow::anyhow!("{}: {}", diagnostic.id, diagnostic.message))?,
    )
}

fn planned_graft_value(
    report: &EditPlanReport,
    kind: &str,
) -> std::result::Result<JsonValue, Diagnostic> {
    let matches = report
        .graft_templates
        .iter()
        .filter(|template| template.kind == kind)
        .map(|template| &template.operation)
        .chain(
            report
                .transaction_templates
                .iter()
                .filter(|template| template.kind == kind)
                .map(|template| &template.transaction),
        )
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(Diagnostic::error(
            "PLAN_GRAFT_KIND_NOT_FOUND",
            format!("no graft or transaction template matched kind `{kind}`"),
        )
        .with_repair_hint(RepairHint::new("inspect_graft_templates").with_replacement(
            "Run `sley plan --json --graft-templates <target>` and choose a template kind",
        ))),
        [value] => Ok((*value).clone()),
        _ => Err(Diagnostic::error(
            "PLAN_GRAFT_KIND_AMBIGUOUS",
            format!(
                "multiple graft or transaction templates matched kind `{kind}`; use --template-surface to narrow the plan"
            ),
        )
        .with_repair_hint(RepairHint::new("narrow_template_surface").with_replacement(
            "Run `sley plan --json --graft-templates --template-surface <surface> <target>`",
        ))),
    }
}

fn load_fix_source_override(
    source: Option<String>,
    source_file: Option<PathBuf>,
) -> std::result::Result<Option<String>, Diagnostic> {
    let Some(source_file) = source_file else {
        return Ok(source);
    };
    fs::read_to_string(&source_file).map(Some).map_err(|error| {
        Diagnostic::error(
            "FIX_SOURCE_READ_FAILED",
            format!(
                "failed to read --source-file {}: {error}",
                source_file.display()
            ),
        )
        .with_repair_hint(
            RepairHint::new("check_source_file_path")
                .with_replacement("Provide a readable source snippet file"),
        )
    })
}

fn apply_fix_template_overrides(
    mut value: JsonValue,
    kind: &str,
    name: Option<String>,
    ty: Option<String>,
    module: Option<String>,
    source: Option<String>,
    position: Option<usize>,
) -> std::result::Result<JsonValue, Diagnostic> {
    if name.is_none() && ty.is_none() && module.is_none() && source.is_none() && position.is_none()
    {
        return Ok(value);
    }
    if value.get("transaction").is_some() || value.get("ops").is_some() {
        return Err(fix_override_unsupported(
            kind,
            "fix payload overrides are supported only for single-operation templates",
        ));
    }
    if let Some(name) = name {
        override_fix_payload_string(&mut value, kind, "name", name)?;
    }
    if let Some(ty) = ty {
        override_fix_payload_string(&mut value, kind, "type", ty)?;
    }
    if let Some(module) = module {
        override_fix_payload_string(&mut value, kind, "module", module)?;
    }
    if let Some(source) = source {
        override_fix_payload_string(&mut value, kind, "source", source)?;
    }
    if let Some(position) = position {
        override_fix_payload_usize(&mut value, kind, "position", position)?;
    }
    Ok(value)
}

fn override_fix_payload_string(
    value: &mut JsonValue,
    kind: &str,
    field: &str,
    replacement: String,
) -> std::result::Result<(), Diagnostic> {
    let pointer = format!("/payload/{field}");
    let Some(slot) = value.pointer_mut(&pointer) else {
        return Err(fix_override_unsupported(
            kind,
            format!("fix template `{kind}` does not expose editable `{pointer}`"),
        ));
    };
    if !slot.is_string() {
        return Err(fix_override_unsupported(
            kind,
            format!("fix template `{kind}` exposes `{pointer}`, but it is not a string"),
        ));
    }
    *slot = JsonValue::String(replacement);
    Ok(())
}

fn override_fix_payload_usize(
    value: &mut JsonValue,
    kind: &str,
    field: &str,
    replacement: usize,
) -> std::result::Result<(), Diagnostic> {
    let pointer = format!("/payload/{field}");
    let Some(slot) = value.pointer_mut(&pointer) else {
        return Err(fix_override_unsupported(
            kind,
            format!("fix template `{kind}` does not expose editable `{pointer}`"),
        ));
    };
    if slot.as_u64().is_none() {
        return Err(fix_override_unsupported(
            kind,
            format!("fix template `{kind}` exposes `{pointer}`, but it is not an unsigned integer"),
        ));
    }
    *slot = serde_json::json!(replacement);
    Ok(())
}

fn fix_override_unsupported(kind: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error("FIX_OVERRIDE_UNSUPPORTED", message)
        .with_repair_hint(RepairHint::new("inspect_editable_pointers").with_replacement(format!(
            "Run `sley plan --json --graft-templates [--template-surface <surface>] <target>` and inspect `editable_json_pointers` before using --name/--type/--module/--source/--source-file/--position with `{kind}`"
        )))
}

fn edit_plan_options_for_target(
    file: &Path,
    deny_warnings: bool,
    include_graft_templates: bool,
    template_surface: Option<String>,
) -> EditPlanOptions {
    EditPlanOptions {
        deny_warnings,
        include_graft_templates,
        template_surface,
        module_name_hint: infer_module_name_hint(file),
    }
}

fn infer_module_name_hint(file: &Path) -> Option<String> {
    if file.extension().and_then(|extension| extension.to_str()) != Some("sley") {
        return None;
    }
    infer_project_module_name_for_file(file).or_else(|| module_name_from_sley_path(file))
}

fn infer_project_module_name_for_file(file: &Path) -> Option<String> {
    let file = file.canonicalize().ok()?;
    let mut directory = file.parent();
    while let Some(current) = directory {
        let manifest_path = current.join("sley.toml");
        if manifest_path.exists() {
            let manifest_source = fs::read_to_string(&manifest_path).ok()?;
            let manifest: ProjectManifest = toml::from_str(&manifest_source).ok()?;
            let module_root = current.join(manifest.project.root).canonicalize().ok()?;
            let relative = file.strip_prefix(module_root).ok()?;
            return module_name_from_project_relative_sley_path(relative);
        }
        directory = current.parent();
    }
    None
}

fn load_target_program(file: &PathBuf) -> Result<sley::Program, Vec<Diagnostic>> {
    if is_project_target(file) {
        return load_project(file).map(|project| project.program);
    }
    let source = fs::read_to_string(file).map_err(|error| {
        vec![Diagnostic::error(
            "SOURCE_READ_FAILED",
            format!("failed to read {}: {error}", file.display()),
        )]
    })?;
    parse_program(&source)
}

fn load_target_program_or_fail(file: &PathBuf) -> Result<sley::Program> {
    load_target_program(file)
        .map_err(|diagnostics| anyhow::anyhow!(format_diagnostics(&diagnostics)))
}

fn load_target_program_and_source_bytes(
    file: &PathBuf,
) -> Result<(sley::Program, Vec<u8>), Vec<Diagnostic>> {
    if is_project_target(file) {
        let project = load_project(file)?;
        let source_bytes = project_source_bytes(&project).map_err(|error| {
            vec![Diagnostic::error(
                "PROJECT_SOURCE_READ_FAILED",
                format!("{error:#}"),
            )]
        })?;
        return Ok((project.program, source_bytes));
    }
    let source = fs::read_to_string(file).map_err(|error| {
        vec![Diagnostic::error(
            "SOURCE_READ_FAILED",
            format!("failed to read {}: {error}", file.display()),
        )]
    })?;
    let program = parse_program(&source)?;
    Ok((program, source.into_bytes()))
}

fn project_source_bytes(project: &ProjectGraph) -> Result<Vec<u8>> {
    let mut paths = Vec::with_capacity(project.modules.len() + 1);
    paths.push(project.manifest_path.clone());
    paths.extend(project.modules.iter().map(|module| module.path.clone()));
    paths.sort();
    paths.dedup();

    let mut bytes = Vec::new();
    for path in paths {
        let relative = path.strip_prefix(&project.root).unwrap_or(&path);
        let source =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        bytes.extend_from_slice(format!("--- {}\n", normalized_path(relative)).as_bytes());
        bytes.extend_from_slice(&source);
        if !source.ends_with(b"\n") {
            bytes.push(b'\n');
        }
    }
    Ok(bytes)
}

fn deploy_artifacts(directory: &Path) -> DeployArtifacts {
    DeployArtifacts {
        directory: normalized_path(directory),
        report: normalized_path(&directory.join("deploy-report.json")),
        seal: normalized_path(&directory.join("seal.json")),
        package: normalized_path(&directory.join("zjx-envelope.json")),
        manifest: normalized_path(&directory.join("manifest.json")),
    }
}

fn write_deploy_artifacts(
    directory: &Path,
    report: &DeployReport,
    seal: Option<&TraceSeal>,
    package: Option<&sley::zjx::SleyZjxEnvelope>,
) -> Result<()> {
    fs::create_dir_all(directory).with_context(|| {
        format!(
            "failed to create deploy artifacts dir {}",
            directory.display()
        )
    })?;
    let artifacts = report
        .artifacts
        .as_ref()
        .context("ready deploy report is missing artifact paths")?;
    let seal = seal.context("ready deploy report is missing trace seal artifact")?;
    let package = package.context("ready deploy report is missing package artifact")?;
    let report_digest = write_json_file(&directory.join("deploy-report.json"), report)?;
    let seal_file_digest = write_json_file(&directory.join("seal.json"), seal)?;
    let package_file_digest = write_json_file(&directory.join("zjx-envelope.json"), package)?;
    let manifest = build_deploy_artifact_manifest(
        report,
        artifacts,
        report_digest,
        seal_file_digest,
        package_file_digest,
    );
    write_json_file(&directory.join("manifest.json"), &manifest)?;
    Ok(())
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<String> {
    let bytes = serde_json::to_vec_pretty(value)
        .with_context(|| format!("failed to serialize JSON {}", path.display()))?;
    fs::write(path, &bytes).with_context(|| format!("failed to write JSON {}", path.display()))?;
    Ok(content_digest(&bytes))
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_project_target(file: &Path) -> bool {
    file.is_dir() || file.file_name().and_then(|name| name.to_str()) == Some("sley.toml")
}

fn parse_program_or_fail(source: &str) -> Result<sley::Program> {
    parse_program(source).map_err(|diagnostics| anyhow::anyhow!(format_diagnostics(&diagnostics)))
}

fn emit_diagnostics_and_fail(diagnostics: Vec<Diagnostic>, json: bool) -> Result<()> {
    if json {
        print_json(&DiagnosticReport::from_diagnostics(diagnostics))?;
    } else {
        print_human_diagnostics(&diagnostics);
    }
    anyhow::bail!("operation failed")
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_human_diagnostics(diagnostics: &[Diagnostic]) {
    eprint!("{}", format_diagnostics(diagnostics));
}

fn print_human_graph_slice(slice: &SymbolGraphSlice) {
    println!(
        "slice target={} kind={} module={} imports={} types={} effects={} tasks={} outbound_calls={} inbound_calls={}",
        slice.target,
        slice.focus.kind,
        slice.focus.module,
        slice.imports.len(),
        slice.types.len(),
        slice.effects.len(),
        slice.tasks.len(),
        slice.outbound_calls.len(),
        slice.inbound_calls.len()
    );
    for call in &slice.outbound_calls {
        let target = call.target.as_deref().unwrap_or(call.status.as_str());
        println!("call {} -> {}", call.callee, target);
    }
}

fn print_human_query_report(report: &QueryReport) {
    println!(
        "query schema={} kind={} entry={} modules={} tasks={} types={} effects={} calls={}",
        report.schema,
        report.kind,
        report.entry_module,
        report.modules.len(),
        report.tasks.len(),
        report.types.len(),
        report.effects.len(),
        report.calls.len()
    );
    for module in &report.modules {
        println!(
            "module {} imports={} types={} effects={} tasks={}",
            module.module,
            module.imports.len(),
            module.types.len(),
            module.effects.len(),
            module.tasks.len()
        );
    }
    for task in &report.tasks {
        let effects = if task.effects.is_empty() {
            "none".to_string()
        } else {
            task.effects.join(",")
        };
        println!(
            "task {} -> {} takes={} effects={} outbound_calls={} inbound_calls={}",
            task.qualified_name,
            task.return_type,
            task.takes.len(),
            effects,
            task.outbound_call_count,
            task.inbound_call_count
        );
    }
    for ty in &report.types {
        println!(
            "type {} = {} exported={} fields={}",
            ty.qualified_name,
            ty.value,
            ty.exported,
            ty.fields.len()
        );
    }
    for effect in &report.effects {
        println!(
            "effect {} exported={}",
            effect.qualified_name, effect.exported
        );
    }
    for call in &report.calls {
        let target = call.target.as_deref().unwrap_or(call.status.as_str());
        println!("call {} -> {}", call.from, target);
    }
}

fn print_human_lint_report(report: &LintReport) {
    println!(
        "lint schema={} status={} entry={} findings={}",
        report.schema,
        report.status,
        report.entry_module,
        report.findings.len()
    );
    for finding in &report.findings {
        println!(
            "{} {} {} [{}] hint={}",
            finding.severity, finding.id, finding.message, finding.node, finding.hint
        );
    }
}

fn print_human_doctor_report(report: &DoctorReport) {
    println!(
        "doctor schema={} status={} target={} entry={} errors={} warnings={} modules={} tasks={} calls={} lint_findings={}",
        report.schema,
        report.status,
        report.target,
        report.entry_module.as_deref().unwrap_or("unknown"),
        report.summary.error_count,
        report.summary.warning_count,
        report.summary.module_count,
        report.summary.task_count,
        report.summary.call_count,
        report.summary.lint_finding_count
    );
    for diagnostic in &report.diagnostics {
        println!("diagnostic {} {}", diagnostic.id, diagnostic.message);
    }
    for action in &report.next_actions {
        println!(
            "next {}: {} -> {}",
            action.kind,
            action.reason,
            action.command.join(" ")
        );
    }
}

fn print_human_edit_plan_report(report: &EditPlanReport) {
    println!(
        "plan schema={} status={} target={} entry={} errors={} warnings={} modules={} tasks={} calls={} lint_findings={} surfaces={}",
        report.schema,
        report.status,
        report.target,
        report.entry_module.as_deref().unwrap_or("unknown"),
        report.summary.error_count,
        report.summary.warning_count,
        report.summary.module_count,
        report.summary.task_count,
        report.summary.call_count,
        report.summary.lint_finding_count,
        report.summary.task_surface_count
    );
    if !report.graft_templates.is_empty() {
        println!("graft_templates={}", report.graft_templates.len());
    }
    for surface in &report.task_surfaces {
        println!(
            "surface {} inbound_calls={} outbound_calls={} notes={}",
            surface.qualified_name,
            surface.inbound_call_count,
            surface.outbound_call_count,
            surface.planning_notes.join(",")
        );
    }
    for diagnostic in &report.diagnostics {
        println!("diagnostic {} {}", diagnostic.id, diagnostic.message);
    }
    for action in &report.next_actions {
        println!(
            "next {}: {} -> {}",
            action.kind,
            action.reason,
            action.command.join(" ")
        );
    }
}

fn print_human_verify_report(report: &VerifyReport) {
    println!(
        "verify schema={} status={} target={} entry={} errors={} warnings={} modules={} tasks={} calls={} lint_findings={} runtime={}",
        report.schema,
        report.status,
        report.target,
        report.entry_module.as_deref().unwrap_or("unknown"),
        report.summary.error_count,
        report.summary.warning_count,
        report.summary.module_count,
        report.summary.task_count,
        report.summary.call_count,
        report.summary.lint_finding_count,
        report.summary.runtime_status
    );
    if let Some(runtime) = &report.runtime
        && let Some(value) = &runtime.value
    {
        println!("runtime value={value:?}");
    }
    for diagnostic in &report.diagnostics {
        println!("diagnostic {} {}", diagnostic.id, diagnostic.message);
    }
    for action in &report.next_actions {
        println!(
            "next {}: {} -> {}",
            action.kind,
            action.reason,
            action.command.join(" ")
        );
    }
}

fn print_human_deploy_report(report: &DeployReport) {
    println!(
        "deploy schema={} status={} mode={} target={} environment={} verify={} seal={} package={} modules={} tasks={} receipts={} live_deploy_allowed={}",
        report.schema,
        report.status,
        report.mode,
        report.target,
        report.environment,
        report.summary.verify_status,
        report.summary.seal_status,
        report.summary.package_status,
        report.summary.module_count,
        report.summary.task_count,
        report.summary.receipt_count,
        report.policy.live_deploy_allowed
    );
    if let Some(digest) = &report.summary.seal_digest {
        println!("seal_digest={digest}");
    }
    if let Some(digest) = &report.summary.graph_digest {
        println!("graph_digest={digest}");
    }
    if let Some(artifacts) = &report.artifacts {
        println!(
            "artifacts dir={} report={} seal={} package={}",
            artifacts.directory, artifacts.report, artifacts.seal, artifacts.package
        );
        println!("artifact_manifest={}", artifacts.manifest);
    }
    for action in &report.next_actions {
        println!(
            "next {}: {} -> {}",
            action.kind,
            action.reason,
            action.command.join(" ")
        );
    }
}

fn print_human_trace(path: &Path, receipts: &[sley::trace::TraceReceipt]) {
    println!("trace path={} receipts={}", path.display(), receipts.len());
    for receipt in receipts {
        for record in &receipt.provenance {
            println!(
                "{} {} {} targets={}",
                record.timestamp,
                record.actor,
                record.operation,
                record.targets.join(",")
            );
        }
    }
}

fn print_human_seal(seal: &TraceSeal) {
    println!(
        "seal schema={} target={} source={} graph={} trace={} seal={} modules={} tasks={} receipts={}",
        seal.schema,
        seal.target,
        seal.source_digest,
        seal.graph_digest,
        seal.trace_digest,
        seal.seal_digest,
        seal.module_count,
        seal.task_count,
        seal.receipt_count
    );
}

fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        out.push_str(&format!(
            "{} {:?}: {}",
            diagnostic.id, diagnostic.severity, diagnostic.message
        ));
        if let Some(node) = &diagnostic.node {
            out.push_str(&format!(" [{node}]"));
        }
        if let Some(span) = &diagnostic.span {
            out.push_str(&format!(" at {}:{}", span.line, span.column));
        }
        out.push('\n');
    }
    out
}
