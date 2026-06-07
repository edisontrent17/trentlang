use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl CompileError {
    fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Int,
    Return,
    Print,
    Ident(String),
    Number(i32),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}

pub fn lex(source: &str) -> Result<Vec<Token>, CompileError> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut line = 1;
    let mut column = 1;

    while let Some(ch) = chars.peek().copied() {
        match ch {
            ' ' | '\t' | '\r' => {
                chars.next();
                column += 1;
            }
            '\n' => {
                chars.next();
                line += 1;
                column = 1;
            }
            '/' => {
                let start_column = column;
                chars.next();
                column += 1;
                if chars.peek() == Some(&'/') {
                    while let Some(next) = chars.peek().copied() {
                        if next == '\n' {
                            break;
                        }
                        chars.next();
                        column += 1;
                    }
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Slash,
                        line,
                        column: start_column,
                    });
                }
            }
            '0'..='9' => {
                let start_column = column;
                let mut text = String::new();
                while let Some(next) = chars.peek().copied() {
                    if !next.is_ascii_digit() {
                        break;
                    }
                    text.push(next);
                    chars.next();
                    column += 1;
                }
                let value = text.parse::<i64>().map_err(|_| {
                    CompileError::new("integer literal is too large", line, start_column)
                })?;
                if value > i32::MAX as i64 {
                    return Err(CompileError::new(
                        "integer literal must fit in a 32-bit signed int",
                        line,
                        start_column,
                    ));
                }
                tokens.push(Token {
                    kind: TokenKind::Number(value as i32),
                    line,
                    column: start_column,
                });
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start_column = column;
                let mut text = String::new();
                while let Some(next) = chars.peek().copied() {
                    if !(next.is_ascii_alphanumeric() || next == '_') {
                        break;
                    }
                    text.push(next);
                    chars.next();
                    column += 1;
                }
                let kind = match text.as_str() {
                    "int" => TokenKind::Int,
                    "return" => TokenKind::Return,
                    "print" => TokenKind::Print,
                    _ => TokenKind::Ident(text),
                };
                tokens.push(Token {
                    kind,
                    line,
                    column: start_column,
                });
            }
            '+' => push_single(&mut tokens, &mut chars, &mut column, TokenKind::Plus, line),
            '-' => push_single(&mut tokens, &mut chars, &mut column, TokenKind::Minus, line),
            '*' => push_single(&mut tokens, &mut chars, &mut column, TokenKind::Star, line),
            '(' => push_single(&mut tokens, &mut chars, &mut column, TokenKind::LParen, line),
            ')' => push_single(&mut tokens, &mut chars, &mut column, TokenKind::RParen, line),
            '{' => push_single(&mut tokens, &mut chars, &mut column, TokenKind::LBrace, line),
            '}' => push_single(&mut tokens, &mut chars, &mut column, TokenKind::RBrace, line),
            ';' => push_single(
                &mut tokens,
                &mut chars,
                &mut column,
                TokenKind::Semicolon,
                line,
            ),
            _ => {
                return Err(CompileError::new(
                    format!("unexpected character '{ch}'"),
                    line,
                    column,
                ));
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        line,
        column,
    });
    Ok(tokens)
}

