//! A small, error-recovering parser for the executable RoboGen MVP grammar.

use robogen_domain::{Diagnostic, SourceSpan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub declarations: Vec<Declaration>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Declaration {
    Parameter(ParameterDecl),
    Material(MaterialDecl),
    Sketch(SketchDecl),
    Part(PartDecl),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParameterDecl {
    pub name: String,
    pub value: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub value: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SketchDecl {
    pub name: String,
    pub plane: String,
    pub rectangles: Vec<RectangleDecl>,
    pub constraints: Vec<ConstraintDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RectangleDecl {
    pub name: String,
    pub origin_x: Expr,
    pub origin_y: Expr,
    pub width: Expr,
    pub height: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstraintDecl {
    pub name: String,
    pub args: Vec<Expr>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartDecl {
    pub name: String,
    pub material: Option<String>,
    pub features: Vec<FeatureDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureDecl {
    pub name: String,
    pub operation: String,
    pub args: Vec<Expr>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExprKind {
    Number { value: f64, unit: Option<String> },
    Reference(String),
    String(String),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ParseOutput {
    pub module: Option<Module>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Ident(String),
    Number(f64),
    String(String),
    Symbol(char),
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    span: SourceSpan,
}

pub fn parse(source: &str) -> ParseOutput {
    let (tokens, mut diagnostics) = lex(source);
    let mut parser = Parser {
        tokens,
        cursor: 0,
        diagnostics: Vec::new(),
    };
    let module = parser.module();
    diagnostics.extend(parser.diagnostics);
    ParseOutput {
        module,
        diagnostics,
    }
}

/// Converts a byte offset to a one-based `(line, column)` pair for editor diagnostics.
pub fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut safe = offset.min(source.len());
    while safe > 0 && !source.is_char_boundary(safe) {
        safe -= 1;
    }
    let prefix = &source[..safe];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.chars().count() + 1, |(_, tail)| {
            tail.chars().count() + 1
        });
    (line, column)
}

fn lex(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b if b.is_ascii_whitespace() => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' => {
                let start = i;
                i += 1;
                let content_start = i;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
                if i >= bytes.len() {
                    diagnostics.push(Diagnostic::error(
                        "E002",
                        "unterminated string",
                        SourceSpan::new(start, i),
                    ));
                } else {
                    let value = String::from_utf8_lossy(&bytes[content_start..i]).into_owned();
                    i += 1;
                    tokens.push(Token {
                        kind: TokenKind::String(value),
                        span: SourceSpan::new(start, i),
                    });
                }
            }
            b if b.is_ascii_digit()
                || (b == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)) =>
            {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                let value = source.get(start..i).and_then(|s| s.parse().ok());
                match value {
                    Some(value) => tokens.push(Token {
                        kind: TokenKind::Number(value),
                        span: SourceSpan::new(start, i),
                    }),
                    None => diagnostics.push(Diagnostic::error(
                        "E003",
                        "invalid number",
                        SourceSpan::new(start, i),
                    )),
                }
            }
            b if b.is_ascii_alphabetic() || b == b'_' => {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'/'))
                {
                    i += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Ident(source[start..i].to_owned()),
                    span: SourceSpan::new(start, i),
                });
            }
            b @ (b'{' | b'}' | b'(' | b')' | b':' | b';' | b'=' | b',' | b'.' | b'-') => {
                tokens.push(Token {
                    kind: TokenKind::Symbol(b as char),
                    span: SourceSpan::new(i, i + 1),
                });
                i += 1;
            }
            _ => {
                let end = source[i..]
                    .chars()
                    .next()
                    .map_or(source.len(), |character| i + character.len_utf8());
                diagnostics.push(Diagnostic::error(
                    "E001",
                    "unexpected character",
                    SourceSpan::new(i, end),
                ));
                i = end;
            }
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: SourceSpan::new(source.len(), source.len()),
    });
    (tokens, diagnostics)
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn module(&mut self) -> Option<Module> {
        let start = self.peek().span.start;
        if !self.take_keyword("module") {
            self.error_here("E100", "expected `module`");
            return None;
        }
        let name = self.take_ident("module name")?;
        self.expect_symbol(';');
        let mut declarations = Vec::new();
        while !self.at_eof() {
            let before = self.cursor;
            let declaration = if self.take_keyword("parameter") {
                self.parameter().map(Declaration::Parameter)
            } else if self.take_keyword("material") {
                self.material().map(Declaration::Material)
            } else if self.take_keyword("sketch") {
                self.sketch().map(Declaration::Sketch)
            } else if self.take_keyword("part") {
                self.part().map(Declaration::Part)
            } else {
                self.error_here("E101", "expected a declaration");
                self.recover_declaration();
                None
            };
            if let Some(declaration) = declaration {
                declarations.push(declaration);
            }
            if self.cursor == before {
                self.bump();
            }
        }
        Some(Module {
            name,
            declarations,
            span: SourceSpan::new(start, self.peek().span.end),
        })
    }

    fn parameter(&mut self) -> Option<ParameterDecl> {
        let start = self.previous_span().start;
        let name = self.take_ident("parameter name")?;
        self.expect_symbol('=');
        let value = self.expr()?;
        let end = self.expect_symbol(';').end;
        Some(ParameterDecl {
            name,
            value,
            span: SourceSpan::new(start, end),
        })
    }

    fn material(&mut self) -> Option<MaterialDecl> {
        let start = self.previous_span().start;
        let name = self.take_ident("material name")?;
        self.expect_symbol('{');
        let mut fields = Vec::new();
        while !self.at_symbol('}') && !self.at_eof() {
            let field_start = self.peek().span.start;
            let Some(field_name) = self.take_ident("material property") else {
                self.recover_statement();
                continue;
            };
            self.expect_symbol(':');
            let Some(value) = self.expr() else {
                self.recover_statement();
                continue;
            };
            let end = self.expect_symbol(';').end;
            fields.push(Field {
                name: field_name,
                value,
                span: SourceSpan::new(field_start, end),
            });
        }
        let end = self.expect_symbol('}').end;
        Some(MaterialDecl {
            name,
            fields,
            span: SourceSpan::new(start, end),
        })
    }

    fn sketch(&mut self) -> Option<SketchDecl> {
        let start = self.previous_span().start;
        let name = self.take_ident("sketch name")?;
        if !self.take_keyword("on") {
            self.error_here("E102", "expected `on`");
        }
        let plane = self
            .take_ident("sketch plane")
            .unwrap_or_else(|| "XY".into());
        self.expect_symbol('{');
        let mut rectangles = Vec::new();
        let mut constraints = Vec::new();
        while !self.at_symbol('}') && !self.at_eof() {
            if self.take_keyword("rectangle") {
                if let Some(rectangle) = self.rectangle() {
                    rectangles.push(rectangle);
                }
            } else if self.take_keyword("constrain") {
                if let Some(constraint) = self.constraint() {
                    constraints.push(constraint);
                }
            } else {
                self.error_here("E103", "expected `rectangle` or `constrain`");
                self.recover_statement();
            }
        }
        let end = self.expect_symbol('}').end;
        Some(SketchDecl {
            name,
            plane,
            rectangles,
            constraints,
            span: SourceSpan::new(start, end),
        })
    }

    fn rectangle(&mut self) -> Option<RectangleDecl> {
        let start = self.previous_span().start;
        let name = self.take_ident("rectangle name")?;
        self.expect_symbol('{');
        let mut origin = None;
        let mut width = None;
        let mut height = None;
        while !self.at_symbol('}') && !self.at_eof() {
            let Some(field) = self.take_ident("rectangle property") else {
                self.recover_statement();
                continue;
            };
            self.expect_symbol(':');
            match field.as_str() {
                "origin" => {
                    self.expect_symbol('(');
                    let x = self.expr();
                    self.expect_symbol(',');
                    let y = self.expr();
                    self.expect_symbol(')');
                    if let (Some(x), Some(y)) = (x, y) {
                        origin = Some((x, y));
                    }
                }
                "width" => width = self.expr(),
                "height" => height = self.expr(),
                _ => {
                    self.error_here("E104", "unknown rectangle property");
                    let _ = self.expr();
                }
            }
            self.expect_symbol(';');
        }
        let end = self.expect_symbol('}').end;
        let fallback = || Expr {
            kind: ExprKind::Number {
                value: 0.0,
                unit: Some("mm".into()),
            },
            span: SourceSpan::new(start, end),
        };
        if origin.is_none() {
            self.diagnostics.push(Diagnostic::error(
                "E105",
                "rectangle requires `origin`",
                SourceSpan::new(start, end),
            ));
        }
        if width.is_none() {
            self.diagnostics.push(Diagnostic::error(
                "E105",
                "rectangle requires `width`",
                SourceSpan::new(start, end),
            ));
        }
        if height.is_none() {
            self.diagnostics.push(Diagnostic::error(
                "E105",
                "rectangle requires `height`",
                SourceSpan::new(start, end),
            ));
        }
        let (origin_x, origin_y) = origin.unwrap_or_else(|| (fallback(), fallback()));
        Some(RectangleDecl {
            name,
            origin_x,
            origin_y,
            width: width.unwrap_or_else(fallback),
            height: height.unwrap_or_else(fallback),
            span: SourceSpan::new(start, end),
        })
    }

    fn constraint(&mut self) -> Option<ConstraintDecl> {
        let start = self.previous_span().start;
        let name = self.take_ident("constraint name")?;
        self.expect_symbol('(');
        let args = self.arg_list();
        self.expect_symbol(')');
        let end = self.expect_symbol(';').end;
        Some(ConstraintDecl {
            name,
            args,
            span: SourceSpan::new(start, end),
        })
    }

    fn part(&mut self) -> Option<PartDecl> {
        let start = self.previous_span().start;
        let name = self.take_ident("part name")?;
        self.expect_symbol('{');
        let mut material = None;
        let mut features = Vec::new();
        while !self.at_symbol('}') && !self.at_eof() {
            let stmt_start = self.peek().span.start;
            let Some(lhs) = self.take_ident("part property or feature") else {
                self.recover_statement();
                continue;
            };
            if lhs == "material" && self.take_symbol(':') {
                material = self.take_path("material reference");
                self.expect_symbol(';');
                continue;
            }
            self.expect_symbol('=');
            let Some(operation) = self.take_ident("feature operation") else {
                self.recover_statement();
                continue;
            };
            self.expect_symbol('(');
            let args = self.arg_list();
            self.expect_symbol(')');
            let end = self.expect_symbol(';').end;
            features.push(FeatureDecl {
                name: lhs,
                operation,
                args,
                span: SourceSpan::new(stmt_start, end),
            });
        }
        let end = self.expect_symbol('}').end;
        Some(PartDecl {
            name,
            material,
            features,
            span: SourceSpan::new(start, end),
        })
    }

    fn arg_list(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        while !self.at_symbol(')') && !self.at_eof() {
            if let Some(arg) = self.expr() {
                args.push(arg);
            } else {
                self.bump();
            }
            if !self.take_symbol(',') {
                break;
            }
        }
        args
    }

    fn expr(&mut self) -> Option<Expr> {
        let negative = self.take_symbol('-');
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Number(value) => {
                self.bump();
                let unit = match &self.peek().kind {
                    TokenKind::Ident(unit) if is_unit(unit) => {
                        let unit = unit.clone();
                        self.bump();
                        Some(unit)
                    }
                    _ => None,
                };
                Some(Expr {
                    kind: ExprKind::Number {
                        value: if negative { -value } else { value },
                        unit,
                    },
                    span: SourceSpan::new(
                        if negative {
                            self.previous_n_span(2).start
                        } else {
                            token.span.start
                        },
                        self.previous_span().end,
                    ),
                })
            }
            TokenKind::Ident(_) if !negative => {
                let value = self.take_path("reference")?;
                Some(Expr {
                    kind: ExprKind::Reference(value),
                    span: SourceSpan::new(token.span.start, self.previous_span().end),
                })
            }
            TokenKind::String(value) if !negative => {
                self.bump();
                Some(Expr {
                    kind: ExprKind::String(value),
                    span: token.span,
                })
            }
            _ => {
                self.error_here("E106", "expected expression");
                None
            }
        }
    }

    fn take_path(&mut self, expected: &str) -> Option<String> {
        let mut result = self.take_ident(expected)?;
        while self.take_symbol('.') {
            if let Some(segment) = self.take_ident("path segment") {
                result.push('.');
                result.push_str(&segment);
            } else {
                break;
            }
        }
        Some(result)
    }

    fn take_ident(&mut self, expected: &str) -> Option<String> {
        match self.peek().kind.clone() {
            TokenKind::Ident(value) => {
                self.bump();
                Some(value)
            }
            _ => {
                self.error_here("E107", format!("expected {expected}"));
                None
            }
        }
    }
    fn take_keyword(&mut self, word: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Ident(value) if value == word) && {
            self.bump();
            true
        }
    }
    fn at_symbol(&self, symbol: char) -> bool {
        matches!(self.peek().kind, TokenKind::Symbol(found) if found == symbol)
    }
    fn take_symbol(&mut self, symbol: char) -> bool {
        self.at_symbol(symbol) && {
            self.bump();
            true
        }
    }
    fn expect_symbol(&mut self, symbol: char) -> SourceSpan {
        if self.take_symbol(symbol) {
            self.previous_span()
        } else {
            let span = self.peek().span;
            self.error_here("E108", format!("expected `{symbol}`"));
            span
        }
    }
    fn at_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor.min(self.tokens.len().saturating_sub(1))]
    }
    fn bump(&mut self) {
        if self.cursor + 1 < self.tokens.len() {
            self.cursor += 1;
        }
    }
    fn previous_span(&self) -> SourceSpan {
        self.previous_n_span(1)
    }
    fn previous_n_span(&self, n: usize) -> SourceSpan {
        self.tokens
            .get(self.cursor.saturating_sub(n))
            .map_or(self.peek().span, |t| t.span)
    }
    fn error_here(&mut self, code: &str, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(code, message, self.peek().span));
    }
    fn recover_statement(&mut self) {
        while !self.at_eof() && !self.at_symbol(';') && !self.at_symbol('}') {
            self.bump();
        }
        if self.at_symbol(';') {
            self.bump();
        }
    }
    fn recover_declaration(&mut self) {
        let mut depth = 0usize;
        while !self.at_eof() {
            if self.at_symbol('{') {
                depth += 1;
            }
            if self.at_symbol('}') {
                if depth == 0 {
                    self.bump();
                    break;
                }
                depth -= 1;
            }
            if self.at_symbol(';') && depth == 0 {
                self.bump();
                break;
            }
            self.bump();
        }
    }
}

