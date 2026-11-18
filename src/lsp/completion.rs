use std::collections::HashSet;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionResponse, CompletionTextEdit,
    InsertTextFormat, Position, Range, TextEdit,
};

use super::document::{Document, SegmentKind};

/// Detects all segment kinds declared in the current document.
fn get_present_segments(doc: &Document) -> HashSet<SegmentKind> {
    let mut present = HashSet::new();
    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        let lower = code_line.to_lowercase();
        if lower.starts_with("segment ") || lower.starts_with("section ") {
            let seg_name = code_line[8..].trim().to_lowercase();
            if seg_name == ".text" || seg_name == "text" {
                present.insert(SegmentKind::Text);
            } else if seg_name == ".data" || seg_name == "data" {
                present.insert(SegmentKind::Data);
            } else if seg_name == ".bss" || seg_name == "bss" {
                present.insert(SegmentKind::Bss);
            }
        }
    }
    present
}

/// Computes intelligent auto-completions based on active segment and instruction operand context.
pub fn get_completions(doc: &Document, position: &Position) -> Option<CompletionResponse> {
    let line = doc.text.lines().nth(position.line as usize).unwrap_or("");
    let char_idx = (position.character as usize).min(line.len());
    let line_prefix = &line[..char_idx];
    let trimmed_prefix = line_prefix.trim_start();

    // 1. Check if typing path inside `%include "..."`
    if let Some(include_items) = get_include_path_completions(doc, position, line, char_idx) {
        return Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: include_items,
        }));
    }

    let code_line = line_prefix.split(';').next().unwrap_or("").trim_start();
    let lower_code = code_line.to_lowercase();

    // 2. Check if typing after `extern ` (Issue 6: suggest only external symbols, no CPU instructions)
    if lower_code.starts_with("extern ")
        || (lower_code == "extern" && line[..char_idx].ends_with(char::is_whitespace))
    {
        let extern_items = get_extern_symbol_completions(doc);
        return Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: extern_items,
        }));
    }

    // 3. Check if typing after `global ` (Issue 7: suggest only unexported local labels, no CPU instructions)
    if lower_code.starts_with("global ")
        || (lower_code == "global" && line[..char_idx].ends_with(char::is_whitespace))
    {
        let global_items = get_global_declaration_completions(doc);
        return Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: global_items,
        }));
    }

    // 4. Check if typing after `%define <NAME> ` (suggest other defined constants)
    if lower_code.starts_with("%define ") {
        let parts: Vec<&str> = code_line.split_whitespace().collect();
        if (parts.len() == 2 && line_prefix.ends_with(char::is_whitespace)) || parts.len() > 2 {
            let current_name = parts[1];
            let (_, constants) = extract_variables_and_constants(doc);
            let items: Vec<CompletionItem> = constants
                .into_iter()
                .filter(|c| !c.eq_ignore_ascii_case(current_name))
                .map(|c| CompletionItem {
                    label: c,
                    kind: Some(CompletionItemKind::CONSTANT),
                    detail: Some("Defined Constant".to_string()),
                    ..Default::default()
                })
                .collect();
            return Some(CompletionResponse::List(CompletionList {
                is_incomplete: false,
                items,
            }));
        }
    }

    // 4b. Check if typing after `%if `, `%elif `, `%ifdef `, or `%ifndef ` (suggest defined constants)
    if lower_code.starts_with("%if ")
        || lower_code.starts_with("%elif ")
        || lower_code.starts_with("%ifdef ")
        || lower_code.starts_with("%ifndef ")
    {
        let (_, constants) = extract_variables_and_constants(doc);
        let items: Vec<CompletionItem> = constants
            .into_iter()
            .map(|c| CompletionItem {
                label: c,
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Defined Constant".to_string()),
                ..Default::default()
            })
            .collect();
        return Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items,
        }));
    }

    // 5. Check if typing after `%repeat ` (in data segment or anywhere)
    if lower_code.contains("%repeat") {
        if let Some(idx) = lower_code.rfind("%repeat") {
            let after_repeat = &code_line[idx + 7..];
            let after_trimmed = after_repeat.trim_start();
            let rep_tokens: Vec<&str> = after_trimmed.split_whitespace().collect();
            if rep_tokens.is_empty() {
                // 1st operand: count -> suggest constants
                let (_, constants) = extract_variables_and_constants(doc);
                let items: Vec<CompletionItem> = constants
                    .into_iter()
                    .map(|c| CompletionItem {
                        label: c,
                        kind: Some(CompletionItemKind::CONSTANT),
                        detail: Some("Defined Constant".to_string()),
                        ..Default::default()
                    })
                    .collect();
                return Some(CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items,
                }));
            } else if rep_tokens.len() == 1 {
                if line_prefix.ends_with(char::is_whitespace) || after_repeat.contains(',') {
                    // 2nd operand: value -> None
                    return Some(CompletionResponse::List(CompletionList {
                        is_incomplete: false,
                        items: vec![],
                    }));
                } else {
                    // Still typing 1st operand
                    let (_, constants) = extract_variables_and_constants(doc);
                    let items: Vec<CompletionItem> = constants
                        .into_iter()
                        .map(|c| CompletionItem {
                            label: c,
                            kind: Some(CompletionItemKind::CONSTANT),
                            detail: Some("Defined Constant".to_string()),
                            ..Default::default()
                        })
                        .collect();
                    return Some(CompletionResponse::List(CompletionList {
                        is_incomplete: false,
                        items,
                    }));
                }
            } else {
                // Both operands satisfied -> None
                return Some(CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items: vec![],
                }));
            }
        }
    }

    // 6. Check if typing after `%len ` anywhere -> suggest variables
    if lower_code.contains("%len") {
        if let Some(idx) = lower_code.rfind("%len") {
            let after_len = &code_line[idx + 4..];
            let after_trimmed = after_len.trim_start();
            let len_tokens: Vec<&str> = after_trimmed.split_whitespace().collect();
            if len_tokens.is_empty()
                || (len_tokens.len() == 1
                    && !line_prefix.ends_with(char::is_whitespace)
                    && !after_len.contains(','))
            {
                let (vars, _) = extract_variables_and_constants(doc);
                let mut items = Vec::new();
                let mut seen = HashSet::new();
                for var in vars {
                    if seen.insert(var.name.clone()) {
                        items.push(CompletionItem {
                            label: var.name,
                            kind: Some(CompletionItemKind::VARIABLE),
                            detail: Some(var.detail),
                            ..Default::default()
                        });
                    }
                }
                return Some(CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items,
                }));
            }
        }
    }

    let active_segment = get_active_segment(doc, position.line as usize);
    let present_segments = get_present_segments(doc);
    let mut items = Vec::new();

    match active_segment {
        Some(SegmentKind::Text) => {
            // Check if cursor is inside an instruction operand context
            if let Some(operand_items) =
                get_instruction_operand_completions(doc, position, code_line)
            {
                items.extend(operand_items);
            } else {
                // At instruction position (start of line / indentation)
                if trimmed_prefix.starts_with('%') {
                    items.extend(get_percent_directive_completions(Some(SegmentKind::Text)));
                } else if lower_code.starts_with("segment ") {
                    items.extend(get_missing_segment_completions(&present_segments));
                } else {
                    items.extend(get_instruction_completions());
                    items.extend(get_macro_completions(doc));
                    items.extend(get_text_directives(&present_segments));
                    items.extend(get_label_completions(doc, position));
                }
            }
        }
        Some(SegmentKind::Data) => {
            if code_line.is_empty() {
                // No suggestions for newline in data segment
                return Some(CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items: vec![],
                }));
            }
            if trimmed_prefix.starts_with('%') {
                items.extend(get_percent_directive_completions(Some(SegmentKind::Data)));
            } else if lower_code.starts_with("segment ") {
                items.extend(get_missing_segment_completions(&present_segments));
            } else {
                let tokens: Vec<&str> = code_line.split_whitespace().collect();
                if tokens.len() == 1 {
                    if line_prefix.ends_with(char::is_whitespace) {
                        // User typed variable name: suggest BYTE and WORD
                        items.push(make_snippet_item("BYTE", "BYTE ${1:value}", "8-bit byte allocation"));
                        items.push(make_snippet_item("WORD", "WORD ${1:value}", "16-bit word allocation"));
                    }
                } else if tokens.len() >= 2 {
                    if tokens[1].eq_ignore_ascii_case("byte") || tokens[1].eq_ignore_ascii_case("word") {
                        // Value position: suggest %repeat, %len <var>, constants, string
                        items.push(make_snippet_item(
                            "%repeat",
                            "%repeat ${1:count} ${2:value}",
                            "Repeat a data element multiple times",
                        ));
                        items.extend(get_constant_and_len_completions(doc));
                        if tokens[1].eq_ignore_ascii_case("byte") {
                            items.push(make_snippet_item(
                                "string",
                                "\"${1:text}\"",
                                "Initialized string literal",
                            ));
                        }
                    }
                }
            }
        }
        Some(SegmentKind::Bss) => {
            if code_line.is_empty() {
                // No suggestions for newline in bss segment
                return Some(CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items: vec![],
                }));
            }
            if trimmed_prefix.starts_with('%') {
                items.extend(get_percent_directive_completions(Some(SegmentKind::Bss)));
            } else if lower_code.starts_with("segment ") {
                items.extend(get_missing_segment_completions(&present_segments));
            } else {
                let tokens: Vec<&str> = code_line.split_whitespace().collect();
                if tokens.len() == 1 {
                    if line_prefix.ends_with(char::is_whitespace) {
                        // User typed buffer name: suggest BYTE and WORD
                        items.push(make_snippet_item("BYTE", "BYTE ${1:size}", "Uninitialized byte buffer"));
                        items.push(make_snippet_item("WORD", "WORD ${1:size}", "Uninitialized word buffer"));
                    }
                } else if tokens.len() >= 2 {
                    if tokens[1].eq_ignore_ascii_case("byte") || tokens[1].eq_ignore_ascii_case("word") {
                        // Buffer size position: suggest %len <var> and constants (no %repeat)
                        items.extend(get_constant_and_len_completions(doc));
                    }
                }
            }
        }
        None => {
            // Before any segment declaration (file header)
            if trimmed_prefix.starts_with('%') {
                items.extend(get_percent_directive_completions(None));
            } else if lower_code.starts_with("segment ") {
                items.extend(get_missing_segment_completions(&present_segments));
            } else {
                items.extend(get_header_directives(&present_segments));
            }
        }
    }

    Some(CompletionResponse::List(CompletionList {
        is_incomplete: false,
        items,
    }))
}

