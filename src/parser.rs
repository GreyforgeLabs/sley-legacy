use crate::ast::{
    Block, EffectDecl, Expr, ExprKind, FunctionDecl, ImportDecl, Param, Program, RecordField,
    Statement, StatementKind, TypeDecl, TypeExpr,
};
use crate::diagnostics::{Diagnostic, SourceSpan};

pub fn parse_program(source: &str) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(source)?;
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse_program()?;
    program.assign_ids();
    Ok(program)
}

pub fn parse_type_expr_source(source: &str) -> Result<TypeExpr, Vec<Diagnostic>> {
    let tokens = lex(source)?;
    let mut parser = Parser::new(tokens);
    parser.skip_newlines();
    let ty = parser.parse_type_expr()?;
    parser.skip_newlines();
    if !parser.at_eof() {
        return Err(vec![parser.error_here(
            "PARSE_TRAILING_TOKENS",
            "unexpected tokens after type expression",
        )]);
    }
    Ok(ty)
}

pub fn parse_block_source(source: &str) -> Result<Block, Vec<Diagnostic>> {
    let synthetic = format!("fn __patch_block() -> Unit {{\n{source}\n}}\n");
    let program = parse_program(&synthetic)?;
    Ok(program
        .functions
        .into_iter()
        .next()
        .map(|function| function.body)
        .unwrap_or(Block {
            statements: Vec::new(),
        }))
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Ident(String),
    String(String),
    Number(String),
    Symbol(char),
    Arrow,
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    text: String,
    span: SourceSpan,
}

impl Token {
    fn is_ident(&self, expected: &str) -> bool {
        matches!(&self.kind, TokenKind::Ident(value) if value == expected)
    }

    fn is_symbol(&self, expected: char) -> bool {
        matches!(self.kind, TokenKind::Symbol(value) if value == expected)
    }
}

