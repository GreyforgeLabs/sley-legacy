use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sley::checker::{check_program, has_errors};
use sley::diagnostics::{Diagnostic, DiagnosticReport};
use sley::formatter::format_program;
use sley::graft::{GraftInput, apply_graft_input};
use sley::parser::parse_program;
use sley::project::load_project;
use sley::runtime::run_main;

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
    Graft {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        write: bool,
        #[arg(long)]
        actor: Option<String>,
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
        Command::Graft {
            json,
            write,
            actor,
            file,
            graft,
        } => {
            let source = read_source(&file)?;
            let program = parse_program_or_fail(&source)?;
            let graft_source = fs::read_to_string(&graft)
                .with_context(|| format!("failed to read {}", graft.display()))?;
            let graft_input: GraftInput = serde_json::from_str(&graft_source)
                .with_context(|| format!("failed to parse {}", graft.display()))?;
            let outcome = apply_graft_input(&program, graft_input, actor);
            if json {
                print_json(&outcome)?;
            } else if let Some(source) = &outcome.source {
                print!("{source}");
            } else {
                print_human_diagnostics(&outcome.diagnostics);
            }
            if outcome.status != "accepted" {
                anyhow::bail!("graft rejected");
            }
            if write && let Some(source) = outcome.source {
                fs::write(&file, source)
                    .with_context(|| format!("failed to write {}", file.display()))?;
            }
            Ok(())
        }
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