fn get_include_path_completions(
    doc: &Document,
    position: &Position,
    line: &str,
    char_idx: usize,
) -> Option<Vec<CompletionItem>> {
    let prefix = &line[..char_idx];
    let trimmed = prefix.trim_start();
    if !trimmed.starts_with("%include") {
        return None;
    }

    let angle_pos = prefix.find('<');
    let quote_pos = prefix.find('"').or_else(|| prefix.find('\''));

    if let Some(pos) = angle_pos {
        if char_idx <= pos {
            return None;
        }

        let raw_rel = &prefix[pos + 1..char_idx];
        let doc_path = doc.uri.to_file_path().ok();
        let stdlib_dir = crate::asm::include::find_stdlib_dir(doc_path.as_deref())
            .unwrap_or_else(|| {
                let project_root = doc_path
                    .as_ref()
                    .map(|p| crate::asm::include::find_project_root(p))
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                project_root.join("stdlib")
            });

        let clean_rel = raw_rel.trim_start_matches('/');
        let (search_dir, typed_prefix, prefix_start_char) = if let Some(last_slash) = clean_rel.rfind('/') {
            let sub = &clean_rel[..last_slash];
            let pre = &clean_rel[last_slash + 1..];
            let offset = pos + 1 + (raw_rel.len() - pre.len());
            (stdlib_dir.join(sub), pre, offset)
        } else {
            let offset = pos + 1 + (raw_rel.len() - clean_rel.len());
            (stdlib_dir.clone(), clean_rel, offset)
        };

        let replace_range = Range {
            start: Position {
                line: position.line,
                character: prefix_start_char as u32,
            },
            end: Position {
                line: position.line,
                character: char_idx as u32,
            },
        };

        let mut items = Vec::new();
        let mut seen = std::collections::HashSet::new();

        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with('.') && !typed_prefix.starts_with('.') {
                    continue;
                }

                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let label = format!("{file_name}/");
                        let insert = label.clone();
                        seen.insert(label.clone());
                        items.push(CompletionItem {
                            label,
                            kind: Some(CompletionItemKind::FOLDER),
                            detail: Some("Standard Library Folder".to_string()),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                range: replace_range,
                                new_text: insert,
                            })),
                            command: Some(tower_lsp::lsp_types::Command {
                                title: "Trigger Suggest".to_string(),
                                command: "editor.action.triggerSuggest".to_string(),
                                arguments: None,
                            }),
                            ..Default::default()
                        });
                    } else if file_name.ends_with(".e8085") {
                        let mod_name = file_name.strip_suffix(".e8085").unwrap_or(&file_name).to_string();
                        seen.insert(mod_name.clone());
                        items.push(CompletionItem {
                            label: mod_name.clone(),
                            kind: Some(CompletionItemKind::MODULE),
                            detail: Some("Standard Library Module".to_string()),
                            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                                range: replace_range,
                                new_text: mod_name,
                            })),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        // Also offer embedded stdlib entries if not already seen
        for (embed_path, _) in crate::asm::include::EMBEDDED_STDLIB {
            let embed_clean = embed_path.strip_suffix(".e8085").unwrap_or(embed_path);
            let (embed_sub, embed_name): (&str, &str) = if let Some(slash) = embed_clean.rfind('/') {
                (&embed_clean[..slash], &embed_clean[slash + 1..])
            } else {
                ("", embed_clean)
            };

            if clean_rel.contains('/') {
                let sub = &clean_rel[..clean_rel.rfind('/').unwrap()];
                if sub == embed_sub && !seen.contains(embed_name) {
                    seen.insert(embed_name.to_string());
                    items.push(CompletionItem {
                        label: embed_name.to_string(),
                        kind: Some(CompletionItemKind::MODULE),
                        detail: Some("Standard Library Module".to_string()),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                            range: replace_range,
                            new_text: embed_name.to_string(),
                        })),
                        ..Default::default()
                    });
                }
            } else if !embed_sub.is_empty() {
                let folder = format!("{embed_sub}/");
                if !seen.contains(&folder) {
                    seen.insert(folder.clone());
                    items.push(CompletionItem {
                        label: folder.clone(),
                        kind: Some(CompletionItemKind::FOLDER),
                        detail: Some("Standard Library Folder".to_string()),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                            range: replace_range,
                            new_text: folder,
                        })),
                        command: Some(tower_lsp::lsp_types::Command {
                            title: "Trigger Suggest".to_string(),
                            command: "editor.action.triggerSuggest".to_string(),
                            arguments: None,
                        }),
                        ..Default::default()
                    });
                }
            } else if !seen.contains(embed_name) {
                seen.insert(embed_name.to_string());
                items.push(CompletionItem {
                    label: embed_name.to_string(),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some("Standard Library Module".to_string()),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range: replace_range,
                        new_text: embed_name.to_string(),
                    })),
                    ..Default::default()
                });
            }
        }

        return Some(items);
    }

    let (raw_rel, rel_start) = if let Some(qpos) = quote_pos {
        if char_idx <= qpos {
            return None;
        }
        (&prefix[qpos + 1..char_idx], qpos + 1)
    } else {
        let inc_end = prefix.find("%include")? + "%include".len();
        let after_inc = &prefix[inc_end..];
        let spaces = after_inc.chars().take_while(|c| c.is_whitespace()).count();
        if spaces == 0 {
            return None;
        }
        let start = inc_end + spaces;
        if char_idx < start {
            return None;
        }
        (&prefix[start..char_idx], start)
    };

    let project_root = doc
        .uri
        .to_file_path()
        .ok()
        .map(|p| crate::asm::include::find_project_root(&p))
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

    let clean_rel = raw_rel.trim_start_matches('/');
    let (search_dir, typed_prefix, prefix_start_char) = if let Some(last_slash) = clean_rel.rfind('/') {
        let sub = &clean_rel[..last_slash];
        let pre = &clean_rel[last_slash + 1..];
        let offset = rel_start + (raw_rel.len() - pre.len());
        (project_root.join(sub), pre, offset)
    } else {
        let offset = rel_start + (raw_rel.len() - clean_rel.len());
        (project_root.clone(), clean_rel, offset)
    };

    let entries = std::fs::read_dir(&search_dir).ok()?;
    let mut items = Vec::new();

    let replace_range = Range {
        start: Position {
            line: position.line,
            character: prefix_start_char as u32,
        },
        end: Position {
            line: position.line,
            character: char_idx as u32,
        },
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') && !typed_prefix.starts_with('.') {
            continue;
        }

        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                items.push(CompletionItem {
                    label: format!("{file_name}/"),
                    kind: Some(CompletionItemKind::FOLDER),
                    detail: Some("Folder".to_string()),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range: replace_range,
                        new_text: format!("{file_name}/"),
                    })),
                    command: Some(tower_lsp::lsp_types::Command {
                        title: "Trigger Suggest".to_string(),
                        command: "editor.action.triggerSuggest".to_string(),
                        arguments: None,
                    }),
                    ..Default::default()
                });
            } else if file_type.is_file() {
                if file_name.ends_with(".e8085") || file_name.ends_with(".inc") {
                    items.push(CompletionItem {
                        label: file_name.clone(),
                        kind: Some(CompletionItemKind::FILE),
                        detail: Some("8085 Assembly file".to_string()),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                            range: replace_range,
                            new_text: file_name,
                        })),
                        ..Default::default()
                    });
                }
            }
        }
    }

    Some(items)
}

fn get_active_segment(doc: &Document, line_num: usize) -> Option<SegmentKind> {
    let mut current_segment = None;
    for (idx, line) in doc.text.lines().enumerate() {
        if idx > line_num {
            break;
        }
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.starts_with("segment ") {
            let seg_name = code_line.strip_prefix("segment ").unwrap().trim();
            if seg_name.eq_ignore_ascii_case(".text") || seg_name.eq_ignore_ascii_case("text") {
                current_segment = Some(SegmentKind::Text);
            } else if seg_name.eq_ignore_ascii_case(".data")
                || seg_name.eq_ignore_ascii_case("data")
            {
                current_segment = Some(SegmentKind::Data);
            } else if seg_name.eq_ignore_ascii_case(".bss") || seg_name.eq_ignore_ascii_case("bss")
            {
                current_segment = Some(SegmentKind::Bss);
            }
        }
    }
    current_segment
}