fn is_unit(value: &str) -> bool {
    matches!(
        value,
        "mm" | "cm" | "m" | "deg" | "rad" | "kg/m3" | "g/cm3" | "Pa" | "kPa" | "MPa" | "GPa"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
module demo;
parameter WIDTH = 40 mm;
material PLA { density: 1240 kg/m3; young: 3.5 GPa; poisson: 0.36; yield_strength: 50 MPa; }
sketch Profile on XY {
  rectangle outline { origin: (0 mm, 0 mm); width: WIDTH; height: 20 mm; }
  constrain horizontal(outline.bottom);
}
part Body { material: PLA; base = extrude(Profile, 4 mm); }
"#;

    #[test]
    fn parses_vertical_slice() {
        let output = parse(VALID);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        assert_eq!(
            output.module.map(|module| module.declarations.len()),
            Some(4)
        );
    }

    #[test]
    fn malformed_input_never_panics_and_has_spans() {
        let output = parse("module broken; sketch X on { rectangle { width: nope");
        assert!(!output.diagnostics.is_empty());
        assert!(output
            .diagnostics
            .iter()
            .all(|d| d.span.end >= d.span.start));
    }

    #[test]
    fn line_mapping_is_one_based() {
        assert_eq!(line_column("a\nbc", 3), (2, 2));
    }
}
