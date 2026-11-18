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
    let mut func_defines = HashMap::new();
    let mut visited_includes = HashSet::new();
    preprocess_with_all_defines(
        tokens,
        base_dir,
        &mut defines,
        &mut func_defines,
        &mut visited_includes,
    )
}

/// Preprocess tokens with a shared `defines` map (useful across `%include` trees).
pub fn preprocess_with_defines(
    tokens: Vec<Token>,
    base_dir: Option<&Path>,
    defines: &mut HashMap<String, i64>,
    visited_includes: &mut HashSet<PathBuf>,
) -> Result<Vec<Token>, AsmError> {
    let mut func_defines = HashMap::new();
    preprocess_with_all_defines(
        tokens,
        base_dir,
        defines,
        &mut func_defines,
        visited_includes,
    )
}

/// Preprocess tokens with shared `defines` and `func_defines` maps.
pub fn preprocess_with_all_defines(
    tokens: Vec<Token>,
    base_dir: Option<&Path>,
    defines: &mut HashMap<String, i64>,
    func_defines: &mut HashMap<String, FuncDefine>,
    visited_includes: &mut HashSet<PathBuf>,
) -> Result<Vec<Token>, AsmError> {
    Preprocessor::new(tokens, base_dir, defines, func_defines, visited_includes).run()
}