fn get_instruction_operand_completions(
    doc: &Document,
    position: &Position,
    line_prefix: &str,
) -> Option<Vec<CompletionItem>> {
    let code_line = line_prefix.split(';').next().unwrap_or("");
    let trimmed_lead = code_line.trim_start();
    if trimmed_lead.is_empty() {
        return None;
    }

    let mut iter = trimmed_lead.split_whitespace();
    let first_word = iter.next()?;
    let mnemonic = first_word.to_uppercase();

    // If user is still typing the mnemonic (no whitespace after mnemonic),
    // let general instruction completions take over.
    let after_first = &trimmed_lead[first_word.len()..];
    if after_first.is_empty() && !code_line.ends_with(char::is_whitespace) {
        return None;
    }

    let operands_str = after_first.trim_start();

    match mnemonic.as_str() {
        // ── 0-operand instructions: any trailing space/operands means satisfied ──
        "NOP" | "HLT" | "EI" | "DI" | "RIM" | "SIM"
        | "XCHG" | "XTHL" | "SPHL" | "PCHL"
        | "DAA" | "CMA" | "CMC" | "STC" | "RLC" | "RRC" | "RAL" | "RAR"
        | "RET" | "RZ" | "RNZ" | "RC" | "RNC" | "RP" | "RM" | "RPE" | "RPO" => {
            Some(vec![])
        }

        // ── 1-operand branch / call labels ──
        "CALL" | "CZ" | "CNZ" | "CC" | "CNC" | "CP" | "CM" | "CPE" | "CPO"
        | "JMP" | "JZ" | "JNZ" | "JC" | "JNC" | "JP" | "JM" | "JPE" | "JPO" => {
            if is_one_operand_satisfied(operands_str) {
                Some(vec![])
            } else {
                Some(get_label_completions(doc, position))
            }
        }

        // ── 1-operand direct memory load/store (variables & constants only, no labels) ──
        "LDA" | "STA" | "LHLD" | "SHLD" => {
            if is_one_operand_satisfied(operands_str) {
                Some(vec![])
            } else {
                Some(get_memory_symbol_completions(doc))
            }
        }

        // ── 1-operand 8-bit registers ──
        "ADD" | "ADC" | "SUB" | "SBB" | "INR" | "DCR" | "ANA" | "XRA" | "ORA" | "CMP" => {
            if is_one_operand_satisfied(operands_str) {
                Some(vec![])
            } else {
                Some(get_8bit_registers())
            }
        }

        // ── 1-operand 16-bit register pairs ──
        "INX" | "DCX" | "DAD" => {
            if is_one_operand_satisfied(operands_str) {
                Some(vec![])
            } else {
                Some(vec![
                    make_reg_item("BC", "Register Pair BC (16-bit)"),
                    make_reg_item("DE", "Register Pair DE (16-bit)"),
                    make_reg_item("HL", "Register Pair HL (16-bit)"),
                    make_reg_item("SP", "Stack Pointer SP (16-bit)"),
                ])
            }
        }

        // ── 1-operand stack pairs ──
        "PUSH" | "POP" => {
            if is_one_operand_satisfied(operands_str) {
                Some(vec![])
            } else {
                Some(vec![
                    make_reg_item("BC", "Register Pair BC (16-bit)"),
                    make_reg_item("DE", "Register Pair DE (16-bit)"),
                    make_reg_item("HL", "Register Pair HL (16-bit)"),
                    make_reg_item("PSW", "Program Status Word (A + Flags)"),
                ])
            }
        }

        // ── 1-operand indirect pointers ──
        "LDAX" | "STAX" => {
            if is_one_operand_satisfied(operands_str) {
                Some(vec![])
            } else {
                Some(vec![
                    make_reg_item("BC", "Register Pair BC pointer"),
                    make_reg_item("DE", "Register Pair DE pointer"),
                ])
            }
        }

        // ── 1-operand 8-bit immediate / port ──
        "ADI" | "ACI" | "SUI" | "SBI" | "ANI" | "XRI" | "ORI" | "CPI" | "IN" | "OUT" => {
            if is_one_operand_satisfied(operands_str) {
                Some(vec![])
            } else {
                Some(get_constant_and_len_completions(doc))
            }
        }

        // ── 1-operand RST interrupt vector ──
        "RST" => {
            if is_one_operand_satisfied(operands_str) {
                Some(vec![])
            } else {
                Some(vec![
                    make_snippet_item("0", "0", "RST 0 (0x0000)"),
                    make_snippet_item("1", "1", "RST 1 (0x0008)"),
                    make_snippet_item("2", "2", "RST 2 (0x0010)"),
                    make_snippet_item("3", "3", "RST 3 (0x0018)"),
                    make_snippet_item("4", "4", "RST 4 (0x0020)"),
                    make_snippet_item("5", "5", "RST 5 (0x0028)"),
                    make_snippet_item("6", "6", "RST 6 (0x0030)"),
                    make_snippet_item("7", "7", "RST 7 (0x0038)"),
                ])
            }
        }

        // ── 2-operand MOV dest, src ──
        "MOV" => {
            if let Some((op1, op2)) = split_two_operands(operands_str) {
                if let Some(op2_str) = op2 {
                    if is_second_operand_satisfied(op2_str) {
                        Some(vec![])
                    } else {
                        Some(get_8bit_registers())
                    }
                } else if is_first_operand_finished_without_comma(op1) {
                    Some(vec![])
                } else {
                    Some(get_8bit_registers())
                }
            } else {
                Some(get_8bit_registers())
            }
        }

        // ── 2-operand MVI dest, imm8 ──
        "MVI" => {
            if let Some((op1, op2)) = split_two_operands(operands_str) {
                if let Some(op2_str) = op2 {
                    if is_second_operand_satisfied(op2_str) {
                        Some(vec![])
                    } else {
                        Some(get_constant_and_len_completions(doc))
                    }
                } else if is_first_operand_finished_without_comma(op1) {
                    Some(vec![])
                } else {
                    Some(get_8bit_registers())
                }
            } else {
                Some(get_8bit_registers())
            }
        }

        // ── 2-operand LXI pair, imm16 (variables & constants only, no labels) ──
        "LXI" => {
            if let Some((op1, op2)) = split_two_operands(operands_str) {
                if let Some(op2_str) = op2 {
                    if is_second_operand_satisfied(op2_str) {
                        Some(vec![])
                    } else {
                        Some(get_memory_symbol_completions(doc))
                    }
                } else if is_first_operand_finished_without_comma(op1) {
                    Some(vec![])
                } else {
                    Some(vec![
                        make_reg_item("BC", "Register Pair BC (16-bit)"),
                        make_reg_item("DE", "Register Pair DE (16-bit)"),
                        make_reg_item("HL", "Register Pair HL (16-bit)"),
                        make_reg_item("SP", "Stack Pointer SP (16-bit)"),
                    ])
                }
            } else {
                Some(vec![
                    make_reg_item("BC", "Register Pair BC (16-bit)"),
                    make_reg_item("DE", "Register Pair DE (16-bit)"),
                    make_reg_item("HL", "Register Pair HL (16-bit)"),
                    make_reg_item("SP", "Stack Pointer SP (16-bit)"),
                ])
            }
        }

        _ => {
            if let Some(arg_count) = get_macro_arg_count(doc, &mnemonic) {
                if arg_count == 0 {
                    Some(vec![])
                } else {
                    let current_ops: Vec<&str> = operands_str.split(',').collect();
                    if current_ops.len() <= arg_count {
                        let last_op = current_ops.last().map(|s| s.trim()).unwrap_or("");
                        if is_one_operand_satisfied(last_op) && current_ops.len() == arg_count {
                            Some(vec![])
                        } else {
                            Some(get_macro_operand_completions(doc))
                        }
                    } else {
                        Some(vec![])
                    }
                }
            } else {
                None
            }
        }
    }
}

fn is_one_operand_satisfied(operands_str: &str) -> bool {
    let trimmed = operands_str.trim_start();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.to_lowercase().starts_with("%len") {
        let after_len = trimmed[4..].trim_start();
        if after_len.is_empty() {
            return false;
        }
        return after_len.contains(char::is_whitespace) || after_len.contains(',');
    }

    trimmed.contains(char::is_whitespace) || trimmed.contains(',')
}

fn split_two_operands(operands_str: &str) -> Option<(&str, Option<&str>)> {
    if let Some(idx) = operands_str.find(',') {
        let op1 = &operands_str[..idx];
        let op2 = &operands_str[idx + 1..];
        Some((op1, Some(op2)))
    } else {
        Some((operands_str, None))
    }
}

fn is_first_operand_finished_without_comma(op1: &str) -> bool {
    let trimmed = op1.trim();
    !trimmed.is_empty() && op1.ends_with(char::is_whitespace)
}

fn is_second_operand_satisfied(op2: &str) -> bool {
    let trimmed = op2.trim_start();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.to_lowercase().starts_with("%len") {
        let after_len = trimmed[4..].trim_start();
        if after_len.is_empty() {
            return false;
        }
        return after_len.contains(char::is_whitespace) || after_len.contains(',');
    }

    trimmed.contains(char::is_whitespace) || trimmed.contains(',')
}

fn get_8bit_registers() -> Vec<CompletionItem> {
    vec![
        make_reg_item("A", "Accumulator A (8-bit)"),
        make_reg_item("B", "Register B (8-bit)"),
        make_reg_item("C", "Register C (8-bit)"),
        make_reg_item("D", "Register D (8-bit)"),
        make_reg_item("E", "Register E (8-bit)"),
        make_reg_item("H", "Register H (8-bit)"),
        make_reg_item("L", "Register L (8-bit)"),
        make_reg_item("M", "Memory Reference [HL]"),
    ]
}

fn make_reg_item(name: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::VARIABLE),
        detail: Some(detail.to_string()),
        ..Default::default()
    }
}

