use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::ast::{
    BindingKind, Block, Expr, ExprKind, ImportDecl, Program, Statement, StatementKind, TaskDecl,
    TypeExpr,
};
use crate::authority::host_effects_for_callee;
use crate::query::{QueryKind, QueryOptions, build_query_report};
use crate::symbols::{
    EffectResolution, TaskResolution, TypeResolution, callee_path, collect_task_calls,
    effect_fq_name, effect_module, import_owner_module, resolve_effect, resolve_task, resolve_type,
    task_fq_name, task_module, type_fq_name, type_module,
};

pub const LINT_REPORT_SCHEMA: &str = "sley.lint.report.v0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LintRule {
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
}

impl LintRule {
    pub fn all() -> Vec<Self> {
        vec![
            Self::UnusedPrivateTask,
            Self::UnreachablePrivateTask,
            Self::UnusedDeclaredEffect,
            Self::UnusedImport,
            Self::UnusedTake,
            Self::UnusedPrivateType,
            Self::UnusedPrivateEffect,
            Self::RawHostAdapter,
            Self::MissingModuleDeclaration,
            Self::UncheckedResult,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnusedPrivateTask => "unused_private_task",
            Self::UnreachablePrivateTask => "unreachable_private_task",
            Self::UnusedDeclaredEffect => "unused_declared_effect",
            Self::UnusedImport => "unused_import",
            Self::UnusedTake => "unused_take",
            Self::UnusedPrivateType => "unused_private_type",
            Self::UnusedPrivateEffect => "unused_private_effect",
            Self::RawHostAdapter => "raw_host_adapter",
            Self::MissingModuleDeclaration => "missing_module_declaration",
            Self::UncheckedResult => "unchecked_result",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LintOptions {
    pub rules: Vec<LintRule>,
    pub module: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LintReport {
    pub schema: String,
    pub status: String,
    pub entry_module: String,
    pub filters: LintFilters,
    pub findings: Vec<LintFinding>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LintFilters {
    pub module: Option<String>,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LintFinding {
    pub id: String,
    pub rule: String,
    pub severity: String,
    pub message: String,
    pub node: String,
    pub module: String,
    pub hint: String,
}

pub fn build_lint_report(program: &Program, options: LintOptions) -> LintReport {
    let rules = selected_rules(options.rules);
    let mut findings = Vec::new();

    if rules.contains(&LintRule::UnusedPrivateTask) {
        findings.extend(lint_unused_private_tasks(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UnreachablePrivateTask) {
        findings.extend(lint_unreachable_private_tasks(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UnusedDeclaredEffect) {
        findings.extend(lint_unused_declared_effects(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UnusedImport) {
        findings.extend(lint_unused_imports(program, options.module.as_deref()));
    }
    if rules.contains(&LintRule::UnusedTake) {
        findings.extend(lint_unused_takes(program, options.module.as_deref()));
    }
    if rules.contains(&LintRule::UnusedPrivateType) {
        findings.extend(lint_unused_private_types(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UnusedPrivateEffect) {
        findings.extend(lint_unused_private_effects(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::RawHostAdapter) {
        findings.extend(lint_raw_host_adapters(program, options.module.as_deref()));
    }
    if rules.contains(&LintRule::MissingModuleDeclaration) {
        findings.extend(lint_missing_module_declaration(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UncheckedResult) {
        findings.extend(lint_unchecked_results(program, options.module.as_deref()));
    }

    findings.sort_by(|left, right| {
        left.rule
            .cmp(&right.rule)
            .then_with(|| left.node.cmp(&right.node))
    });
    let status = if findings.is_empty() {
        "ok"
    } else {
        "findings"
    };

    LintReport {
        schema: LINT_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        entry_module: program.module_name().to_string(),
        filters: LintFilters {
            module: options.module,
            rules: rules
                .into_iter()
                .map(LintRule::as_str)
                .map(str::to_string)
                .collect(),
        },
        findings,
    }
}

fn selected_rules(rules: Vec<LintRule>) -> Vec<LintRule> {
    if rules.is_empty() {
        return LintRule::all();
    }
    rules
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn lint_missing_module_declaration(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    if program.module.is_some() || !module_matches(module, program.module_name()) {
        return Vec::new();
    }
    vec![LintFinding {
        id: "MISSING_MODULE_DECLARATION".to_string(),
        rule: LintRule::MissingModuleDeclaration.as_str().to_string(),
        severity: "warning".to_string(),
        message: "source relies on the implicit `main` module; declare an explicit module for stable project graph ids".to_string(),
        node: "program".to_string(),
        module: program.module_name().to_string(),
        hint: "add `module app.name` at the top of the file before deployable or project code".to_string(),
    }]
}

fn lint_unused_private_tasks(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let report = build_query_report(
        program,
        QueryOptions {
            kind: QueryKind::Tasks,
            module: module.map(str::to_string),
            exported_only: false,
        },
    );
    let entry_task = format!("{}.main", program.module_name());

    report
        .tasks
        .into_iter()
        .filter(|task| !task.exported)
        .filter(|task| task.inbound_call_count == 0)
        .filter(|task| task.qualified_name != entry_task)
        .map(|task| LintFinding {
            id: "UNUSED_PRIVATE_TASK".to_string(),
            rule: LintRule::UnusedPrivateTask.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "private task `{}` is not called by any checked task",
                task.qualified_name
            ),
            node: task.id,
            module: task.module,
            hint: "call it, export it, or delete it".to_string(),
        })
        .collect()
}

fn lint_unreachable_private_tasks(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let task_names = program
        .tasks
        .iter()
        .map(task_fq_name)
        .collect::<BTreeSet<_>>();
    let entry_task = format!("{}.main", program.module_name());
    let mut edges = BTreeMap::<String, Vec<String>>::new();
    let mut inbound_counts = BTreeMap::<String, usize>::new();

    for call in collect_task_calls(program) {
        let Some(target) = call.target else {
            continue;
        };
        if !task_names.contains(&target) {
            continue;
        }
        edges
            .entry(call.from.clone())
            .or_default()
            .push(target.clone());
        *inbound_counts.entry(target).or_default() += 1;
    }

    let mut reachable = BTreeSet::new();
    let mut pending = program
        .tasks
        .iter()
        .filter(|task| task.exported || task_fq_name(task) == entry_task)
        .map(task_fq_name)
        .collect::<Vec<_>>();

    while let Some(task) = pending.pop() {
        if !reachable.insert(task.clone()) {
            continue;
        }
        if let Some(targets) = edges.get(&task) {
            pending.extend(targets.iter().cloned());
        }
    }

    program
        .tasks
        .iter()
        .filter(|task| !task.exported)
        .map(|task| (task, task_fq_name(task)))
        .filter(|(task, _qualified_name)| module_matches(module, &task_module(task)))
        .filter(|(_task, qualified_name)| qualified_name != &entry_task)
        .filter(|(_task, qualified_name)| !reachable.contains(qualified_name))
        .filter(|(_task, qualified_name)| {
            inbound_counts
                .get(qualified_name)
                .copied()
                .unwrap_or_default()
                > 0
        })
        .map(|(task, qualified_name)| LintFinding {
            id: "UNREACHABLE_PRIVATE_TASK".to_string(),
            rule: LintRule::UnreachablePrivateTask.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "private task `{qualified_name}` is not reachable from main or any exported task"
            ),
            node: task.id.clone(),
            module: task_module(task),
            hint:
                "call it from main or an exported task, export a reachable entrypoint, or delete it"
                    .to_string(),
        })
        .collect()
}

fn lint_unused_declared_effects(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let task_effects = program
        .tasks
        .iter()
        .map(|task| {
            let task_module = task_module(task);
            (
                task_fq_name(task),
                task.effects
                    .iter()
                    .map(|effect| normalize_effect_name(program, &task_module, effect))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let calls = collect_task_calls(program);

    program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
        .filter(|task| !task.effects.is_empty())
        .flat_map(|task| {
            let qualified_name = task_fq_name(task);
            let module_name = task_module(task);
            let declared = task
                .effects
                .iter()
                .map(|effect| {
                    (
                        effect.clone(),
                        normalize_effect_name(program, &module_name, effect),
                    )
                })
                .collect::<Vec<_>>();
            let declared_normalized = declared
                .iter()
                .map(|(_raw, normalized)| normalized.clone())
                .collect::<BTreeSet<_>>();
            let mut used = BTreeSet::new();
            for call in calls.iter().filter(|call| call.from == qualified_name) {
                if let Some(effects) = host_effects_for_callee(&call.callee) {
                    for effect in effects {
                        if declared_normalized.contains(*effect) {
                            used.insert((*effect).to_string());
                        }
                    }
                }
                if let Some(target) = &call.target
                    && let Some(effects) = task_effects.get(target)
                {
                    for effect in effects {
                        if declared_normalized.contains(effect) {
                            used.insert(effect.clone());
                        }
                    }
                }
            }
            declared
                .into_iter()
                .filter(move |(_effect, normalized)| !used.contains(normalized))
                .map(move |(effect, _normalized)| LintFinding {
                    id: "UNUSED_DECLARED_EFFECT".to_string(),
                    rule: LintRule::UnusedDeclaredEffect.as_str().to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "task `{qualified_name}` declares effect `{effect}` but no checked call uses it"
                    ),
                    node: task.id.clone(),
                    module: module_name.clone(),
                    hint: format!(
                        "remove `{effect}` from the task uses list, or add a real checked call that requires it"
                    ),
                })
        })
        .collect()
}

fn module_matches(filter: Option<&str>, module: &str) -> bool {
    filter.is_none_or(|filter| filter == module)
}

fn lint_unused_private_types(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let used_types = collect_used_type_names(program);
    program
        .types
        .iter()
        .filter(|ty| module_matches(module, &type_module(ty)))
        .filter(|ty| !ty.exported)
        .filter(|ty| !used_types.contains(&type_fq_name(ty)))
        .map(|ty| {
            let qualified_name = type_fq_name(ty);
            LintFinding {
                id: "UNUSED_PRIVATE_TYPE".to_string(),
                rule: LintRule::UnusedPrivateType.as_str().to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "private type `{qualified_name}` is not referenced by any checked task, type, or record literal"
                ),
                node: ty.id.clone(),
                module: type_module(ty),
                hint: format!("use type `{}` or delete it", ty.name),
            }
        })
        .collect()
}

fn lint_unused_private_effects(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let used_effects = collect_used_effect_names(program);
    program
        .effects
        .iter()
        .filter(|effect| module_matches(module, &effect_module(effect)))
        .filter(|effect| !effect.exported)
        .filter(|effect| !used_effects.contains(&effect_fq_name(effect)))
        .map(|effect| {
            let qualified_name = effect_fq_name(effect);
            LintFinding {
                id: "UNUSED_PRIVATE_EFFECT".to_string(),
                rule: LintRule::UnusedPrivateEffect.as_str().to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "private effect `{qualified_name}` is not declared by any checked task"
                ),
                node: effect.id.clone(),
                module: effect_module(effect),
                hint: format!(
                    "use effect `{}` in a task uses list or delete it",
                    effect.name
                ),
            }
        })
        .collect()
}

fn collect_used_effect_names(program: &Program) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    for task in &program.tasks {
        let module_name = task_module(task);
        for effect in &task.effects {
            if let EffectResolution::Resolved { fq_name, .. } =
                resolve_effect(program, &module_name, effect)
            {
                used.insert(fq_name);
            }
        }
    }
    used
}

fn collect_used_type_names(program: &Program) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    for task in &program.tasks {
        let module_name = task_module(task);
        for take in &task.takes {
            collect_type_expr_references(program, &module_name, &take.ty, &mut used, None);
        }
        collect_type_expr_references(program, &module_name, &task.return_type, &mut used, None);
        collect_block_type_references(program, &module_name, &task.body, &mut used);
    }
    for ty in &program.types {
        let module_name = type_module(ty);
        let skip = type_fq_name(ty);
        collect_type_expr_references(program, &module_name, &ty.value, &mut used, Some(&skip));
    }
    used
}

fn collect_block_type_references(
    program: &Program,
    module: &str,
    block: &Block,
    used: &mut BTreeSet<String>,
) {
    for statement in &block.statements {
        collect_statement_type_references(program, module, statement, used);
    }
}

fn collect_statement_type_references(
    program: &Program,
    module: &str,
    statement: &Statement,
    used: &mut BTreeSet<String>,
) {
    match &statement.kind {
        StatementKind::Binding { type_ann, expr, .. } => {
            if let Some(ty) = type_ann {
                collect_type_expr_references(program, module, ty, used, None);
            }
            collect_expr_type_references(program, module, expr, used);
        }
        StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => collect_expr_type_references(program, module, expr, used),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_expr_type_references(program, module, condition, used);
            collect_block_type_references(program, module, then_block, used);
            if let Some(else_block) = else_block {
                collect_block_type_references(program, module, else_block, used);
            }
        }
        StatementKind::While { condition, body } => {
            collect_expr_type_references(program, module, condition, used);
            collect_block_type_references(program, module, body, used);
        }
        StatementKind::For {
            collection, body, ..
        } => {
            collect_expr_type_references(program, module, collection, used);
            collect_block_type_references(program, module, body, used);
        }
        StatementKind::Forge { body } => collect_block_type_references(program, module, body, used),
    }
}

fn collect_expr_type_references(
    program: &Program,
    module: &str,
    expr: &Expr,
    used: &mut BTreeSet<String>,
) {
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_expr_type_references(program, module, expr, used);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_type_references(program, module, left, used);
            collect_expr_type_references(program, module, right, used);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_type_references(program, module, condition, used);
            collect_expr_type_references(program, module, then_branch, used);
            collect_expr_type_references(program, module, else_branch, used);
        }
        ExprKind::Call { callee, args } => {
            collect_expr_type_references(program, module, callee, used);
            for arg in args {
                collect_expr_type_references(program, module, arg, used);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_expr_type_references(program, module, item, used);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_expr_type_references(program, module, &entry.key, used);
                collect_expr_type_references(program, module, &entry.value, used);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_expr_type_references(program, module, collection, used);
            collect_expr_type_references(program, module, index, used);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_expr_type_references(program, module, receiver, used);
        }
        ExprKind::RecordLiteral { type_name, fields } => {
            if let Some(type_name) = type_name {
                collect_type_name_reference(program, module, type_name, used, None);
            }
            for field in fields {
                collect_expr_type_references(program, module, &field.expr, used);
            }
        }
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => {}
    }
}

fn collect_type_expr_references(
    program: &Program,
    module: &str,
    ty: &TypeExpr,
    used: &mut BTreeSet<String>,
    skip: Option<&str>,
) {
    match ty {
        TypeExpr::Named { name } => collect_type_name_reference(program, module, name, used, skip),
        TypeExpr::Generic { name, args } => {
            collect_type_name_reference(program, module, name, used, skip);
            for arg in args {
                collect_type_expr_references(program, module, arg, used, skip);
            }
        }
        TypeExpr::Record { fields } => {
            for field in fields {
                collect_type_expr_references(program, module, &field.ty, used, skip);
            }
        }
    }
}

fn collect_type_name_reference(
    program: &Program,
    module: &str,
    name: &str,
    used: &mut BTreeSet<String>,
    skip: Option<&str>,
) {
    if let TypeResolution::Resolved { fq_name, .. } = resolve_type(program, module, name)
        && Some(fq_name.as_str()) != skip
    {
        used.insert(fq_name);
    }
}

fn lint_unused_takes(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
        .flat_map(|task| {
            let qualified_name = task_fq_name(task);
            let module_name = task_module(task);
            task.takes
                .iter()
                .filter(|take| take.binding_kind == BindingKind::Take)
                .filter(|take| !block_uses_identifier(&task.body, &take.name))
                .map(move |take| LintFinding {
                    id: "UNUSED_TAKE".to_string(),
                    rule: LintRule::UnusedTake.as_str().to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "task `{qualified_name}` declares take `{}` but never reads it",
                        take.name
                    ),
                    node: take.id.clone(),
                    module: module_name.clone(),
                    hint: format!(
                        "remove take `{}` and update callers, or read it in the task body",
                        take.name
                    ),
                })
        })
        .collect()
}

fn block_uses_identifier(block: &Block, name: &str) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement_uses_identifier(statement, name))
}

fn statement_uses_identifier(statement: &Statement, name: &str) -> bool {
    match &statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => expr_uses_identifier(expr, name),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_uses_identifier(condition, name)
                || block_uses_identifier(then_block, name)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block_uses_identifier(block, name))
        }
        StatementKind::While { condition, body } => {
            expr_uses_identifier(condition, name) || block_uses_identifier(body, name)
        }
        StatementKind::For {
            collection, body, ..
        } => expr_uses_identifier(collection, name) || block_uses_identifier(body, name),
        StatementKind::Forge { body } => block_uses_identifier(body, name),
    }
}

fn expr_uses_identifier(expr: &Expr, name: &str) -> bool {
    match &expr.kind {
        ExprKind::Identifier { name: candidate } => candidate == name,
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => expr_uses_identifier(expr, name),
        ExprKind::Binary { left, right, .. } => {
            expr_uses_identifier(left, name) || expr_uses_identifier(right, name)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_uses_identifier(condition, name)
                || expr_uses_identifier(then_branch, name)
                || expr_uses_identifier(else_branch, name)
        }
        ExprKind::Call { callee, args } => {
            expr_uses_identifier(callee, name)
                || args.iter().any(|arg| expr_uses_identifier(arg, name))
        }
        ExprKind::ListLiteral { items } => {
            items.iter().any(|item| expr_uses_identifier(item, name))
        }
        ExprKind::MapLiteral { entries } => entries.iter().any(|entry| {
            expr_uses_identifier(&entry.key, name) || expr_uses_identifier(&entry.value, name)
        }),
        ExprKind::Index { collection, index } => {
            expr_uses_identifier(collection, name) || expr_uses_identifier(index, name)
        }
        ExprKind::FieldAccess { receiver, .. } => expr_uses_identifier(receiver, name),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .any(|field| expr_uses_identifier(&field.expr, name)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. } => false,
    }
}

fn lint_unused_imports(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    program
        .imports
        .iter()
        .filter(|import| module_matches(module, &import_owner_module(import)))
        .filter(|import| !import_is_used(program, import))
        .map(|import| {
            let owner_module = import_owner_module(import);
            LintFinding {
                id: "UNUSED_IMPORT".to_string(),
                rule: LintRule::UnusedImport.as_str().to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "module `{owner_module}` imports `{}` but no checked task, type, or effect uses it",
                    import.module
                ),
                node: import.id.clone(),
                module: owner_module,
                hint: format!("remove import `{}` or use an exported item from it", import.module),
            }
        })
        .collect()
}

fn import_is_used(program: &Program, import: &ImportDecl) -> bool {
    let owner_module = import_owner_module(import);

    collect_task_calls(program).into_iter().any(|call| {
        call.from_module == owner_module
            && call
                .target
                .as_deref()
                .is_some_and(|target| resolved_path_uses_import(import, &call.callee, target))
    }) || program
        .tasks
        .iter()
        .filter(|task| task_module(task) == owner_module)
        .any(|task| {
            task.takes
                .iter()
                .any(|take| type_expr_uses_import(program, &owner_module, import, &take.ty))
                || type_expr_uses_import(program, &owner_module, import, &task.return_type)
                || task
                    .effects
                    .iter()
                    .any(|effect| effect_path_uses_import(program, &owner_module, import, effect))
                || block_uses_import(program, &owner_module, import, &task.body)
        })
        || program
            .types
            .iter()
            .filter(|ty| type_module(ty) == owner_module)
            .any(|ty| type_expr_uses_import(program, &owner_module, import, &ty.value))
}

fn block_uses_import(
    program: &Program,
    owner_module: &str,
    import: &ImportDecl,
    block: &Block,
) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement_uses_import(program, owner_module, import, statement))
}

fn statement_uses_import(
    program: &Program,
    owner_module: &str,
    import: &ImportDecl,
    statement: &Statement,
) -> bool {
    match &statement.kind {
        StatementKind::Binding { type_ann, expr, .. } => {
            type_ann
                .as_ref()
                .is_some_and(|ty| type_expr_uses_import(program, owner_module, import, ty))
                || expr_uses_import(program, owner_module, import, expr)
        }
        StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => expr_uses_import(program, owner_module, import, expr),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_uses_import(program, owner_module, import, condition)
                || block_uses_import(program, owner_module, import, then_block)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block_uses_import(program, owner_module, import, block))
        }
        StatementKind::While { condition, body } => {
            expr_uses_import(program, owner_module, import, condition)
                || block_uses_import(program, owner_module, import, body)
        }
        StatementKind::For {
            collection, body, ..
        } => {
            expr_uses_import(program, owner_module, import, collection)
                || block_uses_import(program, owner_module, import, body)
        }
        StatementKind::Forge { body } => block_uses_import(program, owner_module, import, body),
    }
}

fn expr_uses_import(
    program: &Program,
    owner_module: &str,
    import: &ImportDecl,
    expr: &Expr,
) -> bool {
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            expr_uses_import(program, owner_module, import, expr)
        }
        ExprKind::Binary { left, right, .. } => {
            expr_uses_import(program, owner_module, import, left)
                || expr_uses_import(program, owner_module, import, right)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_uses_import(program, owner_module, import, condition)
                || expr_uses_import(program, owner_module, import, then_branch)
                || expr_uses_import(program, owner_module, import, else_branch)
        }
        ExprKind::Call { callee, args } => {
            expr_uses_import(program, owner_module, import, callee)
                || args
                    .iter()
                    .any(|arg| expr_uses_import(program, owner_module, import, arg))
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .any(|item| expr_uses_import(program, owner_module, import, item)),
        ExprKind::MapLiteral { entries } => entries.iter().any(|entry| {
            expr_uses_import(program, owner_module, import, &entry.key)
                || expr_uses_import(program, owner_module, import, &entry.value)
        }),
        ExprKind::Index { collection, index } => {
            expr_uses_import(program, owner_module, import, collection)
                || expr_uses_import(program, owner_module, import, index)
        }
        ExprKind::FieldAccess { receiver, .. } => {
            expr_uses_import(program, owner_module, import, receiver)
        }
        ExprKind::RecordLiteral { type_name, fields } => {
            type_name
                .as_ref()
                .is_some_and(|name| type_path_uses_import(program, owner_module, import, name))
                || fields
                    .iter()
                    .any(|field| expr_uses_import(program, owner_module, import, &field.expr))
        }
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => false,
    }
}

fn type_expr_uses_import(
    program: &Program,
    owner_module: &str,
    import: &ImportDecl,
    ty: &TypeExpr,
) -> bool {
    match ty {
        TypeExpr::Named { name } => type_path_uses_import(program, owner_module, import, name),
        TypeExpr::Generic { name, args } => {
            type_path_uses_import(program, owner_module, import, name)
                || args
                    .iter()
                    .any(|arg| type_expr_uses_import(program, owner_module, import, arg))
        }
        TypeExpr::Record { fields } => fields
            .iter()
            .any(|field| type_expr_uses_import(program, owner_module, import, &field.ty)),
    }
}

fn type_path_uses_import(
    program: &Program,
    owner_module: &str,
    import: &ImportDecl,
    path: &str,
) -> bool {
    match resolve_type(program, owner_module, path) {
        TypeResolution::Resolved { fq_name, .. } => {
            resolved_path_uses_import(import, path, &fq_name)
        }
        TypeResolution::Builtin(_)
        | TypeResolution::Unknown
        | TypeResolution::Ambiguous(_)
        | TypeResolution::Private(_) => false,
    }
}

fn effect_path_uses_import(
    program: &Program,
    owner_module: &str,
    import: &ImportDecl,
    path: &str,
) -> bool {
    match resolve_effect(program, owner_module, path) {
        EffectResolution::Resolved { fq_name, .. } => {
            resolved_path_uses_import(import, path, &fq_name)
        }
        EffectResolution::Builtin(_)
        | EffectResolution::Unknown
        | EffectResolution::Ambiguous(_)
        | EffectResolution::Private(_) => false,
    }
}

fn resolved_path_uses_import(
    import: &ImportDecl,
    source_path: &str,
    resolved_fq_name: &str,
) -> bool {
    resolved_fq_name
        .rsplit_once('.')
        .is_some_and(|(module, _name)| {
            module == import.module && source_path_uses_import(import, source_path)
        })
}

fn source_path_uses_import(import: &ImportDecl, source_path: &str) -> bool {
    let parts = source_path
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    match parts.as_slice() {
        [_name] => true,
        [qualifier, _name] => import_matches_qualifier(import, qualifier),
        _ => false,
    }
}

fn import_matches_qualifier(import: &ImportDecl, qualifier: &str) -> bool {
    import.alias.as_deref() == Some(qualifier)
        || import
            .module
            .rsplit('.')
            .next()
            .is_some_and(|segment| segment == qualifier)
}

fn lint_raw_host_adapters(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    collect_task_calls(program)
        .into_iter()
        .filter(|call| module_matches(module, &call.from_module))
        .filter_map(|call| {
            let replacement = raw_host_adapter_replacement(&call.callee)?;
            Some(LintFinding {
                id: "RAW_HOST_ADAPTER".to_string(),
                rule: LintRule::RawHostAdapter.as_str().to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "host call `{}` uses a legacy diagnostic-failing adapter; prefer fallible `{replacement}`",
                    call.callee
                ),
                node: call.expr_id,
                module: call.from_module,
                hint: format!(
                    "replace `{}` with `{replacement}` and handle the Result with `?` inside a Result-returning task",
                    call.callee
                ),
            })
        })
        .collect()
}

