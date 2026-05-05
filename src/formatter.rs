use crate::ast::{Block, Program, StatementKind, TypeExpr};

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();

    if let Some(module) = &program.module {
        out.push_str("module ");
        out.push_str(module);
        out.push_str("\n\n");
    }

    for import in &program.imports {
        out.push_str("import ");
        out.push_str(&import.module);
        out.push('\n');
    }
    if !program.imports.is_empty() {
        out.push('\n');
    }

    for ty in &program.types {
        out.push_str("type ");
        out.push_str(&ty.name);
        out.push_str(" = ");
        match &ty.value {
            TypeExpr::Record { fields } => {
                out.push_str("{\n");
                for field in fields {
                    out.push_str("  ");
                    out.push_str(&field.name);
                    out.push_str(": ");
                    out.push_str(&field.ty.display());
                    out.push('\n');
                }
                out.push_str("}\n\n");
            }
            other => {
                out.push_str(&other.display());
                out.push_str("\n\n");
            }
        }
    }

    for effect in &program.effects {
        out.push_str("effect ");
        out.push_str(&effect.name);
        out.push('\n');
    }
    if !program.effects.is_empty() {
        out.push('\n');
    }

    for function in &program.functions {
        out.push_str("fn ");
        out.push_str(&function.name);
        out.push('(');
        out.push_str(
            &function
                .params
                .iter()
                .map(|param| format!("{}: {}", param.name, param.ty.display()))
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push_str(") -> ");
        out.push_str(&function.return_type.display());
        if !function.effects.is_empty() {
            out.push_str(" uses ");
            out.push_str(&function.effects.join(", "));
        }
        out.push_str(" {\n");
        format_block_statements(&mut out, &function.body, 1);
        out.push_str("}\n\n");
    }

    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn format_block_statements(out: &mut String, block: &Block, indent_level: usize) {
    for statement in &block.statements {
        format_statement(out, &statement.kind, indent_level);
    }
}

fn format_statement(out: &mut String, kind: &StatementKind, indent_level: usize) {
    push_indent(out, indent_level);
    match kind {
        StatementKind::Let {
            name,
            type_ann,
            expr,
        } => {
            out.push_str("let ");
            out.push_str(name);
            if let Some(type_ann) = type_ann {
                out.push_str(": ");
                out.push_str(&type_ann.display());
            }
            out.push_str(" = ");
            out.push_str(&expr.source);
            out.push('\n');
        }
        StatementKind::Set { name, expr } => {
            out.push_str("set ");
            out.push_str(name);
            out.push_str(" = ");
            out.push_str(&expr.source);
            out.push('\n');
        }
        StatementKind::Return { expr } => {
            out.push_str("return ");
            out.push_str(&expr.source);
            out.push('\n');
        }
        StatementKind::Expr { expr } => {
            out.push_str(&expr.source);
            out.push('\n');
        }
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            out.push_str("if ");
            out.push_str(&condition.source);
            out.push_str(" {\n");
            format_block_statements(out, then_block, indent_level + 1);
            push_indent(out, indent_level);
            out.push('}');
            if let Some(else_block) = else_block {
                out.push_str(" else {\n");
                format_block_statements(out, else_block, indent_level + 1);
                push_indent(out, indent_level);
                out.push('}');
            }
            out.push('\n');
        }
        StatementKind::While { condition, body } => {
            out.push_str("while ");
            out.push_str(&condition.source);
            out.push_str(" {\n");
            format_block_statements(out, body, indent_level + 1);
            push_indent(out, indent_level);
            out.push_str("}\n");
        }
    }
}

fn push_indent(out: &mut String, indent_level: usize) {
    for _ in 0..indent_level {
        out.push_str("  ");
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_program;

    use super::*;

    #[test]
    fn formats_stably() {
        let source = "module app\nfn main() -> Text {\nreturn \"ok\"\n}\n";
        let first = parse_program(source).expect("parse");
        let formatted = format_program(&first);
        let second = parse_program(&formatted).expect("parse formatted");
        assert_eq!(formatted, format_program(&second));
    }
}
