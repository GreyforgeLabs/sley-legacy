use crate::ast::{Program, StatementKind, TypeExpr};

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
        for statement in &function.body.statements {
            out.push_str("  ");
            match &statement.kind {
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
                }
                StatementKind::Return { expr } => {
                    out.push_str("return ");
                    out.push_str(&expr.source);
                }
                StatementKind::Expr { expr } => out.push_str(&expr.source),
            }
            out.push('\n');
        }
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
