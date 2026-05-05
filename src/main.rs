use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sley::Program;
use sley::checker::{check_program, has_errors};
use sley::diagnostics::{Diagnostic, DiagnosticReport};
use sley::formatter::format_program;
use sley::graft::{GRAFT_OUTCOME_SCHEMA, GraftInput, GraftOutcome, apply_graft_program};
use sley::parser::parse_program;
use sley::project::{ProjectGraph, load_project};
use sley::runtime::run_main;
use sley::symbols::{
    SymbolGraphSlice, build_symbol_graph, effect_module, import_owner_module, slice_symbol_graph,
    task_module, type_module,
};
use sley::trace::{
    TraceSeal, append_trace_receipt, build_trace_receipt, build_trace_seal, default_trace_path,
    read_trace_receipts,
};
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
        Command::Run { json, file } => {
            let program = match load_target_program(&file) {
                Ok(program) => program,
                Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
            };
            let diagnostics = check_program(&program);
            if has_errors(&diagnostics) {
                return emit_diagnostics_and_fail(diagnostics, json);
            }
            match run_main(&program) {
                Ok(value) => {
                    if json {
                        print_json(&value)?;
                    } else {
                        println!("{value:?}");
                    }
                    Ok(())
                }
                Err(diagnostics) => emit_diagnostics_and_fail(diagnostics, json),
            }
        }
        Command::Ast {
            json: _json,
            node,
            file,
        } => {
            let program = load_target_program_or_fail(&file)?;
            if let Some(node) = node {
                if let Some(task_index) = program.find_task_index(&node) {
                    print_json(&program.tasks[task_index])?;
                    return Ok(());
                }
                anyhow::bail!("node `{node}` was not found");
            }
            print_json(&program)?;
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
        Command::Trace { json, trace, file } => {
            let trace_path = trace.unwrap_or_else(|| default_trace_path(&file));
            let receipts = read_trace_receipts(&trace_path)?;
            if json {
                print_json(&receipts)?;
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
                    "zjx envelope schema={} format={} compression={} modules={} trace_receipts={}",
                    envelope.schema,
                    envelope.format,
                    envelope.compression,
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

            if is_project_target(&file) {
                let project = match load_project(&file) {
                    Ok(project) => project,
                    Err(diagnostics) => return emit_diagnostics_and_fail(diagnostics, json),
                };
                let applied = apply_graft_program(&project.program, graft_input, actor);
                let mut outcome = applied.outcome;
                let write = write && !dry_run;
                if outcome.status == "accepted" && write {
                    let candidate = applied
                        .program
                        .as_ref()
                        .expect("accepted graft should carry candidate program");
                    if let Err(diagnostics) = write_project_graft(
                        &file,
                        &project,
                        candidate,
                        outcome.provenance.clone(),
                        trace.as_deref(),
                    ) {
                        outcome = rejected_graft_outcome(diagnostics);
                    }
                }
                emit_graft_outcome(&outcome, json)?;
                if outcome.status != "accepted" {
                    anyhow::bail!("graft rejected");
                }
                return Ok(());
            }

            let source = read_source(&file)?;
            let program = parse_program_or_fail(&source)?;
            let outcome = apply_graft_program(&program, graft_input, actor).outcome;
            emit_graft_outcome(&outcome, json)?;
            if outcome.status != "accepted" {
                anyhow::bail!("graft rejected");
            }
            let write = write && !dry_run;
            if write && let Some(source) = outcome.source.as_deref() {
                fs::write(&file, source)
                    .with_context(|| format!("failed to write {}", file.display()))?;
                if !outcome.provenance.is_empty() {
                    let trace_path = trace.unwrap_or_else(|| default_trace_path(&file));
                    let receipt = build_trace_receipt(&file, outcome.provenance);
                    append_trace_receipt(&trace_path, &receipt)?;
                }
            }
            Ok(())
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
    let writes = plan_project_writeback(project, candidate)?;
    for write in writes {
        fs::write(&write.path, write.source).map_err(|error| {
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

struct ProjectWrite {
    module: String,
    path: PathBuf,
    source: String,
}

fn plan_project_writeback(
    project: &ProjectGraph,
    candidate: &Program,
) -> Result<Vec<ProjectWrite>, Vec<Diagnostic>> {
    let known_modules = project
        .modules
        .iter()
        .map(|module| module.module.clone())
        .collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();

    for module in candidate_declared_modules(candidate) {
        if !known_modules.contains(&module) {
            diagnostics.push(
                Diagnostic::error(
                    "PROJECT_WRITEBACK_UNKNOWN_MODULE",
                    format!(
                        "graft candidate changed module `{module}`, which is not in this project"
                    ),
                )
                .with_node(format!("module:{module}")),
            );
        }
    }

    for import in &candidate.imports {
        if !known_modules.contains(&import.module) {
            diagnostics.push(
                Diagnostic::error(
                    "PROJECT_WRITEBACK_UNKNOWN_IMPORT",
                    format!(
                        "graft candidate imports `{}`, but project writeback cannot create missing module files",
                        import.module
                    ),
                )
                .with_node(import.id.clone()),
            );
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut writes = Vec::new();
    for module in &project.modules {
        let next_program = project_module_program(candidate, &module.module);
        let current_source = format_program(&module.program);
        let next_source = format_program(&next_program);
        if current_source != next_source {
            writes.push(ProjectWrite {
                module: module.module.clone(),
                path: module.path.clone(),
                source: next_source,
            });
        }
    }
    Ok(writes)
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

fn read_source(file: &PathBuf) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))
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
