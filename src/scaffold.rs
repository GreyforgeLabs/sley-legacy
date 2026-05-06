use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::diagnostics::Diagnostic;

pub const PROJECT_SCAFFOLD_SCHEMA: &str = "sley.project.scaffold.v0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaffoldTemplate {
    Hello,
    Deploy,
    Agent,
}

impl ScaffoldTemplate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::Deploy => "deploy",
            Self::Agent => "agent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldOptions {
    pub name: Option<String>,
    pub module: String,
    pub template: ScaffoldTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectScaffoldReport {
    pub schema: String,
    pub status: String,
    pub project: ProjectScaffoldSummary,
    pub files: Vec<ScaffoldFile>,
    pub next_commands: Vec<Vec<String>>,
    pub next_actions: Vec<ScaffoldNextAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectScaffoldSummary {
    pub name: String,
    pub root: String,
    pub source_root: String,
    pub entry_module: String,
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScaffoldFile {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScaffoldNextAction {
    pub kind: String,
    pub reason: String,
    pub command: Vec<String>,
}

pub fn scaffold_project(
    target: impl AsRef<Path>,
    options: ScaffoldOptions,
) -> Result<ProjectScaffoldReport, Vec<Diagnostic>> {
    let target = target.as_ref();
    let name = match options.name {
        Some(name) => name.trim().to_string(),
        None => default_project_name(target),
    };
    let module = options.module.trim().to_string();
    let source_root = "src";
    let manifest_path = PathBuf::from("sley.toml");
    let readme_path = PathBuf::from("README.md");
    let source_path = module_source_path(source_root, &module);
    let files = vec![
        ScaffoldFile {
            path: normalized_path(&manifest_path),
            kind: "manifest".to_string(),
        },
        ScaffoldFile {
            path: normalized_path(&readme_path),
            kind: "guide".to_string(),
        },
        ScaffoldFile {
            path: normalized_path(&source_path),
            kind: "source".to_string(),
        },
    ];

    let mut diagnostics = Vec::new();
    if !is_project_name(&name) {
        diagnostics.push(Diagnostic::error(
            "PROJECT_SCAFFOLD_NAME_INVALID",
            "project name must contain only letters, digits, `_`, or `-`",
        ));
    }
    if let Some(diagnostic) = validate_module_path(&module) {
        diagnostics.push(diagnostic);
    }
    if target.exists() && !target.is_dir() {
        diagnostics.push(Diagnostic::error(
            "PROJECT_SCAFFOLD_TARGET_NOT_DIRECTORY",
            format!("scaffold target {} is not a directory", target.display()),
        ));
    }
    for file in &files {
        let absolute = target.join(&file.path);
        if absolute.exists() {
            diagnostics.push(
                Diagnostic::error(
                    "PROJECT_SCAFFOLD_FILE_EXISTS",
                    format!("refusing to overwrite existing file {}", absolute.display()),
                )
                .with_node(format!("file:{}", file.path)),
            );
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    fs::create_dir_all(target).map_err(|error| {
        vec![Diagnostic::error(
            "PROJECT_SCAFFOLD_CREATE_DIR_FAILED",
            format!("failed to create {}: {error}", target.display()),
        )]
    })?;
    if let Some(parent) = target.join(&source_path).parent() {
        fs::create_dir_all(parent).map_err(|error| {
            vec![Diagnostic::error(
                "PROJECT_SCAFFOLD_CREATE_DIR_FAILED",
                format!("failed to create {}: {error}", parent.display()),
            )]
        })?;
    }

    write_new_file(
        target.join(&manifest_path),
        &manifest_source(&name, &module),
    )?;
    write_new_file(
        target.join(&readme_path),
        &readme_source(&name, options.template),
    )?;
    write_new_file(
        target.join(&source_path),
        &template_source(&module, options.template),
    )?;

    let next_actions = next_actions(options.template);
    let next_commands = next_actions
        .iter()
        .map(|action| action.command.clone())
        .collect();
    Ok(ProjectScaffoldReport {
        schema: PROJECT_SCAFFOLD_SCHEMA.to_string(),
        status: "created".to_string(),
        project: ProjectScaffoldSummary {
            name,
            root: target.display().to_string(),
            source_root: source_root.to_string(),
            entry_module: module,
            template: options.template.as_str().to_string(),
        },
        files,
        next_commands,
        next_actions,
    })
}

fn write_new_file(path: impl AsRef<Path>, source: &str) -> Result<(), Vec<Diagnostic>> {
    let path = path.as_ref();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            vec![Diagnostic::error(
                "PROJECT_SCAFFOLD_WRITE_FAILED",
                format!("failed to write {}: {error}", path.display()),
            )]
        })?;
    file.write_all(source.as_bytes()).map_err(|error| {
        vec![Diagnostic::error(
            "PROJECT_SCAFFOLD_WRITE_FAILED",
            format!("failed to write {}: {error}", path.display()),
        )]
    })
}

fn default_project_name(target: &Path) -> String {
    target
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("sley-app")
        .to_string()
}

fn manifest_source(name: &str, module: &str) -> String {
    format!(
        "[project]\nname = \"{}\"\nroot = \"src\"\nentry = \"{}\"\n",
        toml_string(name),
        toml_string(module)
    )
}

fn readme_source(name: &str, template: ScaffoldTemplate) -> String {
    let (verify, run) = match template {
        ScaffoldTemplate::Hello => ("sley verify --json --deny-warnings .", "sley run --json ."),
        ScaffoldTemplate::Deploy => (
            "sley verify --json --deny-warnings --cap Deploy --deploy-result staging staged .",
            "sley run --json --cap Deploy --deploy-result staging staged .",
        ),
        ScaffoldTemplate::Agent => (
            "sley verify --json --deny-warnings --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile \"profile ready\" --cap ModelCall --model-output deploy-plan \"plan approved\" --cap Deploy --deploy-result staging staged .",
            "sley run --json --cap SecretRead --secret api_key redacted --cap Network --http-text https://example.test/profile \"profile ready\" --cap ModelCall --model-output deploy-plan \"plan approved\" --cap Deploy --deploy-result staging staged .",
        ),
    };
    format!(
        "# {name}\n\nGenerated Sley project.\n\n```bash\nsley check --json .\nsley doctor --json .\nsley query --json --kind tasks .\nsley plan --json .\nsley lint --json --deny-warnings .\n{verify}\n{run}\nsley seal --json .\nsley zjx --json .\n```\n"
    )
}

fn template_source(module: &str, template: ScaffoldTemplate) -> String {
    match template {
        ScaffoldTemplate::Hello => format!(
            "module {module}\n\n\
task main -> Text {{\n  return \"hello sley\"\n}}\n"
        ),
        ScaffoldTemplate::Deploy => format!(
            "module {module}\n\n\
task main -> Result<Text, Error> uses Deploy {{\n  bind result = call deploy.try_stage(\"staging\")?\n\n  return Ok(result)\n}}\n"
        ),
        ScaffoldTemplate::Agent => format!(
            "module {module}\n\n\
task main -> Result<Text, Error> uses SecretRead, Network, ModelCall, Deploy {{\n  bind token = call secrets.try_get(\"api_key\")?\n  bind profile = call http.try_get_text(\"https://example.test/profile\")?\n  bind plan = call model.try_complete(\"deploy-plan\")?\n  bind staged = call deploy.try_stage(\"staging\")?\n\n  if len(token) > 0 {{\n    return Ok(profile + \" | \" + plan + \" | \" + staged)\n  }}\n\n  return Err(\"api_key seed was empty\")\n}}\n"
        ),
    }
}

fn next_actions(template: ScaffoldTemplate) -> Vec<ScaffoldNextAction> {
    let mut actions = vec![
        next_action(
            "check_project",
            "confirm the scaffold parses and passes strict checks",
            vec!["sley", "check", "--json", "."],
        ),
        next_action(
            "doctor_project",
            "summarize strict checks, query facts, and lint gates in one readiness report",
            vec!["sley", "doctor", "--json", "."],
        ),
        next_action(
            "inspect_tasks",
            "read the entry task surface before editing",
            vec!["sley", "query", "--json", "--kind", "tasks", "."],
        ),
        next_action(
            "plan_first_edit",
            "ask Loom for ranked edit surfaces and post-edit gates",
            vec!["sley", "plan", "--json", "."],
        ),
        next_action(
            "lint_gate",
            "keep warning-grade hygiene strict before runtime work",
            vec!["sley", "lint", "--json", "--deny-warnings", "."],
        ),
    ];
    let extra_actions = match template {
        ScaffoldTemplate::Hello => vec![
            next_action(
                "verify_local",
                "run the deterministic verification gate with denied warnings",
                vec!["sley", "verify", "--json", "--deny-warnings", "."],
            ),
            next_action(
                "run_local",
                "execute the pure starter task",
                vec!["sley", "run", "--json", "."],
            ),
        ],
        ScaffoldTemplate::Deploy => vec![
            next_action(
                "verify_seeded_deploy",
                "verify deploy authority with a seeded provider result and denied warnings",
                vec![
                    "sley",
                    "verify",
                    "--json",
                    "--deny-warnings",
                    "--cap",
                    "Deploy",
                    "--deploy-result",
                    "staging",
                    "staged",
                    ".",
                ],
            ),
            next_action(
                "run_seeded_deploy",
                "execute the deploy-gated starter with deterministic seeded authority",
                vec![
                    "sley",
                    "run",
                    "--json",
                    "--cap",
                    "Deploy",
                    "--deploy-result",
                    "staging",
                    "staged",
                    ".",
                ],
            ),
        ],
        ScaffoldTemplate::Agent => vec![
            next_action(
                "verify_seeded_agent",
                "verify agent authority with deterministic secret, network, model, and deploy seeds",
                seeded_agent_command("sley", "verify"),
            ),
            next_action(
                "ci_verify_seeded_agent",
                "run the same seeded agent gate through the local CI wrapper",
                seeded_agent_command("sley-ci", "verify"),
            ),
            next_action(
                "run_seeded_agent",
                "execute the agent starter with deterministic seeded authority",
                seeded_agent_command("sley", "run"),
            ),
        ],
    };
    actions.extend(extra_actions);
    actions.push(next_action(
        "seal_project",
        "create a content-addressed review artifact for the scaffold",
        vec!["sley", "seal", "--json", "."],
    ));
    actions.push(next_action(
        "package_project",
        "create a ZJX preview envelope for agent handoff",
        vec!["sley", "zjx", "--json", "."],
    ));
    actions
}

fn next_action(kind: &str, reason: &str, command: Vec<&str>) -> ScaffoldNextAction {
    ScaffoldNextAction {
        kind: kind.to_string(),
        reason: reason.to_string(),
        command: command.into_iter().map(str::to_string).collect(),
    }
}

fn seeded_agent_command(binary: &'static str, verb: &'static str) -> Vec<&'static str> {
    let mut command = vec![
        binary,
        verb,
        "--json",
        "--cap",
        "SecretRead",
        "--secret",
        "api_key",
        "redacted",
        "--cap",
        "Network",
        "--http-text",
        "https://example.test/profile",
        "profile ready",
        "--cap",
        "ModelCall",
        "--model-output",
        "deploy-plan",
        "plan approved",
        "--cap",
        "Deploy",
        "--deploy-result",
        "staging",
        "staged",
        ".",
    ];
    if verb == "verify" {
        command.insert(3, "--deny-warnings");
    }
    command
}

fn module_source_path(source_root: &str, module: &str) -> PathBuf {
    let mut path = PathBuf::from(source_root);
    for segment in module.split('.') {
        path.push(segment);
    }
    path.set_extension("sley");
    path
}

fn validate_module_path(module: &str) -> Option<Diagnostic> {
    if module.trim().is_empty() {
        return Some(Diagnostic::error(
            "PROJECT_MODULE_INVALID",
            "module path cannot be empty",
        ));
    }
    for segment in module.split('.') {
        if !is_identifier(segment) {
            return Some(
                Diagnostic::error(
                    "PROJECT_MODULE_INVALID",
                    format!("module segment `{segment}` is not a valid identifier"),
                )
                .with_node(format!("module:{module}")),
            );
        }
    }
    None
}

fn is_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_project_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
}

fn toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