fn get_label_completions(doc: &Document, position: &Position) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let raw_lines: Vec<&str> = doc.text.lines().collect();

    // 1. Determine enclosing parent label scope for local labels
    let current_line = position.line as usize;
    let mut parent_name: Option<String> = None;
    let mut parent_start = 0;

    for i in (0..=current_line.min(raw_lines.len().saturating_sub(1))).rev() {
        let code_line = raw_lines[i].split(';').next().unwrap_or("").trim();
        if let Some(rest) = code_line.strip_suffix(':') {
            let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
            if !clean.starts_with('.') && !clean.is_empty() {
                parent_name = Some(clean.to_string());
                parent_start = i;
                break;
            }
        }
    }

    // 2. Global Labels & Extern Declarations in current doc
    for line in &raw_lines {
        let code_line = line.split(';').next().unwrap_or("").trim();

        if let Some(rest) = code_line.strip_prefix("extern ") {
            let name = rest.trim();
            if !name.is_empty() {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("Extern Symbol".to_string()),
                    ..Default::default()
                });
            }
        }

        if let Some(rest) = code_line.strip_suffix(':') {
            let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
            if !clean.starts_with('.') && !clean.is_empty() {
                items.push(CompletionItem {
                    label: clean.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("Global Subroutine / Label".to_string()),
                    ..Default::default()
                });
            }
        }
    }

    // 3. Local labels inside the current parent subroutine scope
    if parent_name.is_some() {
        for line in raw_lines[parent_start..].iter().skip(1) {
            let code_line = line.split(';').next().unwrap_or("").trim();
            if let Some(rest) = code_line.strip_suffix(':') {
                let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
                if clean.starts_with('.') {
                    items.push(CompletionItem {
                        label: clean.to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some("Local Label".to_string()),
                        ..Default::default()
                    });
                } else if !clean.is_empty() {
                    // Next parent label boundary reached
                    break;
                }
            }
        }
    }

    // 4. Labels from %include imported files (transitive)
    let included_docs = super::document::collect_all_included_documents(doc);
    for inc_doc in &included_docs {
        let origin = if inc_doc.is_system {
            format!("<{}>", inc_doc.spec)
        } else {
            format!("\"{}\"", inc_doc.spec)
        };
        for inc_line in inc_doc.content.lines() {
            let inc_code = inc_line.split(';').next().unwrap_or("").trim();
            if let Some(rest) = inc_code.strip_suffix(':') {
                let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
                if !clean.starts_with('.') && !clean.is_empty() {
                    items.push(CompletionItem {
                        label: clean.to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(format!("Imported from {origin}")),
                        ..Default::default()
                    });
                }
            }
        }
    }

    items
}

struct VariableInfo {
    name: String,
    detail: String,
}

fn extract_variables_and_constants(doc: &Document) -> (Vec<VariableInfo>, Vec<String>) {
    let mut vars = Vec::new();
    let mut constants = Vec::new();

    // 1. Scan doc lines with segment awareness
    let mut current_segment: Option<SegmentKind> = None;

    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.is_empty() {
            continue;
        }

        if code_line.to_lowercase().starts_with("segment ") || code_line.to_lowercase().starts_with("section ") {
            let seg_name = code_line[8..].trim().to_lowercase();
            if seg_name == ".text" || seg_name == "text" {
                current_segment = Some(SegmentKind::Text);
            } else if seg_name == ".data" || seg_name == "data" {
                current_segment = Some(SegmentKind::Data);
            } else if seg_name == ".bss" || seg_name == "bss" {
                current_segment = Some(SegmentKind::Bss);
            }
            continue;
        }

        if code_line.to_lowercase().starts_with("%define") {
            let parts: Vec<&str> = code_line.split_whitespace().collect();
            if parts.len() >= 2 {
                constants.push(parts[1].to_string());
            }
            continue;
        }

        let parts: Vec<&str> = code_line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let first = parts[0].trim_end_matches(':');
        if is_reserved_keyword(first) || first.starts_with('%') || first.is_empty() {
            continue;
        }

        let lower_line = code_line.to_lowercase();

        match current_segment {
            Some(SegmentKind::Data) => {
                vars.push(VariableInfo {
                    name: first.to_string(),
                    detail: "Data Variable".to_string(),
                });
            }
            Some(SegmentKind::Bss) => {
                vars.push(VariableInfo {
                    name: first.to_string(),
                    detail: "BSS Buffer".to_string(),
                });
            }
            _ => {
                if lower_line.contains("byte")
                    || lower_line.contains("word")
                    || lower_line.contains('"')
                {
                    vars.push(VariableInfo {
                        name: first.to_string(),
                        detail: "Variable".to_string(),
                    });
                }
            }
        }
    }

    // 2. Scan %include files (transitive)
    let included_docs = super::document::collect_all_included_documents(doc);
    for inc_doc in &included_docs {
        let mut inc_seg: Option<SegmentKind> = None;
        for inc_line in inc_doc.content.lines() {
            let inc_code = inc_line.split(';').next().unwrap_or("").trim();
            if inc_code.is_empty() {
                continue;
            }
            if inc_code.to_lowercase().starts_with("segment ")
                || inc_code.to_lowercase().starts_with("section ")
            {
                let seg_name = inc_code[8..].trim().to_lowercase();
                if seg_name == ".text" || seg_name == "text" {
                    inc_seg = Some(SegmentKind::Text);
                } else if seg_name == ".data" || seg_name == "data" {
                    inc_seg = Some(SegmentKind::Data);
                } else if seg_name == ".bss" || seg_name == "bss" {
                    inc_seg = Some(SegmentKind::Bss);
                }
                continue;
            }
            if inc_code.to_lowercase().starts_with("%define") {
                let parts: Vec<&str> = inc_code.split_whitespace().collect();
                if parts.len() >= 2 {
                    constants.push(parts[1].to_string());
                }
                continue;
            }
            let parts: Vec<&str> = inc_code.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let first = parts[0].trim_end_matches(':');
            if is_reserved_keyword(first) || first.starts_with('%') || first.is_empty() {
                continue;
            }
            let lower_inc = inc_code.to_lowercase();
            match inc_seg {
                Some(SegmentKind::Data) => {
                    vars.push(VariableInfo {
                        name: first.to_string(),
                        detail: format!("Data Variable from {}", inc_doc.spec),
                    });
                }
                Some(SegmentKind::Bss) => {
                    vars.push(VariableInfo {
                        name: first.to_string(),
                        detail: format!("BSS Buffer from {}", inc_doc.spec),
                    });
                }
                _ => {
                    if lower_inc.contains("byte")
                        || lower_inc.contains("word")
                        || lower_inc.contains('"')
                    {
                        vars.push(VariableInfo {
                            name: first.to_string(),
                            detail: format!("Variable from {}", inc_doc.spec),
                        });
                    }
                }
            }
        }
    }

    (vars, constants)
}

fn get_memory_symbol_completions(doc: &Document) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let (vars, constants) = extract_variables_and_constants(doc);
    let mut seen = HashSet::new();

    // 1. Data and BSS Variables
    for var in vars {
        if seen.insert(var.name.clone()) {
            items.push(CompletionItem {
                label: var.name,
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(var.detail),
                ..Default::default()
            });
        }
    }

    // 2. Constants
    for c in constants {
        if seen.insert(c.clone()) {
            items.push(CompletionItem {
                label: c,
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Defined Constant".to_string()),
                ..Default::default()
            });
        }
    }

    items
}

fn get_constant_and_len_completions(doc: &Document) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let (vars, constants) = extract_variables_and_constants(doc);

    // 1. Defined Constants
    for c in constants {
        items.push(CompletionItem {
            label: c,
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some("Defined Constant".to_string()),
            ..Default::default()
        });
    }

    // 2. %len <variable> for all Data and BSS variables (case-insensitive)
    let mut seen_len = HashSet::new();
    for var in vars {
        if seen_len.insert(var.name.clone()) {
            items.push(CompletionItem {
                label: format!("%len {}", var.name),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("Byte length of {}", var.name)),
                insert_text: Some(format!("%len {}", var.name)),
                ..Default::default()
            });
        }
    }

    items
}

fn get_macro_operand_completions(doc: &Document) -> Vec<CompletionItem> {
    let mut items = get_memory_symbol_completions(doc);
    let (vars, _) = extract_variables_and_constants(doc);
    let mut seen_len = HashSet::new();
    for var in vars {
        if seen_len.insert(var.name.clone()) {
            items.push(CompletionItem {
                label: format!("%len {}", var.name),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("Byte length of {}", var.name)),
                insert_text: Some(format!("%len {}", var.name)),
                ..Default::default()
            });
        }
    }
    items
}

fn get_macro_arg_count(doc: &Document, macro_name: &str) -> Option<usize> {
    for line in doc.text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("%macro") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].eq_ignore_ascii_case(macro_name) {
                return Some(parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0));
            }
        }
    }
    let included_docs = super::document::collect_all_included_documents(doc);
    for inc_doc in &included_docs {
        for inc_line in inc_doc.content.lines() {
            let inc_trimmed = inc_line.trim();
            if inc_trimmed.starts_with("%macro") {
                let parts: Vec<&str> = inc_trimmed.split_whitespace().collect();
                if parts.len() >= 2 && parts[1].eq_ignore_ascii_case(macro_name) {
                    return Some(parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0));
                }
            }
        }
    }
    None
}