/// Preprocess tokens and also collect inactive source line ranges `(start_line, end_line)` (1-based).
pub fn preprocess_with_inactive_ranges(
    tokens: Vec<Token>,
    base_dir: Option<&Path>,
) -> Result<(Vec<Token>, Vec<(u32, u32)>), AsmError> {
    let mut defines = HashMap::new();
    let mut func_defines = HashMap::new();
    let mut visited_includes = HashSet::new();
    let mut inactive_ranges = Vec::new();
    let res_tokens = Preprocessor::new_with_collector(
        tokens,
        base_dir,
        &mut defines,
        &mut func_defines,
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

/// A functional `%define Name(param1, ...) body` definition.
#[derive(Debug, Clone)]
pub struct FuncDefine {
    /// Function name.
    pub name: String,
    /// Formal parameter names.
    pub params: Vec<String>,
    /// Tokens comprising the expansion body.
    pub body: Vec<Token>,
    /// Source position where the functional macro was defined.
    pub span: Span,
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
    func_defines: &'a mut HashMap<String, FuncDefine>,
    visited_includes: &'a mut HashSet<PathBuf>,
    block_stack: Vec<BlockKind>,
    inactive_ranges: Option<&'a mut Vec<(u32, u32)>>,
}

impl<'a> Preprocessor<'a> {
    fn new(
        tokens: Vec<Token>,
        base_dir: Option<&Path>,
        defines: &'a mut HashMap<String, i64>,
        func_defines: &'a mut HashMap<String, FuncDefine>,
        visited_includes: &'a mut HashSet<PathBuf>,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            base_dir: base_dir.map(|p| p.to_path_buf()),
            defines,
            func_defines,
            visited_includes,
            block_stack: Vec::new(),
            inactive_ranges: None,
        }
    }

    fn new_with_collector(
        tokens: Vec<Token>,
        base_dir: Option<&Path>,
        defines: &'a mut HashMap<String, i64>,
        func_defines: &'a mut HashMap<String, FuncDefine>,
        visited_includes: &'a mut HashSet<PathBuf>,
        inactive_ranges: Option<&'a mut Vec<(u32, u32)>>,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            base_dir: base_dir.map(|p| p.to_path_buf()),
            defines,
            func_defines,
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
                                    let expanded_cond = self.expand_func_macros_in_tokens(&line_tokens[first_idx + 2..], 0)?;
                                    self.handle_if(
                                        &expanded_cond,
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
                                    let expanded_cond = self.expand_func_macros_in_tokens(&line_tokens[first_idx + 2..], 0)?;
                                    self.handle_elif(&expanded_cond, dir_span)?;
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
                                        let after_define = &line_tokens[first_idx + 2..];
                                        if let Some(func_def) = self.check_func_define(after_define)? {
                                            self.func_defines.insert(func_def.name.to_ascii_lowercase(), func_def);
                                            if let Some(nl) = newline_tok {
                                                out.push(nl);
                                            }
                                            continue;
                                        }

                                        let useful_after: Vec<&Token> = after_define
                                            .iter()
                                            .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
                                            .collect();

                                        if useful_after.is_empty() {
                                            if let Some(nl) = newline_tok {
                                                out.push(nl);
                                            }
                                            continue;
                                        }

                                        let name_tok = useful_after[0].clone();
                                        let value_tokens: Vec<Token> = if useful_after.len() > 1 {
                                            let raw_val: Vec<Token> = useful_after[1..].iter().map(|&t| t.clone()).collect();
                                            self.expand_func_macros_in_tokens(&raw_val, 0)?
                                        } else {
                                            Vec::new()
                                        };

                                        let mut combined = vec![name_tok];
                                        combined.extend(value_tokens);

                                        let evaluated_num = self.record_define(&combined)?;
                                        if let Some(num) = evaluated_num {
                                            let mut rewritten = Vec::new();
                                            rewritten.push(line_tokens[first_idx].clone());
                                            rewritten.push(line_tokens[first_idx + 1].clone());
                                            rewritten.push(combined[0].clone());
                                            let val_span = combined
                                                .get(1)
                                                .map(|t| t.span)
                                                .unwrap_or(line_tokens[first_idx].span);
                                            rewritten.push(Token::new(TokenKind::Number(num as u32), val_span));
                                            if let Some(nl) = newline_tok {
                                                rewritten.push(nl);
                                            }
                                            out.extend(rewritten);
                                        } else {
                                            out.extend(line_tokens);
                                        }
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
                let expanded_line = self.expand_func_macros_in_tokens(&line_tokens, 0)?;
                out.extend(expanded_line);
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

    fn record_define(&mut self, line_tokens: &[Token]) -> Result<Option<i64>, AsmError> {
        let useful: Vec<&Token> = line_tokens
            .iter()
            .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
            .collect();

        if useful.is_empty() {
            return Ok(None);
        }

        if let TokenKind::Ident(ref name) = useful[0].kind {
            let key = name.to_ascii_lowercase();
            if useful.len() == 1 {
                // %define NAME -> defaults to 1
                self.defines.insert(key, 1);
                return Ok(None);
            } else if useful.len() == 2 && matches!(useful[1].kind, TokenKind::Str(_) | TokenKind::Char(_)) {
                if let TokenKind::Str(ref s) = useful[1].kind {
                    let v = if s.is_empty() { 0 } else { 1 };
                    self.defines.insert(key, v);
                } else if let TokenKind::Char(c) = useful[1].kind {
                    self.defines.insert(key, c as i64);
                }
                return Ok(None);
            } else {
                // Value expression following name
                let val_tokens: Vec<Token> = useful[1..].iter().map(|&t| t.clone()).collect();
                let span = val_tokens[0].span;
                let val = crate::asm::expr::eval_expr(
                    &val_tokens,
                    |k| self.defines.get(&k.to_ascii_lowercase()).copied(),
                    span,
                    false,
                )?;
                self.defines.insert(key, val);
                return Ok(Some(val));
            }
        }

        Ok(None)
    }

    fn scan_include(&mut self, line_tokens: &[Token]) -> Result<(), AsmError> {
        let base_dir = match &self.base_dir {
            Some(d) => d.clone(),
            None => return Ok(()),
        };

        // Check for angle-bracket stdlib include: %include <...>
        if let Some(less_idx) = line_tokens.iter().position(|t| matches!(t.kind, TokenKind::Less)) {
            let mut mod_name = String::new();
            for tok in &line_tokens[less_idx + 1..] {
                match &tok.kind {
                    TokenKind::Ident(s) => mod_name.push_str(s),
                    TokenKind::Slash => mod_name.push('/'),
                    TokenKind::Number(n) => mod_name.push_str(&n.to_string()),
                    TokenKind::Period => mod_name.push('.'),
                    TokenKind::Greater => break,
                    _ => {}
                }
            }

            if !mod_name.is_empty() {
                let file_name = if mod_name.ends_with(".e8085") {
                    mod_name.clone()
                } else {
                    format!("{mod_name}.e8085")
                };

                let stdlib_dir = crate::asm::include::find_stdlib_dir(Some(&base_dir));
                let disk_res = if let Some(ref sdir) = stdlib_dir {
                    let cand = sdir.join(&file_name);
                    if cand.exists() {
                        cand.canonicalize().ok().and_then(|c| {
                            std::fs::read_to_string(&c).ok().map(|s| (c, s))
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                let (canon, inc_src) = if let Some((c, s)) = disk_res {
                    (c, s)
                } else if let Some((embed_path, embed_src)) =
                    crate::asm::include::EMBEDDED_STDLIB.iter().find(|(p, _)| *p == file_name || *p == mod_name)
                {
                    let p = stdlib_dir.unwrap_or_else(|| base_dir.join("stdlib")).join(embed_path);
                    (p, embed_src.to_string())
                } else {
                    return Ok(());
                };

                if self.visited_includes.contains(&canon) {
                    return Ok(());
                }
                self.visited_includes.insert(canon.clone());

                let inc_tokens = match lex(&inc_src) {
                    Ok(t) => t,
                    Err(_) => return Ok(()),
                };

                let inc_dir = canon.parent().unwrap_or(&base_dir);
                let _ = preprocess_with_all_defines(
                    inc_tokens,
                    Some(inc_dir),
                    self.defines,
                    self.func_defines,
                    self.visited_includes,
                );
                return Ok(());
            }
        }

        // Quoted string include: %include "..."
        for tok in line_tokens {
            if let TokenKind::Str(ref path_str) = tok.kind {
                let clean_str = path_str.trim_start_matches('/');
                let project_root = crate::asm::include::find_project_root(&base_dir);
                let inc_path = if project_root.join(clean_str).exists() {
                    project_root.join(clean_str)
                } else {
                    base_dir.join(clean_str)
                };

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
                let _ = preprocess_with_all_defines(
                    inc_tokens,
                    Some(inc_dir),
                    self.defines,
                    self.func_defines,
                    self.visited_includes,
                );
            }
        }

        Ok(())
    }

    fn check_func_define(&self, line_tokens: &[Token]) -> Result<Option<FuncDefine>, AsmError> {
        let useful: Vec<&Token> = line_tokens
            .iter()
            .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
            .collect();

        if useful.len() < 2 {
            return Ok(None);
        }

        let name_tok = useful[0];
        let name = match &name_tok.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Ok(None),
        };

        if useful[1].kind != TokenKind::LParen {
            return Ok(None);
        }

        let lparen_tok = useful[1];
        let is_strictly_attached = lparen_tok.span.line == name_tok.span.line
            && lparen_tok.span.col == name_tok.span.col + name.len() as u32;

        // Try to parse parameter list between LParen and matching RParen
        let mut rparen_idx = None;
        let mut depth = 0;
        for (i, tok) in useful.iter().enumerate().skip(1) {
            match tok.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        rparen_idx = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        let rparen_idx = match rparen_idx {
            Some(idx) => idx,
            None => {
                if is_strictly_attached {
                    return Err(AsmError::new(
                        lparen_tok.span,
                        AsmErrorKind::Unexpected {
                            expected: ")".to_string(),
                            found: "end of line".to_string(),
                        },
                    ));
                } else {
                    return Ok(None);
                }
            }
        };

        let inside = &useful[2..rparen_idx];
        let mut params = Vec::new();
        let mut expect_ident = true;
        let mut is_valid_param_list = true;

        if !inside.is_empty() {
            for tok in inside {
                if expect_ident {
                    if let TokenKind::Ident(ref p) = tok.kind {
                        let p_lower = p.to_ascii_lowercase();
                        if params
                            .iter()
                            .any(|existing: &String| existing.to_ascii_lowercase() == p_lower)
                        {
                            return Err(AsmError::new(
                                tok.span,
                                AsmErrorKind::InvalidFuncMacroDef(format!(
                                    "duplicate parameter '{p}' in functional define '{name}'"
                                )),
                            ));
                        }
                        params.push(p.clone());
                        expect_ident = false;
                    } else {
                        is_valid_param_list = false;
                        break;
                    }
                } else {
                    if tok.kind == TokenKind::Comma {
                        expect_ident = true;
                    } else {
                        is_valid_param_list = false;
                        break;
                    }
                }
            }
            if expect_ident {
                is_valid_param_list = false;
            }
        }

        let has_body_after_rparen = useful.len() > rparen_idx + 1;

        if is_valid_param_list && (is_strictly_attached || has_body_after_rparen) {
            let body_tokens: Vec<Token> = useful[rparen_idx + 1..]
                .iter()
                .map(|&t| t.clone())
                .collect();
            Ok(Some(FuncDefine {
                name,
                params,
                body: body_tokens,
                span: name_tok.span,
            }))
        } else if is_strictly_attached && !is_valid_param_list {
            Err(AsmError::new(
                lparen_tok.span,
                AsmErrorKind::InvalidFuncMacroDef(format!(
                    "invalid parameter list in functional define '{name}'"
                )),
            ))
        } else {
            Ok(None)
        }
    }

    fn expand_func_macros_in_tokens(
        &self,
        tokens: &[Token],
        depth: usize,
    ) -> Result<Vec<Token>, AsmError> {
        if depth > 64 {
            let span = tokens.first().map(|t| t.span).unwrap_or_default();
            return Err(AsmError::new(
                span,
                AsmErrorKind::FuncMacroRecursionLimitExceeded("recursion limit exceeded".to_string()),
            ));
        }

        let mut result = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if let TokenKind::Ident(ref name) = tokens[i].kind {
                let key = name.to_ascii_lowercase();
                if let Some(func_def) = self.func_defines.get(&key).cloned() {
                    if i + 1 < tokens.len() && tokens[i + 1].kind == TokenKind::LParen {
                        let call_span = tokens[i].span;
                        // Parse arguments enclosed in ( ... )
                        let mut arg_tokens: Vec<Vec<Token>> = Vec::new();
                        let mut current_arg: Vec<Token> = Vec::new();
                        let mut paren_depth = 1;
                        let mut j = i + 2;
                        let mut closed = false;

                        while j < tokens.len() {
                            let tok = &tokens[j];
                            match tok.kind {
                                TokenKind::LParen => {
                                    paren_depth += 1;
                                    current_arg.push(tok.clone());
                                }
                                TokenKind::RParen => {
                                    paren_depth -= 1;
                                    if paren_depth == 0 {
                                        closed = true;
                                        if !current_arg.is_empty() || !arg_tokens.is_empty() {
                                            arg_tokens.push(current_arg);
                                        }
                                        j += 1;
                                        break;
                                    } else {
                                        current_arg.push(tok.clone());
                                    }
                                }
                                TokenKind::Comma if paren_depth == 1 => {
                                    arg_tokens.push(std::mem::take(&mut current_arg));
                                }
                                TokenKind::Newline | TokenKind::Eof => {
                                    break;
                                }
                                _ => {
                                    current_arg.push(tok.clone());
                                }
                            }
                            j += 1;
                        }

                        if !closed {
                            return Err(AsmError::new(
                                call_span,
                                AsmErrorKind::Unexpected {
                                    expected: ")".to_string(),
                                    found: "end of line".to_string(),
                                },
                            ));
                        }

                        // If any argument contains '%', it might be a macro parameter in a %macro definition.
                        // In that case, do not expand this functional macro call.
                        let has_percent = arg_tokens
                            .iter()
                            .any(|arg| arg.iter().any(|t| t.kind == TokenKind::Percent));
                        if has_percent {
                            result.push(tokens[i].clone());
                            i += 1;
                            continue;
                        }

                        let actual_arg_count = if func_def.params.is_empty() {
                            if arg_tokens.is_empty()
                                || (arg_tokens.len() == 1 && arg_tokens[0].is_empty())
                            {
                                0
                            } else {
                                arg_tokens.len()
                            }
                        } else {
                            arg_tokens.len()
                        };

                        if actual_arg_count != func_def.params.len() {
                            return Err(AsmError::new(
                                call_span,
                                AsmErrorKind::FuncMacroArgCountMismatch {
                                    name: func_def.name.clone(),
                                    expected: func_def.params.len(),
                                    found: actual_arg_count,
                                },
                            ));
                        }

                        // Recursively expand each argument first
                        let mut expanded_args = Vec::new();
                        for arg in arg_tokens {
                            let exp = self.expand_func_macros_in_tokens(&arg, depth + 1)?;
                            expanded_args.push(exp);
                        }

                        // Substitute parameters into func_def.body
                        let mut substituted: Vec<Token> = Vec::new();
                        for btok in &func_def.body {
                            if let TokenKind::Ident(ref id) = btok.kind {
                                let id_lower = id.to_ascii_lowercase();
                                if let Some(param_idx) = func_def
                                    .params
                                    .iter()
                                    .position(|p| p.eq_ignore_ascii_case(&id_lower))
                                {
                                    for atok in &expanded_args[param_idx] {
                                        let mut cloned = atok.clone();
                                        cloned.span = call_span;
                                        substituted.push(cloned);
                                    }
                                    continue;
                                }
                            }
                            let mut cloned = btok.clone();
                            cloned.span = call_span;
                            substituted.push(cloned);
                        }

                        // Recursively expand any functional macros formed in the substituted body
                        let fully_expanded_body =
                            self.expand_func_macros_in_tokens(&substituted, depth + 1)?;

                        // Evaluate comma-separated slices
                        let evaluated =
                            self.eval_comma_separated_slices(&fully_expanded_body, call_span)?;
                        result.extend(evaluated);

                        i = j;
                        continue;
                    }
                }
            }
            result.push(tokens[i].clone());
            i += 1;
        }

        Ok(result)
    }

    fn eval_comma_separated_slices(
        &self,
        tokens: &[Token],
        call_span: Span,
    ) -> Result<Vec<Token>, AsmError> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let mut slices: Vec<Vec<Token>> = Vec::new();
        let mut cur: Vec<Token> = Vec::new();
        let mut depth = 0;

        for tok in tokens {
            match tok.kind {
                TokenKind::LParen => {
                    depth += 1;
                    cur.push(tok.clone());
                }
                TokenKind::RParen => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    cur.push(tok.clone());
                }
                TokenKind::Comma if depth == 0 => {
                    slices.push(std::mem::take(&mut cur));
                }
                _ => {
                    cur.push(tok.clone());
                }
            }
        }
        if !cur.is_empty() || !slices.is_empty() {
            slices.push(cur);
        }

        let mut out = Vec::new();
        for (idx, slice) in slices.into_iter().enumerate() {
            if idx > 0 {
                out.push(Token::new(TokenKind::Comma, call_span));
            }

            let useful: Vec<Token> = slice
                .into_iter()
                .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
                .collect();

            if useful.is_empty() {
                continue;
            }

            if useful.iter().any(|t| t.kind == TokenKind::Percent) {
                out.extend(useful);
                continue;
            }

            let span = useful[0].span;
            let eval_res = crate::asm::expr::eval_expr(
                &useful,
                |k| self.defines.get(&k.to_ascii_lowercase()).copied(),
                span,
                false,
            );

            match eval_res {
                Ok(num) => {
                    out.push(Token::new(TokenKind::Number(num as u32), span));
                }
                Err(e) => {
                    if useful.len() == 1 {
                        match &useful[0].kind {
                            TokenKind::Str(_) | TokenKind::Char(_) | TokenKind::Ident(_) => {
                                out.push(useful[0].clone());
                                continue;
                            }
                            _ => return Err(e),
                        }
                    }
                    return Err(e);
                }
            }
        }

        Ok(out)
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
    crate::asm::expr::eval_expr(
        tokens,
        |k| defines.get(&k.to_ascii_lowercase()).copied(),
        span,
        true,
    )
    .map_err(|e| match e.kind {
        AsmErrorKind::InvalidExpression(s) => AsmError::new(e.span, AsmErrorKind::InvalidCondition(s)),
        _ => e,
    })
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

    #[test]
    fn test_func_define_single_param() {
        let src = "%define Hi(x) (x >> 8)\nmvi B, Hi(0x1234)\n";
        let tokens = lex(src).unwrap();
        let res = preprocess(tokens, None).unwrap();
        let useful: Vec<_> = res.iter().filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof).collect();
        // Expect: mvi, B, Comma, Number(0x12)
        assert_eq!(useful.len(), 4);
        assert_eq!(useful[3].kind, TokenKind::Number(0x12));
    }

    #[test]
    fn test_func_define_multi_param() {
        let src = "%define Add(a, b) a + b\nmvi A, Add(10, 25)\n";
        let tokens = lex(src).unwrap();
        let res = preprocess(tokens, None).unwrap();
        let useful: Vec<_> = res.iter().filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof).collect();
        assert_eq!(useful.len(), 4);
        assert_eq!(useful[3].kind, TokenKind::Number(35));
    }

    #[test]
    fn test_func_define_multi_value_and_referenced_define() {
        let src = "%define RED 0xF00F\n%define Color(x) (x >> 8), (x & 0xFF)\nBYTE Color(RED)\n";
        let tokens = lex(src).unwrap();
        let res = preprocess(tokens, None).unwrap();
        let useful: Vec<_> = res.iter().filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof).collect();
        // %define RED 0xF00F is emitted as scalar define: % define RED 61455
        // Then BYTE 240 , 15
        let byte_pos = useful.iter().position(|t| matches!(t.kind, TokenKind::Ident(ref s) if s.eq_ignore_ascii_case("BYTE"))).unwrap();
        let after_byte = &useful[byte_pos + 1..];
        assert_eq!(after_byte.len(), 3);
        assert_eq!(after_byte[0].kind, TokenKind::Number(0xF0));
        assert_eq!(after_byte[1].kind, TokenKind::Comma);
        assert_eq!(after_byte[2].kind, TokenKind::Number(0x0F));
    }

    #[test]
    fn test_func_define_nested_and_chained() {
        let src = "%define Double(x) x * 2\n%define Quad(x) Double(Double(x))\nmvi A, Quad(5)\n";
        let tokens = lex(src).unwrap();
        let res = preprocess(tokens, None).unwrap();
        let useful: Vec<_> = res.iter().filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof).collect();
        assert_eq!(useful.len(), 4);
        assert_eq!(useful[3].kind, TokenKind::Number(20));
    }

    #[test]
    fn test_func_define_in_scalar_define() {
        let src = "%define RED 0xF00F\n%define Hi(x) (x >> 8)\n%define RED_HI Hi(RED)\nmvi A, RED_HI\n";
        let tokens = lex(src).unwrap();
        let res = preprocess(tokens, None).unwrap();
        let useful: Vec<_> = res.iter().filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof).collect();
        // The last line is mvi A, RED_HI
        let mvi_pos = useful.iter().position(|t| matches!(t.kind, TokenKind::Ident(ref s) if s.eq_ignore_ascii_case("mvi"))).unwrap();
        assert_eq!(useful[mvi_pos + 3].kind, TokenKind::Ident("RED_HI".to_string()));
    }

    #[test]
    fn test_func_define_in_conditional() {
        let src = "%define RED 0xF00F\n%define Hi(x) (x >> 8)\n%if Hi(RED) == 0xF0\nmvi A, 1\n%else\nmvi A, 0\n%endif\n";
        let tokens = lex(src).unwrap();
        let res = preprocess(tokens, None).unwrap();
        let useful: Vec<_> = res.iter().filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof).collect();
        // Expect mvi A, 1 to be active and mvi A, 0 to be filtered out
        let one_count = useful.iter().filter(|t| t.kind == TokenKind::Number(1)).count();
        let zero_count = useful.iter().filter(|t| t.kind == TokenKind::Number(0)).count();
        assert_eq!(one_count, 1);
        assert_eq!(zero_count, 0);
    }

    #[test]
    fn test_func_define_errors() {
        // Arg count mismatch
        let src1 = "%define Add(a, b) a + b\nmvi A, Add(1)\n";
        let tokens1 = lex(src1).unwrap();
        assert!(matches!(
            preprocess(tokens1, None).unwrap_err().kind,
            AsmErrorKind::FuncMacroArgCountMismatch { expected: 2, found: 1, .. }
        ));

        // Division by zero in func call
        let src2 = "%define Div(a, b) a / b\nmvi A, Div(10, 0)\n";
        let tokens2 = lex(src2).unwrap();
        assert_eq!(
            preprocess(tokens2, None).unwrap_err().kind,
            AsmErrorKind::DivisionByZero
        );

        // Duplicate parameter name
        let src3 = "%define Dup(x, x) x + x\n";
        let tokens3 = lex(src3).unwrap();
        assert!(matches!(
            preprocess(tokens3, None).unwrap_err().kind,
            AsmErrorKind::InvalidFuncMacroDef(_)
        ));

        // Recursion limit exceeded
        let src4 = "%define Loop(x) Loop(x)\nmvi A, Loop(1)\n";
        let tokens4 = lex(src4).unwrap();
        assert!(matches!(
            preprocess(tokens4, None).unwrap_err().kind,
            AsmErrorKind::FuncMacroRecursionLimitExceeded(_)
        ));
    }
}
