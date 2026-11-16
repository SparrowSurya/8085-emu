//! Conditional compilation preprocessor for `.e8085`.
//!
//! Evaluates conditional compilation directives (`%if`, `%elif`, `%else`, `%end`, `%endif`,
//! `%ifdef`, `%ifndef`), tracks active `%define` values, evaluates condition expressions,
//! and filters the token stream so that only active lines are passed to the parser.
//!
//! Excluded lines emit blank newlines so that token source spans (line numbers) remain
//! exactly 1:1 with the original file on disk.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::error::{AsmError, AsmErrorKind, Span};
use super::lexer::lex;
use super::token::{Token, TokenKind};

/// Preprocess a stream of tokens, filtering out inactive conditional compilation blocks.
pub fn preprocess(tokens: Vec<Token>, base_dir: Option<&Path>) -> Result<Vec<Token>, AsmError> {
    let mut defines = HashMap::new();
    let mut visited_includes = HashSet::new();
    preprocess_with_defines(tokens, base_dir, &mut defines, &mut visited_includes)
}

/// Preprocess tokens with a shared `defines` map (useful across `%include` trees).
pub fn preprocess_with_defines(
    tokens: Vec<Token>,
    base_dir: Option<&Path>,
    defines: &mut HashMap<String, i64>,
    visited_includes: &mut HashSet<PathBuf>,
) -> Result<Vec<Token>, AsmError> {
    Preprocessor::new(tokens, base_dir, defines, visited_includes).run()
}

/// Preprocess tokens and also collect inactive source line ranges `(start_line, end_line)` (1-based).
pub fn preprocess_with_inactive_ranges(
    tokens: Vec<Token>,
    base_dir: Option<&Path>,
) -> Result<(Vec<Token>, Vec<(u32, u32)>), AsmError> {
    let mut defines = HashMap::new();
    let mut visited_includes = HashSet::new();
    let mut inactive_ranges = Vec::new();
    let res_tokens = Preprocessor::new_with_collector(
        tokens,
        base_dir,
        &mut defines,
        &mut visited_includes,
        Some(&mut inactive_ranges),
    )
    .run()?;
    Ok((res_tokens, inactive_ranges))
}

/// Helper to find all inactive source line ranges for an assembly source string (1-based).
pub fn find_inactive_ranges(source: &str, base_dir: Option<&Path>) -> Vec<(u32, u32)> {
    if let Ok(tokens) = lex(source) {
        if let Ok((_, ranges)) = preprocess_with_inactive_ranges(tokens, base_dir) {
            return ranges;
        }
    }
    Vec::new()
}

#[derive(Debug, Clone)]
struct CondFrame {
    span: Span,
    parent_active: bool,
    branch_taken: bool,
    currently_active: bool,
    has_else: bool,
    inactive_start_line: Option<u32>,
}

#[derive(Debug)]
enum BlockKind {
    If(CondFrame),
    Macro { active: bool },
}

struct Preprocessor<'a> {
    tokens: Vec<Token>,
    pos: usize,
    base_dir: Option<PathBuf>,
    defines: &'a mut HashMap<String, i64>,
    visited_includes: &'a mut HashSet<PathBuf>,
    block_stack: Vec<BlockKind>,
    inactive_ranges: Option<&'a mut Vec<(u32, u32)>>,
}