fn get_macro_completions(doc: &Document) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    // Local macros
    for line in doc.text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("%macro") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[1];
                let count: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                if seen.insert(name.to_lowercase()) {
                    let insert_text = if count == 0 {
                        name.to_string()
                    } else {
                        let placeholders: Vec<String> = (1..=count)
                            .map(|i| format!("${{{}:arg{}}}", i, i))
                            .collect();
                        format!("{} {}", name, placeholders.join(", "))
                    };
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(format!("Macro ({} arg{})", count, if count == 1 { "" } else { "s" })),
                        insert_text: Some(insert_text),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        ..Default::default()
                    });
                }
            }
        }
    }

    // Included files (transitive)
    let included_docs = super::document::collect_all_included_documents(doc);
    for inc_doc in &included_docs {
        let origin = if inc_doc.is_system {
            format!("<{}>", inc_doc.spec)
        } else {
            format!("\"{}\"", inc_doc.spec)
        };
        for inc_line in inc_doc.content.lines() {
            let inc_trimmed = inc_line.trim();
            if inc_trimmed.starts_with("%macro") {
                let parts: Vec<&str> = inc_trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[1];
                    let count: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                    if seen.insert(name.to_lowercase()) {
                        let insert_text = if count == 0 {
                            name.to_string()
                        } else {
                            let placeholders: Vec<String> = (1..=count)
                                .map(|i| format!("${{{}:arg{}}}", i, i))
                                .collect();
                            format!("{} {}", name, placeholders.join(", "))
                        };
                        items.push(CompletionItem {
                            label: name.to_string(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            detail: Some(format!(
                                "Macro ({} arg{}) from {}",
                                count,
                                if count == 1 { "" } else { "s" },
                                origin
                            )),
                            insert_text: Some(insert_text),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    items
}

fn get_percent_directive_completions(seg_kind: Option<SegmentKind>) -> Vec<CompletionItem> {
    let mut items = match seg_kind {
        Some(SegmentKind::Text) => vec![
            make_snippet_item(
                "len",
                "len ${1:var_name}",
                "Compute byte length of a symbol",
            ),
            make_snippet_item(
                "macro",
                "macro ${1:macro_name} ${2:arg_count}\n\t$0\n%end",
                "Define a macro with positional arguments",
            ),
            make_snippet_item(
                "end",
                "end",
                "End of macro or conditional block",
            ),
        ],
        Some(SegmentKind::Data) => vec![
            make_snippet_item(
                "repeat",
                "repeat ${1:count} ${2:value}",
                "Repeat a data element multiple times",
            ),
            make_snippet_item(
                "len",
                "len ${1:var_name}",
                "Compute byte length of a symbol",
            ),
            make_snippet_item(
                "define",
                "define ${1:NAME} ${2:value}",
                "Define a constant value",
            ),
            make_snippet_item(
                "include",
                "include \"${1}\"",
                "Include an external source file",
            ),
        ],
        Some(SegmentKind::Bss) => vec![
            make_snippet_item(
                "len",
                "len ${1:var_name}",
                "Compute byte length of a symbol",
            ),
            make_snippet_item(
                "define",
                "define ${1:NAME} ${2:value}",
                "Define a constant value",
            ),
            make_snippet_item(
                "include",
                "include \"${1}\"",
                "Include an external source file",
            ),
        ],
        _ => vec![
            make_snippet_item(
                "define",
                "define ${1:NAME} ${2:value}",
                "Define a constant value",
            ),
            make_snippet_item(
                "macro",
                "macro ${1:macro_name} ${2:arg_count}\n\t$0\n%end",
                "Define a macro with positional arguments",
            ),
            make_snippet_item(
                "end",
                "end",
                "End of macro or conditional block",
            ),
            make_snippet_item(
                "include",
                "include \"${1}\"",
                "Include an external source file",
            ),
        ],
    };

    items.extend(vec![
        make_snippet_item(
            "if",
            "if ${1:condition}\n\t$0\n%end",
            "Conditional compilation block",
        ),
        make_snippet_item(
            "ifdef",
            "ifdef ${1:NAME}\n\t$0\n%end",
            "Conditional compilation if defined",
        ),
        make_snippet_item(
            "ifndef",
            "ifndef ${1:NAME}\n\t$0\n%end",
            "Conditional compilation if not defined",
        ),
        make_snippet_item(
            "elif",
            "elif ${1:condition}",
            "Alternative conditional branch",
        ),
        make_snippet_item(
            "else",
            "else",
            "Fallback conditional branch",
        ),
        make_snippet_item(
            "endif",
            "endif",
            "End conditional block",
        ),
    ]);

    items
}

fn get_header_directives(present: &HashSet<SegmentKind>) -> Vec<CompletionItem> {
    let mut items = vec![
        make_snippet_item(
            "%define",
            "%define ${1:NAME} ${2:value}",
            "Define a constant value",
        ),
        make_snippet_item(
            "%macro",
            "%macro ${1:macro_name} ${2:arg_count}\n\t$0\n%end",
            "Define a macro with positional arguments",
        ),
        make_snippet_item(
            "%end",
            "%end",
            "End of macro or conditional block",
        ),
        make_snippet_item(
            "%include",
            "%include \"${1}\"",
            "Include an external source file",
        ),
        make_snippet_item(
            "%if",
            "%if ${1:condition}\n\t$0\n%end",
            "Conditional compilation block",
        ),
        make_snippet_item(
            "%ifdef",
            "%ifdef ${1:NAME}\n\t$0\n%end",
            "Conditional compilation if defined",
        ),
        make_snippet_item(
            "%ifndef",
            "%ifndef ${1:NAME}\n\t$0\n%end",
            "Conditional compilation if not defined",
        ),
        make_snippet_item(
            "%elif",
            "%elif ${1:condition}",
            "Alternative conditional branch",
        ),
        make_snippet_item(
            "%else",
            "%else",
            "Fallback conditional branch",
        ),
        make_snippet_item(
            "%endif",
            "%endif",
            "End conditional block",
        ),
    ];
    items.extend(get_missing_segment_completions(present));
    items
}

fn get_missing_segment_completions(present: &HashSet<SegmentKind>) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if !present.contains(&SegmentKind::Text) {
        items.push(make_snippet_item(
            "segment .text",
            "segment .text\n$0",
            "Executable code segment",
        ));
        items.push(make_snippet_item(
            "section .text",
            "section .text\n$0",
            "Executable code section",
        ));
    }
    if !present.contains(&SegmentKind::Data) {
        items.push(make_snippet_item(
            "segment .data",
            "segment .data\n$0",
            "Initialized data segment",
        ));
        items.push(make_snippet_item(
            "section .data",
            "section .data\n$0",
            "Initialized data section",
        ));
    }
    if !present.contains(&SegmentKind::Bss) {
        items.push(make_snippet_item(
            "segment .bss",
            "segment .bss\n$0",
            "Uninitialized memory buffer segment",
        ));
        items.push(make_snippet_item(
            "section .bss",
            "section .bss\n$0",
            "Uninitialized memory buffer section",
        ));
    }
    items
}

fn get_text_directives(present: &HashSet<SegmentKind>) -> Vec<CompletionItem> {
    let mut items = vec![
        make_snippet_item(
            "global",
            "global ${1:subroutine_name}:\n    $0\n    ret",
            "Export global subroutine",
        ),
        make_snippet_item(
            "extern",
            "extern ${1:function_name}",
            "Declare external linked symbol",
        ),
        make_snippet_item(
            "%if",
            "%if ${1:condition}\n\t$0\n%end",
            "Conditional compilation block",
        ),
        make_snippet_item(
            "%ifdef",
            "%ifdef ${1:NAME}\n\t$0\n%end",
            "Conditional compilation if defined",
        ),
        make_snippet_item(
            "%ifndef",
            "%ifndef ${1:NAME}\n\t$0\n%end",
            "Conditional compilation if not defined",
        ),
        make_snippet_item(
            "%elif",
            "%elif ${1:condition}",
            "Alternative conditional branch",
        ),
        make_snippet_item(
            "%else",
            "%else",
            "Fallback conditional branch",
        ),
        make_snippet_item(
            "%endif",
            "%endif",
            "End conditional block",
        ),
    ];
    if !present.contains(&SegmentKind::Data) {
        items.push(make_snippet_item(
            "segment .data",
            "segment .data\n$0",
            "Switch to data segment",
        ));
        items.push(make_snippet_item(
            "section .data",
            "section .data\n$0",
            "Switch to data section",
        ));
    }
    if !present.contains(&SegmentKind::Bss) {
        items.push(make_snippet_item(
            "segment .bss",
            "segment .bss\n$0",
            "Switch to bss segment",
        ));
        items.push(make_snippet_item(
            "section .bss",
            "section .bss\n$0",
            "Switch to bss section",
        ));
    }
    items
}

fn get_extern_symbol_completions(doc: &Document) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    // 1. Gather all symbols already declared/defined in the current doc to avoid duplicate suggestions
    let mut local_symbols = HashSet::new();
    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if let Some(rest) = code_line.strip_prefix("extern ") {
            local_symbols.insert(rest.trim().to_string());
        }
        if let Some(rest) = code_line.strip_prefix("global ") {
            let clean = rest.trim().trim_end_matches(':');
            local_symbols.insert(clean.to_string());
        }
        if let Some(rest) = code_line.strip_suffix(':') {
            let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
            local_symbols.insert(clean.to_string());
        }
    }

    // 2. Check %include files for global exports (transitive)
    let included_docs = super::document::collect_all_included_documents(doc);
    for inc_doc in &included_docs {
        let origin = if inc_doc.is_system {
            format!("<{}>", inc_doc.spec)
        } else {
            format!("\"{}\"", inc_doc.spec)
        };
        extract_exported_globals_from_str(
            &inc_doc.content,
            &format!("Imported from {origin}"),
            &local_symbols,
            &mut seen,
            &mut items,
        );
    }

    // 3. Scan workspace / library directories (stdlib, stdlib/device)
    if let Ok(doc_path) = doc.uri.to_file_path() {
        let project_root = crate::asm::include::find_project_root(&doc_path);
        let search_dirs = [
            project_root.join("stdlib"),
            project_root.join("stdlib").join("device"),
        ];

        for sdir in &search_dirs {
            if sdir.exists() && sdir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(sdir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_file()
                            && (p.extension().is_some_and(|e| e == "e8085" || e == "inc"))
                        {
                            if let Ok(content) = std::fs::read_to_string(&p) {
                                let file_name =
                                    p.file_name().unwrap_or_default().to_string_lossy();
                                extract_exported_globals_from_str(
                                    &content,
                                    &format!("Library symbol ({file_name})"),
                                    &local_symbols,
                                    &mut seen,
                                    &mut items,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    items
}

fn extract_exported_globals_from_str(
    content: &str,
    detail: &str,
    local_symbols: &HashSet<String>,
    seen: &mut HashSet<String>,
    items: &mut Vec<CompletionItem>,
) {
    for line in content.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.is_empty() {
            continue;
        }
        if let Some(rest) = code_line.strip_prefix("global ") {
            let clean = rest.trim().trim_end_matches(':');
            if !clean.is_empty()
                && !clean.eq_ignore_ascii_case("main")
                && !clean.starts_with('.')
            {
                if !local_symbols.contains(clean) && seen.insert(clean.to_string()) {
                    items.push(CompletionItem {
                        label: clean.to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(detail.to_string()),
                        ..Default::default()
                    });
                }
            }
        } else if let Some(rest) = code_line.strip_suffix(':') {
            if let Some(global_name) = rest.strip_prefix("global ") {
                let clean = global_name.trim();
                if !clean.is_empty()
                    && !clean.eq_ignore_ascii_case("main")
                    && !clean.starts_with('.')
                {
                    if !local_symbols.contains(clean) && seen.insert(clean.to_string()) {
                        items.push(CompletionItem {
                            label: clean.to_string(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            detail: Some(detail.to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
}

fn get_global_declaration_completions(doc: &Document) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut already_global = HashSet::new();
    let mut local_labels = Vec::new();

    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.is_empty() {
            continue;
        }

        // Top-level or inline `global `
        if let Some(rest) = code_line.strip_prefix("global ") {
            let clean = rest.trim().trim_end_matches(':');
            if !clean.is_empty() {
                already_global.insert(clean.to_string());
            }
            continue;
        }

        // Local labels ending in `:`
        if let Some(rest) = code_line.strip_suffix(':') {
            let clean = rest.trim();
            if !clean.is_empty() && !clean.eq_ignore_ascii_case("main") && !clean.starts_with('.') {
                local_labels.push(clean.to_string());
            }
        }
    }

    let mut seen = HashSet::new();
    for label in local_labels {
        if !already_global.contains(&label) && seen.insert(label.clone()) {
            items.push(CompletionItem {
                label: label.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Local Label to Export".to_string()),
                ..Default::default()
            });
        }
    }

    items
}

fn get_instruction_completions() -> Vec<CompletionItem> {
    vec![
        make_snippet_item("mov", "mov ${1:dest}, ${2:src}", "Copy register/memory"),
        make_snippet_item("mvi", "mvi ${1:dest}, ${2:byte}", "Move immediate byte"),
        make_snippet_item(
            "lxi",
            "lxi ${1:pair}, ${2:addr16}",
            "Load 16-bit register pair",
        ),
        make_snippet_item("lda", "lda ${1:addr16}", "Load Accumulator direct"),
        make_snippet_item("sta", "sta ${1:addr16}", "Store Accumulator direct"),
        make_snippet_item("lhld", "lhld ${1:addr16}", "Load HL direct"),
        make_snippet_item("shld", "shld ${1:addr16}", "Store HL direct"),
        make_snippet_item("ldax", "ldax ${1|BC,DE|}", "Load Accumulator indirect"),
        make_snippet_item("stax", "stax ${1|BC,DE|}", "Store Accumulator indirect"),
        make_snippet_item("xchg", "xchg", "Exchange HL with DE"),
        make_snippet_item("add", "add ${1:reg}", "Add to Accumulator"),
        make_snippet_item("adi", "adi ${1:byte}", "Add immediate to Accumulator"),
        make_snippet_item("adc", "adc ${1:reg}", "Add with Carry to Accumulator"),
        make_snippet_item("aci", "aci ${1:byte}", "Add immediate with Carry"),
        make_snippet_item("sub", "sub ${1:reg}", "Subtract from Accumulator"),
        make_snippet_item(
            "sui",
            "sui ${1:byte}",
            "Subtract immediate from Accumulator",
        ),
        make_snippet_item("sbb", "sbb ${1:reg}", "Subtract with Borrow"),
        make_snippet_item("sbi", "sbi ${1:byte}", "Subtract immediate with Borrow"),
        make_snippet_item("inr", "inr ${1:reg}", "Increment register"),
        make_snippet_item("dcr", "dcr ${1:reg}", "Decrement register"),
        make_snippet_item(
            "inx",
            "inx ${1|BC,DE,HL,SP|}",
            "Increment register pair (16-bit)",
        ),
        make_snippet_item(
            "dcx",
            "dcx ${1|BC,DE,HL,SP|}",
            "Decrement register pair (16-bit)",
        ),
        make_snippet_item("dad", "dad ${1|BC,DE,HL,SP|}", "16-bit add to HL"),
        make_snippet_item("daa", "daa", "Decimal adjust Accumulator"),
        make_snippet_item("ana", "ana ${1:reg}", "Logical AND with Accumulator"),
        make_snippet_item("ani", "ani ${1:byte}", "Logical AND immediate"),
        make_snippet_item("xra", "xra ${1:reg}", "Logical XOR with Accumulator"),
        make_snippet_item("xri", "xri ${1:byte}", "Logical XOR immediate"),
        make_snippet_item("ora", "ora ${1:reg}", "Logical OR with Accumulator"),
        make_snippet_item("ori", "ori ${1:byte}", "Logical OR immediate"),
        make_snippet_item("cmp", "cmp ${1:reg}", "Compare with Accumulator"),
        make_snippet_item("cpi", "cpi ${1:byte}", "Compare immediate with Accumulator"),
        make_snippet_item("rlc", "rlc", "Rotate Accumulator left"),
        make_snippet_item("rrc", "rrc", "Rotate Accumulator right"),
        make_snippet_item("ral", "ral", "Rotate Accumulator left through carry"),
        make_snippet_item("rar", "rar", "Rotate Accumulator right through carry"),
        make_snippet_item("cma", "cma", "Complement Accumulator"),
        make_snippet_item("cmc", "cmc", "Complement Carry flag"),
        make_snippet_item("stc", "stc", "Set Carry flag"),
        make_snippet_item("jmp", "jmp ${1:label}", "Unconditional jump"),
        make_snippet_item("jz", "jz ${1:label}", "Jump if Zero"),
        make_snippet_item("jnz", "jnz ${1:label}", "Jump if Not Zero"),
        make_snippet_item("jc", "jc ${1:label}", "Jump if Carry"),
        make_snippet_item("jnc", "jnc ${1:label}", "Jump if No Carry"),
        make_snippet_item("jp", "jp ${1:label}", "Jump if Positive"),
        make_snippet_item("jm", "jm ${1:label}", "Jump if Minus"),
        make_snippet_item("jpe", "jpe ${1:label}", "Jump if Parity Even"),
        make_snippet_item("jpo", "jpo ${1:label}", "Jump if Parity Odd"),
        make_snippet_item("call", "call ${1:subroutine}", "Call subroutine"),
        make_snippet_item("cz", "cz ${1:subroutine}", "Call if Zero"),
        make_snippet_item("cnz", "cnz ${1:subroutine}", "Call if Not Zero"),
        make_snippet_item("cc", "cc ${1:subroutine}", "Call if Carry"),
        make_snippet_item("cnc", "cnc ${1:subroutine}", "Call if No Carry"),
        make_snippet_item("cp", "cp ${1:subroutine}", "Call if Positive"),
        make_snippet_item("cm", "cm ${1:subroutine}", "Call if Minus"),
        make_snippet_item("cpe", "cpe ${1:subroutine}", "Call if Parity Even"),
        make_snippet_item("cpo", "cpo ${1:subroutine}", "Call if Parity Odd"),
        make_snippet_item("ret", "ret", "Return from subroutine"),
        make_snippet_item("rz", "rz", "Return if Zero"),
        make_snippet_item("rnz", "rnz", "Return if Not Zero"),
        make_snippet_item("rc", "rc", "Return if Carry"),
        make_snippet_item("rnc", "rnc", "Return if No Carry"),
        make_snippet_item("rp", "rp", "Return if Positive"),
        make_snippet_item("rm", "rm", "Return if Minus"),
        make_snippet_item("rpe", "rpe", "Return if Parity Even"),
        make_snippet_item("rpo", "rpo", "Return if Parity Odd"),
        make_snippet_item(
            "rst",
            "rst ${1|0,1,2,3,4,5,6,7|}",
            "Restart software interrupt",
        ),
        make_snippet_item("push", "push ${1|BC,DE,HL,PSW|}", "Push pair onto stack"),
        make_snippet_item("pop", "pop ${1|BC,DE,HL,PSW|}", "Pop pair from stack"),
        make_snippet_item("in", "in ${1:port}", "Input from I/O port"),
        make_snippet_item("out", "out ${1:port}", "Output to I/O port"),
        make_snippet_item("nop", "nop", "No operation"),
        make_snippet_item("hlt", "hlt", "Halt CPU"),
        make_snippet_item("ei", "ei", "Enable maskable interrupts"),
        make_snippet_item("di", "di", "Disable maskable interrupts"),
        make_snippet_item("rim", "rim", "Read interrupt mask & SID"),
        make_snippet_item("sim", "sim", "Set interrupt mask & SOD"),
        make_snippet_item("pchl", "pchl", "Jump to HL indirect"),
        make_snippet_item("sphl", "sphl", "Copy HL to Stack Pointer"),
        make_snippet_item("xthl", "xthl", "Exchange top of stack with HL"),
    ]
}

fn make_snippet_item(label: &str, snippet: &str, doc: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(doc.to_string()),
        insert_text: Some(snippet.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

fn is_reserved_keyword(word: &str) -> bool {
    matches!(
        word.to_uppercase().as_str(),
        "MOV"
            | "MVI"
            | "LXI"
            | "LDA"
            | "STA"
            | "LHLD"
            | "SHLD"
            | "LDAX"
            | "STAX"
            | "XCHG"
            | "XTHL"
            | "SPHL"
            | "PCHL"
            | "ADD"
            | "ADI"
            | "ADC"
            | "ACI"
            | "SUB"
            | "SUI"
            | "SBB"
            | "SBI"
            | "INR"
            | "DCR"
            | "INX"
            | "DCX"
            | "DAD"
            | "DAA"
            | "ANA"
            | "ANI"
            | "XRA"
            | "XRI"
            | "ORA"
            | "ORI"
            | "CMP"
            | "CPI"
            | "CMA"
            | "CMC"
            | "STC"
            | "RLC"
            | "RRC"
            | "RAL"
            | "RAR"
            | "PUSH"
            | "POP"
            | "IN"
            | "OUT"
            | "NOP"
            | "HLT"
            | "EI"
            | "DI"
            | "RIM"
            | "SIM"
            | "JMP"
            | "JZ"
            | "JNZ"
            | "JC"
            | "JNC"
            | "JP"
            | "JM"
            | "JPE"
            | "JPO"
            | "CALL"
            | "CZ"
            | "CNZ"
            | "CC"
            | "CNC"
            | "CP"
            | "CM"
            | "CPE"
            | "CPO"
            | "RET"
            | "RZ"
            | "RNZ"
            | "RC"
            | "RNC"
            | "RP"
            | "RM"
            | "RPE"
            | "RPO"
            | "RST"
            | "BYTE"
            | "WORD"
            | "SEGMENT"
            | "GLOBAL"
            | "EXTERN"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_instruction_completions_only_in_text_segment() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "segment .text\nmain:\n    ".to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 2,
                character: 4,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "lxi"));
            assert!(list.items.iter().any(|item| item.label == "mov"));
            assert!(list.items.iter().any(|item| item.label == "call"));
        } else {
            panic!("expected list of completions");
        }
    }

    #[test]
    fn test_empty_line_and_percent_prefix_directives() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        // 1. Empty line before any segment (header)
        let text_empty = "".to_string();
        let doc_empty = Document::new(uri.clone(), 1, text_empty);
        let res_empty = get_completions(
            &doc_empty,
            &Position {
                line: 0,
                character: 0,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_empty {
            assert!(list.items.iter().any(|item| item.label == "%define"));
            assert!(list.items.iter().any(|item| item.label == "%include"));
            assert!(list.items.iter().any(|item| item.label == "segment .text"));
            assert!(list.items.iter().any(|item| item.label == "segment .data"));
            assert!(list.items.iter().any(|item| item.label == "segment .bss"));
        } else {
            panic!("expected list");
        }

        // 2. Typing '%' in header/text
        let text_pct = "%".to_string();
        let doc_pct = Document::new(uri.clone(), 1, text_pct);
        let res_pct = get_completions(
            &doc_pct,
            &Position {
                line: 0,
                character: 1,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_pct {
            let def_item = list.items.iter().find(|item| item.label == "define").unwrap();
            assert_eq!(def_item.insert_text.as_deref(), Some("define ${1:NAME} ${2:value}"));
            let inc_item = list.items.iter().find(|item| item.label == "include").unwrap();
            assert_eq!(inc_item.insert_text.as_deref(), Some("include \"${1}\""));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_define_value_completion() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "%define BUFFER_SIZE 64\n%define MAX_LIMIT ".to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 1,
                character: 18,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "BUFFER_SIZE"));
            assert!(!list.items.iter().any(|item| item.label == "MAX_LIMIT"));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_len_directive_suggests_variables() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
segment .data
scores BYTE 10, 20

segment .bss
buffer BYTE 64

segment .text
main:
    mvi A, %len 
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 9,
                character: 16,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "scores"));
            assert!(list.items.iter().any(|item| item.label == "buffer"));
            // Shouldn't suggest labels or non-variables
            assert!(!list.items.iter().any(|item| item.label == "main"));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_repeat_directive_operands() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
%define COUNT 10

segment .data
arr BYTE %repeat 
arr2 BYTE %repeat 10 
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        // 1. %repeat <cursor> (1st operand: count -> suggest constants)
        let res_op1 = get_completions(
            &doc,
            &Position {
                line: 4,
                character: 17,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_op1 {
            assert!(list.items.iter().any(|item| item.label == "COUNT"));
        } else {
            panic!("expected list");
        }

        // 2. %repeat 10 <cursor> (2nd operand: value -> None)
        let res_op2 = get_completions(
            &doc,
            &Position {
                line: 5,
                character: 21,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_op2 {
            assert!(list.items.is_empty(), "expected no suggestions for repeat 2nd operand");
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_data_and_bss_segment_variable_typing_flow() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
%define MAX_LEN 32

segment .data
    
my_var 
my_var BYTE 

segment .bss
    
buf 
buf BYTE 
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        // 1. Empty line in .data -> no suggestions
        let res_data_empty = get_completions(
            &doc,
            &Position {
                line: 4,
                character: 4,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_data_empty {
            assert!(list.items.is_empty(), "expected 0 completions on empty line in data");
        } else {
            panic!("expected list");
        }

        // 2. Variable name provided: "my_var " -> suggest BYTE, WORD
        let res_data_var = get_completions(
            &doc,
            &Position {
                line: 5,
                character: 7,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_data_var {
            assert!(list.items.iter().any(|item| item.label == "BYTE"));
            assert!(list.items.iter().any(|item| item.label == "WORD"));
        } else {
            panic!("expected list");
        }

        // 3. Variable + size: "my_var BYTE " in .data -> suggest %repeat, %len, constants, string
        let res_data_val = get_completions(
            &doc,
            &Position {
                line: 6,
                character: 12,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_data_val {
            assert!(list.items.iter().any(|item| item.label == "%repeat"));
            assert!(list.items.iter().any(|item| item.label == "%len my_var"));
            assert!(list.items.iter().any(|item| item.label == "MAX_LEN"));
            assert!(list.items.iter().any(|item| item.label == "string"));
        } else {
            panic!("expected list");
        }

        // 4. Empty line in .bss -> no suggestions
        let res_bss_empty = get_completions(
            &doc,
            &Position {
                line: 9,
                character: 4,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_bss_empty {
            assert!(list.items.is_empty(), "expected 0 completions on empty line in bss");
        } else {
            panic!("expected list");
        }

        // 5. Buffer name provided: "buf " -> suggest BYTE, WORD
        let res_bss_buf = get_completions(
            &doc,
            &Position {
                line: 10,
                character: 4,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_bss_buf {
            assert!(list.items.iter().any(|item| item.label == "BYTE"));
            assert!(list.items.iter().any(|item| item.label == "WORD"));
        } else {
            panic!("expected list");
        }

        // 6. Buffer + size: "buf BYTE " in .bss -> suggest %len, constants (NO %repeat)
        let res_bss_val = get_completions(
            &doc,
            &Position {
                line: 11,
                character: 9,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_bss_val {
            assert!(list.items.iter().any(|item| item.label == "%len my_var"));
            assert!(list.items.iter().any(|item| item.label == "MAX_LEN"));
            assert!(!list.items.iter().any(|item| item.label == "%repeat"));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_call_operand_suggests_labels() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
extern print_str

segment .text
func_a:
.loop:
    dcr A
    jnz .loop
    ret

main:
    call 
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 11,
                character: 9,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "print_str"));
            assert!(list.items.iter().any(|item| item.label == "func_a"));
            assert!(list.items.iter().any(|item| item.label == "main"));
            // Shouldn't suggest instructions like `mov` or registers like `A` after call
            assert!(!list.items.iter().any(|item| item.label == "mov"));
            assert!(!list.items.iter().any(|item| item.label == "A"));
        } else {
            panic!("expected list of completions");
        }
    }

    #[test]
    fn test_contextual_register_completions() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "segment .text\nmain:\n    lxi ".to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 2,
                character: 8,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "HL"));
            assert!(list.items.iter().any(|item| item.label == "BC"));
            assert!(list.items.iter().any(|item| item.label == "SP"));
        } else {
            panic!("expected list of completions");
        }
    }

    #[test]
    fn test_immediate_and_address_operand_completions() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
%define MAX_LIMIT 100

segment .data
; comment_var BYTE 5
scores BYTE 10, 20

segment .bss
buffer byte 64

segment .text
main:
    mvi A, 
    adi 
    lda 
    lxi H, 
    hlt
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        // 1. mvi A, <cursor> -> constants and %len for both data and lowercase BSS variables
        let res_mvi = get_completions(
            &doc,
            &Position {
                line: 12,
                character: 11,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_mvi {
            assert!(list.items.iter().any(|item| item.label == "MAX_LIMIT"));
            assert!(list.items.iter().any(|item| item.label == "%len scores"));
            assert!(list.items.iter().any(|item| item.label == "%len buffer"));
            // Comments must NOT be suggested as variables
            assert!(
                !list
                    .items
                    .iter()
                    .any(|item| item.label.contains("comment_var"))
            );
        } else {
            panic!("expected list");
        }

        // 2. adi <cursor> -> constants
        let res_adi = get_completions(
            &doc,
            &Position {
                line: 13,
                character: 8,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_adi {
            assert!(list.items.iter().any(|item| item.label == "MAX_LIMIT"));
        } else {
            panic!("expected list");
        }

        // 3. lda <cursor> -> variables and constants (no labels)
        let res_lda = get_completions(
            &doc,
            &Position {
                line: 14,
                character: 8,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_lda {
            assert!(list.items.iter().any(|item| item.label == "scores"));
            assert!(list.items.iter().any(|item| item.label == "buffer"));
            assert!(list.items.iter().any(|item| item.label == "MAX_LIMIT"));
            assert!(!list.items.iter().any(|item| item.label == "main"));
            assert!(
                !list
                    .items
                    .iter()
                    .any(|item| item.label.contains("comment_var"))
            );
        } else {
            panic!("expected list");
        }

        // 4. lxi H, <cursor> -> variables & constants (no labels)
        let res_lxi = get_completions(
            &doc,
            &Position {
                line: 15,
                character: 11,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_lxi {
            assert!(list.items.iter().any(|item| item.label == "scores"));
            assert!(list.items.iter().any(|item| item.label == "buffer"));
            assert!(list.items.iter().any(|item| item.label == "MAX_LIMIT"));
            assert!(!list.items.iter().any(|item| item.label == "main"));
            assert!(
                !list
                    .items
                    .iter()
                    .any(|item| item.label.contains("comment_var"))
            );
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_duplicate_segment_not_suggested_and_missing_suggested() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
segment .data
arr BYTE 10

segment .text
main:
    
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 6,
                character: 4,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            // .data and .text are already present, so only .bss should be suggested as missing segment
            assert!(list.items.iter().any(|item| item.label == "segment .bss"));
            assert!(!list.items.iter().any(|item| item.label == "segment .data"));
            assert!(!list.items.iter().any(|item| item.label == "segment .text"));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_no_completions_when_instruction_operands_satisfied() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
segment .text
main:
    nop 
    hlt 
    add B 
    mov A, B 
    lxi HL, 0x1000 
    jmp main 
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        // nop <space>
        let res_nop = get_completions(
            &doc,
            &Position {
                line: 3,
                character: 8,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_nop {
            assert!(list.items.is_empty(), "expected 0 completions after nop <space>");
        }

        // mov A, B <space>
        let res_mov = get_completions(
            &doc,
            &Position {
                line: 6,
                character: 13,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_mov {
            assert!(list.items.is_empty(), "expected 0 completions after mov A, B <space>");
        }

        // lxi HL, 0x1000 <space>
        let res_lxi = get_completions(
            &doc,
            &Position {
                line: 7,
                character: 19,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_lxi {
            assert!(list.items.is_empty(), "expected 0 completions after lxi HL, addr <space>");
        }
    }

    #[test]
    fn test_extern_completion_suggests_no_instructions() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "extern \nsegment .text\nmain:\n    hlt".to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 0,
                character: 7,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(!list.items.iter().any(|item| item.label == "mov"));
            assert!(!list.items.iter().any(|item| item.label == "lxi"));
            assert!(!list.items.iter().any(|item| item.label == "segment .text"));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_global_completion_suggests_local_labels_not_instructions() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
global 

segment .text
helper_fn:
    ret

calc_sum:
    ret

global already_exported:
    ret

main:
    ret
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 1,
                character: 7,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "helper_fn"));
            assert!(list.items.iter().any(|item| item.label == "calc_sum"));
            assert!(!list.items.iter().any(|item| item.label == "main"));
            assert!(!list.items.iter().any(|item| item.label == "already_exported"));
            assert!(!list.items.iter().any(|item| item.label == "mov"));
            assert!(!list.items.iter().any(|item| item.label == "segment .text"));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_conditional_directives_and_defined_constant_completions() {
        let uri = Url::parse("file:///cond_comp.e8085").unwrap();
        let text = r#"%define DEBUG
%define BUFFER_SIZE 64

%if 
%
"#
        .to_string();
        let doc = Document::new(uri, 1, text);

        // 1. Typing after `%if ` on line 3
        let res_if = get_completions(
            &doc,
            &Position {
                line: 3,
                character: 4,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_if {
            assert!(list.items.iter().any(|item| item.label == "DEBUG"));
            assert!(list.items.iter().any(|item| item.label == "BUFFER_SIZE"));
        } else {
            panic!("expected list");
        }

        // 2. Typing after `%` on line 4
        let res_pct = get_completions(
            &doc,
            &Position {
                line: 4,
                character: 1,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_pct {
            assert!(list.items.iter().any(|item| item.label == "if"));
            assert!(list.items.iter().any(|item| item.label == "elif"));
            assert!(list.items.iter().any(|item| item.label == "else"));
            assert!(list.items.iter().any(|item| item.label == "endif"));
            assert!(list.items.iter().any(|item| item.label == "ifdef"));
            assert!(list.items.iter().any(|item| item.label == "ifndef"));
            assert!(list.items.iter().any(|item| item.label == "define"));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_include_path_completion_with_trigger_suggest() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let main_file = manifest.join("main.e8085");
        let uri = Url::from_file_path(&main_file).unwrap();
        let text = "%include <".to_string();
        let doc = Document::new(uri.clone(), 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 0,
                character: 10,
            },
        )
        .unwrap();

        if let CompletionResponse::List(list) = res {
            // Check that device/ folder is offered with triggerSuggest
            let folder_item = list.items.iter().find(|item| item.label == "device/").expect("expected device/ folder");
            assert_eq!(folder_item.kind, Some(CompletionItemKind::FOLDER));
            assert!(folder_item.command.as_ref().map(|c| c.command.as_str()) == Some("editor.action.triggerSuggest"));

            // Check that math module is offered
            assert!(list.items.iter().any(|item| item.label == "math" && item.kind == Some(CompletionItemKind::MODULE)));
        } else {
            panic!("expected list");
        }

        let text_device = "%include <device/".to_string();
        let doc_device = Document::new(uri.clone(), 2, text_device);
        let res_device = get_completions(
            &doc_device,
            &Position {
                line: 0,
                character: 17,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_device {
            assert!(list.items.iter().any(|item| item.label == "canvas" && item.kind == Some(CompletionItemKind::MODULE)));
            assert!(list.items.iter().any(|item| item.label == "text_io" && item.kind == Some(CompletionItemKind::MODULE)));
        } else {
            panic!("expected list");
        }

        // Test unquoted raw include path completion
        let text_raw = "%include stdlib/".to_string();
        let doc_raw = Document::new(uri, 3, text_raw);
        let res_raw = get_completions(
            &doc_raw,
            &Position {
                line: 0,
                character: 16,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res_raw {
            assert!(list.items.iter().any(|item| item.label == "math.e8085"));
            assert!(list.items.iter().any(|item| item.label == "device/"));
        } else {
            panic!("expected list for raw include path");
        }
    }

    #[test]
    fn test_system_include_macro_and_operand_completions() {
        let uri = Url::parse("file:///workspace/game.e8085").unwrap();
        let text = "%include <device/canvas>\nsegment .text\nmain:\n    \n".to_string();
        let doc = Document::new(uri.clone(), 1, text);

        // 1. In text segment at indentation, macros from <device/canvas> must be suggested
        let res = get_completions(
            &doc,
            &Position {
                line: 3,
                character: 4,
            },
        )
        .unwrap();
        let items = match res {
            CompletionResponse::List(l) => l.items,
            CompletionResponse::Array(a) => a,
        };
        assert!(
            items.iter().any(|i| i.label == "InitCanvas"),
            "InitCanvas macro from <device/canvas> should be suggested in completions"
        );
        assert!(
            items.iter().any(|i| i.label == "FillRectangle"),
            "FillRectangle macro from <device/canvas> should be suggested in completions"
        );

        // 2. Typing macro operands: InitCanvas <cursor> should suggest canvas constants
        let text_op = "%include <device/canvas>\nsegment .text\nmain:\n    InitCanvas \n".to_string();
        let doc_op = Document::new(uri, 2, text_op);
        let res_op = get_completions(
            &doc_op,
            &Position {
                line: 3,
                character: 15,
            },
        )
        .unwrap();
        let items_op = match res_op {
            CompletionResponse::List(l) => l.items,
            CompletionResponse::Array(a) => a,
        };
        assert!(
            items_op.iter().any(|i| i.label == "CANVAS_CMD_PORT"),
            "CANVAS_CMD_PORT constant should be suggested as InitCanvas operand"
        );
        assert!(
            items_op.iter().any(|i| i.label == "CMD_INIT_CANVAS"),
            "CMD_INIT_CANVAS constant should be suggested as InitCanvas operand"
        );
    }

    #[test]
    fn test_transitive_include_constants_and_variables() {
        let temp_dir = std::env::temp_dir().join("emu8085_test_transitive_comp");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let sub2 = temp_dir.join("sub2.e8085");
        let sub1 = temp_dir.join("sub1.e8085");
        let main = temp_dir.join("main.e8085");

        std::fs::write(&sub2, "%define SUB2_VAL 0x77\nsegment .data\nsub2_var BYTE 1\n").unwrap();
        std::fs::write(&sub1, "%include \"sub2.e8085\"\n%define SUB1_VAL 0x88\n").unwrap();
        std::fs::write(&main, "%include \"sub1.e8085\"\nsegment .text\nmain:\n    mvi A, \n").unwrap();

        let doc = Document::new(
            Url::from_file_path(&main).unwrap(),
            1,
            std::fs::read_to_string(&main).unwrap(),
        );

        let res = get_completions(
            &doc,
            &Position {
                line: 3,
                character: 11,
            },
        )
        .unwrap();
        let items = match res {
            CompletionResponse::List(l) => l.items,
            CompletionResponse::Array(a) => a,
        };

        assert!(
            items.iter().any(|i| i.label == "SUB1_VAL"),
            "Immediate constant SUB1_VAL should be suggested"
        );
        assert!(
            items.iter().any(|i| i.label == "SUB2_VAL"),
            "Transitive constant SUB2_VAL should be suggested"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
