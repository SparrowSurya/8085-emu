//! The parser: turns a token stream into a [`Program`] AST.
//!
//! Grammar highlights: all `%define`s come first and must precede any `segment`; each
//! statement occupies its own line; labels sit alone on a line ending in `:`; data/`bss`
//! values and directive expressions (`%repeat`, `%len`) are kept unresolved for a later
//! stage. Reserved keywords may not be used as label/variable/define names.

use super::ast::*;
use super::error::{AsmError, AsmErrorKind, Span};
use super::keyword;
use super::token::{Token, TokenKind};

/// Parse a token stream (as produced by the lexer) into a [`Program`].
pub fn parse(tokens: Vec<Token>) -> Result<Program, AsmError> {
    parse_with_base_dir(tokens, None)
}

/// Parse a token stream with base directory context for `%include` define resolution.
pub fn parse_with_base_dir(
    tokens: Vec<Token>,
    base_dir: Option<&std::path::Path>,
) -> Result<Program, AsmError> {
    let filtered_tokens = super::preprocessor::preprocess(tokens, base_dir)?;
    Parser {
        toks: filtered_tokens,
        pos: 0,
        seen_globals: std::collections::HashSet::new(),
        seen_externs: std::collections::HashSet::new(),
        known_defines: std::collections::HashMap::new(),
    }
    .program()
}

/// Parse a single line's tokens into a [`TextItem`].
pub fn parse_text_item(mut tokens: Vec<Token>) -> Result<TextItem, AsmError> {
    let span = tokens.last().map(|t| t.span).unwrap_or_default();
    tokens.push(Token::new(TokenKind::Newline, span));
    tokens.push(Token::new(TokenKind::Eof, span));
    Parser {
        toks: tokens,
        pos: 0,
        seen_globals: std::collections::HashSet::new(),
        seen_externs: std::collections::HashSet::new(),
        known_defines: std::collections::HashMap::new(),
    }
    .parse_single_text_item()
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    seen_globals: std::collections::HashSet<String>,
    seen_externs: std::collections::HashSet<String>,
    known_defines: std::collections::HashMap<String, i64>,
}