fn lint_unchecked_results(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
        .flat_map(|task| lint_unchecked_result_block(program, task, &task.body))
        .collect()
}

fn lint_unchecked_result_block(
    program: &Program,
    task: &TaskDecl,
    block: &Block,
) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Expr { expr } => {
                if let Some(source) = unchecked_result_source(program, task, expr) {
                    let task_name = task_fq_name(task);
                    findings.push(LintFinding {
                        id: "UNCHECKED_RESULT".to_string(),
                        rule: LintRule::UncheckedResult.as_str().to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "task `{task_name}` discards Result from `{source}` in an expression statement"
                        ),
                        node: expr.id.clone(),
                        module: task_module(task),
                        hint:
                            "use `?` to propagate failure, return the Result, or bind it for explicit handling"
                                .to_string(),
                    });
                }
            }
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                findings.extend(lint_unchecked_result_block(program, task, then_block));
                if let Some(else_block) = else_block {
                    findings.extend(lint_unchecked_result_block(program, task, else_block));
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                findings.extend(lint_unchecked_result_block(program, task, body));
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. } => {}
        }
    }
    findings
}

fn unchecked_result_source(program: &Program, task: &TaskDecl, expr: &Expr) -> Option<String> {
    let ExprKind::Call { callee, .. } = &expr.kind else {
        return None;
    };
    let callee_name = callee_path(callee)?;
    if fallible_host_call(&callee_name) {
        return Some(callee_name);
    }
    if user_task_returns_result(program, task, &callee_name) {
        return Some(callee_name);
    }
    None
}

