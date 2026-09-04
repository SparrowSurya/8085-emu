use std::collections::{HashMap, HashSet};
use std::path::Path;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, Position, Range};

use super::document::Document;
use crate::asm::ast::{Program, Segment, TextItem};
use crate::asm::parse;

/// Analyzes an `.e8085` source document and returns compiler error and static analysis diagnostics.
pub fn compute_diagnostics(doc: &Document) -> Vec<Diagnostic> {
    compute_diagnostics_with_externs(doc, &HashMap::new())
}

/// Analyzes an `.e8085` source document with optional extra external symbols (e.g. from linked containers).
pub fn compute_diagnostics_with_externs(
    doc: &Document,
    extra_externs: &HashMap<String, u16>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let main_file = doc.uri.to_file_path().ok();
    let base_dir = main_file.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let base_ref = base_dir.as_deref().unwrap_or_else(|| Path::new("."));
    let project_root = main_file
        .as_ref()
        .map(|p| crate::asm::include::find_project_root(p))
        .unwrap_or_else(|| crate::asm::include::find_project_root(base_ref));

    // Collect all declared `extern <name>` symbols so references to external functions do not produce false errors
    let mut extern_symbols = extra_externs.clone();
    let mut parsed_program: Option<Program> = None;

    if let Ok(tokens) = crate::asm::lex(&doc.text) {
        if let Ok(program) = parse(tokens) {
            for ext in &program.externs {
                extern_symbols.insert(ext.clone(), 0x8000);
            }
            for seg in &program.segments {
                if let Segment::Text(items) = seg {
                    for item in items {
                        if let TextItem::ExternDecl(name, _) = item {
                            extern_symbols.insert(name.clone(), 0x8000);
                        }
                    }
                }
            }

            // Validate top-level includes immediately (self-include, duplicate-include, outside root)
            if validate_top_level_includes(
                doc,
                &program,
                main_file.as_deref(),
                base_ref,
                &project_root,
                &mut diagnostics,
            ) {
                return diagnostics;
            }

            parsed_program = Some(program);
        }
    }

    // Also fallback scan lines for `extern <ident>`
    for line in doc.text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("extern ") {
            let name = rest.trim();
            if !name.is_empty() {
                extern_symbols.insert(name.to_string(), 0x8000);
            }
        }
    }

    // 1. Run assembler front-end in analysis mode with declared extern symbols
    if let Err(err) = crate::asm::assemble_with_full_context(
        &doc.text,
        main_file.as_deref(),
        Some(base_ref),
        Some(&project_root),
        &extern_symbols,
    ) {
        let line = err.span.line.saturating_sub(1);
        let col = err.span.col.saturating_sub(1);

        let end_col = if let Some(doc_line) = doc.text.lines().nth(line as usize) {
            (col + 1).max(doc_line.len() as u32)
        } else {
            col + 1
        };

        let related_information = if let crate::asm::AsmErrorKind::DuplicateDefinition {
            ref name,
            first_defined,
        } = err.kind
        {
            let f_line = first_defined.line.saturating_sub(1);
            let f_col = first_defined.col.saturating_sub(1);
            Some(vec![tower_lsp::lsp_types::DiagnosticRelatedInformation {
                location: tower_lsp::lsp_types::Location {
                    uri: doc.uri.clone(),
                    range: Range {
                        start: Position {
                            line: f_line,
                            character: f_col,
                        },
                        end: Position {
                            line: f_line,
                            character: f_col + name.len() as u32,
                        },
                    },
                },
                message: format!("symbol '{name}' was first defined here"),
            }])
        } else {
            None
        };

        diagnostics.push(Diagnostic {
            range: Range {
                start: Position {
                    line,
                    character: col,
                },
                end: Position {
                    line,
                    character: end_col,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("e8085".to_string()),
            message: err.kind.to_string(),
            related_information,
            tags: None,
            data: None,
        });

        // Fatal compilation error: stop further static analysis
        return diagnostics;
    }

    // 2. Static Analysis: CFG Reachability, Halt Check, Unused Variables, Unused Labels
    if let Some(program) = parsed_program {
        run_static_analysis(doc, &program, &mut diagnostics);
    }

    diagnostics
}

fn validate_top_level_includes(
    doc: &Document,
    program: &Program,
    main_file: Option<&Path>,
    base_dir: &Path,
    project_root: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut has_error = false;
    let mut seen_includes: HashSet<std::path::PathBuf> = HashSet::new();
    let canon_main = main_file.map(crate::asm::include::canonicalize_or_norm);

    for inc in &program.includes {
        let inc_path = base_dir.join(&inc.path);

        // 1. Outside project root
        if crate::asm::include::is_outside_root(&inc_path, project_root) {
            diagnostics.push(make_include_diagnostic(
                doc,
                inc.span,
                format!("cannot include '{}': path is outside project root", inc.path),
            ));
            has_error = true;
            continue;
        }

        // 2. Existence and canonicalization
        let canon_inc = match inc_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                diagnostics.push(make_include_diagnostic(
                    doc,
                    inc.span,
                    format!("cannot read '{}': {e}", inc.path),
                ));
                has_error = true;
                continue;
            }
        };

        // Re-check canonicalized path against root
        if crate::asm::include::is_outside_root(&canon_inc, project_root) {
            diagnostics.push(make_include_diagnostic(
                doc,
                inc.span,
                format!("cannot include '{}': path is outside project root", inc.path),
            ));
            has_error = true;
            continue;
        }

        // 3. Self-inclusion
        if let Some(ref main) = canon_main {
            if &canon_inc == main {
                diagnostics.push(make_include_diagnostic(
                    doc,
                    inc.span,
                    format!("file cannot include itself: '{}'", inc.path),
                ));
                has_error = true;
                continue;
            }
        }

        // 4. Duplicate include
        if seen_includes.contains(&canon_inc) {
            diagnostics.push(make_include_diagnostic(
                doc,
                inc.span,
                format!("duplicate include: file '{}' has already been included", inc.path),
            ));
            has_error = true;
            continue;
        }

        seen_includes.insert(canon_inc);
    }

    has_error
}

