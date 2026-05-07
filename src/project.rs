use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Program;
use crate::diagnostics::Diagnostic;
use crate::parser::parse_program;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectManifest {
    pub project: ProjectConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default = "default_source_root")]
    pub root: String,
    pub entry: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProjectGraph {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    #[serde(skip_serializing)]
    pub manifest_source: String,
    pub module_root: PathBuf,
    pub entry: String,
    pub modules: Vec<ProjectModule>,
    pub program: Program,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProjectModule {
    pub module: String,
    pub path: PathBuf,
    pub program: Program,
}

pub fn load_project(target: impl AsRef<Path>) -> Result<ProjectGraph, Vec<Diagnostic>> {
    let overlays = HashMap::new();
    load_project_with_source_overlays(target, &overlays)
}

pub fn load_project_with_source_overlays(
    target: impl AsRef<Path>,
    source_overlays: &HashMap<PathBuf, String>,
) -> Result<ProjectGraph, Vec<Diagnostic>> {
    let target = target.as_ref();
    let manifest_path = manifest_path_for(target);
    let root = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let manifest_source = fs::read_to_string(&manifest_path).map_err(|error| {
        vec![Diagnostic::error(
            "PROJECT_MANIFEST_NOT_FOUND",
            format!("failed to read {}: {error}", manifest_path.display()),
        )]
    })?;
    let manifest: ProjectManifest = toml::from_str(&manifest_source).map_err(|error| {
        vec![Diagnostic::error(
            "PROJECT_MANIFEST_INVALID",
            format!("failed to parse {}: {error}", manifest_path.display()),
        )]
    })?;

    let mut diagnostics = Vec::new();
    if manifest.project.entry.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "PROJECT_ENTRY_MISSING",
            "`project.entry` cannot be empty",
        ));
    }
    if manifest.project.root.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "PROJECT_ROOT_MISSING",
            "`project.root` cannot be empty",
        ));
    }
    if !is_relative_project_path(Path::new(&manifest.project.root)) {
        diagnostics.push(Diagnostic::error(
            "PROJECT_ROOT_NOT_RELATIVE",
            "`project.root` must be a relative path inside the project",
        ));
    }
    if let Some(diagnostic) = validate_module_path(&manifest.project.entry) {
        diagnostics.push(diagnostic);
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let module_root = root.join(&manifest.project.root);
    let mut loader = ModuleLoader {
        module_root: module_root.clone(),
        source_overlays,
        loading: Vec::new(),
        loaded: HashSet::new(),
        modules: Vec::new(),
        diagnostics: Vec::new(),
    };
    loader.load_module(&manifest.project.entry);

    if !loader.diagnostics.is_empty() {
        return Err(loader.diagnostics);
    }

    let entry = manifest.project.entry.clone();
    let program = bundle_program(&entry, &loader.modules);
    Ok(ProjectGraph {
        root,
        manifest_path,
        manifest_source,
        module_root,
        entry,
        modules: loader.modules,
        program,
    })
}

fn default_source_root() -> String {
    "src".to_string()
}

fn manifest_path_for(target: &Path) -> PathBuf {
    if target.file_name().and_then(|name| name.to_str()) == Some("sley.toml") {
        target.to_path_buf()
    } else {
        target.join("sley.toml")
    }
}

struct ModuleLoader<'a> {
    module_root: PathBuf,
    source_overlays: &'a HashMap<PathBuf, String>,
    loading: Vec<String>,
    loaded: HashSet<String>,
    modules: Vec<ProjectModule>,
    diagnostics: Vec<Diagnostic>,
}

impl ModuleLoader<'_> {
    fn load_module(&mut self, module: &str) {
        if self.loaded.contains(module) {
            return;
        }
        if let Some(diagnostic) = validate_module_path(module) {
            self.diagnostics.push(diagnostic);
            return;
        }
        if let Some(index) = self.loading.iter().position(|item| item == module) {
            let mut cycle = self.loading[index..].to_vec();
            cycle.push(module.to_string());
            self.diagnostics.push(Diagnostic::error(
                "PROJECT_IMPORT_CYCLE",
                format!("import cycle detected: {}", cycle.join(" -> ")),
            ));
            return;
        }

        self.loading.push(module.to_string());
        let candidates = module_paths(&self.module_root, module);
        let mut loaded_source = None;
        let mut last_error = None;
        for path in &candidates {
            if let Some(source) = self.source_overlays.get(path) {
                loaded_source = Some((path.clone(), source.clone()));
                break;
            }
            match fs::read_to_string(path) {
                Ok(source) => {
                    loaded_source = Some((path.clone(), source));
                    break;
                }
                Err(error) => {
                    last_error = Some(error);
                }
            }
        }
        let Some((path, source)) = loaded_source else {
            let tried = candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            self.diagnostics.push(
                Diagnostic::error(
                    "PROJECT_MODULE_NOT_FOUND",
                    format!(
                        "failed to read module `{module}`; tried {tried}: {}",
                        last_error
                            .map(|error| error.to_string())
                            .unwrap_or_else(|| "no source candidates".to_string())
                    ),
                )
                .with_node(format!("module:{module}")),
            );
            self.loading.pop();
            return;
        };
        let program = match parse_program(&source) {
            Ok(program) => program,
            Err(mut diagnostics) => {
                for diagnostic in &mut diagnostics {
                    diagnostic.message = format!("{} in {}", diagnostic.message, path.display());
                }
                self.diagnostics.extend(diagnostics);
                self.loading.pop();
                return;
            }
        };

        if program.module.as_deref() != Some(module) {
            self.diagnostics.push(
                Diagnostic::error(
                    "PROJECT_MODULE_MISMATCH",
                    format!(
                        "module file {} declares `{}` but project expected `{module}`",
                        path.display(),
                        program.module_name()
                    ),
                )
                .with_node(format!("module:{module}")),
            );
        }

        let imports = program
            .imports
            .iter()
            .map(|import| import.module.clone())
            .collect::<Vec<_>>();
        self.modules.push(ProjectModule {
            module: module.to_string(),
            path,
            program,
        });
        self.loaded.insert(module.to_string());

        for import in imports {
            self.load_module(&import);
        }
        self.loading.pop();
    }
}

fn bundle_program(entry: &str, modules: &[ProjectModule]) -> Program {
    let mut program = Program::new();
    program.module = Some(entry.to_string());
    for module in modules {
        program.imports.extend(module.program.imports.clone());
        program.types.extend(module.program.types.clone());
        program.effects.extend(module.program.effects.clone());
        program.tasks.extend(module.program.tasks.clone());
        program.provenance.extend(module.program.provenance.clone());
    }
    program
}

fn module_paths(root: &Path, module: &str) -> Vec<PathBuf> {
    vec![module_path_with_extension(root, module, "sley")]
}

fn module_path_with_extension(root: &Path, module: &str, extension: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in module.split('.') {
        path.push(segment);
    }
    path.set_extension(extension);
    path
}

fn validate_module_path(module: &str) -> Option<Diagnostic> {
    if module.trim().is_empty() {
        return Some(Diagnostic::error(
            "PROJECT_INVALID_MODULE_PATH",
            "module path cannot be empty",
        ));
    }
    for segment in module.split('.') {
        if !is_identifier(segment) {
            return Some(Diagnostic::error(
                "PROJECT_INVALID_MODULE_PATH",
                format!("module path `{module}` contains invalid segment `{segment}`"),
            ));
        }
    }
    None
}

fn is_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_relative_project_path(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}