impl Parser {
    fn peek(&self) -> &TokenKind {
        &self.toks[self.pos].kind
    }
    fn peek_at(&self, n: usize) -> &TokenKind {
        self.toks
            .get(self.pos + n)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }
    fn span(&self) -> Span {
        self.toks[self.pos].span
    }
    fn bump(&mut self) -> TokenKind {
        let k = self.toks[self.pos].kind.clone();
        if !matches!(k, TokenKind::Eof) {
            self.pos += 1;
        }
        return k;
    }
    fn at_eof(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    fn err(&self, kind: AsmErrorKind) -> AsmError {
        AsmError::new(self.span(), kind)
    }
    fn unexpected(&self, expected: &str) -> AsmError {
        self.err(AsmErrorKind::Unexpected {
            expected: expected.to_string(),
            found: describe(self.peek()),
        })
    }

    /// Skip blank lines.
    fn skip_newlines(&mut self) {
        while matches!(self.peek(), TokenKind::Newline) {
            self.bump();
        }
    }

    /// Consume an expected `Newline` (or end of input).
    fn expect_newline(&mut self) -> Result<(), AsmError> {
        match self.peek() {
            TokenKind::Newline => {
                self.bump();
                Ok(())
            }
            TokenKind::Eof => Ok(()),
            _ => Err(self.unexpected("end of line")),
        }
    }

    /// Read an identifier, returning its text.
    fn ident(&mut self) -> Result<(String, Span), AsmError> {
        let span = self.span();
        match self.peek() {
            TokenKind::Ident(_) => {
                if let TokenKind::Ident(s) = self.bump() {
                    Ok((s, span))
                } else {
                    unreachable!()
                }
            }
            _ => Err(self.unexpected("an identifier")),
        }
    }

    /// Read an identifier that is not a reserved keyword (a user name).
    fn name(&mut self) -> Result<(String, Span), AsmError> {
        let (s, span) = self.ident()?;
        if keyword::is_reserved(&s) {
            return Err(AsmError::new(span, AsmErrorKind::ReservedName(s)));
        }
        Ok((s, span))
    }

    fn declare_extern(&mut self, name: &str, span: Span) -> Result<(), AsmError> {
        if self.seen_externs.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            return Err(AsmError::new(
                span,
                AsmErrorKind::DuplicateExtern(name.to_string()),
            ));
        }
        if self.seen_globals.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            return Err(AsmError::new(
                span,
                AsmErrorKind::ConflictingDeclaration(name.to_string()),
            ));
        }
        self.seen_externs.insert(name.to_string());
        Ok(())
    }

    fn declare_global(&mut self, name: &str, span: Span) -> Result<(), AsmError> {
        if name.eq_ignore_ascii_case("main") {
            return Err(AsmError::new(span, AsmErrorKind::GlobalMainForbidden));
        }
        if self.seen_globals.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            return Err(AsmError::new(
                span,
                AsmErrorKind::DuplicateGlobal(name.to_string()),
            ));
        }
        if self.seen_externs.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            return Err(AsmError::new(
                span,
                AsmErrorKind::ConflictingDeclaration(name.to_string()),
            ));
        }
        self.seen_globals.insert(name.to_string());
        Ok(())
    }

    // ── top level ──────────────────────────────────────────────────────────

    fn program(&mut self) -> Result<Program, AsmError> {
        let mut includes = Vec::new();
        let mut defines = Vec::new();
        let mut macros = Vec::new();
        let mut externs = Vec::new();
        let mut globals = Vec::new();
        let mut segments = Vec::new();

        // Top-level directives: %include, %define, %macro, extern, global
        loop {
            self.skip_newlines();
            if self.is_include_ahead() {
                includes.push(self.include()?);
            } else if self.is_define_ahead() {
                defines.push(self.define()?);
            } else if self.is_macro_ahead() {
                macros.push(self.parse_macro_def()?);
            } else if self.is_extern_ahead() {
                let span = self.span();
                self.bump(); // 'extern'
                let (name, nspan) = self.ident()?;
                self.expect_newline()?;
                self.declare_extern(&name, nspan)?;
                externs.push(name);
                let _ = span;
            } else if self.is_global_ahead() && !matches!(self.peek_at(2), TokenKind::Colon) {
                let span = self.span();
                self.bump(); // 'global'
                let (name, nspan) = self.ident()?;
                self.declare_global(&name, nspan)?;
                self.expect_newline()?;
                globals.push(name);
                let _ = span;
            } else {
                break;
            }
        }

        let mut seen_segments = std::collections::HashSet::new();

        // Then segments; a %define or %include here is an error.
        loop {
            self.skip_newlines();
            if self.at_eof() {
                break;
            }
            if self.is_define_ahead() {
                return Err(self.err(AsmErrorKind::DefineAfterSegment));
            }
            if self.is_include_ahead() {
                return Err(self.err(AsmErrorKind::DefineAfterSegment));
            }
            if self.is_macro_ahead() {
                return Err(self.err(AsmErrorKind::DefineAfterSegment));
            }
            segments.push(self.segment(&mut seen_segments)?);
        }

        Ok(Program {
            includes,
            defines,
            macros,
            externs,
            globals,
            segments,
        })
    }

    fn is_include_ahead(&self) -> bool {
        matches!(self.peek(), TokenKind::Percent)
            && matches!(self.peek_at(1), TokenKind::Ident(s) if s.eq_ignore_ascii_case("include"))
    }

    fn is_define_ahead(&self) -> bool {
        matches!(self.peek(), TokenKind::Percent)
            && matches!(self.peek_at(1), TokenKind::Ident(s) if s.eq_ignore_ascii_case("define"))
    }

    fn is_macro_ahead(&self) -> bool {
        matches!(self.peek(), TokenKind::Percent)
            && matches!(self.peek_at(1), TokenKind::Ident(s) if s.eq_ignore_ascii_case("macro"))
    }

    fn is_end_ahead(&self) -> bool {
        matches!(self.peek(), TokenKind::Percent)
            && matches!(self.peek_at(1), TokenKind::Ident(s) if s.eq_ignore_ascii_case("end"))
    }

    fn is_extern_ahead(&self) -> bool {
        matches!(self.peek(), TokenKind::Ident(s) if s.eq_ignore_ascii_case("extern"))
    }

    fn is_global_ahead(&self) -> bool {
        matches!(self.peek(), TokenKind::Ident(s) if s.eq_ignore_ascii_case("global"))
    }

    fn is_segment_ahead(&self) -> bool {
        matches!(
            self.peek(),
            TokenKind::Ident(s)
                if s.eq_ignore_ascii_case("segment") || s.eq_ignore_ascii_case("section")
        )
    }

    fn include(&mut self) -> Result<Include, AsmError> {
        let span = self.span();
        self.bump(); // %
        self.bump(); // include

        match self.peek() {
            TokenKind::Str(s) => {
                let path = s.clone();
                self.bump();
                self.expect_newline()?;
                Ok(Include {
                    path,
                    is_system: false,
                    span,
                })
            }
            TokenKind::Less => {
                self.bump(); // <
                let mut path = String::new();
                let mut expect_sep = false;
                while !matches!(
                    self.peek(),
                    TokenKind::Greater | TokenKind::Newline | TokenKind::Eof
                ) {
                    match self.peek() {
                        TokenKind::Ident(s) => {
                            if expect_sep {
                                return Err(self.err(AsmErrorKind::IncludeError(
                                    "unexpected whitespace or token in standard library include path; path components must be separated by '/'".to_string(),
                                )));
                            }
                            path.push_str(s);
                            self.bump();
                            expect_sep = true;
                        }
                        TokenKind::Number(n) => {
                            if expect_sep {
                                return Err(self.err(AsmErrorKind::IncludeError(
                                    "unexpected whitespace or token in standard library include path; path components must be separated by '/'".to_string(),
                                )));
                            }
                            path.push_str(&n.to_string());
                            self.bump();
                            expect_sep = true;
                        }
                        TokenKind::Slash => {
                            path.push('/');
                            self.bump();
                            expect_sep = false;
                        }
                        TokenKind::Period => {
                            path.push('.');
                            self.bump();
                            expect_sep = false;
                        }
                        _ => {
                            return Err(self.err(AsmErrorKind::IncludeError(
                                "invalid character in standard library include path; only alphanumeric characters, underscores, and '/' are allowed".to_string(),
                            )));
                        }
                    }
                }

                if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                    return Err(self.err(AsmErrorKind::IncludeError(
                        "expected '>' to close standard library include path".to_string(),
                    )));
                }

                self.bump(); // >

                if path.is_empty() {
                    return Err(self.err(AsmErrorKind::IncludeError(
                        "standard library include path cannot be empty".to_string(),
                    )));
                }

                // Validate path components: alphanumeric and underscores only
                for segment in path.split('/') {
                    let seg_name = segment.strip_suffix(".e8085").unwrap_or(segment);
                    if seg_name.is_empty() || !seg_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                        return Err(self.err(AsmErrorKind::IncludeError(format!(
                            "invalid segment '{segment}' in standard library path: only alphanumeric characters and underscores are allowed"
                        ))));
                    }
                }

                self.expect_newline()?;
                Ok(Include {
                    path,
                    is_system: true,
                    span,
                })
            }
            TokenKind::Ident(_) | TokenKind::Period | TokenKind::Slash => {
                let mut path = String::new();
                while !matches!(
                    self.peek(),
                    TokenKind::Newline | TokenKind::Eof
                ) {
                    match self.peek() {
                        TokenKind::Ident(s) => {
                            path.push_str(s);
                            self.bump();
                        }
                        TokenKind::Number(n) => {
                            path.push_str(&n.to_string());
                            self.bump();
                        }
                        TokenKind::Slash => {
                            path.push('/');
                            self.bump();
                        }
                        TokenKind::Period => {
                            path.push('.');
                            self.bump();
                        }
                        _ => {
                            return Err(self.err(AsmErrorKind::IncludeError(
                                "invalid character in include path".to_string(),
                            )));
                        }
                    }
                }
                if path.is_empty() {
                    return Err(self.err(AsmErrorKind::IncludeError(
                        "include path cannot be empty".to_string(),
                    )));
                }
                self.expect_newline()?;
                Ok(Include {
                    path,
                    is_system: false,
                    span,
                })
            }
            _ => Err(self.unexpected("a quoted string path, raw path, or '<...>'")),
        }
    }

    fn define(&mut self) -> Result<Define, AsmError> {
        let span = self.span();
        self.bump(); // %
        self.bump(); // define
        let (name, _) = self.name()?;
        let value = match self.peek() {
            TokenKind::Newline | TokenKind::Eof => Value::Number(1),
            TokenKind::Str(_) if matches!(self.peek_at(1), TokenKind::Newline | TokenKind::Eof) => {
                self.value()?
            }
            TokenKind::Char(c) if matches!(self.peek_at(1), TokenKind::Newline | TokenKind::Eof) => {
                let c_val = *c;
                self.bump();
                Value::Char(c_val)
            }
            _ => {
                let mut expr_tokens = Vec::new();
                let expr_span = self.span();
                while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                    expr_tokens.push(self.toks[self.pos].clone());
                    self.bump();
                }
                let num = crate::asm::expr::eval_expr(
                    &expr_tokens,
                    |k| self.known_defines.get(&k.to_ascii_lowercase()).copied(),
                    expr_span,
                    false,
                )?;
                Value::Number(num as u32)
            }
        };
        self.expect_newline()?;
        if let Value::Number(n) = value {
            self.known_defines.insert(name.to_ascii_lowercase(), n as i64);
        } else if let Value::Char(c) = value {
            self.known_defines.insert(name.to_ascii_lowercase(), c as i64);
        }
        Ok(Define { name, value, span })
    }

    fn parse_macro_def(&mut self) -> Result<MacroDef, AsmError> {
        let span = self.span();
        self.bump(); // %
        self.bump(); // macro

        let (name, nspan) = self.ident()?;
        if keyword::is_reserved(&name) {
            return Err(AsmError::new(
                nspan,
                AsmErrorKind::MacroCollision {
                    name,
                    collision_with: "instruction".to_string(),
                },
            ));
        }

        let arg_count = match self.peek() {
            TokenKind::Number(n) => {
                let val = *n as usize;
                self.bump();
                val
            }
            TokenKind::Ident(s) => {
                let s_clone = s.clone();
                let sp = self.span();
                self.bump();
                return Err(AsmError::new(sp, AsmErrorKind::InvalidMacroArgCount(s_clone)));
            }
            _ => {
                let sp = self.span();
                let found = describe(self.peek());
                self.bump();
                return Err(AsmError::new(sp, AsmErrorKind::InvalidMacroArgCount(found)));
            }
        };

        self.expect_newline()?;

        let mut body_lines = Vec::new();
        loop {
            while matches!(self.peek(), TokenKind::Newline) {
                self.bump();
            }

            if self.at_eof() {
                return Err(AsmError::new(span, AsmErrorKind::UnterminatedMacro(name)));
            }

            if self.is_end_ahead() {
                self.bump(); // %
                self.bump(); // end
                self.expect_newline()?;
                break;
            }

            let line_span = self.span();
            let mut line_tokens = Vec::new();
            while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                let tok = self.toks[self.pos].clone();
                self.bump();
                line_tokens.push(tok);
            }
            self.expect_newline()?;

            if !line_tokens.is_empty() {
                body_lines.push(MacroLine {
                    text: String::new(),
                    tokens: line_tokens,
                    span: line_span,
                });
            }
        }

        Ok(MacroDef {
            name,
            arg_count,
            body_lines,
            span,
            doc_comment: None,
        })
    }

    // ── segments ───────────────────────────────────────────────────────────

    fn segment(&mut self, seen_segments: &mut std::collections::HashSet<String>) -> Result<Segment, AsmError> {
        match self.peek() {
            TokenKind::Ident(s)
                if s.eq_ignore_ascii_case("segment") || s.eq_ignore_ascii_case("section") =>
            {
                self.bump();
            }
            _ => return Err(self.unexpected("`segment` or `section`")),
        }
        match self.peek() {
            TokenKind::Period => {
                self.bump();
            }
            _ => return Err(self.unexpected("`.` before the segment name")),
        }
        let (kind, kspan) = self.ident()?;
        let norm_kind = kind.to_ascii_lowercase();
        if seen_segments.contains(&norm_kind) {
            return Err(AsmError::new(kspan, AsmErrorKind::DuplicateSegment(norm_kind)));
        }
        seen_segments.insert(norm_kind.clone());
        let seg = match norm_kind.as_str() {
            "data" => {
                self.expect_newline()?;
                Segment::Data(self.data_body()?)
            }
            "bss" => {
                self.expect_newline()?;
                Segment::Bss(self.bss_body()?)
            }
            "text" => {
                self.expect_newline()?;
                Segment::Text(self.text_body()?)
            }
            _ => return Err(AsmError::new(kspan, AsmErrorKind::UnknownSegment(kind))),
        };
        Ok(seg)
    }

    /// True when the current position starts a new segment, a `%define`, or the input has
    /// ended — i.e. the current segment body is finished. Stopping at `%define` lets the
    /// top-level loop report it as [`AsmErrorKind::DefineAfterSegment`].
    fn at_body_end(&self) -> bool {
        self.at_eof()
            || self.is_segment_ahead()
            || self.is_define_ahead()
            || self.is_include_ahead()
    }

    fn data_body(&mut self) -> Result<Vec<DataDef>, AsmError> {
        let mut defs = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_body_end() {
                break;
            }
            let (name, span) = self.name()?;
            let size = self.optional_size()?;
            let mut values = vec![self.value()?];
            while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                if matches!(self.peek(), TokenKind::Comma) {
                    self.bump();
                }
                if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                    break;
                }
                values.push(self.value()?);
            }
            self.expect_newline()?;
            defs.push(DataDef {
                name,
                size,
                values,
                span,
            });
        }
        Ok(defs)
    }

    fn optional_size(&mut self) -> Result<Size, AsmError> {
        if let TokenKind::Ident(s) = self.peek() {
            if let Some(sz) = keyword::size(s) {
                self.bump();
                return Ok(sz);
            }
        }
        Ok(Size::Byte)
    }

    fn bss_body(&mut self) -> Result<Vec<BssDecl>, AsmError> {
        let mut decls = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_body_end() {
                break;
            }
            let (name, span) = self.name()?;
            let size = self.size()?;
            let count = self.value()?;
            self.expect_newline()?;
            decls.push(BssDecl {
                name,
                size,
                count,
                span,
            });
        }
        Ok(decls)
    }

    fn text_body(&mut self) -> Result<Vec<TextItem>, AsmError> {
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_body_end() {
                break;
            }
            items.push(self.parse_single_text_item()?);
        }
        Ok(items)
    }

    fn parse_single_text_item(&mut self) -> Result<TextItem, AsmError> {
        self.skip_newlines();

        if self.is_macro_ahead() {
            let m = self.parse_macro_def()?;
            return Ok(TextItem::MacroDef(m));
        }

        // 1. `extern NAME` declaration
        if self.is_extern_ahead() {
            let span = self.span();
            self.bump(); // 'extern'
            let (name, nspan) = self.ident()?;
            self.expect_newline()?;
            self.declare_extern(&name, nspan)?;
            return Ok(TextItem::ExternDecl(name, span));
        }

        // 2. `global NAME:` (inline) or `global NAME` (standalone)
        if self.is_global_ahead() {
            let span = self.span();
            self.bump(); // 'global'
            let (name, nspan) = self.ident()?;
            self.declare_global(&name, nspan)?;
            if matches!(self.peek(), TokenKind::Colon) {
                self.bump(); // ':'
                self.expect_newline()?;
                return Ok(TextItem::GlobalLabel(name, span));
            } else {
                self.expect_newline()?;
                return Ok(TextItem::GlobalDecl(name, span));
            }
        }

        // 3. Local label: `.NAME:` on its own line
        if matches!(self.peek(), TokenKind::Period)
            && matches!(self.peek_at(1), TokenKind::Ident(_))
            && matches!(self.peek_at(2), TokenKind::Colon)
        {
            let span = self.span();
            self.bump(); // '.'
            let (name, _) = self.ident()?;
            self.bump(); // ':'
            self.expect_newline()?;
            return Ok(TextItem::LocalLabel(name, span));
        }

        // 4. Standard label: `IDENT ':'` on its own line
        if matches!(self.peek(), TokenKind::Ident(_))
            && matches!(self.peek_at(1), TokenKind::Colon)
        {
            let (name, span) = self.name()?;
            self.bump(); // ':'
            self.expect_newline()?;
            return Ok(TextItem::Label(name, span));
        }

        // Otherwise an instruction or macro call.
        let (mnemonic, span) = self.ident()?;
        let operands = self.operands()?;
        self.expect_newline()?;
        Ok(TextItem::Instr(Instr {
            mnemonic,
            operands,
            span,
        }))
    }

    fn size(&mut self) -> Result<Size, AsmError> {
        let (word, span) = self.ident()?;
        keyword::size(&word).ok_or_else(|| {
            AsmError::new(
                span,
                AsmErrorKind::Unexpected {
                    expected: "BYTE or WORD".to_string(),
                    found: format!("`{word}`"),
                },
            )
        })
    }

    // ── operands & values ─────────────────────────────────────────────────

    fn operands(&mut self) -> Result<Vec<POperand>, AsmError> {
        let mut ops = Vec::new();
        if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            return Ok(ops);
        }
        ops.push(self.operand()?);
        while matches!(self.peek(), TokenKind::Comma) {
            self.bump();
            ops.push(self.operand()?);
        }
        Ok(ops)
    }

    fn operand(&mut self) -> Result<POperand, AsmError> {
        match self.peek().clone() {
            TokenKind::Number(v) => {
                self.bump();
                Ok(POperand::Num(v))
            }
            TokenKind::Char(b) => {
                self.bump();
                Ok(POperand::Char(b))
            }
            TokenKind::Period => {
                // Local symbol reference: `.loop`
                self.bump(); // '.'
                let (name, _) = self.ident()?;
                Ok(POperand::LocalSym(name))
            }
            TokenKind::Ident(s) => {
                self.bump();
                if let Some(r) = keyword::reg8(&s) {
                    Ok(POperand::Reg8(r))
                } else if let Some(r) = keyword::reg16(&s) {
                    Ok(POperand::Reg16(r))
                } else {
                    Ok(POperand::Sym(s))
                }
            }
            TokenKind::Percent => {
                // Only %len is allowed in operand position.
                self.bump();
                let (d, dspan) = self.ident()?;
                if d.eq_ignore_ascii_case("len") {
                    let (name, _) = self.ident()?;
                    Ok(POperand::Len(name))
                } else {
                    Err(AsmError::new(dspan, AsmErrorKind::UnknownDirective(d)))
                }
            }
            _ => Err(self.unexpected("an operand")),
        }
    }

    /// Parse one data/directive value (recursive for `%repeat`).
    fn value(&mut self) -> Result<Value, AsmError> {
        match self.peek().clone() {
            TokenKind::Number(v) => {
                self.bump();
                Ok(Value::Number(v))
            }
            TokenKind::Str(s) => {
                self.bump();
                Ok(Value::Str(s))
            }
            TokenKind::Char(b) => {
                self.bump();
                Ok(Value::Char(b))
            }
            TokenKind::Ident(s) => {
                self.bump();
                Ok(Value::Ident(s))
            }
            TokenKind::Percent => {
                self.bump();
                let (d, dspan) = self.ident()?;
                match d.to_ascii_lowercase().as_str() {
                    "len" => {
                        let (name, _) = self.ident()?;
                        Ok(Value::Len(name))
                    }
                    "repeat" => {
                        let count = Box::new(self.value()?);
                        let value = Box::new(self.value()?);
                        Ok(Value::Repeat { count, value })
                    }
                    _ => Err(AsmError::new(dspan, AsmErrorKind::UnknownDirective(d))),
                }
            }
            _ => Err(self.unexpected("a value")),
        }
    }
}