impl<'a> Preprocessor<'a> {
    fn new(
        tokens: Vec<Token>,
        base_dir: Option<&Path>,
        defines: &'a mut HashMap<String, i64>,
        visited_includes: &'a mut HashSet<PathBuf>,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            base_dir: base_dir.map(|p| p.to_path_buf()),
            defines,
            visited_includes,
            block_stack: Vec::new(),
            inactive_ranges: None,
        }
    }

    fn new_with_collector(
        tokens: Vec<Token>,
        base_dir: Option<&Path>,
        defines: &'a mut HashMap<String, i64>,
        visited_includes: &'a mut HashSet<PathBuf>,
        inactive_ranges: Option<&'a mut Vec<(u32, u32)>>,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            base_dir: base_dir.map(|p| p.to_path_buf()),
            defines,
            visited_includes,
            block_stack: Vec::new(),
            inactive_ranges,
        }
    }

    fn is_currently_active(&self) -> bool {
        for b in &self.block_stack {
            match b {
                BlockKind::If(frame) => {
                    if !frame.currently_active {
                        return false;
                    }
                }
                BlockKind::Macro { active } => {
                    if !*active {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn parent_cond_active(&self) -> bool {
        for b in &self.block_stack {
            match b {
                BlockKind::If(frame) => {
                    if !frame.currently_active {
                        return false;
                    }
                }
                BlockKind::Macro { active } => {
                    if !*active {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn run(mut self) -> Result<Vec<Token>, AsmError> {
        let mut out = Vec::new();

        while self.pos < self.tokens.len() {
            let line_start_pos = self.pos;
            let mut line_tokens = Vec::new();

            while self.pos < self.tokens.len() {
                let tok = self.tokens[self.pos].clone();
                self.pos += 1;
                if tok.kind == TokenKind::Newline {
                    line_tokens.push(tok);
                    break;
                }
                if tok.kind == TokenKind::Eof {
                    line_tokens.push(tok);
                    break;
                }
                line_tokens.push(tok);
            }

            if line_tokens.is_empty() {
                continue;
            }

            // Check if last token is EOF
            let is_eof = line_tokens
                .last()
                .map(|t| t.kind == TokenKind::Eof)
                .unwrap_or(false);

            let first_non_newline = line_tokens
                .iter()
                .find(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof);

            let newline_tok = line_tokens
                .iter()
                .find(|t| t.kind == TokenKind::Newline)
                .cloned();

            if let Some(first) = first_non_newline {
                if first.kind == TokenKind::Percent {
                    let first_idx = line_tokens
                        .iter()
                        .position(|t| t.kind == TokenKind::Percent)
                        .unwrap();

                    if first_idx + 1 < line_tokens.len() {
                        if let TokenKind::Ident(ref directive) = line_tokens[first_idx + 1].kind {
                            let dir_lower = directive.to_ascii_lowercase();
                            let dir_span = line_tokens[first_idx].span;

                            match dir_lower.as_str() {
                                "if" => {
                                    self.handle_if(
                                        &line_tokens[first_idx + 2..],
                                        dir_span,
                                        false,
                                        false,
                                    )?;
                                    if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                "ifdef" => {
                                    self.handle_if(
                                        &line_tokens[first_idx + 2..],
                                        dir_span,
                                        true,
                                        false,
                                    )?;
                                    if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                "ifndef" => {
                                    self.handle_if(
                                        &line_tokens[first_idx + 2..],
                                        dir_span,
                                        true,
                                        true,
                                    )?;
                                    if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                "elif" => {
                                    self.handle_elif(&line_tokens[first_idx + 2..], dir_span)?;
                                    if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                "else" => {
                                    self.handle_else(dir_span)?;
                                    if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                "endif" => {
                                    self.handle_endif(dir_span)?;
                                    if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                "end" => {
                                    // %end can close either %macro or %if
                                    let closed = self.handle_end(dir_span)?;
                                    match closed {
                                        ClosedBlock::If => {
                                            if let Some(nl) = newline_tok {
                                                out.push(nl);
                                            }
                                            continue;
                                        }
                                        ClosedBlock::Macro(was_active) => {
                                            if was_active {
                                                out.extend(line_tokens);
                                            } else if let Some(nl) = newline_tok {
                                                out.push(nl);
                                            }
                                            continue;
                                        }
                                    }
                                }
                                "macro" => {
                                    let active = self.is_currently_active();
                                    self.block_stack.push(BlockKind::Macro { active });
                                    if active {
                                        out.extend(line_tokens);
                                    } else if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                "define" => {
                                    if self.is_currently_active() {
                                        self.record_define(&line_tokens[first_idx + 2..])?;
                                        out.extend(line_tokens);
                                    } else if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                "include" => {
                                    if self.is_currently_active() {
                                        self.scan_include(&line_tokens[first_idx + 2..])?;
                                        out.extend(line_tokens);
                                    } else if let Some(nl) = newline_tok {
                                        out.push(nl);
                                    }
                                    continue;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            // Normal line: emit if currently active, otherwise emit blank newline
            if self.is_currently_active() {
                out.extend(line_tokens);
            } else {
                if let Some(nl) = newline_tok {
                    out.push(nl);
                }
                if is_eof {
                    out.push(Token::new(TokenKind::Eof, self.tokens[line_start_pos].span));
                }
            }
        }

        // Check for unterminated %if
        for b in &self.block_stack {
            if let BlockKind::If(frame) = b {
                return Err(AsmError::new(frame.span, AsmErrorKind::UnterminatedIf));
            }
        }

        Ok(out)
    }

    fn handle_if(
        &mut self,
        cond_tokens: &[Token],
        span: Span,
        is_def_check: bool,
        invert: bool,
    ) -> Result<(), AsmError> {
        let parent_active = self.parent_cond_active();
        let (cond_val, inactive_start) = if parent_active {
            let active = if is_def_check {
                let name = extract_ident_name(cond_tokens, span)?;
                let is_def = self.defines.contains_key(&name.to_ascii_lowercase());
                if invert { !is_def } else { is_def }
            } else {
                let raw_val = eval_condition(cond_tokens, self.defines, span)?;
                raw_val != 0
            };
            let inact = if !active { Some(span.line + 1) } else { None };
            (active, inact)
        } else {
            (false, None)
        };

        let frame = CondFrame {
            span,
            parent_active,
            branch_taken: cond_val,
            currently_active: parent_active && cond_val,
            has_else: false,
            inactive_start_line: inactive_start,
        };

        self.block_stack.push(BlockKind::If(frame));
        Ok(())
    }

    fn handle_elif(&mut self, cond_tokens: &[Token], span: Span) -> Result<(), AsmError> {
        let (parent_active, branch_taken, has_else, prev_inactive_start) = {
            let frame = self.get_innermost_if_mut(span, AsmErrorKind::ElifWithoutIf)?;
            (
                frame.parent_active,
                frame.branch_taken,
                frame.has_else,
                frame.inactive_start_line,
            )
        };
        if has_else {
            return Err(AsmError::new(span, AsmErrorKind::ElifAfterElse));
        }

        if let Some(start) = prev_inactive_start {
            if span.line > start {
                if let Some(ref mut ranges) = self.inactive_ranges {
                    ranges.push((start, span.line - 1));
                }
            }
        }

        let (cond_val, new_inactive_start) = if parent_active && !branch_taken {
            let active = eval_condition(cond_tokens, self.defines, span)? != 0;
            let inact = if !active { Some(span.line + 1) } else { None };
            (active, inact)
        } else if parent_active {
            (false, Some(span.line + 1))
        } else {
            (false, None)
        };

        let frame = self.get_innermost_if_mut(span, AsmErrorKind::ElifWithoutIf)?;
        if cond_val {
            frame.branch_taken = true;
            frame.currently_active = true;
        } else {
            frame.currently_active = false;
        }
        frame.inactive_start_line = new_inactive_start;

        Ok(())
    }

    fn handle_else(&mut self, span: Span) -> Result<(), AsmError> {
        let (parent_active, branch_taken, prev_inactive_start) = {
            let frame = self.get_innermost_if_mut(span, AsmErrorKind::ElseWithoutIf)?;
            if frame.has_else {
                return Err(AsmError::new(span, AsmErrorKind::DuplicateElse));
            }
            frame.has_else = true;
            (
                frame.parent_active,
                frame.branch_taken,
                frame.inactive_start_line,
            )
        };

        if let Some(start) = prev_inactive_start {
            if span.line > start {
                if let Some(ref mut ranges) = self.inactive_ranges {
                    ranges.push((start, span.line - 1));
                }
            }
        }

        let (is_active, new_inactive_start) = if parent_active && !branch_taken {
            (true, None)
        } else if parent_active {
            (false, Some(span.line + 1))
        } else {
            (false, None)
        };

        let frame = self.get_innermost_if_mut(span, AsmErrorKind::ElseWithoutIf)?;
        if is_active {
            frame.branch_taken = true;
            frame.currently_active = true;
        } else {
            frame.currently_active = false;
        }
        frame.inactive_start_line = new_inactive_start;

        Ok(())
    }

    fn handle_endif(&mut self, span: Span) -> Result<(), AsmError> {
        // Must pop innermost BlockKind::If
        match self.block_stack.pop() {
            Some(BlockKind::If(frame)) => {
                if let Some(start) = frame.inactive_start_line {
                    if span.line > start {
                        if let Some(ref mut ranges) = self.inactive_ranges {
                            ranges.push((start, span.line - 1));
                        }
                    }
                }
                Ok(())
            }
            Some(BlockKind::Macro { .. }) => {
                Err(AsmError::new(span, AsmErrorKind::UnexpectedEnd))
            }
            None => Err(AsmError::new(span, AsmErrorKind::UnexpectedEnd)),
        }
    }

    fn handle_end(&mut self, span: Span) -> Result<ClosedBlock, AsmError> {
        match self.block_stack.pop() {
            Some(BlockKind::If(frame)) => {
                if let Some(start) = frame.inactive_start_line {
                    if span.line > start {
                        if let Some(ref mut ranges) = self.inactive_ranges {
                            ranges.push((start, span.line - 1));
                        }
                    }
                }
                Ok(ClosedBlock::If)
            }
            Some(BlockKind::Macro { active }) => Ok(ClosedBlock::Macro(active)),
            None => Err(AsmError::new(span, AsmErrorKind::UnexpectedEnd)),
        }
    }

    fn get_innermost_if_mut(
        &mut self,
        span: Span,
        err_if_none: AsmErrorKind,
    ) -> Result<&mut CondFrame, AsmError> {
        for b in self.block_stack.iter_mut().rev() {
            if let BlockKind::If(frame) = b {
                return Ok(frame);
            }
        }
        Err(AsmError::new(span, err_if_none))
    }

    fn record_define(&mut self, line_tokens: &[Token]) -> Result<(), AsmError> {
        let useful: Vec<&Token> = line_tokens
            .iter()
            .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
            .collect();

        if useful.is_empty() {
            return Ok(());
        }

        if let TokenKind::Ident(ref name) = useful[0].kind {
            let key = name.to_ascii_lowercase();
            if useful.len() == 1 {
                // %define NAME -> defaults to 1
                self.defines.insert(key, 1);
            } else {
                // Value expression following name
                let val_tokens: Vec<Token> = useful[1..].iter().map(|&t| t.clone()).collect();
                let span = val_tokens[0].span;
                let val = eval_condition(&val_tokens, self.defines, span).unwrap_or(1);
                self.defines.insert(key, val);
            }
        }

        Ok(())
    }

    fn scan_include(&mut self, line_tokens: &[Token]) -> Result<(), AsmError> {
        let base_dir = match &self.base_dir {
            Some(d) => d.clone(),
            None => return Ok(()),
        };

        for tok in line_tokens {
            if let TokenKind::Str(ref path_str) = tok.kind {
                let inc_path = base_dir.join(path_str);
                let canon = match inc_path.canonicalize() {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if self.visited_includes.contains(&canon) {
                    continue;
                }
                self.visited_includes.insert(canon.clone());

                let inc_src = match std::fs::read_to_string(&canon) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let inc_tokens = match lex(&inc_src) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let inc_dir = canon.parent().unwrap_or(&base_dir);
                let _ = preprocess_with_defines(
                    inc_tokens,
                    Some(inc_dir),
                    self.defines,
                    self.visited_includes,
                );
            }
        }

        Ok(())
    }
}

enum ClosedBlock {
    If,
    Macro(bool),
}

fn extract_ident_name(tokens: &[Token], span: Span) -> Result<String, AsmError> {
    for t in tokens {
        if let TokenKind::Ident(ref name) = t.kind {
            return Ok(name.clone());
        }
    }
    Err(AsmError::new(
        span,
        AsmErrorKind::InvalidCondition("expected an identifier".to_string()),
    ))
}

/// Evaluates a condition expression into a numeric value (non-zero = true, 0 = false).
pub fn eval_condition(
    tokens: &[Token],
    defines: &HashMap<String, i64>,
    span: Span,
) -> Result<i64, AsmError> {
    let clean: Vec<Token> = tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
        .cloned()
        .collect();

    if clean.is_empty() {
        return Err(AsmError::new(
            span,
            AsmErrorKind::InvalidCondition("empty condition expression".to_string()),
        ));
    }

    let mut parser = CondExprParser::new(&clean, defines, span);
    let result = parser.parse_expr()?;
    if parser.pos < clean.len() {
        return Err(AsmError::new(
            clean[parser.pos].span,
            AsmErrorKind::InvalidCondition(format!("unexpected token '{:?}'", clean[parser.pos].kind)),
        ));
    }
    Ok(result)
}

struct CondExprParser<'a> {
    tokens: &'a [Token],
    pos: usize,
    defines: &'a HashMap<String, i64>,
    default_span: Span,
}

impl<'a> CondExprParser<'a> {
    fn new(tokens: &'a [Token], defines: &'a HashMap<String, i64>, default_span: Span) -> Self {
        Self {
            tokens,
            pos: 0,
            defines,
            default_span,
        }
    }

    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }

    fn span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(self.default_span)
    }

    fn bump(&mut self) -> Option<&TokenKind> {
        let k = self.tokens.get(self.pos).map(|t| &t.kind);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        k
    }

    fn parse_expr(&mut self) -> Result<i64, AsmError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_and()?;
        while let Some(TokenKind::PipePipe) = self.peek() {
            self.bump();
            let right = self.parse_and()?;
            left = if left != 0 || right != 0 { 1 } else { 0 };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_equality()?;
        while let Some(TokenKind::AmpAmp) = self.peek() {
            self.bump();
            let right = self.parse_equality()?;
            left = if left != 0 && right != 0 { 1 } else { 0 };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_relational()?;
        while let Some(k) = self.peek() {
            match k {
                TokenKind::DoubleEqual => {
                    self.bump();
                    let right = self.parse_relational()?;
                    left = if left == right { 1 } else { 0 };
                }
                TokenKind::NotEqual => {
                    self.bump();
                    let right = self.parse_relational()?;
                    left = if left != right { 1 } else { 0 };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<i64, AsmError> {
        let mut left = self.parse_primary()?;
        while let Some(k) = self.peek() {
            match k {
                TokenKind::Less => {
                    self.bump();
                    let right = self.parse_primary()?;
                    left = if left < right { 1 } else { 0 };
                }
                TokenKind::LessEqual => {
                    self.bump();
                    let right = self.parse_primary()?;
                    left = if left <= right { 1 } else { 0 };
                }
                TokenKind::Greater => {
                    self.bump();
                    let right = self.parse_primary()?;
                    left = if left > right { 1 } else { 0 };
                }
                TokenKind::GreaterEqual => {
                    self.bump();
                    let right = self.parse_primary()?;
                    left = if left >= right { 1 } else { 0 };
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<i64, AsmError> {
        let sp = self.span();
        match self.peek() {
            Some(TokenKind::Exclamation) => {
                self.bump();
                let val = self.parse_primary()?;
                Ok(if val == 0 { 1 } else { 0 })
            }
            Some(TokenKind::LParen) => {
                self.bump();
                let val = self.parse_expr()?;
                if let Some(TokenKind::RParen) = self.peek() {
                    self.bump();
                    Ok(val)
                } else {
                    Err(AsmError::new(
                        self.span(),
                        AsmErrorKind::InvalidCondition("missing closing ')'".to_string()),
                    ))
                }
            }
            Some(TokenKind::Number(n)) => {
                let v = *n as i64;
                self.bump();
                Ok(v)
            }
            Some(TokenKind::Char(c)) => {
                let v = *c as i64;
                self.bump();
                Ok(v)
            }
            Some(TokenKind::Str(s)) => {
                let v = if s.is_empty() { 0 } else { 1 };
                self.bump();
                Ok(v)
            }
            Some(TokenKind::Ident(s)) => {
                let ident = s.clone();
                self.bump();

                // Check for defined(NAME)
                if ident.eq_ignore_ascii_case("defined") && matches!(self.peek(), Some(TokenKind::LParen)) {
                    self.bump(); // consume '('
                    let name = match self.peek() {
                        Some(TokenKind::Ident(n)) => {
                            let n = n.clone();
                            self.bump();
                            n
                        }
                        _ => {
                            return Err(AsmError::new(
                                self.span(),
                                AsmErrorKind::InvalidCondition(
                                    "expected identifier inside defined(...)".to_string(),
                                ),
                            ));
                        }
                    };
                    if !matches!(self.peek(), Some(TokenKind::RParen)) {
                        return Err(AsmError::new(
                            self.span(),
                            AsmErrorKind::InvalidCondition(
                                "missing ')' after defined(NAME)".to_string(),
                            ),
                        ));
                    }
                    self.bump(); // consume ')'

                    let is_def = self.defines.contains_key(&name.to_ascii_lowercase());
                    return Ok(if is_def { 1 } else { 0 });
                }

                // Normal identifier lookup
                let val = self
                    .defines
                    .get(&ident.to_ascii_lowercase())
                    .copied()
                    .unwrap_or(0);
                Ok(val)
            }
            other => Err(AsmError::new(
                sp,
                AsmErrorKind::InvalidCondition(format!("unexpected token '{other:?}'")),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_inactive_ranges_simple_if_else() {
        let src = r#"%define DEBUG 0
%if DEBUG
    mov a, 1
    mov b, 2
%else
    mov a, 3
%end"#;
        let ranges = find_inactive_ranges(src, None);
        assert_eq!(ranges, vec![(3, 4)]);
    }

    #[test]
    fn test_find_inactive_ranges_elif() {
        let src = r#"%define MODE 2
%if MODE == 1
    mov a, 1
%elif MODE == 2
    mov a, 2
%else
    mov a, 3
%end"#;
        let ranges = find_inactive_ranges(src, None);
        assert_eq!(ranges, vec![(3, 3), (7, 7)]);
    }

    #[test]
    fn test_find_inactive_ranges_valueless_define() {
        let src = r#"%define FEATURE
%if FEATURE
    mov a, 10
%else
    mov a, 20
%end"#;
        let ranges = find_inactive_ranges(src, None);
        assert_eq!(ranges, vec![(5, 5)]);
    }

    #[test]
    fn test_find_inactive_ranges_nested() {
        let src = r#"%define OUTER 1
%define INNER 0
%if OUTER
    mov a, 1
    %if INNER
        mov b, 2
    %end
    mov c, 3
%else
    mov d, 4
%end"#;
        let ranges = find_inactive_ranges(src, None);
        assert_eq!(ranges, vec![(6, 6), (10, 10)]);
    }
}
