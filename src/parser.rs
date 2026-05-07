use crate::ast::{
    BinaryOp, BindingKind, Block, EffectDecl, Expr, ExprField, ExprKind, ExprMapEntry, ImportDecl,
    Program, RecordField, Statement, StatementKind, TakeDecl, TaskDecl, TypeDecl, TypeExpr,
    UnaryOp,
};
use crate::diagnostics::{Diagnostic, RepairHint, SourceSpan};

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
    let synthetic = format!("task __graft_block -> Unit {{\n{source}\n}}\n");
    let program = parse_program(&synthetic)?;
    Ok(program
        .tasks
        .into_iter()
        .next()
        .map(|task| task.body)
        .unwrap_or(Block {
            statements: Vec::new(),
        }))
}

pub fn parse_expr_source(source: &str) -> Result<Expr, Vec<Diagnostic>> {
    let synthetic = format!("task __graft_expr -> Unit {{\nreturn {source}\n}}\n");
    let program = parse_program(&synthetic)?;
    let Some(task) = program.tasks.into_iter().next() else {
        return Err(vec![
            Diagnostic::error("PARSE_EXPECTED_EXPRESSION", "expected expression")
                .with_repair_hint(expected_expression_hint()),
        ]);
    };
    let Some(statement) = task.body.statements.into_iter().next() else {
        return Err(vec![
            Diagnostic::error("PARSE_EXPECTED_EXPRESSION", "expected expression")
                .with_repair_hint(expected_expression_hint()),
        ]);
    };
    match statement.kind {
        StatementKind::Return { expr } => Ok(expr),
        _ => Err(vec![
            Diagnostic::error("PARSE_EXPECTED_EXPRESSION", "expected expression")
                .with_repair_hint(expected_expression_hint()),
        ]),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Ident(String),
    String(String),
    Number(String),
    Symbol(char),
    Operator(String),
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
            '=' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(Token {
                        kind: TokenKind::Operator("==".to_string()),
                        text: "==".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Symbol('='),
                        text: "=".to_string(),
                        span,
                    });
                }
            }
            '!' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(Token {
                        kind: TokenKind::Operator("!=".to_string()),
                        text: "!=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Symbol('!'),
                        text: "!".to_string(),
                        span,
                    });
                }
            }
            '<' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(Token {
                        kind: TokenKind::Operator("<=".to_string()),
                        text: "<=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Symbol('<'),
                        text: "<".to_string(),
                        span,
                    });
                }
            }
            '>' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'=') {
                    chars.next();
                    column += 1;
                    tokens.push(Token {
                        kind: TokenKind::Operator(">=".to_string()),
                        text: ">=".to_string(),
                        span,
                    });
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Symbol('>'),
                        text: ">".to_string(),
                        span,
                    });
                }
            }
            '&' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'&') {
                    chars.next();
                    column += 1;
                    tokens.push(Token {
                        kind: TokenKind::Operator("&&".to_string()),
                        text: "&&".to_string(),
                        span,
                    });
                } else {
                    diagnostics.push(
                        Diagnostic::error(
                            "LEX_UNKNOWN_CHARACTER",
                            "`&` is only valid as part of `&&`",
                        )
                        .with_span(span),
                    );
                }
            }
            '|' => {
                chars.next();
                column += 1;
                if chars.peek() == Some(&'|') {
                    chars.next();
                    column += 1;
                    tokens.push(Token {
                        kind: TokenKind::Operator("||".to_string()),
                        text: "||".to_string(),
                        span,
                    });
                } else {
                    diagnostics.push(
                        Diagnostic::error(
                            "LEX_UNKNOWN_CHARACTER",
                            "`|` is only valid as part of `||`",
                        )
                        .with_span(span),
                    );
                }
            }
            '{' | '}' | '(' | ')' | ':' | ',' | '?' | '.' | ';' | '+' | '*' | '%' | '[' | ']' => {
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
                let module = self.parse_module_path()?;
                let alias = if self.current().is_ident("as") {
                    self.bump();
                    Some(self.expect_ident("expected import alias")?)
                } else {
                    None
                };
                program.imports.push(ImportDecl {
                    id: String::new(),
                    owner_module: None,
                    module,
                    alias,
                    span: Some(span),
                });
            } else if self.current().is_ident("type") {
                program.types.push(self.parse_type_decl(false)?);
            } else if self.current().is_ident("effect") {
                program.effects.push(self.parse_effect_decl(false)?);
            } else if self.current().is_ident("task") {
                program.tasks.push(self.parse_task_decl(false)?);
            } else if self.current().is_ident("export") {
                let span = self.current().span.clone();
                self.bump();
                if self.current().is_ident("type") {
                    program.types.push(self.parse_type_decl(true)?);
                } else if self.current().is_ident("effect") {
                    program.effects.push(self.parse_effect_decl(true)?);
                } else if self.current().is_ident("task") {
                    program.tasks.push(self.parse_task_decl(true)?);
                } else {
                    return Err(vec![
                        Diagnostic::error(
                            "PARSE_EXPECTED_EXPORT_ITEM",
                            "expected type, effect, or task after export",
                        )
                        .with_span(span)
                        .with_repair_hint(
                            RepairHint::new("choose_expected_item")
                                .with_replacement("type | effect | task"),
                        ),
                    ]);
                }
            } else if self.current().is_symbol(';') {
                self.bump();
            } else {
                return Err(vec![
                    self.error_here(
                        "PARSE_EXPECTED_ITEM",
                        "expected module, import, export, type, effect, or task declaration",
                    )
                    .with_repair_hint(
                        RepairHint::new("choose_expected_item")
                            .with_replacement("module | import | export | type | effect | task"),
                    ),
                ]);
            }
            self.skip_statement_gap();
        }

        Ok(program)
    }

    fn parse_type_decl(&mut self, exported: bool) -> Result<TypeDecl, Vec<Diagnostic>> {
        let span = self.current().span.clone();
        self.expect_keyword("type", "expected type declaration")?;
        let name = self.expect_ident("expected type name")?;
        self.expect_symbol('=', "expected `=` after type name")?;
        let value = self.parse_type_expr()?;
        Ok(TypeDecl {
            id: String::new(),
            module: None,
            exported,
            name,
            value,
            span: Some(span),
        })
    }

    fn parse_effect_decl(&mut self, exported: bool) -> Result<EffectDecl, Vec<Diagnostic>> {
        let span = self.current().span.clone();
        self.expect_keyword("effect", "expected effect declaration")?;
        let name = self.expect_ident("expected effect name")?;
        Ok(EffectDecl {
            id: String::new(),
            module: None,
            exported,
            name,
            span: Some(span),
        })
    }

    fn parse_task_decl(&mut self, exported: bool) -> Result<TaskDecl, Vec<Diagnostic>> {
        let span = self.current().span.clone();
        self.expect_keyword("task", "expected task declaration")?;
        let name = self.expect_ident("expected task name")?;
        let mut takes = Vec::new();
        self.expect_arrow("expected `->` after task name")?;
        let return_type = self.parse_type_expr()?;

        let mut effects = Vec::new();
        if self.current().is_ident("uses") {
            self.bump();
            loop {
                effects.push(self.parse_module_path()?);
                if self.current().is_symbol(',') {
                    self.bump();
                } else {
                    break;
                }
            }
        }

        let body = self.parse_task_block(&mut takes)?;
        Ok(TaskDecl {
            id: String::new(),
            module: None,
            exported,
            name,
            takes,
            return_type,
            effects,
            body,
            span: Some(span),
        })
    }

    fn parse_take(&mut self, binding_kind: BindingKind) -> Result<TakeDecl, Vec<Diagnostic>> {
        let take_span = self.current().span.clone();
        let take_name = self.expect_ident("expected take name")?;
        self.expect_symbol(':', "expected `:` after take name")?;
        let ty = self.parse_type_expr()?;
        Ok(TakeDecl {
            id: String::new(),
            name: take_name,
            binding_kind,
            ty,
            span: Some(take_span),
        })
    }

    fn parse_task_block(&mut self, takes: &mut Vec<TakeDecl>) -> Result<Block, Vec<Diagnostic>> {
        self.expect_symbol('{', "expected `{` to start task block")?;
        let mut statements = Vec::new();
        self.skip_newlines();
        while self.current().is_ident("take") {
            self.bump();
            let binding_kind = if let TokenKind::Ident(value) = &self.current().kind {
                if value == "gate" || value == "veil" || value == "taint" || value == "view" {
                    let binding_kind = BindingKind::from_source_keyword(value)
                        .expect("take binding qualifier is known");
                    self.bump();
                    binding_kind
                } else {
                    BindingKind::Take
                }
            } else {
                BindingKind::Take
            };
            takes.push(self.parse_take(binding_kind)?);
            self.skip_statement_gap();
        }
        while !self.at_eof() && !self.current().is_symbol('}') {
            statements.push(self.parse_statement()?);
            self.skip_statement_gap();
        }
        self.expect_symbol('}', "expected `}` to close task block")?;
        Ok(Block { statements })
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
        if let Some(binding_kind) = self.current_binding_kind() {
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
                kind: StatementKind::Binding {
                    binding_kind,
                    name,
                    type_ann,
                    expr,
                },
                span: Some(span),
            })
        } else if self.current().is_ident("set") {
            self.bump();
            let name = self.expect_ident("expected local binding name")?;
            self.expect_symbol('=', "expected `=` in set statement")?;
            let expr = self.parse_statement_expr(span.clone())?;
            Ok(Statement {
                id: String::new(),
                kind: StatementKind::Set { name, expr },
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
        } else if self.current().is_ident("if") {
            self.bump();
            let condition = self.parse_expr_before_block(span.clone())?;
            let then_block = self.parse_block()?;
            self.skip_newlines();
            let else_block = if self.current().is_ident("else") {
                self.bump();
                Some(self.parse_block()?)
            } else {
                None
            };
            Ok(Statement {
                id: String::new(),
                kind: StatementKind::If {
                    condition,
                    then_block,
                    else_block,
                },
                span: Some(span),
            })
        } else if self.current().is_ident("while") {
            self.bump();
            let condition = self.parse_expr_before_block(span.clone())?;
            let body = self.parse_block()?;
            Ok(Statement {
                id: String::new(),
                kind: StatementKind::While { condition, body },
                span: Some(span),
            })
        } else if self.current().is_ident("for") || self.current().is_ident("each") {
            self.bump();
            let item = self.expect_ident("expected loop binding name")?;
            self.expect_keyword("in", "expected `in` after loop binding")?;
            let collection = self.parse_expr_before_block(span.clone())?;
            let body = self.parse_block()?;
            Ok(Statement {
                id: String::new(),
                kind: StatementKind::For {
                    item,
                    collection,
                    body,
                },
                span: Some(span),
            })
        } else if self.current().is_ident("forge") {
            self.bump();
            let body = self.parse_block()?;
            Ok(Statement {
                id: String::new(),
                kind: StatementKind::Forge { body },
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

    fn current_binding_kind(&self) -> Option<BindingKind> {
        match &self.current().kind {
            TokenKind::Ident(value) => BindingKind::from_source_keyword(value),
            _ => None,
        }
    }

    fn parse_expr_before_block(&mut self, span: SourceSpan) -> Result<Expr, Vec<Diagnostic>> {
        let mut tokens = Vec::new();
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;

        while !self.at_eof() {
            let current = self.current().clone();
            match current.kind {
                TokenKind::Symbol('{') if paren_depth == 0 && bracket_depth == 0 => break,
                TokenKind::Newline if paren_depth == 0 && bracket_depth == 0 => {
                    return Err(vec![
                        Diagnostic::error(
                            "PARSE_EXPECTED_BLOCK",
                            "expected `{` after control-flow condition",
                        )
                        .with_span(current.span)
                        .with_repair_hint(expected_token_hint("{")),
                    ]);
                }
                TokenKind::Symbol('(') => paren_depth += 1,
                TokenKind::Symbol(')') => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::Symbol('[') => bracket_depth += 1,
                TokenKind::Symbol(']') => bracket_depth = bracket_depth.saturating_sub(1),
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
                    .with_span(span)
                    .with_repair_hint(expected_expression_hint()),
            ]);
        }

        let source = tokens_to_source(&tokens);
        Ok(parse_expr_tokens(&tokens, source, span))
    }

    fn parse_statement_expr(&mut self, span: SourceSpan) -> Result<Expr, Vec<Diagnostic>> {
        let mut tokens = Vec::new();
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;

        while !self.at_eof() {
            let current = self.current().clone();
            match current.kind {
                TokenKind::Newline
                    if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 =>
                {
                    break;
                }
                TokenKind::Symbol('}')
                    if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 =>
                {
                    break;
                }
                TokenKind::Symbol('(') => paren_depth += 1,
                TokenKind::Symbol(')') => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::Symbol('{') => brace_depth += 1,
                TokenKind::Symbol('}') => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::Symbol('[') => bracket_depth += 1,
                TokenKind::Symbol(']') => bracket_depth = bracket_depth.saturating_sub(1),
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
                    .with_span(span)
                    .with_repair_hint(expected_expression_hint()),
            ]);
        }

        let source = tokens_to_source(&tokens);
        Ok(parse_expr_tokens(&tokens, source, span))
    }

    fn parse_type_expr(&mut self) -> Result<TypeExpr, Vec<Diagnostic>> {
        self.skip_newlines();
        if self.current().is_symbol('{') {
            self.bump();
            let mut fields = Vec::new();
            self.skip_newlines();
            while !self.at_eof() && !self.current().is_symbol('}') {
                if self.current().is_ident("slot") {
                    self.bump();
                }
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

        let name = self.parse_module_path()?;
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
            Err(vec![self.expected_token_error(
                "PARSE_EXPECTED_KEYWORD",
                message,
                expected,
            )])
        }
    }

    fn expect_ident(&mut self, message: &str) -> Result<String, Vec<Diagnostic>> {
        match &self.current().kind {
            TokenKind::Ident(value) => {
                let value = value.clone();
                self.bump();
                Ok(value)
            }
            _ => Err(vec![
                self.error_here("PARSE_EXPECTED_IDENTIFIER", message)
                    .with_repair_hint(
                        RepairHint::new("provide_identifier").with_replacement("identifier"),
                    ),
            ]),
        }
    }

    fn expect_symbol(&mut self, expected: char, message: &str) -> Result<(), Vec<Diagnostic>> {
        if self.current().is_symbol(expected) {
            self.bump();
            Ok(())
        } else {
            Err(vec![self.expected_token_error(
                "PARSE_EXPECTED_SYMBOL",
                message,
                expected.to_string(),
            )])
        }
    }

    fn expect_arrow(&mut self, message: &str) -> Result<(), Vec<Diagnostic>> {
        if matches!(self.current().kind, TokenKind::Arrow) {
            self.bump();
            Ok(())
        } else {
            Err(vec![self.expected_token_error(
                "PARSE_EXPECTED_ARROW",
                message,
                "->",
            )])
        }
    }

    fn error_here(&self, id: &str, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error(id, message).with_span(self.current().span.clone())
    }

    fn expected_token_error(
        &self,
        id: &str,
        message: impl Into<String>,
        expected: impl Into<String>,
    ) -> Diagnostic {
        self.error_here(id, message)
            .with_repair_hint(expected_token_hint(expected))
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

fn parse_expr_tokens(tokens: &[Token], source: String, span: SourceSpan) -> Expr {
    let mut parser = ExprParser::new(tokens);
    match parser.parse_expression() {
        Ok(expr) if parser.at_end() => expr,
        _ => classify_expr(source, span),
    }
}

struct ExprParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    allow_record_literals: bool,
}

impl<'a> ExprParser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            allow_record_literals: true,
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, ()> {
        self.parse_binary(1)
    }

    fn parse_expression_before_block(&mut self) -> Result<Expr, ()> {
        let previous = self.allow_record_literals;
        self.allow_record_literals = false;
        let result = self.parse_expression();
        self.allow_record_literals = previous;
        result
    }

    fn parse_binary(&mut self, min_precedence: u8) -> Result<Expr, ()> {
        let mut left = self.parse_unary()?;

        while let Some((op, precedence)) = self.current_binary_op() {
            if precedence < min_precedence {
                break;
            }
            self.bump();
            let right = self.parse_binary(precedence + 1)?;
            let source = format!("{} {} {}", left.source, op.as_str(), right.source);
            left = Expr {
                id: String::new(),
                source,
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span: None,
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ()> {
        if let Some(op) = self.current_unary_op() {
            self.bump();
            let expr = self.parse_unary()?;
            let source = format!("{}{}", op.as_str(), expr.source);
            return Ok(Expr {
                id: String::new(),
                source,
                kind: ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span: None,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ()> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.current_is_symbol('.') {
                self.bump();
                let field = self.expect_ident()?;
                let source = format!("{}.{}", expr.source, field);
                expr = Expr {
                    id: String::new(),
                    source,
                    kind: ExprKind::FieldAccess {
                        receiver: Box::new(expr),
                        field,
                    },
                    span: None,
                };
            } else if self.current_is_symbol('(') {
                self.bump();
                let mut args = Vec::new();
                if !self.current_is_symbol(')') {
                    loop {
                        args.push(self.parse_expression()?);
                        if self.current_is_symbol(',') {
                            self.bump();
                            if self.current_is_symbol(')') {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                self.expect_symbol(')')?;
                let args_source = args
                    .iter()
                    .map(|arg| arg.source.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let source = format!("{}({args_source})", expr.source);
                expr = Expr {
                    id: String::new(),
                    source,
                    kind: ExprKind::Call {
                        callee: Box::new(expr),
                        args,
                    },
                    span: None,
                };
            } else if self.current_is_symbol('[') {
                self.bump();
                let index = self.parse_expression()?;
                self.expect_symbol(']')?;
                let source = format!("{}[{}]", expr.source, index.source);
                expr = Expr {
                    id: String::new(),
                    source,
                    kind: ExprKind::Index {
                        collection: Box::new(expr),
                        index: Box::new(index),
                    },
                    span: None,
                };
            } else if self.allow_record_literals && self.current_is_symbol('{') {
                let type_name = match &expr.kind {
                    ExprKind::Identifier { .. } | ExprKind::FieldAccess { .. } => {
                        Some(expr.source.clone())
                    }
                    _ => return Err(()),
                };
                let fields = self.parse_record_fields()?;
                let fields_source = format_expr_fields(&fields);
                let source = format!("{} {{ {fields_source} }}", expr.source);
                expr = Expr {
                    id: String::new(),
                    source,
                    kind: ExprKind::RecordLiteral { type_name, fields },
                    span: None,
                };
            } else if self.current_is_symbol('?') {
                self.bump();
                let source = format!("{}?", expr.source);
                expr = Expr {
                    id: String::new(),
                    source,
                    kind: ExprKind::Try {
                        expr: Box::new(expr),
                    },
                    span: None,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ()> {
        let token = self.current().ok_or(())?.clone();
        match token.kind {
            TokenKind::String(value) => {
                self.bump();
                Ok(Expr {
                    id: String::new(),
                    source: token.text,
                    kind: ExprKind::StringLiteral { value },
                    span: Some(token.span),
                })
            }
            TokenKind::Number(text) => {
                self.bump();
                if text.contains('.') {
                    let value = text.parse::<f64>().map_err(|_| ())?;
                    Ok(Expr {
                        id: String::new(),
                        source: text,
                        kind: ExprKind::FloatLiteral { value },
                        span: Some(token.span),
                    })
                } else {
                    let value = text.parse::<i64>().map_err(|_| ())?;
                    Ok(Expr {
                        id: String::new(),
                        source: text,
                        kind: ExprKind::IntLiteral { value },
                        span: Some(token.span),
                    })
                }
            }
            TokenKind::Ident(name) if name == "if" => self.parse_if_expression(token.span),
            TokenKind::Ident(name) if name == "call" => self.parse_call_keyword(token.span),
            TokenKind::Ident(name) if name == "map" && self.next_is_symbol('{') => {
                self.parse_map_literal(token.span)
            }
            TokenKind::Ident(name) if name == "true" || name == "false" => {
                self.bump();
                Ok(Expr {
                    id: String::new(),
                    source: name.clone(),
                    kind: ExprKind::BoolLiteral {
                        value: name == "true",
                    },
                    span: Some(token.span),
                })
            }
            TokenKind::Ident(name) => {
                self.bump();
                Ok(Expr {
                    id: String::new(),
                    source: name.clone(),
                    kind: ExprKind::Identifier { name },
                    span: Some(token.span),
                })
            }
            TokenKind::Symbol('(') => {
                self.bump();
                let mut expr = self.parse_expression()?;
                self.expect_symbol(')')?;
                expr.source = format!("({})", expr.source);
                Ok(expr)
            }
            TokenKind::Symbol('[') => self.parse_list_literal(token.span),
            TokenKind::Symbol('{') => {
                let fields = self.parse_record_fields()?;
                let fields_source = format_expr_fields(&fields);
                Ok(Expr {
                    id: String::new(),
                    source: format!("{{ {fields_source} }}"),
                    kind: ExprKind::RecordLiteral {
                        type_name: None,
                        fields,
                    },
                    span: Some(token.span),
                })
            }
            _ => Err(()),
        }
    }

    fn parse_call_keyword(&mut self, span: SourceSpan) -> Result<Expr, ()> {
        self.expect_keyword("call")?;
        let mut expr = self.parse_postfix()?;
        match &expr.kind {
            ExprKind::Call { .. } | ExprKind::Try { .. } => {
                expr.source = format!("call {}", expr.source);
                expr.span = Some(span);
                Ok(expr)
            }
            _ => Err(()),
        }
    }

    fn parse_map_literal(&mut self, span: SourceSpan) -> Result<Expr, ()> {
        self.expect_keyword("map")?;
        self.expect_symbol('{')?;
        let mut entries = Vec::new();
        if !self.current_is_symbol('}') {
            loop {
                let key = self.parse_expression()?;
                self.expect_symbol(':')?;
                let value = self.parse_expression()?;
                entries.push(ExprMapEntry { key, value });
                if self.current_is_symbol(',') {
                    self.bump();
                    if self.current_is_symbol('}') {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect_symbol('}')?;
        let source = format!("map {{ {} }}", format_map_entries(&entries));
        Ok(Expr {
            id: String::new(),
            source,
            kind: ExprKind::MapLiteral { entries },
            span: Some(span),
        })
    }

    fn parse_list_literal(&mut self, span: SourceSpan) -> Result<Expr, ()> {
        self.expect_symbol('[')?;
        let mut items = Vec::new();
        if !self.current_is_symbol(']') {
            loop {
                items.push(self.parse_expression()?);
                if self.current_is_symbol(',') {
                    self.bump();
                    if self.current_is_symbol(']') {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect_symbol(']')?;
        let source = format!(
            "[{}]",
            items
                .iter()
                .map(|item| item.source.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(Expr {
            id: String::new(),
            source,
            kind: ExprKind::ListLiteral { items },
            span: Some(span),
        })
    }

    fn parse_if_expression(&mut self, span: SourceSpan) -> Result<Expr, ()> {
        self.expect_keyword("if")?;
        let condition = self.parse_expression_before_block()?;
        self.expect_symbol('{')?;
        let then_branch = self.parse_expression()?;
        self.expect_symbol('}')?;
        self.expect_keyword("else")?;
        self.expect_symbol('{')?;
        let else_branch = self.parse_expression()?;
        self.expect_symbol('}')?;

        let source = format!(
            "if {} {{ {} }} else {{ {} }}",
            condition.source, then_branch.source, else_branch.source
        );
        Ok(Expr {
            id: String::new(),
            source,
            kind: ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
            span: Some(span),
        })
    }

    fn parse_record_fields(&mut self) -> Result<Vec<ExprField>, ()> {
        self.expect_symbol('{')?;
        let mut fields = Vec::new();
        if !self.current_is_symbol('}') {
            loop {
                let name = self.expect_ident()?;
                self.expect_symbol(':')?;
                let expr = self.parse_expression()?;
                fields.push(ExprField { name, expr });
                if self.current_is_symbol(',') {
                    self.bump();
                    if self.current_is_symbol('}') {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        self.expect_symbol('}')?;
        Ok(fields)
    }

    fn expect_keyword(&mut self, expected: &str) -> Result<(), ()> {
        match self.current().map(|token| &token.kind) {
            Some(TokenKind::Ident(value)) if value == expected => {
                self.bump();
                Ok(())
            }
            _ => Err(()),
        }
    }

    fn expect_ident(&mut self) -> Result<String, ()> {
        match self.current().map(|token| &token.kind) {
            Some(TokenKind::Ident(value)) => {
                let value = value.clone();
                self.bump();
                Ok(value)
            }
            _ => Err(()),
        }
    }

    fn expect_symbol(&mut self, expected: char) -> Result<(), ()> {
        if self.current_is_symbol(expected) {
            self.bump();
            Ok(())
        } else {
            Err(())
        }
    }

    fn current_is_symbol(&self, expected: char) -> bool {
        matches!(self.current().map(|token| &token.kind), Some(TokenKind::Symbol(value)) if *value == expected)
    }

    fn next_is_symbol(&self, expected: char) -> bool {
        matches!(self.tokens.get(self.pos + 1).map(|token| &token.kind), Some(TokenKind::Symbol(value)) if *value == expected)
    }

    fn current_unary_op(&self) -> Option<UnaryOp> {
        match self.current().map(|token| &token.kind) {
            Some(TokenKind::Symbol('!')) => Some(UnaryOp::Not),
            Some(TokenKind::Symbol('-')) => Some(UnaryOp::Negate),
            _ => None,
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.current().map(|token| &token.kind) {
            Some(TokenKind::Operator(value)) if value == "||" => Some((BinaryOp::Or, 1)),
            Some(TokenKind::Operator(value)) if value == "&&" => Some((BinaryOp::And, 2)),
            Some(TokenKind::Operator(value)) if value == "==" => Some((BinaryOp::Equal, 3)),
            Some(TokenKind::Operator(value)) if value == "!=" => Some((BinaryOp::NotEqual, 3)),
            Some(TokenKind::Symbol('<')) => Some((BinaryOp::Less, 4)),
            Some(TokenKind::Operator(value)) if value == "<=" => Some((BinaryOp::LessEqual, 4)),
            Some(TokenKind::Symbol('>')) => Some((BinaryOp::Greater, 4)),
            Some(TokenKind::Operator(value)) if value == ">=" => Some((BinaryOp::GreaterEqual, 4)),
            Some(TokenKind::Symbol('+')) => Some((BinaryOp::Add, 5)),
            Some(TokenKind::Symbol('-')) => Some((BinaryOp::Subtract, 5)),
            Some(TokenKind::Symbol('*')) => Some((BinaryOp::Multiply, 6)),
            Some(TokenKind::Symbol('/')) => Some((BinaryOp::Divide, 6)),
            Some(TokenKind::Symbol('%')) => Some((BinaryOp::Remainder, 6)),
            _ => None,
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn current(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) {
        self.pos += 1;
    }
}

fn format_expr_fields(fields: &[ExprField]) -> String {
    fields
        .iter()
        .map(|field| format!("{}: {}", field.name, field.expr.source))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_map_entries(entries: &[ExprMapEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}: {}", entry.key.source, entry.value.source))
        .collect::<Vec<_>>()
        .join(", ")
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
        if out.is_empty()
            || needs_no_space_before(current)
            || previous.is_some_and(needs_no_space_after)
        {
            out.push_str(current);
        } else {
            out.push(' ');
            out.push_str(current);
        }
        previous = Some(current);
    }
    out
}

fn expected_token_hint(expected: impl Into<String>) -> RepairHint {
    RepairHint::new("insert_expected_token").with_replacement(expected.into())
}

fn expected_expression_hint() -> RepairHint {
    RepairHint::new("provide_expression").with_replacement("0")
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
  slot id: Text
  slot name: Text
}

effect DatabaseRead

task get_user -> Result<User, Error> uses DatabaseRead {
  take id: Text

  bind row = call db.query_one("select * from users where id = ?", id)?
  return Ok(User { id: row.text("id"), name: row.text("name") })
}
"#;
        let program = parse_program(source).expect("parse");
        assert_eq!(program.module.as_deref(), Some("app.profile"));
        assert_eq!(program.types.len(), 1);
        assert_eq!(program.effects.len(), 1);
        assert_eq!(program.tasks.len(), 1);
        assert_eq!(program.tasks[0].takes[0].name, "id");
        assert_eq!(program.tasks[0].body.statements.len(), 2);
    }
}
