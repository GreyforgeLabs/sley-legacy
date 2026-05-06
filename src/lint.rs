use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::ast::{
    BinaryOp, BindingKind, Block, Expr, ExprKind, ImportDecl, Program, Statement, StatementKind,
    TaskDecl, TypeExpr, UnaryOp,
};
use crate::authority::host_effects_for_callee;
use crate::formatter::format_statement_source;
use crate::query::{QueryKind, QueryOptions, build_query_report};
use crate::symbols::{
    EffectResolution, TaskCallSummary, TaskResolution, TypeResolution, callee_path,
    collect_task_calls, effect_fq_name, effect_module, import_owner_module, resolve_effect,
    resolve_task, resolve_type, task_fq_name, task_module, type_fq_name, type_module,
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
    UnqualifiedImportedCall,
    UnusedPureBinding,
    UnusedPureExpressionStatement,
    MutableBindingNeverSet,
    ConstantIfExpression,
    ConstantIfStatement,
    ConstantFalseWhileStatement,
    ConstantComparisonExpression,
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
            Self::UnqualifiedImportedCall,
            Self::UnusedPureBinding,
            Self::UnusedPureExpressionStatement,
            Self::MutableBindingNeverSet,
            Self::ConstantIfExpression,
            Self::ConstantIfStatement,
            Self::ConstantFalseWhileStatement,
            Self::ConstantComparisonExpression,
            Self::EmptyIfStatement,
            Self::EmptyForStatement,
            Self::EmptyForgeStatement,
            Self::IdentityBinaryExpression,
            Self::RedundantBooleanComparison,
            Self::AbsorbingBooleanExpression,
            Self::SelfComparisonExpression,
            Self::DoubleNegationExpression,
            Self::NegatedComparisonExpression,
            Self::RedundantBooleanIfExpression,
            Self::RedundantBooleanIfStatement,
            Self::SameBranchIfExpression,
            Self::SameBranchIfStatement,
            Self::UnreachableStatement,
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
            Self::UnqualifiedImportedCall => "unqualified_imported_call",
            Self::UnusedPureBinding => "unused_pure_binding",
            Self::UnusedPureExpressionStatement => "unused_pure_expression_statement",
            Self::MutableBindingNeverSet => "mutable_binding_never_set",
            Self::ConstantIfExpression => "constant_if_expression",
            Self::ConstantIfStatement => "constant_if_statement",
            Self::ConstantFalseWhileStatement => "constant_false_while_statement",
            Self::ConstantComparisonExpression => "constant_comparison_expression",
            Self::EmptyIfStatement => "empty_if_statement",
            Self::EmptyForStatement => "empty_for_statement",
            Self::EmptyForgeStatement => "empty_forge_statement",
            Self::IdentityBinaryExpression => "identity_binary_expression",
            Self::RedundantBooleanComparison => "redundant_boolean_comparison",
            Self::AbsorbingBooleanExpression => "absorbing_boolean_expression",
            Self::SelfComparisonExpression => "self_comparison_expression",
            Self::DoubleNegationExpression => "double_negation_expression",
            Self::NegatedComparisonExpression => "negated_comparison_expression",
            Self::RedundantBooleanIfExpression => "redundant_boolean_if_expression",
            Self::RedundantBooleanIfStatement => "redundant_boolean_if_statement",
            Self::SameBranchIfExpression => "same_branch_if_expression",
            Self::SameBranchIfStatement => "same_branch_if_statement",
            Self::UnreachableStatement => "unreachable_statement",
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
    if rules.contains(&LintRule::UnqualifiedImportedCall) {
        findings.extend(lint_unqualified_imported_calls(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UnusedPureBinding) {
        findings.extend(lint_unused_pure_bindings(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UnusedPureExpressionStatement) {
        findings.extend(lint_unused_pure_expression_statements(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::MutableBindingNeverSet) {
        findings.extend(lint_mutable_bindings_never_set(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::ConstantIfExpression) {
        findings.extend(lint_constant_if_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::ConstantIfStatement) {
        findings.extend(lint_constant_if_statements(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::ConstantFalseWhileStatement) {
        findings.extend(lint_constant_false_while_statements(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::ConstantComparisonExpression) {
        findings.extend(lint_constant_comparison_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::EmptyIfStatement) {
        findings.extend(lint_empty_if_statements(program, options.module.as_deref()));
    }
    if rules.contains(&LintRule::EmptyForStatement) {
        findings.extend(lint_empty_for_statements(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::EmptyForgeStatement) {
        findings.extend(lint_empty_forge_statements(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::IdentityBinaryExpression) {
        findings.extend(lint_identity_binary_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::RedundantBooleanComparison) {
        findings.extend(lint_redundant_boolean_comparisons(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::DoubleNegationExpression) {
        findings.extend(lint_double_negation_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::NegatedComparisonExpression) {
        findings.extend(lint_negated_comparison_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::RedundantBooleanIfExpression) {
        findings.extend(lint_redundant_boolean_if_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::RedundantBooleanIfStatement) {
        findings.extend(lint_redundant_boolean_if_statements(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::SameBranchIfExpression) {
        findings.extend(lint_same_branch_if_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::SameBranchIfStatement) {
        findings.extend(lint_same_branch_if_statements(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::AbsorbingBooleanExpression) {
        findings.extend(lint_absorbing_boolean_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::SelfComparisonExpression) {
        findings.extend(lint_self_comparison_expressions(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UnreachableStatement) {
        findings.extend(lint_unreachable_statements(
            program,
            options.module.as_deref(),
        ));
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

fn lint_unqualified_imported_calls(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    collect_task_calls(program)
        .into_iter()
        .filter(|call| module_matches(module, &call.from_module))
        .filter_map(|call| {
            let replacement_callee = qualified_imported_call_callee(program, &call)?;
            Some(LintFinding {
                id: "UNQUALIFIED_IMPORTED_CALL".to_string(),
                rule: LintRule::UnqualifiedImportedCall.as_str().to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "task `{}` calls imported task `{}` without a module qualifier",
                    call.from, call.callee
                ),
                node: call.expr_id,
                module: call.from_module,
                hint: format!(
                    "replace `{}` with `{replacement_callee}` so future imports cannot change resolution",
                    call.callee
                ),
            })
        })
        .collect()
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
                .enumerate()
                .map(|(index, effect)| {
                    (
                        index,
                        effect.clone(),
                        normalize_effect_name(program, &module_name, effect),
                    )
                })
                .collect::<Vec<_>>();
            let declared_normalized = declared
                .iter()
                .map(|(_index, _raw, normalized)| normalized.clone())
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
                .filter(move |(_index, _effect, normalized)| !used.contains(normalized))
                .map(move |(index, effect, _normalized)| LintFinding {
                    id: "UNUSED_DECLARED_EFFECT".to_string(),
                    rule: LintRule::UnusedDeclaredEffect.as_str().to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "task `{qualified_name}` declares effect `{effect}` but no checked call uses it"
                    ),
                    node: format!("effect-use:{}:{index}:{effect}", task.id),
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

fn lint_unused_pure_bindings(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_unused_pure_bindings_in_block(task, &task.body, &task.body, &mut findings);
    }
    findings
}

fn lint_unused_pure_expression_statements(
    program: &Program,
    module: Option<&str>,
) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_unused_pure_expression_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_unreachable_statements(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_unreachable_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_mutable_bindings_never_set(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_mutable_bindings_never_set_in_block(task, &task.body, &task.body, &mut findings);
    }
    findings
}

fn lint_constant_if_expressions(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_constant_if_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_constant_if_statements(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_constant_if_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_constant_false_while_statements(
    program: &Program,
    module: Option<&str>,
) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_constant_false_while_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_empty_if_statements(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_empty_if_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_empty_for_statements(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_empty_for_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_empty_forge_statements(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_empty_forge_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_identity_binary_expressions(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_identity_binary_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_redundant_boolean_comparisons(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_redundant_boolean_comparisons_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_self_comparison_expressions(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_self_comparison_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_double_negation_expressions(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_double_negation_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_negated_comparison_expressions(
    program: &Program,
    module: Option<&str>,
) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_negated_comparison_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_redundant_boolean_if_expressions(
    program: &Program,
    module: Option<&str>,
) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_redundant_boolean_if_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_redundant_boolean_if_statements(
    program: &Program,
    module: Option<&str>,
) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_redundant_boolean_if_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_same_branch_if_expressions(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_same_branch_if_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn lint_same_branch_if_statements(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_same_branch_if_statements_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn collect_same_branch_if_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_same_branch_if_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_same_branch_if_expressions_in_expr(task, condition, findings);
                collect_same_branch_if_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_same_branch_if_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_same_branch_if_expressions_in_expr(task, condition, findings);
                collect_same_branch_if_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_same_branch_if_expressions_in_expr(task, collection, findings);
                collect_same_branch_if_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_same_branch_if_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_same_branch_if_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = same_branch_if_expression_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "SAME_BRANCH_IF_EXPRESSION".to_string(),
            rule: LintRule::SameBranchIfExpression.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has an if expression with identical branches `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!("replace the if expression with `{replacement}`"),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_same_branch_if_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_same_branch_if_expressions_in_expr(task, left, findings);
            collect_same_branch_if_expressions_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_same_branch_if_expressions_in_expr(task, condition, findings);
            collect_same_branch_if_expressions_in_expr(task, then_branch, findings);
            collect_same_branch_if_expressions_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_same_branch_if_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_same_branch_if_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_same_branch_if_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_same_branch_if_expressions_in_expr(task, &entry.key, findings);
                collect_same_branch_if_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_same_branch_if_expressions_in_expr(task, collection, findings);
            collect_same_branch_if_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_same_branch_if_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_same_branch_if_expressions_in_expr(task, &field.expr, findings);
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

fn same_branch_if_expression_replacement(expr: &Expr) -> Option<String> {
    let ExprKind::If {
        condition,
        then_branch,
        else_branch,
    } = &expr.kind
    else {
        return None;
    };
    if bool_literal_value(condition).is_some() || !expr_is_delete_safe_pure(condition) {
        return None;
    }
    if then_branch.source == else_branch.source {
        Some(then_branch.source.clone())
    } else {
        None
    }
}

fn collect_same_branch_if_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                if let Some(replacement) = same_branch_if_statement_replacement(statement) {
                    let task_name = task_fq_name(task);
                    findings.push(LintFinding {
                        id: "SAME_BRANCH_IF_STATEMENT".to_string(),
                        rule: LintRule::SameBranchIfStatement.as_str().to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "task `{task_name}` has an if statement with identical branches"
                        ),
                        node: statement.id.clone(),
                        module: task_module(task),
                        hint: format!("replace the if statement with `{replacement}`"),
                    });
                }
                collect_same_branch_if_statements_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_same_branch_if_statements_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                collect_same_branch_if_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn same_branch_if_statement_replacement(statement: &Statement) -> Option<String> {
    let StatementKind::If {
        condition,
        then_block,
        else_block,
    } = &statement.kind
    else {
        return None;
    };
    if bool_literal_value(condition).is_some() || !expr_is_delete_safe_pure(condition) {
        return None;
    }
    let else_block = else_block.as_ref()?;
    let then_source = single_statement_source(then_block)?;
    let else_source = single_statement_source(else_block)?;
    (then_source == else_source).then_some(then_source)
}

fn single_statement_source(block: &Block) -> Option<String> {
    let [statement] = block.statements.as_slice() else {
        return None;
    };
    Some(format_statement_source(statement))
}

fn collect_redundant_boolean_if_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                if let Some(replacement) = redundant_boolean_if_statement_replacement(statement) {
                    let task_name = task_fq_name(task);
                    findings.push(LintFinding {
                        id: "REDUNDANT_BOOLEAN_IF_STATEMENT".to_string(),
                        rule: LintRule::RedundantBooleanIfStatement.as_str().to_string(),
                        severity: "warning".to_string(),
                        message: format!("task `{task_name}` has a redundant boolean if statement"),
                        node: statement.id.clone(),
                        module: task_module(task),
                        hint: format!("replace the boolean if statement with `{replacement}`"),
                    });
                }
                collect_redundant_boolean_if_statements_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_redundant_boolean_if_statements_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                collect_redundant_boolean_if_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn redundant_boolean_if_statement_replacement(statement: &Statement) -> Option<String> {
    let StatementKind::If {
        condition,
        then_block,
        else_block,
    } = &statement.kind
    else {
        return None;
    };
    if bool_literal_value(condition).is_some() {
        return None;
    }
    let else_block = else_block.as_ref()?;
    match (
        single_return_bool_literal(then_block),
        single_return_bool_literal(else_block),
    ) {
        (Some(true), Some(false)) => Some(format!("return {}", condition.source)),
        (Some(false), Some(true)) => Some(format!("return {}", negated_boolean_source(condition))),
        _ => None,
    }
}

fn single_return_bool_literal(block: &Block) -> Option<bool> {
    let [statement] = block.statements.as_slice() else {
        return None;
    };
    let StatementKind::Return { expr } = &statement.kind else {
        return None;
    };
    bool_literal_value(expr)
}

fn collect_redundant_boolean_if_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_redundant_boolean_if_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_redundant_boolean_if_expressions_in_expr(task, condition, findings);
                collect_redundant_boolean_if_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_redundant_boolean_if_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_redundant_boolean_if_expressions_in_expr(task, condition, findings);
                collect_redundant_boolean_if_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_redundant_boolean_if_expressions_in_expr(task, collection, findings);
                collect_redundant_boolean_if_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_redundant_boolean_if_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_redundant_boolean_if_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = redundant_boolean_if_expression_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "REDUNDANT_BOOLEAN_IF_EXPRESSION".to_string(),
            rule: LintRule::RedundantBooleanIfExpression.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has a redundant boolean if expression `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!("replace the boolean if expression with `{replacement}`"),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_redundant_boolean_if_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_redundant_boolean_if_expressions_in_expr(task, left, findings);
            collect_redundant_boolean_if_expressions_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_redundant_boolean_if_expressions_in_expr(task, condition, findings);
            collect_redundant_boolean_if_expressions_in_expr(task, then_branch, findings);
            collect_redundant_boolean_if_expressions_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_redundant_boolean_if_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_redundant_boolean_if_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_redundant_boolean_if_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_redundant_boolean_if_expressions_in_expr(task, &entry.key, findings);
                collect_redundant_boolean_if_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_redundant_boolean_if_expressions_in_expr(task, collection, findings);
            collect_redundant_boolean_if_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_redundant_boolean_if_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_redundant_boolean_if_expressions_in_expr(task, &field.expr, findings);
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

fn redundant_boolean_if_expression_replacement(expr: &Expr) -> Option<String> {
    let ExprKind::If {
        condition,
        then_branch,
        else_branch,
    } = &expr.kind
    else {
        return None;
    };
    if bool_literal_value(condition).is_some() {
        return None;
    }
    match (
        bool_literal_value(then_branch),
        bool_literal_value(else_branch),
    ) {
        (Some(true), Some(false)) => Some(condition.source.clone()),
        (Some(false), Some(true)) => Some(negated_boolean_source(condition)),
        _ => None,
    }
}

fn collect_double_negation_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_double_negation_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_double_negation_expressions_in_expr(task, condition, findings);
                collect_double_negation_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_double_negation_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_double_negation_expressions_in_expr(task, condition, findings);
                collect_double_negation_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_double_negation_expressions_in_expr(task, collection, findings);
                collect_double_negation_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_double_negation_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_double_negation_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = double_negation_expression_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "DOUBLE_NEGATION_EXPRESSION".to_string(),
            rule: LintRule::DoubleNegationExpression.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has a double negation expression `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!(
                "replace the double negation expression with `{}`",
                replacement.source
            ),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_double_negation_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_double_negation_expressions_in_expr(task, left, findings);
            collect_double_negation_expressions_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_double_negation_expressions_in_expr(task, condition, findings);
            collect_double_negation_expressions_in_expr(task, then_branch, findings);
            collect_double_negation_expressions_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_double_negation_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_double_negation_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_double_negation_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_double_negation_expressions_in_expr(task, &entry.key, findings);
                collect_double_negation_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_double_negation_expressions_in_expr(task, collection, findings);
            collect_double_negation_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_double_negation_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_double_negation_expressions_in_expr(task, &field.expr, findings);
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

fn double_negation_expression_replacement(expr: &Expr) -> Option<&Expr> {
    let ExprKind::Unary {
        op: UnaryOp::Not,
        expr: inner,
    } = &expr.kind
    else {
        return None;
    };
    let ExprKind::Unary {
        op: UnaryOp::Not,
        expr: replacement,
    } = &inner.kind
    else {
        return None;
    };
    Some(replacement)
}

fn collect_negated_comparison_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_negated_comparison_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_negated_comparison_expressions_in_expr(task, condition, findings);
                collect_negated_comparison_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_negated_comparison_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_negated_comparison_expressions_in_expr(task, condition, findings);
                collect_negated_comparison_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_negated_comparison_expressions_in_expr(task, collection, findings);
                collect_negated_comparison_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_negated_comparison_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_negated_comparison_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = negated_comparison_expression_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "NEGATED_COMPARISON_EXPRESSION".to_string(),
            rule: LintRule::NegatedComparisonExpression.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has a negated comparison expression `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!("replace the negated comparison expression with `{replacement}`"),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_negated_comparison_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_negated_comparison_expressions_in_expr(task, left, findings);
            collect_negated_comparison_expressions_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_negated_comparison_expressions_in_expr(task, condition, findings);
            collect_negated_comparison_expressions_in_expr(task, then_branch, findings);
            collect_negated_comparison_expressions_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_negated_comparison_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_negated_comparison_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_negated_comparison_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_negated_comparison_expressions_in_expr(task, &entry.key, findings);
                collect_negated_comparison_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_negated_comparison_expressions_in_expr(task, collection, findings);
            collect_negated_comparison_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_negated_comparison_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_negated_comparison_expressions_in_expr(task, &field.expr, findings);
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

fn negated_comparison_expression_replacement(expr: &Expr) -> Option<String> {
    let ExprKind::Unary {
        op: UnaryOp::Not,
        expr: inner,
    } = &expr.kind
    else {
        return None;
    };
    let ExprKind::Binary { op, left, right } = &inner.kind else {
        return None;
    };
    let replacement_op = match op {
        BinaryOp::Equal => BinaryOp::NotEqual,
        BinaryOp::NotEqual => BinaryOp::Equal,
        BinaryOp::Less => BinaryOp::GreaterEqual,
        BinaryOp::LessEqual => BinaryOp::Greater,
        BinaryOp::Greater => BinaryOp::LessEqual,
        BinaryOp::GreaterEqual => BinaryOp::Less,
        _ => return None,
    };
    Some(format!(
        "{} {} {}",
        left.source,
        replacement_op.as_str(),
        right.source
    ))
}

fn collect_redundant_boolean_comparisons_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_redundant_boolean_comparisons_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_redundant_boolean_comparisons_in_expr(task, condition, findings);
                collect_redundant_boolean_comparisons_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_redundant_boolean_comparisons_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_redundant_boolean_comparisons_in_expr(task, condition, findings);
                collect_redundant_boolean_comparisons_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_redundant_boolean_comparisons_in_expr(task, collection, findings);
                collect_redundant_boolean_comparisons_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_redundant_boolean_comparisons_in_block(task, body, findings);
            }
        }
    }
}

fn collect_redundant_boolean_comparisons_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = redundant_boolean_comparison_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "REDUNDANT_BOOLEAN_COMPARISON".to_string(),
            rule: LintRule::RedundantBooleanComparison.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has a redundant boolean comparison `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!("replace the boolean comparison with `{replacement}`"),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_redundant_boolean_comparisons_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_redundant_boolean_comparisons_in_expr(task, left, findings);
            collect_redundant_boolean_comparisons_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_redundant_boolean_comparisons_in_expr(task, condition, findings);
            collect_redundant_boolean_comparisons_in_expr(task, then_branch, findings);
            collect_redundant_boolean_comparisons_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_redundant_boolean_comparisons_in_expr(task, callee, findings);
            for arg in args {
                collect_redundant_boolean_comparisons_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_redundant_boolean_comparisons_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_redundant_boolean_comparisons_in_expr(task, &entry.key, findings);
                collect_redundant_boolean_comparisons_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_redundant_boolean_comparisons_in_expr(task, collection, findings);
            collect_redundant_boolean_comparisons_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_redundant_boolean_comparisons_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_redundant_boolean_comparisons_in_expr(task, &field.expr, findings);
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

fn redundant_boolean_comparison_replacement(expr: &Expr) -> Option<String> {
    let ExprKind::Binary { op, left, right } = &expr.kind else {
        return None;
    };
    let left_bool = bool_literal_value(left);
    let right_bool = bool_literal_value(right);
    if left_bool.is_some() && right_bool.is_some() {
        return None;
    }
    match (op, left_bool, right_bool) {
        (BinaryOp::Equal, Some(true), None) | (BinaryOp::NotEqual, Some(false), None) => {
            Some(right.source.clone())
        }
        (BinaryOp::Equal, None, Some(true)) | (BinaryOp::NotEqual, None, Some(false)) => {
            Some(left.source.clone())
        }
        (BinaryOp::Equal, Some(false), None) | (BinaryOp::NotEqual, Some(true), None) => {
            Some(negated_boolean_source(right))
        }
        (BinaryOp::Equal, None, Some(false)) | (BinaryOp::NotEqual, None, Some(true)) => {
            Some(negated_boolean_source(left))
        }
        _ => None,
    }
}

fn lint_constant_comparison_expressions(
    program: &Program,
    module: Option<&str>,
) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_constant_comparison_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn collect_constant_comparison_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_constant_comparison_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_constant_comparison_expressions_in_expr(task, condition, findings);
                collect_constant_comparison_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_constant_comparison_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_constant_comparison_expressions_in_expr(task, condition, findings);
                collect_constant_comparison_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_constant_comparison_expressions_in_expr(task, collection, findings);
                collect_constant_comparison_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_constant_comparison_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_constant_comparison_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = constant_comparison_expression_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "CONSTANT_COMPARISON_EXPRESSION".to_string(),
            rule: LintRule::ConstantComparisonExpression.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has a constant comparison expression `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!("replace the constant comparison expression with `{replacement}`"),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_constant_comparison_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_constant_comparison_expressions_in_expr(task, left, findings);
            collect_constant_comparison_expressions_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_constant_comparison_expressions_in_expr(task, condition, findings);
            collect_constant_comparison_expressions_in_expr(task, then_branch, findings);
            collect_constant_comparison_expressions_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_constant_comparison_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_constant_comparison_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_constant_comparison_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_constant_comparison_expressions_in_expr(task, &entry.key, findings);
                collect_constant_comparison_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_constant_comparison_expressions_in_expr(task, collection, findings);
            collect_constant_comparison_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_constant_comparison_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_constant_comparison_expressions_in_expr(task, &field.expr, findings);
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

fn constant_comparison_expression_replacement(expr: &Expr) -> Option<String> {
    let ExprKind::Binary { op, left, right } = &expr.kind else {
        return None;
    };
    if left.source == right.source {
        return None;
    }
    let value = match op {
        BinaryOp::Equal => constant_equality_value(left, right)?,
        BinaryOp::NotEqual => !constant_equality_value(left, right)?,
        BinaryOp::Less => numeric_literal_value(left)? < numeric_literal_value(right)?,
        BinaryOp::LessEqual => numeric_literal_value(left)? <= numeric_literal_value(right)?,
        BinaryOp::Greater => numeric_literal_value(left)? > numeric_literal_value(right)?,
        BinaryOp::GreaterEqual => numeric_literal_value(left)? >= numeric_literal_value(right)?,
        _ => return None,
    };
    Some(value.to_string())
}

fn constant_equality_value(left: &Expr, right: &Expr) -> Option<bool> {
    match (&left.kind, &right.kind) {
        (ExprKind::StringLiteral { value: left }, ExprKind::StringLiteral { value: right }) => {
            Some(left == right)
        }
        (ExprKind::IntLiteral { value: left }, ExprKind::IntLiteral { value: right }) => {
            Some(left == right)
        }
        (ExprKind::FloatLiteral { value: left }, ExprKind::FloatLiteral { value: right }) => {
            Some(left == right)
        }
        _ => None,
    }
}

fn numeric_literal_value(expr: &Expr) -> Option<f64> {
    match &expr.kind {
        ExprKind::IntLiteral { value } => Some(*value as f64),
        ExprKind::FloatLiteral { value } => Some(*value),
        _ => None,
    }
}

fn lint_absorbing_boolean_expressions(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for task in program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
    {
        collect_absorbing_boolean_expressions_in_block(task, &task.body, &mut findings);
    }
    findings
}

fn collect_absorbing_boolean_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_absorbing_boolean_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_absorbing_boolean_expressions_in_expr(task, condition, findings);
                collect_absorbing_boolean_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_absorbing_boolean_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_absorbing_boolean_expressions_in_expr(task, condition, findings);
                collect_absorbing_boolean_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_absorbing_boolean_expressions_in_expr(task, collection, findings);
                collect_absorbing_boolean_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_absorbing_boolean_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_absorbing_boolean_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = absorbing_boolean_expression_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "ABSORBING_BOOLEAN_EXPRESSION".to_string(),
            rule: LintRule::AbsorbingBooleanExpression.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has an absorbing boolean expression `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!("replace the absorbing boolean expression with `{replacement}`"),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_absorbing_boolean_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_absorbing_boolean_expressions_in_expr(task, left, findings);
            collect_absorbing_boolean_expressions_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_absorbing_boolean_expressions_in_expr(task, condition, findings);
            collect_absorbing_boolean_expressions_in_expr(task, then_branch, findings);
            collect_absorbing_boolean_expressions_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_absorbing_boolean_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_absorbing_boolean_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_absorbing_boolean_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_absorbing_boolean_expressions_in_expr(task, &entry.key, findings);
                collect_absorbing_boolean_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_absorbing_boolean_expressions_in_expr(task, collection, findings);
            collect_absorbing_boolean_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_absorbing_boolean_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_absorbing_boolean_expressions_in_expr(task, &field.expr, findings);
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

fn absorbing_boolean_expression_replacement(expr: &Expr) -> Option<String> {
    let ExprKind::Binary { op, left, right } = &expr.kind else {
        return None;
    };
    match op {
        BinaryOp::And if bool_literal_value(left) == Some(false) => Some("false".to_string()),
        BinaryOp::And
            if bool_literal_value(right) == Some(false) && expr_is_delete_safe_pure(left) =>
        {
            Some("false".to_string())
        }
        BinaryOp::Or if bool_literal_value(left) == Some(true) => Some("true".to_string()),
        BinaryOp::Or
            if bool_literal_value(right) == Some(true) && expr_is_delete_safe_pure(left) =>
        {
            Some("true".to_string())
        }
        _ => None,
    }
}

fn collect_self_comparison_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_self_comparison_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_self_comparison_expressions_in_expr(task, condition, findings);
                collect_self_comparison_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_self_comparison_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_self_comparison_expressions_in_expr(task, condition, findings);
                collect_self_comparison_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_self_comparison_expressions_in_expr(task, collection, findings);
                collect_self_comparison_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_self_comparison_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_self_comparison_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = self_comparison_expression_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "SELF_COMPARISON_EXPRESSION".to_string(),
            rule: LintRule::SelfComparisonExpression.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has a self-comparison expression `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!("replace the self-comparison expression with `{replacement}`"),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_self_comparison_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_self_comparison_expressions_in_expr(task, left, findings);
            collect_self_comparison_expressions_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_self_comparison_expressions_in_expr(task, condition, findings);
            collect_self_comparison_expressions_in_expr(task, then_branch, findings);
            collect_self_comparison_expressions_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_self_comparison_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_self_comparison_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_self_comparison_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_self_comparison_expressions_in_expr(task, &entry.key, findings);
                collect_self_comparison_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_self_comparison_expressions_in_expr(task, collection, findings);
            collect_self_comparison_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_self_comparison_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_self_comparison_expressions_in_expr(task, &field.expr, findings);
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

fn self_comparison_expression_replacement(expr: &Expr) -> Option<String> {
    let ExprKind::Binary { op, left, right } = &expr.kind else {
        return None;
    };
    if left.source != right.source
        || !expr_is_delete_safe_pure(left)
        || !expr_is_delete_safe_pure(right)
    {
        return None;
    }
    match op {
        BinaryOp::Equal => Some("true".to_string()),
        BinaryOp::NotEqual => Some("false".to_string()),
        BinaryOp::Less | BinaryOp::Greater => Some("false".to_string()),
        _ => None,
    }
}

fn bool_literal_value(expr: &Expr) -> Option<bool> {
    match &expr.kind {
        ExprKind::BoolLiteral { value } => Some(*value),
        _ => None,
    }
}

fn negated_boolean_source(expr: &Expr) -> String {
    if boolean_negation_operand_is_simple(expr) {
        format!("!{}", expr.source)
    } else {
        format!("!({})", expr.source)
    }
}

fn boolean_negation_operand_is_simple(expr: &Expr) -> bool {
    matches!(
        &expr.kind,
        ExprKind::Identifier { .. }
            | ExprKind::BoolLiteral { .. }
            | ExprKind::Call { .. }
            | ExprKind::Index { .. }
            | ExprKind::FieldAccess { .. }
    )
}

fn collect_identity_binary_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_identity_binary_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_identity_binary_expressions_in_expr(task, condition, findings);
                collect_identity_binary_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_identity_binary_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_identity_binary_expressions_in_expr(task, condition, findings);
                collect_identity_binary_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_identity_binary_expressions_in_expr(task, collection, findings);
                collect_identity_binary_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_identity_binary_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_identity_binary_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let Some(replacement) = identity_binary_expression_replacement(expr) {
        let task_name = task_fq_name(task);
        findings.push(LintFinding {
            id: "IDENTITY_BINARY_EXPRESSION".to_string(),
            rule: LintRule::IdentityBinaryExpression.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "task `{task_name}` has an identity binary expression `{}`",
                expr.source
            ),
            node: expr.id.clone(),
            module: task_module(task),
            hint: format!(
                "replace the identity binary expression with `{}`",
                replacement.source
            ),
        });
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_identity_binary_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_identity_binary_expressions_in_expr(task, left, findings);
            collect_identity_binary_expressions_in_expr(task, right, findings);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_identity_binary_expressions_in_expr(task, condition, findings);
            collect_identity_binary_expressions_in_expr(task, then_branch, findings);
            collect_identity_binary_expressions_in_expr(task, else_branch, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_identity_binary_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_identity_binary_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_identity_binary_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_identity_binary_expressions_in_expr(task, &entry.key, findings);
                collect_identity_binary_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_identity_binary_expressions_in_expr(task, collection, findings);
            collect_identity_binary_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_identity_binary_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_identity_binary_expressions_in_expr(task, &field.expr, findings);
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

fn identity_binary_expression_replacement(expr: &Expr) -> Option<&Expr> {
    let ExprKind::Binary { op, left, right } = &expr.kind else {
        return None;
    };
    match op {
        BinaryOp::Add if is_zero_literal(left) => Some(right),
        BinaryOp::Add if is_zero_literal(right) => Some(left),
        BinaryOp::Subtract if is_zero_literal(right) => Some(left),
        BinaryOp::Multiply if is_one_literal(left) => Some(right),
        BinaryOp::Multiply if is_one_literal(right) => Some(left),
        BinaryOp::Divide if is_one_literal(right) => Some(left),
        BinaryOp::And if is_true_literal(left) => Some(right),
        BinaryOp::And if is_true_literal(right) => Some(left),
        BinaryOp::Or if is_false_literal(left) => Some(right),
        BinaryOp::Or if is_false_literal(right) => Some(left),
        _ => None,
    }
}

fn is_zero_literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::IntLiteral { value } => *value == 0,
        ExprKind::FloatLiteral { value } => *value == 0.0,
        _ => false,
    }
}

fn is_one_literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::IntLiteral { value } => *value == 1,
        ExprKind::FloatLiteral { value } => *value == 1.0,
        _ => false,
    }
}

fn is_true_literal(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::BoolLiteral { value: true })
}

fn is_false_literal(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::BoolLiteral { value: false })
}

fn is_empty_list_literal(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::ListLiteral { items } if items.is_empty())
}

fn collect_constant_if_expressions_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                collect_constant_if_expressions_in_expr(task, expr, findings);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_constant_if_expressions_in_expr(task, condition, findings);
                collect_constant_if_expressions_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_constant_if_expressions_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { condition, body } => {
                collect_constant_if_expressions_in_expr(task, condition, findings);
                collect_constant_if_expressions_in_block(task, body, findings);
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_constant_if_expressions_in_expr(task, collection, findings);
                collect_constant_if_expressions_in_block(task, body, findings);
            }
            StatementKind::Forge { body } => {
                collect_constant_if_expressions_in_block(task, body, findings);
            }
        }
    }
}

fn collect_constant_if_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                if let Some(value) = bool_literal_value(condition)
                    && constant_if_statement_replacement(statement).is_some()
                {
                    let task_name = task_fq_name(task);
                    findings.push(LintFinding {
                        id: "CONSTANT_IF_STATEMENT".to_string(),
                        rule: LintRule::ConstantIfStatement.as_str().to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "task `{task_name}` has an if statement with constant `{value}` condition"
                        ),
                        node: statement.id.clone(),
                        module: task_module(task),
                        hint: "replace this if statement with its single executing branch statement"
                            .to_string(),
                    });
                } else {
                    collect_constant_if_statements_in_block(task, then_block, findings);
                    if let Some(else_block) = else_block {
                        collect_constant_if_statements_in_block(task, else_block, findings);
                    }
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                collect_constant_if_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_constant_false_while_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::While { condition, body } => {
                if is_false_literal(condition) {
                    let task_name = task_fq_name(task);
                    findings.push(LintFinding {
                        id: "CONSTANT_FALSE_WHILE_STATEMENT".to_string(),
                        rule: LintRule::ConstantFalseWhileStatement.as_str().to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "task `{task_name}` has a while statement with constant `false` condition"
                        ),
                        node: statement.id.clone(),
                        module: task_module(task),
                        hint: "delete this never-executed while statement".to_string(),
                    });
                } else {
                    collect_constant_false_while_statements_in_block(task, body, findings);
                }
            }
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_constant_false_while_statements_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_constant_false_while_statements_in_block(task, else_block, findings);
                }
            }
            StatementKind::For { body, .. } | StatementKind::Forge { body } => {
                collect_constant_false_while_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_empty_if_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let else_empty = else_block
                    .as_ref()
                    .map_or(true, |else_block| else_block.statements.is_empty());
                if expr_is_delete_safe_pure(condition)
                    && then_block.statements.is_empty()
                    && else_empty
                {
                    let task_name = task_fq_name(task);
                    findings.push(LintFinding {
                        id: "EMPTY_IF_STATEMENT".to_string(),
                        rule: LintRule::EmptyIfStatement.as_str().to_string(),
                        severity: "warning".to_string(),
                        message: format!("task `{task_name}` has an empty if statement"),
                        node: statement.id.clone(),
                        module: task_module(task),
                        hint: "delete this no-op if statement".to_string(),
                    });
                } else {
                    collect_empty_if_statements_in_block(task, then_block, findings);
                    if let Some(else_block) = else_block {
                        collect_empty_if_statements_in_block(task, else_block, findings);
                    }
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                collect_empty_if_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_empty_for_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::For {
                collection, body, ..
            } => {
                if is_empty_list_literal(collection) {
                    let task_name = task_fq_name(task);
                    findings.push(LintFinding {
                        id: "EMPTY_FOR_STATEMENT".to_string(),
                        rule: LintRule::EmptyForStatement.as_str().to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "task `{task_name}` has a for statement over an empty list"
                        ),
                        node: statement.id.clone(),
                        module: task_module(task),
                        hint: "delete this never-executed for statement".to_string(),
                    });
                } else {
                    collect_empty_for_statements_in_block(task, body, findings);
                }
            }
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_empty_for_statements_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_empty_for_statements_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { body, .. } | StatementKind::Forge { body } => {
                collect_empty_for_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_empty_forge_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Forge { body } => {
                if body.statements.is_empty() {
                    let task_name = task_fq_name(task);
                    findings.push(LintFinding {
                        id: "EMPTY_FORGE_STATEMENT".to_string(),
                        rule: LintRule::EmptyForgeStatement.as_str().to_string(),
                        severity: "warning".to_string(),
                        message: format!("task `{task_name}` has an empty forge statement"),
                        node: statement.id.clone(),
                        module: task_module(task),
                        hint: "delete this no-op forge statement".to_string(),
                    });
                } else {
                    collect_empty_forge_statements_in_block(task, body, findings);
                }
            }
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_empty_forge_statements_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_empty_forge_statements_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { body, .. } | StatementKind::For { body, .. } => {
                collect_empty_forge_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_constant_if_expressions_in_expr(
    task: &TaskDecl,
    expr: &Expr,
    findings: &mut Vec<LintFinding>,
) {
    if let ExprKind::If {
        condition,
        then_branch,
        else_branch,
    } = &expr.kind
    {
        if let ExprKind::BoolLiteral { value } = &condition.kind {
            let value = *value;
            let task_name = task_fq_name(task);
            let module_name = task_module(task);
            let branch = if value { "then" } else { "else" };
            findings.push(LintFinding {
                id: "CONSTANT_IF_EXPRESSION".to_string(),
                rule: LintRule::ConstantIfExpression.as_str().to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "task `{task_name}` has an if expression with constant `{value}` condition"
                ),
                node: expr.id.clone(),
                module: module_name,
                hint: format!("replace the if expression with its `{branch}` branch"),
            });
        }
        collect_constant_if_expressions_in_expr(task, condition, findings);
        collect_constant_if_expressions_in_expr(task, then_branch, findings);
        collect_constant_if_expressions_in_expr(task, else_branch, findings);
        return;
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_constant_if_expressions_in_expr(task, expr, findings);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_constant_if_expressions_in_expr(task, left, findings);
            collect_constant_if_expressions_in_expr(task, right, findings);
        }
        ExprKind::Call { callee, args } => {
            collect_constant_if_expressions_in_expr(task, callee, findings);
            for arg in args {
                collect_constant_if_expressions_in_expr(task, arg, findings);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_constant_if_expressions_in_expr(task, item, findings);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_constant_if_expressions_in_expr(task, &entry.key, findings);
                collect_constant_if_expressions_in_expr(task, &entry.value, findings);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_constant_if_expressions_in_expr(task, collection, findings);
            collect_constant_if_expressions_in_expr(task, index, findings);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_constant_if_expressions_in_expr(task, receiver, findings);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_constant_if_expressions_in_expr(task, &field.expr, findings);
            }
        }
        ExprKind::If { .. }
        | ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => {}
    }
}

fn collect_mutable_bindings_never_set_in_block(
    task: &TaskDecl,
    task_body: &Block,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding {
                binding_kind, name, ..
            } if binding_kind.is_mutable_local() && !block_sets_identifier(task_body, name) => {
                let task_name = task_fq_name(task);
                let module_name = task_module(task);
                findings.push(LintFinding {
                    id: "MUTABLE_BINDING_NEVER_SET".to_string(),
                    rule: LintRule::MutableBindingNeverSet.as_str().to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "task `{task_name}` declares mutable `{}` binding `{name}` but never sets it",
                        binding_kind.as_source_keyword()
                    ),
                    node: statement.id.clone(),
                    module: module_name,
                    hint: format!(
                        "use `bind {name} = ...` for immutable data, or add a real `set {name} = ...` mutation"
                    ),
                });
            }
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_mutable_bindings_never_set_in_block(task, task_body, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_mutable_bindings_never_set_in_block(
                        task, task_body, else_block, findings,
                    );
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                collect_mutable_bindings_never_set_in_block(task, task_body, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_unused_pure_bindings_in_block(
    task: &TaskDecl,
    task_body: &Block,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding {
                binding_kind,
                name,
                expr,
                ..
            } if binding_kind == &BindingKind::Bind
                && expr_is_delete_safe_pure(expr)
                && !block_uses_identifier_except_statement(task_body, name, &statement.id) =>
            {
                let task_name = task_fq_name(task);
                let module_name = task_module(task);
                findings.push(LintFinding {
                    id: "UNUSED_PURE_BINDING".to_string(),
                    rule: LintRule::UnusedPureBinding.as_str().to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "task `{task_name}` binds `{name}` to a pure value but never reads it"
                    ),
                    node: statement.id.clone(),
                    module: module_name,
                    hint: format!("delete bind `{name}` or read it in the task body"),
                });
            }
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_unused_pure_bindings_in_block(task, task_body, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_unused_pure_bindings_in_block(task, task_body, else_block, findings);
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                collect_unused_pure_bindings_in_block(task, task_body, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_unused_pure_expression_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Expr { expr } if expr_is_delete_safe_pure(expr) => {
                let task_name = task_fq_name(task);
                findings.push(LintFinding {
                    id: "UNUSED_PURE_EXPRESSION_STATEMENT".to_string(),
                    rule: LintRule::UnusedPureExpressionStatement.as_str().to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "task `{task_name}` has an unused pure expression statement `{}`",
                        expr.source
                    ),
                    node: statement.id.clone(),
                    module: task_module(task),
                    hint: "delete this no-op expression statement".to_string(),
                });
            }
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_unused_pure_expression_statements_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_unused_pure_expression_statements_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                collect_unused_pure_expression_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_unreachable_statements_in_block(
    task: &TaskDecl,
    block: &Block,
    findings: &mut Vec<LintFinding>,
) {
    let mut unreachable = false;
    for statement in &block.statements {
        if unreachable {
            let task_name = task_fq_name(task);
            findings.push(LintFinding {
                id: "UNREACHABLE_STATEMENT".to_string(),
                rule: LintRule::UnreachableStatement.as_str().to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "task `{task_name}` has an unreachable statement after a guaranteed return"
                ),
                node: statement.id.clone(),
                module: task_module(task),
                hint: "delete this unreachable statement".to_string(),
            });
            continue;
        }

        match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_unreachable_statements_in_block(task, then_block, findings);
                if let Some(else_block) = else_block {
                    collect_unreachable_statements_in_block(task, else_block, findings);
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                collect_unreachable_statements_in_block(task, body, findings);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }

        if statement_guarantees_return_for_lint(statement) {
            unreachable = true;
        }
    }
}

fn block_guarantees_return_for_lint(block: &Block) -> bool {
    block
        .statements
        .iter()
        .any(statement_guarantees_return_for_lint)
}

fn statement_guarantees_return_for_lint(statement: &Statement) -> bool {
    match &statement.kind {
        StatementKind::Return { .. } => true,
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => else_block.as_ref().is_some_and(|else_block| {
            block_guarantees_return_for_lint(then_block)
                && block_guarantees_return_for_lint(else_block)
        }),
        StatementKind::Binding { .. }
        | StatementKind::Set { .. }
        | StatementKind::Expr { .. }
        | StatementKind::While { .. }
        | StatementKind::For { .. }
        | StatementKind::Forge { .. } => false,
    }
}

fn expr_is_delete_safe_pure(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Raw { .. }
        | ExprKind::Call { .. }
        | ExprKind::Index { .. }
        | ExprKind::Try { .. } => false,
        ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => true,
        ExprKind::Unary { expr, .. } => expr_is_delete_safe_pure(expr),
        ExprKind::Binary { op, left, right } => {
            !matches!(op, BinaryOp::Divide | BinaryOp::Remainder)
                && expr_is_delete_safe_pure(left)
                && expr_is_delete_safe_pure(right)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_is_delete_safe_pure(condition)
                && expr_is_delete_safe_pure(then_branch)
                && expr_is_delete_safe_pure(else_branch)
        }
        ExprKind::ListLiteral { items } => items.iter().all(expr_is_delete_safe_pure),
        ExprKind::MapLiteral { entries } => entries.iter().all(|entry| {
            expr_is_delete_safe_pure(&entry.key) && expr_is_delete_safe_pure(&entry.value)
        }),
        ExprKind::FieldAccess { receiver, .. } => expr_is_delete_safe_pure(receiver),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .all(|field| expr_is_delete_safe_pure(&field.expr)),
    }
}

fn block_uses_identifier_except_statement(
    block: &Block,
    name: &str,
    skipped_statement_id: &str,
) -> bool {
    block.statements.iter().any(|statement| {
        statement_uses_identifier_except_statement(statement, name, skipped_statement_id)
    })
}

fn statement_uses_identifier_except_statement(
    statement: &Statement,
    name: &str,
    skipped_statement_id: &str,
) -> bool {
    if statement.id == skipped_statement_id {
        return false;
    }
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
                || block_uses_identifier_except_statement(then_block, name, skipped_statement_id)
                || else_block.as_ref().is_some_and(|block| {
                    block_uses_identifier_except_statement(block, name, skipped_statement_id)
                })
        }
        StatementKind::While { condition, body } => {
            expr_uses_identifier(condition, name)
                || block_uses_identifier_except_statement(body, name, skipped_statement_id)
        }
        StatementKind::For {
            item,
            collection,
            body,
        } => {
            expr_uses_identifier(collection, name)
                || (item != name
                    && block_uses_identifier_except_statement(body, name, skipped_statement_id))
        }
        StatementKind::Forge { body } => {
            block_uses_identifier_except_statement(body, name, skipped_statement_id)
        }
    }
}

fn block_uses_identifier(block: &Block, name: &str) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement_uses_identifier(statement, name))
}

fn block_sets_identifier(block: &Block, name: &str) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement_sets_identifier(statement, name))
}

fn statement_sets_identifier(statement: &Statement, name: &str) -> bool {
    match &statement.kind {
        StatementKind::Set {
            name: candidate, ..
        } => candidate == name,
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => {
            block_sets_identifier(then_block, name)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block_sets_identifier(block, name))
        }
        StatementKind::While { body, .. }
        | StatementKind::For { body, .. }
        | StatementKind::Forge { body } => block_sets_identifier(body, name),
        StatementKind::Binding { .. }
        | StatementKind::Return { .. }
        | StatementKind::Expr { .. } => false,
    }
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

pub fn raw_host_adapter_replacement(callee: &str) -> Option<&'static str> {
    match callee {
        "fs.read_text" => Some("fs.try_read_text"),
        "fs.write_text" => Some("fs.try_write_text"),
        "db.query_one" => Some("db.try_query_one"),
        "db.query" => Some("db.try_query"),
        _ => None,
    }
}

pub fn qualified_imported_call_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    let call = collect_task_calls(program)
        .into_iter()
        .find(|call| call.expr_id == target)?;
    let replacement_callee = qualified_imported_call_callee(program, &call)?;
    replace_call_callee(&call.source, &call.callee, &replacement_callee)
}

fn qualified_imported_call_callee(program: &Program, call: &TaskCallSummary) -> Option<String> {
    if call.status != "resolved" || call.callee.contains('.') {
        return None;
    }
    let target = call.target.as_deref()?;
    let (target_module, target_name) = target.rsplit_once('.')?;
    if target_module == call.from_module || target_name != call.callee {
        return None;
    }
    let qualifier = import_qualifier_for_module(program, &call.from_module, target_module)?;
    Some(format!("{qualifier}.{}", call.callee))
}

fn import_qualifier_for_module(
    program: &Program,
    owner_module: &str,
    imported_module: &str,
) -> Option<String> {
    program
        .imports
        .iter()
        .find(|import| {
            import_owner_module(import) == owner_module && import.module == imported_module
        })
        .and_then(|import| {
            import.alias.clone().or_else(|| {
                import
                    .module
                    .rsplit('.')
                    .next()
                    .filter(|segment| !segment.is_empty())
                    .map(str::to_string)
            })
        })
}

fn replace_call_callee(source: &str, callee: &str, replacement_callee: &str) -> Option<String> {
    let trimmed = source.trim_start();
    let leading = &source[..source.len() - trimmed.len()];
    let (prefix, rest) = if let Some(rest) = trimmed.strip_prefix("call ") {
        ("call ", rest)
    } else {
        ("", trimmed)
    };
    let suffix = rest.strip_prefix(callee)?;
    if !suffix.starts_with('(') {
        return None;
    }
    Some(format!("{leading}{prefix}{replacement_callee}{suffix}"))
}

pub fn constant_if_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| constant_if_expression_replacement_in_block(&task.body, target))
}

fn constant_if_expression_replacement_in_block(block: &Block, target: &str) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                constant_if_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => constant_if_expression_replacement_in_expr(condition, target)
                .or_else(|| constant_if_expression_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        constant_if_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                constant_if_expression_replacement_in_expr(condition, target)
                    .or_else(|| constant_if_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => constant_if_expression_replacement_in_expr(collection, target)
                .or_else(|| constant_if_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                constant_if_expression_replacement_in_block(body, target)
            }
        })
}

fn constant_if_expression_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        if let ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } = &expr.kind
        {
            if let ExprKind::BoolLiteral { value } = &condition.kind {
                return Some(if *value {
                    then_branch.source.clone()
                } else {
                    else_branch.source.clone()
                });
            }
        }
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            constant_if_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            constant_if_expression_replacement_in_expr(left, target)
                .or_else(|| constant_if_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => constant_if_expression_replacement_in_expr(condition, target)
            .or_else(|| constant_if_expression_replacement_in_expr(then_branch, target))
            .or_else(|| constant_if_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            constant_if_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| constant_if_expression_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| constant_if_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            constant_if_expression_replacement_in_expr(&entry.key, target)
                .or_else(|| constant_if_expression_replacement_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => {
            constant_if_expression_replacement_in_expr(collection, target)
                .or_else(|| constant_if_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            constant_if_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| constant_if_expression_replacement_in_expr(&field.expr, target)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn constant_if_statement_replacement_source(program: &Program, target: &str) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| constant_if_statement_replacement_in_block(&task.body, target))
}

fn constant_if_statement_replacement_in_block(block: &Block, target: &str) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                if statement.id == target {
                    return constant_if_statement_replacement(statement);
                }
                constant_if_statement_replacement_in_block(then_block, target).or_else(|| {
                    else_block
                        .as_ref()
                        .and_then(|block| constant_if_statement_replacement_in_block(block, target))
                })
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                constant_if_statement_replacement_in_block(body, target)
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => None,
        })
}

fn constant_if_statement_replacement(statement: &Statement) -> Option<String> {
    let StatementKind::If {
        condition,
        then_block,
        else_block,
    } = &statement.kind
    else {
        return None;
    };
    let selected_block = if bool_literal_value(condition)? {
        then_block
    } else {
        else_block.as_ref()?
    };
    let [selected_statement] = selected_block.statements.as_slice() else {
        return None;
    };
    Some(format_statement_source(selected_statement))
}

pub fn constant_comparison_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| constant_comparison_expression_replacement_in_block(&task.body, target))
}

fn constant_comparison_expression_replacement_in_block(
    block: &Block,
    target: &str,
) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                constant_comparison_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => constant_comparison_expression_replacement_in_expr(condition, target)
                .or_else(|| constant_comparison_expression_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        constant_comparison_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                constant_comparison_expression_replacement_in_expr(condition, target)
                    .or_else(|| constant_comparison_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => constant_comparison_expression_replacement_in_expr(collection, target)
                .or_else(|| constant_comparison_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                constant_comparison_expression_replacement_in_block(body, target)
            }
        })
}

fn constant_comparison_expression_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        return constant_comparison_expression_replacement(expr);
    }
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            constant_comparison_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            constant_comparison_expression_replacement_in_expr(left, target)
                .or_else(|| constant_comparison_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => constant_comparison_expression_replacement_in_expr(condition, target)
            .or_else(|| constant_comparison_expression_replacement_in_expr(then_branch, target))
            .or_else(|| constant_comparison_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            constant_comparison_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| constant_comparison_expression_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| constant_comparison_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            constant_comparison_expression_replacement_in_expr(&entry.key, target).or_else(|| {
                constant_comparison_expression_replacement_in_expr(&entry.value, target)
            })
        }),
        ExprKind::Index { collection, index } => {
            constant_comparison_expression_replacement_in_expr(collection, target)
                .or_else(|| constant_comparison_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            constant_comparison_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields.iter().find_map(|field| {
            constant_comparison_expression_replacement_in_expr(&field.expr, target)
        }),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn identity_binary_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| identity_binary_expression_replacement_in_block(&task.body, target))
}

fn identity_binary_expression_replacement_in_block(block: &Block, target: &str) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                identity_binary_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => identity_binary_expression_replacement_in_expr(condition, target)
                .or_else(|| identity_binary_expression_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        identity_binary_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                identity_binary_expression_replacement_in_expr(condition, target)
                    .or_else(|| identity_binary_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => identity_binary_expression_replacement_in_expr(collection, target)
                .or_else(|| identity_binary_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                identity_binary_expression_replacement_in_block(body, target)
            }
        })
}

fn identity_binary_expression_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        return identity_binary_expression_replacement(expr)
            .map(|replacement| replacement.source.clone());
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            identity_binary_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            identity_binary_expression_replacement_in_expr(left, target)
                .or_else(|| identity_binary_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => identity_binary_expression_replacement_in_expr(condition, target)
            .or_else(|| identity_binary_expression_replacement_in_expr(then_branch, target))
            .or_else(|| identity_binary_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            identity_binary_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| identity_binary_expression_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| identity_binary_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            identity_binary_expression_replacement_in_expr(&entry.key, target)
                .or_else(|| identity_binary_expression_replacement_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => {
            identity_binary_expression_replacement_in_expr(collection, target)
                .or_else(|| identity_binary_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            identity_binary_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| identity_binary_expression_replacement_in_expr(&field.expr, target)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn redundant_boolean_comparison_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| redundant_boolean_comparison_replacement_in_block(&task.body, target))
}

fn redundant_boolean_comparison_replacement_in_block(
    block: &Block,
    target: &str,
) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                redundant_boolean_comparison_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => redundant_boolean_comparison_replacement_in_expr(condition, target)
                .or_else(|| redundant_boolean_comparison_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        redundant_boolean_comparison_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                redundant_boolean_comparison_replacement_in_expr(condition, target)
                    .or_else(|| redundant_boolean_comparison_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => redundant_boolean_comparison_replacement_in_expr(collection, target)
                .or_else(|| redundant_boolean_comparison_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                redundant_boolean_comparison_replacement_in_block(body, target)
            }
        })
}

fn redundant_boolean_comparison_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        return redundant_boolean_comparison_replacement(expr);
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            redundant_boolean_comparison_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            redundant_boolean_comparison_replacement_in_expr(left, target)
                .or_else(|| redundant_boolean_comparison_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => redundant_boolean_comparison_replacement_in_expr(condition, target)
            .or_else(|| redundant_boolean_comparison_replacement_in_expr(then_branch, target))
            .or_else(|| redundant_boolean_comparison_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            redundant_boolean_comparison_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| redundant_boolean_comparison_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| redundant_boolean_comparison_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            redundant_boolean_comparison_replacement_in_expr(&entry.key, target)
                .or_else(|| redundant_boolean_comparison_replacement_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => {
            redundant_boolean_comparison_replacement_in_expr(collection, target)
                .or_else(|| redundant_boolean_comparison_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            redundant_boolean_comparison_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields.iter().find_map(|field| {
            redundant_boolean_comparison_replacement_in_expr(&field.expr, target)
        }),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn absorbing_boolean_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| absorbing_boolean_expression_replacement_in_block(&task.body, target))
}

fn absorbing_boolean_expression_replacement_in_block(
    block: &Block,
    target: &str,
) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                absorbing_boolean_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => absorbing_boolean_expression_replacement_in_expr(condition, target)
                .or_else(|| absorbing_boolean_expression_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        absorbing_boolean_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                absorbing_boolean_expression_replacement_in_expr(condition, target)
                    .or_else(|| absorbing_boolean_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => absorbing_boolean_expression_replacement_in_expr(collection, target)
                .or_else(|| absorbing_boolean_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                absorbing_boolean_expression_replacement_in_block(body, target)
            }
        })
}

fn absorbing_boolean_expression_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        return absorbing_boolean_expression_replacement(expr);
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            absorbing_boolean_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            absorbing_boolean_expression_replacement_in_expr(left, target)
                .or_else(|| absorbing_boolean_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => absorbing_boolean_expression_replacement_in_expr(condition, target)
            .or_else(|| absorbing_boolean_expression_replacement_in_expr(then_branch, target))
            .or_else(|| absorbing_boolean_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            absorbing_boolean_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| absorbing_boolean_expression_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| absorbing_boolean_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            absorbing_boolean_expression_replacement_in_expr(&entry.key, target)
                .or_else(|| absorbing_boolean_expression_replacement_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => {
            absorbing_boolean_expression_replacement_in_expr(collection, target)
                .or_else(|| absorbing_boolean_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            absorbing_boolean_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields.iter().find_map(|field| {
            absorbing_boolean_expression_replacement_in_expr(&field.expr, target)
        }),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn self_comparison_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| self_comparison_expression_replacement_in_block(&task.body, target))
}

fn self_comparison_expression_replacement_in_block(block: &Block, target: &str) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                self_comparison_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => self_comparison_expression_replacement_in_expr(condition, target)
                .or_else(|| self_comparison_expression_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        self_comparison_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                self_comparison_expression_replacement_in_expr(condition, target)
                    .or_else(|| self_comparison_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => self_comparison_expression_replacement_in_expr(collection, target)
                .or_else(|| self_comparison_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                self_comparison_expression_replacement_in_block(body, target)
            }
        })
}

fn self_comparison_expression_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        return self_comparison_expression_replacement(expr);
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            self_comparison_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            self_comparison_expression_replacement_in_expr(left, target)
                .or_else(|| self_comparison_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => self_comparison_expression_replacement_in_expr(condition, target)
            .or_else(|| self_comparison_expression_replacement_in_expr(then_branch, target))
            .or_else(|| self_comparison_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            self_comparison_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| self_comparison_expression_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| self_comparison_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            self_comparison_expression_replacement_in_expr(&entry.key, target)
                .or_else(|| self_comparison_expression_replacement_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => {
            self_comparison_expression_replacement_in_expr(collection, target)
                .or_else(|| self_comparison_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            self_comparison_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| self_comparison_expression_replacement_in_expr(&field.expr, target)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn double_negation_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| double_negation_expression_replacement_in_block(&task.body, target))
}

fn double_negation_expression_replacement_in_block(block: &Block, target: &str) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                double_negation_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => double_negation_expression_replacement_in_expr(condition, target)
                .or_else(|| double_negation_expression_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        double_negation_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                double_negation_expression_replacement_in_expr(condition, target)
                    .or_else(|| double_negation_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => double_negation_expression_replacement_in_expr(collection, target)
                .or_else(|| double_negation_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                double_negation_expression_replacement_in_block(body, target)
            }
        })
}

fn double_negation_expression_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        return double_negation_expression_replacement(expr)
            .map(|replacement| replacement.source.clone());
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            double_negation_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            double_negation_expression_replacement_in_expr(left, target)
                .or_else(|| double_negation_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => double_negation_expression_replacement_in_expr(condition, target)
            .or_else(|| double_negation_expression_replacement_in_expr(then_branch, target))
            .or_else(|| double_negation_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            double_negation_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| double_negation_expression_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| double_negation_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            double_negation_expression_replacement_in_expr(&entry.key, target)
                .or_else(|| double_negation_expression_replacement_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => {
            double_negation_expression_replacement_in_expr(collection, target)
                .or_else(|| double_negation_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            double_negation_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| double_negation_expression_replacement_in_expr(&field.expr, target)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn negated_comparison_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| negated_comparison_expression_replacement_in_block(&task.body, target))
}

fn negated_comparison_expression_replacement_in_block(
    block: &Block,
    target: &str,
) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                negated_comparison_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => negated_comparison_expression_replacement_in_expr(condition, target)
                .or_else(|| negated_comparison_expression_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        negated_comparison_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                negated_comparison_expression_replacement_in_expr(condition, target)
                    .or_else(|| negated_comparison_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => negated_comparison_expression_replacement_in_expr(collection, target)
                .or_else(|| negated_comparison_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                negated_comparison_expression_replacement_in_block(body, target)
            }
        })
}

fn negated_comparison_expression_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        return negated_comparison_expression_replacement(expr);
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            negated_comparison_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            negated_comparison_expression_replacement_in_expr(left, target)
                .or_else(|| negated_comparison_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => negated_comparison_expression_replacement_in_expr(condition, target)
            .or_else(|| negated_comparison_expression_replacement_in_expr(then_branch, target))
            .or_else(|| negated_comparison_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            negated_comparison_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| negated_comparison_expression_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| negated_comparison_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            negated_comparison_expression_replacement_in_expr(&entry.key, target)
                .or_else(|| negated_comparison_expression_replacement_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => {
            negated_comparison_expression_replacement_in_expr(collection, target)
                .or_else(|| negated_comparison_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            negated_comparison_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields.iter().find_map(|field| {
            negated_comparison_expression_replacement_in_expr(&field.expr, target)
        }),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn redundant_boolean_if_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| redundant_boolean_if_expression_replacement_in_block(&task.body, target))
}

fn redundant_boolean_if_expression_replacement_in_block(
    block: &Block,
    target: &str,
) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                redundant_boolean_if_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => redundant_boolean_if_expression_replacement_in_expr(condition, target)
                .or_else(|| {
                    redundant_boolean_if_expression_replacement_in_block(then_block, target)
                })
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        redundant_boolean_if_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                redundant_boolean_if_expression_replacement_in_expr(condition, target)
                    .or_else(|| redundant_boolean_if_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => redundant_boolean_if_expression_replacement_in_expr(collection, target)
                .or_else(|| redundant_boolean_if_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                redundant_boolean_if_expression_replacement_in_block(body, target)
            }
        })
}

fn redundant_boolean_if_expression_replacement_in_expr(
    expr: &Expr,
    target: &str,
) -> Option<String> {
    if expr.id == target {
        return redundant_boolean_if_expression_replacement(expr);
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            redundant_boolean_if_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            redundant_boolean_if_expression_replacement_in_expr(left, target)
                .or_else(|| redundant_boolean_if_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => redundant_boolean_if_expression_replacement_in_expr(condition, target)
            .or_else(|| redundant_boolean_if_expression_replacement_in_expr(then_branch, target))
            .or_else(|| redundant_boolean_if_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            redundant_boolean_if_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter().find_map(|arg| {
                    redundant_boolean_if_expression_replacement_in_expr(arg, target)
                })
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| redundant_boolean_if_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            redundant_boolean_if_expression_replacement_in_expr(&entry.key, target).or_else(|| {
                redundant_boolean_if_expression_replacement_in_expr(&entry.value, target)
            })
        }),
        ExprKind::Index { collection, index } => {
            redundant_boolean_if_expression_replacement_in_expr(collection, target)
                .or_else(|| redundant_boolean_if_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            redundant_boolean_if_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields.iter().find_map(|field| {
            redundant_boolean_if_expression_replacement_in_expr(&field.expr, target)
        }),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn redundant_boolean_if_statement_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| redundant_boolean_if_statement_replacement_in_block(&task.body, target))
}

fn redundant_boolean_if_statement_replacement_in_block(
    block: &Block,
    target: &str,
) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                if statement.id == target {
                    return redundant_boolean_if_statement_replacement(statement);
                }
                redundant_boolean_if_statement_replacement_in_block(then_block, target).or_else(
                    || {
                        else_block.as_ref().and_then(|block| {
                            redundant_boolean_if_statement_replacement_in_block(block, target)
                        })
                    },
                )
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                redundant_boolean_if_statement_replacement_in_block(body, target)
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => None,
        })
}

pub fn same_branch_if_expression_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| same_branch_if_expression_replacement_in_block(&task.body, target))
}

fn same_branch_if_expression_replacement_in_block(block: &Block, target: &str) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                same_branch_if_expression_replacement_in_expr(expr, target)
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => same_branch_if_expression_replacement_in_expr(condition, target)
                .or_else(|| same_branch_if_expression_replacement_in_block(then_block, target))
                .or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        same_branch_if_expression_replacement_in_block(block, target)
                    })
                }),
            StatementKind::While { condition, body } => {
                same_branch_if_expression_replacement_in_expr(condition, target)
                    .or_else(|| same_branch_if_expression_replacement_in_block(body, target))
            }
            StatementKind::For {
                collection, body, ..
            } => same_branch_if_expression_replacement_in_expr(collection, target)
                .or_else(|| same_branch_if_expression_replacement_in_block(body, target)),
            StatementKind::Forge { body } => {
                same_branch_if_expression_replacement_in_block(body, target)
            }
        })
}

fn same_branch_if_expression_replacement_in_expr(expr: &Expr, target: &str) -> Option<String> {
    if expr.id == target {
        return same_branch_if_expression_replacement(expr);
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            same_branch_if_expression_replacement_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => {
            same_branch_if_expression_replacement_in_expr(left, target)
                .or_else(|| same_branch_if_expression_replacement_in_expr(right, target))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => same_branch_if_expression_replacement_in_expr(condition, target)
            .or_else(|| same_branch_if_expression_replacement_in_expr(then_branch, target))
            .or_else(|| same_branch_if_expression_replacement_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            same_branch_if_expression_replacement_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| same_branch_if_expression_replacement_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| same_branch_if_expression_replacement_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            same_branch_if_expression_replacement_in_expr(&entry.key, target)
                .or_else(|| same_branch_if_expression_replacement_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => {
            same_branch_if_expression_replacement_in_expr(collection, target)
                .or_else(|| same_branch_if_expression_replacement_in_expr(index, target))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            same_branch_if_expression_replacement_in_expr(receiver, target)
        }
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| same_branch_if_expression_replacement_in_expr(&field.expr, target)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

pub fn same_branch_if_statement_replacement_source(
    program: &Program,
    target: &str,
) -> Option<String> {
    program
        .tasks
        .iter()
        .find_map(|task| same_branch_if_statement_replacement_in_block(&task.body, target))
}

fn same_branch_if_statement_replacement_in_block(block: &Block, target: &str) -> Option<String> {
    block
        .statements
        .iter()
        .find_map(|statement| match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                if statement.id == target {
                    return same_branch_if_statement_replacement(statement);
                }
                same_branch_if_statement_replacement_in_block(then_block, target).or_else(|| {
                    else_block.as_ref().and_then(|block| {
                        same_branch_if_statement_replacement_in_block(block, target)
                    })
                })
            }
            StatementKind::While { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::Forge { body } => {
                same_branch_if_statement_replacement_in_block(body, target)
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => None,
        })
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