fn push_single(
    tokens: &mut Vec<Token>,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    column: &mut usize,
    kind: TokenKind,
    line: usize,
) {
    let start_column = *column;
    chars.next();
    *column += 1;
    tokens.push(Token {
        kind,
        line,
        column: start_column,
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Print(Expr),
    Return(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Number(i32),
    UnaryMinus(Box<Expr>),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub fn parse(source: &str) -> Result<Program, CompileError> {
    Parser::new(lex(source)?).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_program(&mut self) -> Result<Program, CompileError> {
        self.expect_exact(TokenKind::Int, "expected 'int' before main")?;
        match self.peek().kind.clone() {
            TokenKind::Ident(name) if name == "main" => self.advance(),
            _ => {
                let token = self.peek();
                return Err(CompileError::new(
                    "expected entry point named 'main'",
                    token.line,
                    token.column,
                ));
            }
        }
        self.expect_exact(TokenKind::LParen, "expected '(' after main")?;
        self.expect_exact(TokenKind::RParen, "expected ')' after main(")?;
        self.expect_exact(TokenKind::LBrace, "expected '{' before main body")?;

        let mut statements = Vec::new();
        while !self.at_exact(&TokenKind::RBrace) {
            if self.at_exact(&TokenKind::Eof) {
                let token = self.peek();
                return Err(CompileError::new(
                    "expected '}' before end of file",
                    token.line,
                    token.column,
                ));
            }
            statements.push(self.parse_statement()?);
        }

        self.expect_exact(TokenKind::RBrace, "expected '}' after main body")?;
        self.expect_exact(TokenKind::Eof, "unexpected tokens after main function")?;
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt, CompileError> {
        if self.at_exact(&TokenKind::Print) {
            self.advance();
            self.expect_exact(TokenKind::LParen, "expected '(' after print")?;
            let expr = self.parse_expression()?;
            self.expect_exact(TokenKind::RParen, "expected ')' after print expression")?;
            self.expect_exact(TokenKind::Semicolon, "expected ';' after print statement")?;
            return Ok(Stmt::Print(expr));
        }

        if self.at_exact(&TokenKind::Return) {
            self.advance();
            let expr = self.parse_expression()?;
            self.expect_exact(TokenKind::Semicolon, "expected ';' after return statement")?;
            return Ok(Stmt::Return(expr));
        }

        let token = self.peek();
        Err(CompileError::new(
            "expected statement: print(expr); or return expr;",
            token.line,
            token.column,
        ))
    }

    fn parse_expression(&mut self) -> Result<Expr, CompileError> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_multiplicative()?;
        loop {
            let op = if self.at_exact(&TokenKind::Plus) {
                BinaryOp::Add
            } else if self.at_exact(&TokenKind::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.at_exact(&TokenKind::Star) {
                BinaryOp::Mul
            } else if self.at_exact(&TokenKind::Slash) {
                BinaryOp::Div
            } else {
                break;
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        if self.at_exact(&TokenKind::Minus) {
            self.advance();
            return Ok(Expr::UnaryMinus(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        match self.peek().kind.clone() {
            TokenKind::Number(value) => {
                self.advance();
                Ok(Expr::Number(value))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_exact(TokenKind::RParen, "expected ')' after expression")?;
                Ok(expr)
            }
            _ => {
                let token = self.peek();
                Err(CompileError::new(
                    "expected integer literal or parenthesized expression",
                    token.line,
                    token.column,
                ))
            }
        }
    }

    fn expect_exact(&mut self, expected: TokenKind, message: &str) -> Result<(), CompileError> {
        if self.at_exact(&expected) {
            self.advance();
            Ok(())
        } else {
            let token = self.peek();
            Err(CompileError::new(message, token.line, token.column))
        }
    }

    fn at_exact(&self, expected: &TokenKind) -> bool {
        &self.peek().kind == expected
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn advance(&mut self) {
        if self.index < self.tokens.len() - 1 {
            self.index += 1;
        }
    }
}

pub fn generate_llvm_ir(program: &Program) -> String {
    let mut codegen = Codegen::default();
    codegen.generate(program)
}

#[derive(Default)]
struct Codegen {
    next_temp: usize,
    lines: Vec<String>,
}

impl Codegen {
    fn generate(&mut self, program: &Program) -> String {
        self.lines
            .push("@.fmt_int = private unnamed_addr constant [4 x i8] c\"%d\\0A\\00\"".to_string());
        self.lines.push(String::new());
        self.lines.push("declare i32 @printf(ptr, ...)".to_string());
        self.lines.push(String::new());
        self.lines.push("define i32 @main() {".to_string());
        self.lines.push("entry:".to_string());

        let mut returned = false;
        for statement in &program.statements {
            if returned {
                break;
            }
            match statement {
                Stmt::Print(expr) => {
                    let value = self.emit_expr(expr);
                    let temp = self.temp();
                    self.lines.push(format!(
                        "  {temp} = call i32 (ptr, ...) @printf(ptr @.fmt_int, i32 {value})"
                    ));
                }
                Stmt::Return(expr) => {
                    let value = self.emit_expr(expr);
                    self.lines.push(format!("  ret i32 {value}"));
                    returned = true;
                }
            }
        }

        if !returned {
            self.lines.push("  ret i32 0".to_string());
        }
        self.lines.push("}".to_string());
        self.lines.push(String::new());
        self.lines.join("\n")
    }

    fn emit_expr(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Number(value) => value.to_string(),
            Expr::UnaryMinus(inner) => {
                let value = self.emit_expr(inner);
                let temp = self.temp();
                self.lines.push(format!("  {temp} = sub i32 0, {value}"));
                temp
            }
            Expr::Binary { op, left, right } => {
                let left = self.emit_expr(left);
                let right = self.emit_expr(right);
                let instruction = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Sub => "sub",
                    BinaryOp::Mul => "mul",
                    BinaryOp::Div => "sdiv",
                };
                let temp = self.temp();
                self.lines
                    .push(format!("  {temp} = {instruction} i32 {left}, {right}"));
                temp
            }
        }
    }

    fn temp(&mut self) -> String {
        let temp = format!("%t{}", self.next_temp);
        self.next_temp += 1;
        temp
    }
}

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub emit_ir_only: bool,
    pub output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutput {
    pub ir_path: PathBuf,
    pub executable_path: Option<PathBuf>,
}

pub fn compile_file(
    input_path: impl AsRef<Path>,
    options: &CompileOptions,
) -> Result<CompileOutput, CompileError> {
    let input_path = input_path.as_ref();
    let source = fs::read_to_string(input_path).map_err(|err| {
        CompileError::new(
            format!("failed to read {}: {err}", input_path.display()),
            1,
            1,
        )
    })?;
    let program = parse(&source)?;
    let ir = generate_llvm_ir(&program);
    let ir_path = input_path.with_extension("ll");
    fs::write(&ir_path, ir).map_err(|err| {
        CompileError::new(
            format!("failed to write {}: {err}", ir_path.display()),
            1,
            1,
        )
    })?;

    if options.emit_ir_only {
        return Ok(CompileOutput {
            ir_path,
            executable_path: None,
        });
    }

    let executable_path = options
        .output_path
        .clone()
        .unwrap_or_else(|| default_executable_path(input_path));
    let output = Command::new("clang")
        .arg(&ir_path)
        .arg("-o")
        .arg(&executable_path)
        .output()
        .map_err(|err| CompileError::new(format!("failed to run clang: {err}"), 1, 1))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CompileError::new(
            format!("clang failed while building native executable:\n{stderr}"),
            1,
            1,
        ));
    }

    Ok(CompileOutput {
        ir_path,
        executable_path: Some(executable_path),
    })
}

fn default_executable_path(input_path: &Path) -> PathBuf {
    let stem = input_path.file_stem().unwrap_or_default();
    match input_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(stem),
        _ => PathBuf::from(stem),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_tokenizes_basic_program() {
        let tokens = lex("int main() { print(7); return 0; }").unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Int,
                TokenKind::Ident("main".to_string()),
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::Print,
                TokenKind::LParen,
                TokenKind::Number(7),
                TokenKind::RParen,
                TokenKind::Semicolon,
                TokenKind::Return,
                TokenKind::Number(0),
                TokenKind::Semicolon,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn parser_respects_arithmetic_precedence() {
        let program = parse("int main() { return 1 + 2 * 3; }").unwrap();
        assert_eq!(
            program,
            Program {
                statements: vec![Stmt::Return(Expr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(Expr::Number(1)),
                    right: Box::new(Expr::Binary {
                        op: BinaryOp::Mul,
                        left: Box::new(Expr::Number(2)),
                        right: Box::new(Expr::Number(3)),
                    }),
                })],
            }
        );
    }

    #[test]
    fn parser_rejects_missing_semicolon() {
        let err = parse("int main() { print(7) return 0; }").unwrap_err();
        assert!(err.message.contains("expected ';'"));
    }

    #[test]
    fn parser_rejects_non_main_entry_point() {
        let err = parse("int not_main() { return 0; }").unwrap_err();
        assert!(err.message.contains("main"));
    }

    #[test]
    fn codegen_emits_llvm_for_print_and_return() {
        let program = parse("int main() { print(1 + 2 * 3); return 5; }").unwrap();
        let ir = generate_llvm_ir(&program);
        assert!(ir.contains("declare i32 @printf(ptr, ...)"));
        assert!(ir.contains("mul i32 2, 3"));
        assert!(ir.contains("add i32 1, %t0"));
        assert!(ir.contains("call i32 (ptr, ...) @printf"));
        assert!(ir.contains("ret i32 5"));
    }
}