fn lex(source: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut line = 1usize;
    let mut column = 1usize;
    let mut diagnostics = Vec::new();

    while let Some(ch) = chars.peek().copied() {
        let span = SourceSpan { line, column };
        match ch {
            ' ' | '\t' => {
                chars.next();
                column += 1;
            }
            '\r' => {
                chars.next();
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    text: "\n".to_string(),
                    span,
                });
                line += 1;
                column = 1;
            }
            '\n' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    text: "\n".to_string(),
                    span,
                });
                line += 1;
                column = 1;
            }
            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    while let Some(comment_ch) = chars.peek().copied() {
                        if comment_ch == '\n' || comment_ch == '\r' {
                            break;
                        }
                        chars.next();
                        column += 1;
                    }
                    column += 1;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Symbol('/'),
                        text: "/".to_string(),
                        span,
                    });
                    column += 1;
                }
            }
            '#' => {
                while let Some(comment_ch) = chars.peek().copied() {
                    if comment_ch == '\n' || comment_ch == '\r' {
                        break;
                    }
                    chars.next();
                    column += 1;
                }
            }
            '"' => {
                let mut text = String::from("\"");
                let mut value = String::new();
                chars.next();
                column += 1;
                let mut closed = false;
                while let Some(next) = chars.next() {
                    column += 1;
                    text.push(next);
                    match next {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' => {
                            if let Some(escaped) = chars.next() {
                                column += 1;
                                text.push(escaped);
                                value.push(match escaped {
                                    'n' => '\n',
                                    'r' => '\r',
                                    't' => '\t',
                                    '"' => '"',
                                    '\\' => '\\',
                                    other => other,
                                });
                            }
                        }
                        '\n' | '\r' => {
                            diagnostics.push(
                                Diagnostic::error(
                                    "LEX_UNTERMINATED_STRING",
                                    "string literal cannot contain a raw newline",
                                )
                                .with_span(span.clone()),
                            );
                            line += 1;
                            column = 1;
                            break;
                        }
                        other => value.push(other),
                    }
                }
                if !closed {
                    diagnostics.push(
                        Diagnostic::error("LEX_UNTERMINATED_STRING", "unterminated string literal")
                            .with_span(span.clone()),
                    );
                }
                tokens.push(Token {
                    kind: TokenKind::String(value),
                    text,
                    span,
                });
            }
            '0'..='9' => {
                let mut text = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_digit() || next == '.' {
                        text.push(next);
                        chars.next();
                        column += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Number(text.clone()),
                    text,
                    span,
                });
            }
            'A'..='Z' | 'a'..='z' | '_' => {
                let mut text = String::new();
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_alphanumeric() || next == '_' {
                        text.push(next);
                        chars.next();
                        column += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Ident(text.clone()),
                    text,
                    span,
                });
            }
            '-' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'>') {
                    chars.next();
                    column += 1;
                    tokens.push(Token {
                        kind: TokenKind::Arrow,
                        text: "->".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Symbol('-'),
                        text: "-".to_string(),
                        span,
                    });
                }
            }
            '{' | '}' | '(' | ')' | ':' | ',' | '=' | '<' | '>' | '?' | '.' | ';' | '+' | '*'
            | '[' | ']' => {
                chars.next();
                column += 1;
                tokens.push(Token {
                    kind: TokenKind::Symbol(ch),
                    text: ch.to_string(),
                    span,
                });
            }
            other => {
                diagnostics.push(
                    Diagnostic::error(
                        "LEX_UNKNOWN_CHARACTER",
                        format!("unknown character `{other}`"),
                    )
                    .with_span(span),
                );
                chars.next();
                column += 1;
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        span: SourceSpan { line, column },
    });

    if diagnostics.is_empty() {
        Ok(tokens)
    } else {
        Err(diagnostics)
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<Program, Vec<Diagnostic>> {
        let mut program = Program::new();
        self.skip_newlines();

        while !self.at_eof() {
            if self.current().is_ident("module") {
                let span = self.current().span.clone();
                self.bump();
                if program.module.is_some() {
                    return Err(vec![
                        Diagnostic::error("PARSE_DUPLICATE_MODULE", "module already declared")
                            .with_span(span),
                    ]);
                }
                program.module = Some(self.parse_module_path()?);
            } else if self.current().is_ident("import") {
                let span = self.current().span.clone();
                self.bump();
                program.imports.push(ImportDecl {
                    id: String::new(),
                    module: self.parse_module_path()?,
                    span: Some(span),
                });
            } else if self.current().is_ident("type") {
                program.types.push(self.parse_type_decl()?);
            } else if self.current().is_ident("effect") {
                let span = self.current().span.clone();
                self.bump();
                let name = self.expect_ident("expected effect name")?;
                program.effects.push(EffectDecl {
                    id: String::new(),
                    name,
                    span: Some(span),
                });
            } else if self.current().is_ident("fn") {
                program.functions.push(self.parse_function_decl()?);
            } else if self.current().is_symbol(';') {
                self.bump();
            } else {
                return Err(vec![self.error_here(
                    "PARSE_EXPECTED_ITEM",
                    "expected module, import, type, effect, or fn declaration",
                )]);
            }
            self.skip_statement_gap();
        }

        Ok(program)
    }

    fn parse_type_decl(&mut self) -> Result<TypeDecl, Vec<Diagnostic>> {
        let span = self.current().span.clone();
        self.expect_keyword("type", "expected type declaration")?;
        let name = self.expect_ident("expected type name")?;
        self.expect_symbol('=', "expected `=` after type name")?;
        let value = self.parse_type_expr()?;
        Ok(TypeDecl {
            id: String::new(),
            name,
            value,
            span: Some(span),
        })
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, Vec<Diagnostic>> {
        let span = self.current().span.clone();
        self.expect_keyword("fn", "expected function declaration")?;
        let name = self.expect_ident("expected function name")?;
        self.expect_symbol('(', "expected `(` after function name")?;
        let mut params = Vec::new();
        self.skip_newlines();
        if !self.current().is_symbol(')') {
            loop {
                let param_span = self.current().span.clone();
                let param_name = self.expect_ident("expected parameter name")?;
                self.expect_symbol(':', "expected `:` after parameter name")?;
                let ty = self.parse_type_expr()?;
                params.push(Param {
                    id: String::new(),
                    name: param_name,
                    ty,
                    span: Some(param_span),
                });
                self.skip_newlines();
                if self.current().is_symbol(',') {
                    self.bump();
                    self.skip_newlines();
                    if self.current().is_symbol(')') {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect_symbol(')', "expected `)` after parameters")?;
        self.expect_arrow("expected `->` after parameter list")?;
        let return_type = self.parse_type_expr()?;

        let mut effects = Vec::new();
        if self.current().is_ident("uses") {
            self.bump();
            loop {
                effects.push(self.expect_ident("expected effect name")?);
                if self.current().is_symbol(',') {
                    self.bump();
                } else {
                    break;
                }
            }
        }

        let body = self.parse_block()?;
        Ok(FunctionDecl {
            id: String::new(),
            name,
            params,
            return_type,
            effects,
            body,
            span: Some(span),
        })
    }

    fn parse_block(&mut self) -> Result<Block, Vec<Diagnostic>> {
        self.expect_symbol('{', "expected `{` to start block")?;
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.at_eof() && !self.current().is_symbol('}') {
            statements.push(self.parse_statement()?);
            self.skip_statement_gap();
        }
        self.expect_symbol('}', "expected `}` to close block")?;
        Ok(Block { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, Vec<Diagnostic>> {
        self.skip_newlines();
        let span = self.current().span.clone();
        if self.current().is_ident("let") {
            self.bump();
            let name = self.expect_ident("expected local binding name")?;
            let type_ann = if self.current().is_symbol(':') {
                self.bump();
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            self.expect_symbol('=', "expected `=` in let statement")?;
            let expr = self.parse_statement_expr(span.clone())?;
            Ok(Statement {
                id: String::new(),
                kind: StatementKind::Let {
                    name,
                    type_ann,
                    expr,
                },
                span: Some(span),
            })
        } else if self.current().is_ident("return") {
            self.bump();
            let expr = self.parse_statement_expr(span.clone())?;
            Ok(Statement {
                id: String::new(),
                kind: StatementKind::Return { expr },
                span: Some(span),
            })
        } else {
            let expr = self.parse_statement_expr(span.clone())?;
            Ok(Statement {
                id: String::new(),
                kind: StatementKind::Expr { expr },
                span: Some(span),
            })
        }
    }

    fn parse_statement_expr(&mut self, span: SourceSpan) -> Result<Expr, Vec<Diagnostic>> {
        let mut tokens = Vec::new();
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;

        while !self.at_eof() {
            let current = self.current().clone();
            match current.kind {
                TokenKind::Newline
                    if paren_depth == 0
                        && brace_depth == 0
                        && bracket_depth == 0
                        && angle_depth == 0 =>
                {
                    break;
                }
                TokenKind::Symbol('}')
                    if paren_depth == 0
                        && brace_depth == 0
                        && bracket_depth == 0
                        && angle_depth == 0 =>
                {
                    break;
                }
                TokenKind::Symbol('(') => paren_depth += 1,
                TokenKind::Symbol(')') => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::Symbol('{') => brace_depth += 1,
                TokenKind::Symbol('}') => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Symbol('[') => bracket_depth += 1,
                TokenKind::Symbol(']') => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::Symbol('<') => angle_depth += 1,
                TokenKind::Symbol('>') => angle_depth = angle_depth.saturating_sub(1),
                _ => {}
            }
            if !matches!(current.kind, TokenKind::Newline) {
                tokens.push(current);
            }
            self.bump();
        }

        if tokens.is_empty() {
            return Err(vec![
                Diagnostic::error("PARSE_EXPECTED_EXPRESSION", "expected expression")
                    .with_span(span),
            ]);
        }

        let source = tokens_to_source(&tokens);
        Ok(classify_expr(source, span))
    }

    fn parse_type_expr(&mut self) -> Result<TypeExpr, Vec<Diagnostic>> {
        self.skip_newlines();
        if self.current().is_symbol('{') {
            self.bump();
            let mut fields = Vec::new();
            self.skip_newlines();
            while !self.at_eof() && !self.current().is_symbol('}') {
                let name = self.expect_ident("expected record field name")?;
                self.expect_symbol(':', "expected `:` after record field name")?;
                let ty = self.parse_type_expr()?;
                fields.push(RecordField { name, ty });
                self.skip_newlines();
                if self.current().is_symbol(',') {
                    self.bump();
                    self.skip_newlines();
                }
            }
            self.expect_symbol('}', "expected `}` after record type")?;
            return Ok(TypeExpr::Record { fields });
        }

        let name = self.expect_ident("expected type name")?;
        if self.current().is_symbol('<') {
            self.bump();
            let mut args = Vec::new();
            loop {
                args.push(self.parse_type_expr()?);
                self.skip_newlines();
                if self.current().is_symbol(',') {
                    self.bump();
                    self.skip_newlines();
                } else {
                    break;
                }
            }
            self.expect_symbol('>', "expected `>` after type arguments")?;
            Ok(TypeExpr::Generic { name, args })
        } else {
            Ok(TypeExpr::Named { name })
        }
    }

    fn parse_module_path(&mut self) -> Result<String, Vec<Diagnostic>> {
        let mut parts = vec![self.expect_ident("expected module path")?];
        while self.current().is_symbol('.') {
            self.bump();
            parts.push(self.expect_ident("expected module path segment")?);
        }
        Ok(parts.join("."))
    }

    fn skip_statement_gap(&mut self) {
        while self.current().is_symbol(';') || matches!(self.current().kind, TokenKind::Newline) {
            self.bump();
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current().kind, TokenKind::Newline) {
            self.bump();
        }
    }

    fn expect_keyword(&mut self, expected: &str, message: &str) -> Result<(), Vec<Diagnostic>> {
        if self.current().is_ident(expected) {
            self.bump();
            Ok(())
        } else {
            Err(vec![self.error_here("PARSE_EXPECTED_KEYWORD", message)])
        }
    }

    fn expect_ident(&mut self, message: &str) -> Result<String, Vec<Diagnostic>> {
        match &self.current().kind {
            TokenKind::Ident(value) => {
                let value = value.clone();
                self.bump();
                Ok(value)
            }
            _ => Err(vec![self.error_here("PARSE_EXPECTED_IDENTIFIER", message)]),
        }
    }

    fn expect_symbol(&mut self, expected: char, message: &str) -> Result<(), Vec<Diagnostic>> {
        if self.current().is_symbol(expected) {
            self.bump();
            Ok(())
        } else {
            Err(vec![self.error_here("PARSE_EXPECTED_SYMBOL", message)])
        }
    }

    fn expect_arrow(&mut self, message: &str) -> Result<(), Vec<Diagnostic>> {
        if matches!(self.current().kind, TokenKind::Arrow) {
            self.bump();
            Ok(())
        } else {
            Err(vec![self.error_here("PARSE_EXPECTED_ARROW", message)])
        }
    }

    fn error_here(&self, id: &str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error(id, message).with_span(self.current().span.clone())
    }

    fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn bump(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }
}

fn classify_expr(source: String, span: SourceSpan) -> Expr {
    let trimmed = source.trim();
    let kind = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        ExprKind::StringLiteral {
            value: trimmed[1..trimmed.len() - 1].to_string(),
        }
    } else if trimmed == "true" {
        ExprKind::BoolLiteral { value: true }
    } else if trimmed == "false" {
        ExprKind::BoolLiteral { value: false }
    } else if let Ok(value) = trimmed.parse::<i64>() {
        ExprKind::IntLiteral { value }
    } else if trimmed.contains('.') {
        match trimmed.parse::<f64>() {
            Ok(value) => ExprKind::FloatLiteral { value },
            Err(_) => ExprKind::Raw {
                fallible: trimmed.ends_with('?') || trimmed.contains("? "),
            },
        }
    } else if is_plain_identifier(trimmed) {
        ExprKind::Identifier {
            name: trimmed.to_string(),
        }
    } else {
        ExprKind::Raw {
            fallible: trimmed.ends_with('?') || trimmed.contains("? "),
        }
    };

    Expr {
        id: String::new(),
        source,
        kind,
        span: Some(span),
    }
}

fn is_plain_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn tokens_to_source(tokens: &[Token]) -> String {
    let mut out = String::new();
    let mut previous: Option<&str> = None;
    for token in tokens {
        let current = token.text.as_str();
        if out.is_empty() {
            out.push_str(current);
        } else if needs_no_space_before(current) || previous.is_some_and(needs_no_space_after) {
            out.push_str(current);
        } else {
            out.push(' ');
            out.push_str(current);
        }
        previous = Some(current);
    }
    out
}

fn needs_no_space_before(text: &str) -> bool {
    matches!(text, ")" | "]" | "," | "." | "?" | ":" | ">" | "(" | "[")
}

fn needs_no_space_after(text: &str) -> bool {
    matches!(text, "." | "(" | "[" | "<")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_profile_fixture_shape() {
        let source = r#"
module app.profile

type User = {
  id: Text
  name: Text
}

effect DatabaseRead

fn get_user(id: Text) -> Result<User, Error> uses DatabaseRead {
  let row = db.query_one("select * from users where id = ?", id)?
  return Ok(User { id: row.text("id"), name: row.text("name") })
}
"#;
        let program = parse_program(source).expect("parse");
        assert_eq!(program.module.as_deref(), Some("app.profile"));
        assert_eq!(program.types.len(), 1);
        assert_eq!(program.effects.len(), 1);
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].params[0].name, "id");
        assert_eq!(program.functions[0].body.statements.len(), 2);
    }
}