fn user_task_returns_result(program: &Program, task: &TaskDecl, callee_name: &str) -> bool {
    let TaskResolution::Resolved { index, .. } =
        resolve_task(program, &task_module(task), callee_name)
    else {
        return false;
    };
    program.tasks[index].return_type.generic_name() == Some("Result")
}

fn fallible_host_call(callee: &str) -> bool {
    matches!(
        callee,
        "fs.try_read_text"
            | "fs.try_write_text"
            | "db.try_query_one"
            | "db.try_query"
            | "db.try_insert"
            | "http.try_get_text"
            | "shell.try_run"
            | "model.try_complete"
            | "secrets.try_get"
            | "deploy.try_stage"
            | "spend.try_authorize"
    )
}

fn raw_host_adapter_replacement(callee: &str) -> Option<&'static str> {
    match callee {
        "fs.read_text" => Some("fs.try_read_text"),
        "fs.write_text" => Some("fs.try_write_text"),
        "db.query_one" => Some("db.try_query_one"),
        "db.query" => Some("db.try_query"),
        _ => None,
    }
}

fn normalize_effect_name(program: &Program, module: &str, effect: &str) -> String {
    match resolve_effect(program, module, effect) {
        EffectResolution::Builtin(name) => name,
        EffectResolution::Resolved { fq_name, .. } => fq_name,
        EffectResolution::Unknown
        | EffectResolution::Ambiguous(_)
        | EffectResolution::Private(_) => effect.to_string(),
    }
}
