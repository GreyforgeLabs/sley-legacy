use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use weavelang::checker::{check_program, has_errors};
use weavelang::diagnostics::{Diagnostic, DiagnosticReport};
use weavelang::formatter::format_program;
use weavelang::parser::parse_program;
use weavelang::patch::{PatchInput, apply_patch_input};
use weavelang::runtime::run_main;

#[derive(Debug, Parser)]
#[command(name = "weave")]
#[command(about = "WeaveLang v0 compiler and patch tool")]
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
    Patch {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        write: bool,
        #[arg(long)]
        actor: Option<String>,
        file: PathBuf,
        patch: PathBuf,
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
        Command::Parse { json, file } => {
            let source = read_source(&file)?;
            match parse_program(&source) {
                Ok(program) => {
                    if json {
                        print_json(&program)?;
                    } else {
                        println!(
                            "parsed module={} imports={} types={} effects={} functions={}",
                            program.module_name(),
                            program.imports.len(),
                            program.types.len(),
                            program.effects.len(),
                            program.functions.len()
                        );
                    }
                    Ok(())
                }
                Err(diagnostics) => emit_diagnostics_and_fail(diagnostics, json),
            }
        }
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
            let source = read_source(&file)?;
            let program = parse_program_or_fail(&source)?;
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
            let source = read_source(&file)?;
            let program = parse_program_or_fail(&source)?;
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
        Command::Ast { json, node, file } => {
            let source = read_source(&file)?;
            let program = parse_program_or_fail(&source)?;
            if let Some(node) = node {
                if let Some(function_index) = program.find_function_index(&node) {
                    print_json(&program.functions[function_index])?;
                    return Ok(());
                }
                anyhow::bail!("node `{node}` was not found");
            }
            if json {
                print_json(&program)?;
            } else {
                print_json(&program)?;
            }
            Ok(())
        }
        Command::Patch {
            json,
            write,
            actor,
            file,
            patch,
        } => {
            let source = read_source(&file)?;
            let program = parse_program_or_fail(&source)?;
            let patch_source = fs::read_to_string(&patch)
                .with_context(|| format!("failed to read {}", patch.display()))?;
            let patch_input: PatchInput = serde_json::from_str(&patch_source)
                .with_context(|| format!("failed to parse {}", patch.display()))?;
            let outcome = apply_patch_input(&program, patch_input, actor);
            if json {
                print_json(&outcome)?;
            } else if let Some(source) = &outcome.source {
                print!("{source}");
            } else {
                print_human_diagnostics(&outcome.diagnostics);
            }
            if outcome.status != "accepted" {
                anyhow::bail!("patch rejected");
            }
            if write {
                if let Some(source) = outcome.source {
                    fs::write(&file, source)
                        .with_context(|| format!("failed to write {}", file.display()))?;
                }
            }
            Ok(())
        }
    }
}

fn read_source(file: &PathBuf) -> Result<String> {
    fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))
}

fn parse_program_or_fail(source: &str) -> Result<weavelang::Program> {
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