fn make_include_diagnostic(doc: &Document, span: crate::asm::Span, message: String) -> Diagnostic {
    let line = span.line.saturating_sub(1);
    let col = span.col.saturating_sub(1);
    let end_col = if let Some(doc_line) = doc.text.lines().nth(line as usize) {
        (col + 1).max(doc_line.len() as u32)
    } else {
        col + 1
    };

    Diagnostic {
        range: Range {
            start: Position {
                line,
                character: col,
            },
            end: Position {
                line,
                character: end_col,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("e8085".to_string()),
        message: format!("include error: {message}"),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn run_static_analysis(doc: &Document, program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    // 1. Unused Variables in .data and .bss
    check_unused_variables(doc, program, diagnostics);

    // 2. Unused Labels
    check_unused_labels(doc, program, diagnostics);

    // 3. Control Flow Graph & Halt Analysis
    check_cfg_and_halt(doc, program, diagnostics);
}

fn check_unused_variables(_doc: &Document, program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut declared_vars = Vec::new();

    for seg in &program.segments {
        match seg {
            Segment::Data(defs) => {
                for d in defs {
                    declared_vars.push((d.name.clone(), d.span));
                }
            }
            Segment::Bss(decls) => {
                for b in decls {
                    declared_vars.push((b.name.clone(), b.span));
                }
            }
            _ => {}
        }
    }

    if declared_vars.is_empty() {
        return;
    }

    // Check if variable name is referenced in .text, .data, or defines
    for (var_name, span) in declared_vars {
        let mut is_used = false;

        for seg in &program.segments {
            match seg {
                Segment::Text(items) => {
                    for item in items {
                        if let TextItem::Instr(ins) = item {
                            for op in &ins.operands {
                                match op {
                                    crate::asm::ast::POperand::Sym(s) if s == &var_name => {
                                        is_used = true;
                                        break;
                                    }
                                    crate::asm::ast::POperand::Len(s) if s == &var_name => {
                                        is_used = true;
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if is_used {
                            break;
                        }
                    }
                }
                Segment::Data(defs) => {
                    for d in defs {
                        if d.name != var_name {
                            for val in &d.values {
                                if value_references_symbol(val, &var_name) {
                                    is_used = true;
                                    break;
                                }
                            }
                        }
                        if is_used {
                            break;
                        }
                    }
                }
                _ => {}
            }
            if is_used {
                break;
            }
        }

        if !is_used {
            for d in &program.defines {
                if value_references_symbol(&d.value, &var_name) {
                    is_used = true;
                    break;
                }
            }
        }

        if !is_used {
            let line = span.line.saturating_sub(1);
            let col = span.col.saturating_sub(1);
            let end_col = col + var_name.len() as u32;

            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line,
                        character: col,
                    },
                    end: Position {
                        line,
                        character: end_col,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("e8085".to_string()),
                message: format!("variable '{var_name}' is declared but never used"),
                related_information: None,
                tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                data: None,
            });
        }
    }
}

fn check_unused_labels(_doc: &Document, program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut candidate_labels = Vec::new(); // (scoped_name, display_name, span)

    let mut current_parent: Option<String> = None;
    for seg in &program.segments {
        if let Segment::Text(items) = seg {
            for item in items {
                match item {
                    TextItem::Label(name, span) => {
                        current_parent = Some(name.clone());
                        if !name.eq_ignore_ascii_case("main")
                            && !is_vector_hook(name)
                            && !program.globals.contains(name)
                        {
                            candidate_labels.push((name.clone(), name.clone(), *span));
                        }
                    }
                    TextItem::GlobalLabel(name, _) => {
                        current_parent = Some(name.clone());
                    }
                    TextItem::LocalLabel(name, span) => {
                        if let Some(parent) = &current_parent {
                            let scoped = format!("{parent}.{name}");
                            candidate_labels.push((scoped, format!(".{name}"), *span));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if candidate_labels.is_empty() {
        return;
    }

    for (scoped_name, display_name, span) in candidate_labels {
        let mut is_referenced = false;

        let local_suffix = display_name.strip_prefix('.').unwrap_or(&display_name);

        for seg in &program.segments {
            match seg {
                Segment::Text(items) => {
                    let mut current_parent: Option<String> = None;
                    for item in items {
                        match item {
                            TextItem::Label(name, _) | TextItem::GlobalLabel(name, _) => {
                                current_parent = Some(name.clone());
                            }
                            TextItem::Instr(ins) => {
                                for op in &ins.operands {
                                    match op {
                                        crate::asm::ast::POperand::Sym(s) => {
                                            if s == &scoped_name
                                                || s == &display_name
                                                || s == local_suffix
                                            {
                                                is_referenced = true;
                                                break;
                                            }
                                            if let Some(parent) = &current_parent {
                                                if format!("{parent}.{s}") == scoped_name {
                                                    is_referenced = true;
                                                    break;
                                                }
                                            }
                                        }
                                        crate::asm::ast::POperand::LocalSym(s) => {
                                            if let Some(parent) = &current_parent {
                                                if format!("{parent}.{s}") == scoped_name {
                                                    is_referenced = true;
                                                    break;
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                        if is_referenced {
                            break;
                        }
                    }
                }
                Segment::Data(defs) => {
                    for d in defs {
                        for val in &d.values {
                            if value_references_symbol(val, &scoped_name)
                                || value_references_symbol(val, &display_name)
                                || value_references_symbol(val, local_suffix)
                            {
                                is_referenced = true;
                                break;
                            }
                        }
                        if is_referenced {
                            break;
                        }
                    }
                }
                _ => {}
            }
            if is_referenced {
                break;
            }
        }

        if !is_referenced {
            let line = span.line.saturating_sub(1);
            let col = span.col.saturating_sub(1);
            let end_col = col + display_name.len() as u32;

            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line,
                        character: col,
                    },
                    end: Position {
                        line,
                        character: end_col,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("e8085".to_string()),
                message: format!("label '{display_name}' is defined but never used"),
                related_information: None,
                tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                data: None,
            });
        }
    }
}

fn check_cfg_and_halt(doc: &Document, program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut text_items = Vec::new();
    let mut label_to_idx = HashMap::new();
    let mut has_main = false;
    let mut main_span = None;
    let mut main_idx = None;
    let mut entry_indices = Vec::new();

    let mut current_parent: Option<String> = None;

    for seg in &program.segments {
        if let Segment::Text(items) = seg {
            for item in items {
                let idx = text_items.len();
                text_items.push(item.clone());

                match item {
                    TextItem::Label(name, span) => {
                        current_parent = Some(name.clone());
                        label_to_idx.insert(name.clone(), idx);
                        if name.eq_ignore_ascii_case("main") {
                            has_main = true;
                            main_span = Some(*span);
                            main_idx = Some(idx);
                        } else if is_vector_hook(name) {
                            entry_indices.push(idx);
                        }
                    }
                    TextItem::GlobalLabel(name, _) => {
                        current_parent = Some(name.clone());
                        label_to_idx.insert(name.clone(), idx);
                        entry_indices.push(idx);
                    }
                    TextItem::LocalLabel(name, _) => {
                        if let Some(parent) = &current_parent {
                            label_to_idx.insert(format!("{parent}.{name}"), idx);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if text_items.is_empty() {
        return;
    }

    // Determine entry points
    if has_main {
        if let Some(m_idx) = main_idx {
            entry_indices.push(m_idx);
        }
    } else {
        // Library mode: treat all global and top-level labels as entry points
        for (name, &idx) in &label_to_idx {
            if !name.contains('.') {
                entry_indices.push(idx);
            }
        }
    }

    // BFS Reachability traversal
    let mut reachable = HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    let mut reaches_halt = false;

    for entry in entry_indices {
        queue.push_back(entry);
    }

    while let Some(idx) = queue.pop_front() {
        if idx >= text_items.len() || !reachable.insert(idx) {
            continue;
        }

        match &text_items[idx] {
            TextItem::Label(_, _)
            | TextItem::GlobalLabel(_, _)
            | TextItem::LocalLabel(_, _)
            | TextItem::GlobalDecl(_, _)
            | TextItem::ExternDecl(_, _) => {
                // Fall through to next item
                if idx + 1 < text_items.len() {
                    queue.push_back(idx + 1);
                }
            }
            TextItem::Instr(ins) => {
                let m = ins.mnemonic.to_uppercase();

                if m == "HLT" {
                    reaches_halt = true;
                    // HLT terminates control flow
                } else if m == "JMP" {
                    // Unconditional jump: target only
                    if let Some(target_idx) =
                        resolve_instr_target(ins, &text_items, idx, &label_to_idx)
                    {
                        if target_idx == idx {
                            // Infinite loop (jmp self) is a valid halt equivalent
                            reaches_halt = true;
                        } else {
                            queue.push_back(target_idx);
                        }
                    }
                } else if m == "RET" || m == "PCHL" {
                    // Unconditional return terminates flow in caller scope
                } else if is_conditional_jump(&m) {
                    // Fall through + branch target
                    if idx + 1 < text_items.len() {
                        queue.push_back(idx + 1);
                    }
                    if let Some(target_idx) =
                        resolve_instr_target(ins, &text_items, idx, &label_to_idx)
                    {
                        queue.push_back(target_idx);
                    }
                } else if is_call_instruction(&m) {
                    // Call jumps to subroutine AND returns to next instruction
                    if idx + 1 < text_items.len() {
                        queue.push_back(idx + 1);
                    }
                    if let Some(target_idx) =
                        resolve_instr_target(ins, &text_items, idx, &label_to_idx)
                    {
                        queue.push_back(target_idx);
                    }
                } else {
                    // Sequential instruction falls through
                    if idx + 1 < text_items.len() {
                        queue.push_back(idx + 1);
                    }
                }
            }
        }
    }

    // 1. Report unreachable instructions as dead code
    for (idx, item) in text_items.iter().enumerate() {
        if let TextItem::Instr(ins) = item {
            if !reachable.contains(&idx) {
                let line = ins.span.line.saturating_sub(1);
                let col = ins.span.col.saturating_sub(1);
                let end_col = if let Some(doc_line) = doc.text.lines().nth(line as usize) {
                    (col + ins.mnemonic.len() as u32).max(doc_line.len() as u32)
                } else {
                    col + 1
                };

                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line,
                            character: col,
                        },
                        end: Position {
                            line,
                            character: end_col,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: Some("e8085".to_string()),
                    message: "unreachable code".to_string(),
                    related_information: None,
                    tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                    data: None,
                });
            }
        }
    }

    // 2. Halt Check: Only when `main` is defined
    if has_main && !reaches_halt {
        let span = main_span.unwrap_or_default();
        let line = span.line.saturating_sub(1);
        let col = span.col.saturating_sub(1);

        diagnostics.push(Diagnostic {
            range: Range {
                start: Position {
                    line,
                    character: col,
                },
                end: Position {
                    line,
                    character: col + 4,
                },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("e8085".to_string()),
            message: "program entry point 'main' does not terminate with an 'hlt' instruction"
                .to_string(),
            related_information: None,
            tags: None,
            data: None,
        });
    }
}

fn resolve_instr_target(
    ins: &crate::asm::ast::Instr,
    text_items: &[TextItem],
    curr_idx: usize,
    label_to_idx: &HashMap<String, usize>,
) -> Option<usize> {
    // Find enclosing parent label
    let mut parent = None;
    for i in (0..=curr_idx).rev() {
        if let TextItem::Label(name, _) | TextItem::GlobalLabel(name, _) = &text_items[i] {
            parent = Some(name.clone());
            break;
        }
    }

    for op in &ins.operands {
        match op {
            crate::asm::ast::POperand::Sym(s) => {
                if let Some(&idx) = label_to_idx.get(s) {
                    return Some(idx);
                }
                if let Some(p) = &parent {
                    let scoped = format!("{p}.{s}");
                    if let Some(&idx) = label_to_idx.get(&scoped) {
                        return Some(idx);
                    }
                }
            }
            crate::asm::ast::POperand::LocalSym(s) => {
                if let Some(p) = &parent {
                    let scoped = format!("{p}.{s}");
                    if let Some(&idx) = label_to_idx.get(&scoped) {
                        return Some(idx);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn is_conditional_jump(m: &str) -> bool {
    matches!(m, "JZ" | "JNZ" | "JC" | "JNC" | "JP" | "JM" | "JPE" | "JPO")
}

fn is_call_instruction(m: &str) -> bool {
    matches!(
        m,
        "CALL" | "CZ" | "CNZ" | "CC" | "CNC" | "CP" | "CM" | "CPE" | "CPO"
    )
}

fn is_vector_hook(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "isr_rst1"
            | "rst1_isr"
            | "isr_rst_1"
            | "isr_rst2"
            | "rst2_isr"
            | "isr_rst_2"
            | "isr_rst3"
            | "rst3_isr"
            | "isr_rst_3"
            | "isr_rst4"
            | "rst4_isr"
            | "isr_rst_4"
            | "isr_trap"
            | "trap_isr"
            | "isr_trap_handler"
            | "isr_rst5"
            | "rst5_isr"
            | "isr_rst_5"
            | "isr_rst55"
            | "rst55_isr"
            | "isr_rst_5_5"
            | "isr_rst5_5"
            | "isr_rst6"
            | "rst6_isr"
            | "isr_rst_6"
            | "isr_rst65"
            | "rst65_isr"
            | "isr_rst_6_5"
            | "isr_rst6_5"
            | "isr_rst7"
            | "rst7_isr"
            | "isr_rst_7"
            | "isr_rst75"
            | "rst75_isr"
            | "isr_rst_7_5"
            | "isr_rst7_5"
    )
}

fn value_references_symbol(val: &crate::asm::ast::Value, sym: &str) -> bool {
    use crate::asm::ast::Value;
    match val {
        Value::Ident(s) | Value::Len(s) => s == sym,
        Value::Repeat { count, value } => {
            value_references_symbol(count, sym) || value_references_symbol(value, sym)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_clean_document_has_no_diagnostics() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "segment .text\nmain:\n    mvi A, 0x05\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert!(
            diags.is_empty(),
            "clean doc should have no diagnostics: {:?}",
            diags
        );
    }

    #[test]
    fn test_extern_label_produces_no_diagnostic() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text =
            "extern my_label\nsegment .text\nmain:\n    call my_label\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert!(
            diags.is_empty(),
            "declared extern symbol should not produce diagnostic errors: {:?}",
            diags
        );
    }

    #[test]
    fn test_undeclared_undefined_label_produces_diagnostic() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "segment .text\nmain:\n    call undefined_label\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("undefined name"));
    }

    #[test]
    fn test_syntax_error_produces_diagnostic() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "segment .text\nmain:\n    mvi\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].range.start.line, 2);
    }

    #[test]
    fn test_empty_document_produces_missing_text_segment_diagnostic() {
        let uri = Url::parse("file:///empty.e8085").unwrap();
        let text = "".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("missing text segment"));
    }

    #[test]
    fn test_unreachable_code_diagnostic() {
        let uri = Url::parse("file:///dead_code.e8085").unwrap();
        let text = "segment .text\nmain:\n    hlt\n    mov A, B\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert!(diags.iter().any(|d| d.message.contains("unreachable code")));
    }

    #[test]
    fn test_main_missing_hlt_diagnostic() {
        let uri = Url::parse("file:///no_hlt.e8085").unwrap();
        let text = "segment .text\nmain:\n    mov A, B\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert!(diags.iter().any(|d| {
            d.message
                .contains("does not terminate with an 'hlt' instruction")
        }));
    }

    #[test]
    fn test_library_without_main_no_halt_diagnostic() {
        let uri = Url::parse("file:///lib.e8085").unwrap();
        let text = "segment .text\nglobal my_sub:\n    mov A, B\n    ret\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert!(!diags.iter().any(|d| d.message.contains("hlt")));
    }

    #[test]
    fn test_unused_variable_diagnostic() {
        let uri = Url::parse("file:///unused_var.e8085").unwrap();
        let text = "segment .data\nunused_buf BYTE 10\nsegment .text\nmain:\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert!(diags.iter().any(|d| {
            d.message
                .contains("variable 'unused_buf' is declared but never used")
        }));
    }

    #[test]
    fn test_unused_local_label_diagnostic() {
        let uri = Url::parse("file:///unused_label.e8085").unwrap();
        let text = "segment .text\nmain:\n.unused_loop:\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert!(diags.iter().any(|d| {
            d.message
                .contains("label '.unused_loop' is defined but never used")
        }));
    }

    #[test]
    fn test_duplicate_definition_reports_first_defined_location() {
        let uri = Url::parse("file:///dup.e8085").unwrap();
        let text = "segment .data\nmy_var BYTE 1\nmy_var BYTE 2\nsegment .text\nmain:\n    hlt\n"
            .to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0]
                .message
                .contains("duplicate definition of 'my_var' (first defined at 2:1)")
        );
        assert!(diags[0].related_information.is_some());
        let rel = diags[0].related_information.as_ref().unwrap();
        assert_eq!(rel.len(), 1);
        assert_eq!(rel[0].location.range.start.line, 1);
    }

    #[test]
    fn test_triangle_pattern_program_has_no_diagnostics() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
        let triangle_file = workspace.join("programs/triangle_pattern.e8085");
        if let Ok(text) = std::fs::read_to_string(&triangle_file) {
            let uri = Url::from_file_path(&triangle_file).unwrap();
            let doc = Document::new(uri, 1, text);
            let diags = compute_diagnostics(&doc);
            assert_eq!(diags, vec![], "unexpected diagnostics: {diags:?}");
        }
    }

    #[test]
    fn test_lsp_self_include_diagnostic() {
        let temp_dir = std::env::temp_dir().join("emu8085_lsp_self_inc");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let main_file = temp_dir.join("main.e8085");
        let text = "%include \"main.e8085\"\nsegment .text\nmain:\n    hlt\n".to_string();
        std::fs::write(&main_file, &text).unwrap();

        let uri = Url::from_file_path(&main_file).unwrap();
        let doc = Document::new(uri, 1, text);
        let diags = compute_diagnostics(&doc);

        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("file cannot include itself"),
            "unexpected diag message: {}",
            diags[0].message
        );
        assert_eq!(diags[0].range.start.line, 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_lsp_duplicate_include_diagnostic() {
        let temp_dir = std::env::temp_dir().join("emu8085_lsp_dup_inc");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let sub_file = temp_dir.join("sub.e8085");
        std::fs::write(&sub_file, "segment .text\nsub_fn:\n    ret\n").unwrap();

        let main_file = temp_dir.join("main.e8085");
        let text = "%include \"sub.e8085\"\n%include \"sub.e8085\"\nsegment .text\nmain:\n    call sub_fn\n    hlt\n".to_string();
        std::fs::write(&main_file, &text).unwrap();

        let uri = Url::from_file_path(&main_file).unwrap();
        let doc = Document::new(uri, 1, text);
        let diags = compute_diagnostics(&doc);

        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("duplicate include"),
            "unexpected diag message: {}",
            diags[0].message
        );
        assert_eq!(diags[0].range.start.line, 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_lsp_outside_project_root_diagnostic() {
        let temp_root = std::env::temp_dir().join("emu8085_lsp_proj_root");
        let proj_dir = temp_root.join("proj");
        std::fs::create_dir_all(&proj_dir).unwrap();

        let main_file = proj_dir.join("main.e8085");
        let text = "%include \"../outside.e8085\"\nsegment .text\nmain:\n    hlt\n".to_string();
        std::fs::write(&main_file, &text).unwrap();

        let uri = Url::from_file_path(&main_file).unwrap();
        let doc = Document::new(uri, 1, text);
        let diags = compute_diagnostics(&doc);

        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("path is outside project root"),
            "unexpected diag message: {}",
            diags[0].message
        );
        assert_eq!(diags[0].range.start.line, 0);

        let _ = std::fs::remove_dir_all(&temp_root);
    }
}