/// A short human description of a token kind, for error messages.
fn describe(k: &TokenKind) -> String {
    match k {
        TokenKind::Percent => "`%`".into(),
        TokenKind::Ident(s) => format!("`{s}`"),
        TokenKind::Number(n) => format!("number {n}"),
        TokenKind::Str(_) => "a string".into(),
        TokenKind::Char(_) => "a character".into(),
        TokenKind::Comma => "`,`".into(),
        TokenKind::Colon => "`:`".into(),
        TokenKind::Period => "`.`".into(),
        TokenKind::Slash => "`/`".into(),
        TokenKind::Plus => "`+`".into(),
        TokenKind::Minus => "`-`".into(),
        TokenKind::Star => "`*`".into(),
        TokenKind::Shl => "`<<`".into(),
        TokenKind::Shr => "`>>`".into(),
        TokenKind::Amp => "`&`".into(),
        TokenKind::Pipe => "`|`".into(),
        TokenKind::Tilde => "`~`".into(),
        TokenKind::Caret => "`^`".into(),
        TokenKind::Exclamation => "`!`".into(),
        TokenKind::DoubleEqual => "`==`".into(),
        TokenKind::NotEqual => "`!=`".into(),
        TokenKind::Less => "`<`".into(),
        TokenKind::LessEqual => "`<=`".into(),
        TokenKind::Greater => "`>`".into(),
        TokenKind::GreaterEqual => "`>=`".into(),
        TokenKind::AmpAmp => "`&&`".into(),
        TokenKind::PipePipe => "`||`".into(),
        TokenKind::LParen => "`(`".into(),
        TokenKind::RParen => "`)`".into(),
        TokenKind::Newline => "end of line".into(),
        TokenKind::Eof => "end of input".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::encode::{AReg8, AReg16};
    use crate::asm::lex;

    fn prog(src: &str) -> Program {
        parse(lex(src).unwrap()).unwrap()
    }
    fn perr(src: &str) -> AsmErrorKind {
        parse(lex(src).unwrap()).unwrap_err().kind
    }

    #[test]
    fn parses_a_full_program() {
        let p = prog(
            "%define BUF 16\n\
             segment .data\n\
             prompt BYTE \"Hi\" 0x0A\n\
             segment .bss\n\
             buf BYTE BUF\n\
             segment .text\n\
             main:\n\
             mvi A, 0x05\n\
             loop:\n\
             dcr A\n\
             jnz loop\n\
             hlt\n",
        );
        assert_eq!(p.defines.len(), 1);
        assert_eq!(p.defines[0].name, "BUF");
        assert_eq!(p.defines[0].value, Value::Number(16));
        assert_eq!(p.segments.len(), 3);

        match &p.segments[0] {
            Segment::Data(d) => {
                assert_eq!(d[0].name, "prompt");
                assert_eq!(d[0].size, Size::Byte);
                assert_eq!(
                    d[0].values,
                    vec![Value::Str("Hi".into()), Value::Number(0x0A)]
                );
            }
            _ => panic!("expected data"),
        }
        match &p.segments[2] {
            Segment::Text(items) => {
                assert_eq!(
                    items[0],
                    TextItem::Label("main".into(), items_span(&items[0]))
                );
                assert!(
                    matches!(&items[1], TextItem::Instr(i) if i.mnemonic == "mvi"
                    && i.operands == vec![POperand::Reg8(AReg8::A), POperand::Num(5)])
                );
                assert!(
                    matches!(&items[4], TextItem::Instr(i) if i.mnemonic == "jnz"
                    && i.operands == vec![POperand::Sym("loop".into())])
                );
            }
            _ => panic!("expected text"),
        }
    }

    fn items_span(item: &TextItem) -> Span {
        match item {
            TextItem::Label(_, s) => *s,
            TextItem::GlobalLabel(_, s) => *s,
            TextItem::LocalLabel(_, s) => *s,
            TextItem::GlobalDecl(_, s) => *s,
            TextItem::ExternDecl(_, s) => *s,
            TextItem::MacroDef(m) => m.span,
            TextItem::MacroCall(_, _, s) => *s,
            TextItem::MacroEnd => Span::default(),
            TextItem::Instr(i) => i.span,
        }
    }

    #[test]
    fn parses_directive_nesting_in_data() {
        let p = prog("segment .data\nbanner BYTE %repeat %len X '-'\n");
        match &p.segments[0] {
            Segment::Data(d) => match &d[0].values[0] {
                Value::Repeat { count, value } => {
                    assert_eq!(**count, Value::Len("X".into()));
                    assert_eq!(**value, Value::Char(b'-'));
                }
                other => panic!("expected repeat, got {other:?}"),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn registers_vs_symbols_in_operands() {
        let p = prog("segment .text\nlxi SP, 0xF000\nmov A, M\nadd B\njmp done\n");
        let Segment::Text(items) = &p.segments[0] else {
            panic!()
        };
        assert!(matches!(&items[0], TextItem::Instr(i)
            if i.operands == vec![POperand::Reg16(AReg16::SP), POperand::Num(0xF000)]));
        assert!(matches!(&items[1], TextItem::Instr(i)
            if i.operands == vec![POperand::Reg8(AReg8::A), POperand::Reg8(AReg8::M)]));
        assert!(matches!(&items[3], TextItem::Instr(i)
            if i.operands == vec![POperand::Sym("done".into())]));
    }

    #[test]
    fn len_in_operand_position() {
        let p = prog("segment .text\nmvi A, %len prompt\n");
        let Segment::Text(items) = &p.segments[0] else {
            panic!()
        };
        assert!(matches!(&items[0], TextItem::Instr(i)
            if i.operands == vec![POperand::Reg8(AReg8::A), POperand::Len("prompt".into())]));
    }

    #[test]
    fn errors() {
        assert_eq!(
            perr("segment .text\nnop\n%define X 1\n"),
            AsmErrorKind::DefineAfterSegment
        );
        assert_eq!(
            perr("segment .code\nnop\n"),
            AsmErrorKind::UnknownSegment("code".into())
        );
        assert_eq!(
            perr("segment .data\nmov BYTE 1\n"),
            AsmErrorKind::ReservedName("mov".into())
        );
        assert!(matches!(
            perr("segment .text\nmov A B\n"),
            AsmErrorKind::Unexpected { .. }
        ));
    }

    #[test]
    fn test_parse_system_and_quoted_includes() {
        let p = prog("%include <math>\n%include <device/canvas>\n%include \"my/file.e8085\"\n%include raw/path/lib.e8085\nsegment .text\nhlt\n");
        assert_eq!(p.includes.len(), 4);
        assert_eq!(p.includes[0].path, "math");
        assert!(p.includes[0].is_system);
        assert_eq!(p.includes[1].path, "device/canvas");
        assert!(p.includes[1].is_system);
        assert_eq!(p.includes[2].path, "my/file.e8085");
        assert!(!p.includes[2].is_system);
        assert_eq!(p.includes[3].path, "raw/path/lib.e8085");
        assert!(!p.includes[3].is_system);

        assert!(matches!(
            perr("%include <>\n"),
            AsmErrorKind::IncludeError(_)
        ));
        assert!(matches!(
            perr("%include <invalid space>\n"),
            AsmErrorKind::IncludeError(_)
        ));
        assert!(matches!(
            perr("%include <unclosed\n"),
            AsmErrorKind::IncludeError(_)
        ));
    }
}
