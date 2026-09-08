//! Macro expander: gathers macro definitions, validates collisions/arguments, and expands calls.

use std::collections::HashMap;

use super::ast::{MacroDef, POperand, Program, Segment, TextItem};
use super::error::{AsmError, AsmErrorKind, Span};
use super::keyword;
use super::parser;
use super::token::{Token, TokenKind};

const MAX_MACRO_DEPTH: usize = 64;

/// Collect all macro definitions, check for collisions, validate parameter references,
/// and expand all macro invocations in `.text` segments.
pub fn expand_macros(program: &mut Program) -> Result<(), AsmError> {
    // 1. Gather all macros from top level and .text segments
    let mut macro_table: HashMap<String, MacroDef> = HashMap::new();
    let mut all_macro_defs: Vec<MacroDef> = Vec::new();

    for def in &program.macros {
        let key = def.name.to_ascii_lowercase();
        if macro_table.contains_key(&key) {
            return Err(AsmError::new(
                def.span,
                AsmErrorKind::DuplicateMacro(def.name.clone()),
            ));
        }
        macro_table.insert(key, def.clone());
        all_macro_defs.push(def.clone());
    }

    for seg in &program.segments {
        if let Segment::Text(items) = seg {
            for item in items {
                if let TextItem::MacroDef(def) = item {
                    let key = def.name.to_ascii_lowercase();
                    if macro_table.contains_key(&key) {
                        return Err(AsmError::new(
                            def.span,
                            AsmErrorKind::DuplicateMacro(def.name.clone()),
                        ));
                    }
                    macro_table.insert(key, def.clone());
                    all_macro_defs.push(def.clone());
                }
            }
        }
    }

    if macro_table.is_empty() {
        return Ok(());
    }

    // 2. Validate collisions against instructions/keywords, defines, variables, and labels
    for def in &all_macro_defs {
        if keyword::is_reserved(&def.name) {
            return Err(AsmError::new(
                def.span,
                AsmErrorKind::MacroCollision {
                    name: def.name.clone(),
                    collision_with: "instruction".to_string(),
                },
            ));
        }

        if program
            .defines
            .iter()
            .any(|d| d.name.eq_ignore_ascii_case(&def.name))
        {
            return Err(AsmError::new(
                def.span,
                AsmErrorKind::MacroCollision {
                    name: def.name.clone(),
                    collision_with: "constant".to_string(),
                },
            ));
        }

        for seg in &program.segments {
            match seg {
                Segment::Data(defs) => {
                    if defs.iter().any(|d| d.name.eq_ignore_ascii_case(&def.name)) {
                        return Err(AsmError::new(
                            def.span,
                            AsmErrorKind::MacroCollision {
                                name: def.name.clone(),
                                collision_with: "variable".to_string(),
                            },
                        ));
                    }
                }
                Segment::Bss(decls) => {
                    if decls.iter().any(|b| b.name.eq_ignore_ascii_case(&def.name)) {
                        return Err(AsmError::new(
                            def.span,
                            AsmErrorKind::MacroCollision {
                                name: def.name.clone(),
                                collision_with: "variable".to_string(),
                            },
                        ));
                    }
                }
                Segment::Text(items) => {
                    for item in items {
                        match item {
                            TextItem::Label(l, _) | TextItem::GlobalLabel(l, _) => {
                                if l.eq_ignore_ascii_case(&def.name) {
                                    return Err(AsmError::new(
                                        def.span,
                                        AsmErrorKind::MacroCollision {
                                            name: def.name.clone(),
                                            collision_with: "label".to_string(),
                                        },
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // 3. Validate parameter references in macro definition lines (%1 .. %N)
        for line in &def.body_lines {
            let mut i = 0;
            while i < line.tokens.len() {
                if line.tokens[i].kind == TokenKind::Percent && i + 1 < line.tokens.len() {
                    if let TokenKind::Number(k) = line.tokens[i + 1].kind {
                        let idx = k as usize;
                        if idx == 0 || idx > def.arg_count {
                            return Err(AsmError::new(
                                line.tokens[i].span,
                                AsmErrorKind::InvalidMacroArgRef {
                                    index: idx,
                                    max: def.arg_count,
                                },
                            ));
                        }
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }
        }
    }

    // 4. Update program.macros with all gathered macros
    program.macros = all_macro_defs;

    // 5. Expand macro invocations in .text segments
    for seg in &mut program.segments {
        if let Segment::Text(items) = seg {
            let old_items = std::mem::take(items);
            let new_items = expand_text_items(old_items, &macro_table, 0)?;
            *items = new_items;
        }
    }

    Ok(())
}

fn expand_text_items(
    items: Vec<TextItem>,
    macro_table: &HashMap<String, MacroDef>,
    depth: usize,
) -> Result<Vec<TextItem>, AsmError> {
    let mut out = Vec::new();

    for item in items {
        match item {
            TextItem::MacroDef(_) => {
                // Inline macro definition: consumed, not emitted as an instruction
            }
            TextItem::Instr(ins) => {
                let lower = ins.mnemonic.to_ascii_lowercase();
                if let Some(macro_def) = macro_table.get(&lower) {
                    if depth > MAX_MACRO_DEPTH {
                        return Err(AsmError::new(
                            ins.span,
                            AsmErrorKind::MacroRecursionLimitExceeded(macro_def.name.clone()),
                        ));
                    }

                    // Validate argument count
                    if ins.operands.len() != macro_def.arg_count {
                        return Err(AsmError::new(
                            ins.span,
                            AsmErrorKind::MacroArgCountMismatch {
                                name: macro_def.name.clone(),
                                expected: macro_def.arg_count,
                                found: ins.operands.len(),
                            },
                        ));
                    }

                    // Validate argument kinds: only variables, constants, and literals allowed
                    for op in &ins.operands {
                        match op {
                            POperand::Num(_)
                            | POperand::Char(_)
                            | POperand::Sym(_)
                            | POperand::Len(_) => {}
                            POperand::Reg8(r) => {
                                return Err(AsmError::new(
                                    ins.span,
                                    AsmErrorKind::InvalidMacroArg(format!("{r:?}")),
                                ));
                            }
                            POperand::Reg16(r) => {
                                return Err(AsmError::new(
                                    ins.span,
                                    AsmErrorKind::InvalidMacroArg(format!("{r:?}")),
                                ));
                            }
                            POperand::LocalSym(s) => {
                                return Err(AsmError::new(
                                    ins.span,
                                    AsmErrorKind::InvalidMacroArg(format!(".{s}")),
                                ));
                            }
                        }
                    }

                    // Expand body lines
                    let mut unexpanded_body_items = Vec::new();
                    for line in &macro_def.body_lines {
                        let sub_tokens =
                            substitute_tokens(&line.tokens, &ins.operands, ins.span)?;
                        let parsed_item = parser::parse_text_item(sub_tokens)?;
                        unexpanded_body_items.push(parsed_item);
                    }

                    // Recursively expand nested macro calls
                    let expanded_items =
                        expand_text_items(unexpanded_body_items, macro_table, depth + 1)?;
                    let raw_args: Vec<String> = ins.operands.iter().map(operand_to_text).collect();
                    out.push(TextItem::MacroCall(ins.mnemonic.clone(), raw_args, ins.span));
                    out.extend(expanded_items);
                    out.push(TextItem::MacroEnd);
                } else {
                    out.push(TextItem::Instr(ins));
                }
            }
            other => {
                out.push(other);
            }
        }
    }

    Ok(out)
}

fn substitute_tokens(
    tokens: &[Token],
    args: &[POperand],
    _call_span: Span,
) -> Result<Vec<Token>, AsmError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].kind == TokenKind::Percent && i + 1 < tokens.len() {
            if let TokenKind::Number(k) = tokens[i + 1].kind {
                let idx = k as usize;
                if idx == 0 || idx > args.len() {
                    return Err(AsmError::new(
                        tokens[i].span,
                        AsmErrorKind::InvalidMacroArgRef {
                            index: idx,
                            max: args.len(),
                        },
                    ));
                }
                let arg_op = &args[idx - 1];
                let arg_tokens = operand_to_tokens(arg_op, tokens[i].span);
                out.extend(arg_tokens);
                i += 2;
                continue;
            }
        }
        let t = tokens[i].clone();
        out.push(t);
        i += 1;
    }
    Ok(out)
}

fn operand_to_tokens(op: &POperand, span: Span) -> Vec<Token> {
    match op {
        POperand::Num(n) => vec![Token::new(TokenKind::Number(*n), span)],
        POperand::Char(c) => vec![Token::new(TokenKind::Char(*c), span)],
        POperand::Sym(s) => vec![Token::new(TokenKind::Ident(s.clone()), span)],
        POperand::Len(s) => vec![
            Token::new(TokenKind::Percent, span),
            Token::new(TokenKind::Ident("len".to_string()), span),
            Token::new(TokenKind::Ident(s.clone()), span),
        ],
        POperand::LocalSym(s) => vec![
            Token::new(TokenKind::Period, span),
            Token::new(TokenKind::Ident(s.clone()), span),
        ],
        POperand::Reg8(r) => vec![Token::new(TokenKind::Ident(format!("{r:?}")), span)],
        POperand::Reg16(r) => vec![Token::new(TokenKind::Ident(format!("{r:?}")), span)],
    }
}

fn operand_to_text(op: &POperand) -> String {
    match op {
        POperand::Num(n) => format!("{n}"),
        POperand::Char(c) => format!("'{c}'"),
        POperand::Sym(s) => s.clone(),
        POperand::Len(s) => format!("%len {s}"),
        POperand::LocalSym(s) => format!(".{s}"),
        POperand::Reg8(r) => format!("{r:?}"),
        POperand::Reg16(r) => format!("{r:?}"),
    }
}

