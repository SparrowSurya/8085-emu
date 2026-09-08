use std::collections::HashSet;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionResponse, InsertTextFormat,
    Position,
};

use super::document::{Document, resolve_relative_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SegmentKind {
    Text,
    Data,
    Bss,
}

/// Detects all segment kinds declared in the current document.
fn get_present_segments(doc: &Document) -> HashSet<SegmentKind> {
    let mut present = HashSet::new();
    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.to_lowercase().starts_with("segment ") {
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
    let line_prefix = line[..char_idx].trim_start();

    // 1. Check if typing path inside `%include "..."`
    if let Some(include_items) = get_include_path_completions(doc, position, line, char_idx) {
        return Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: include_items,
        }));
    }

    let lower_prefix = line_prefix.to_lowercase();

    // 2. Check if typing after `extern ` (Issue 6: suggest only external symbols, no CPU instructions)
    if lower_prefix.starts_with("extern ")
        || (lower_prefix == "extern" && line[..char_idx].ends_with(char::is_whitespace))
    {
        let extern_items = get_extern_symbol_completions(doc);
        return Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: extern_items,
        }));
    }

    // 3. Check if typing after `global ` (Issue 7: suggest only unexported local labels, no CPU instructions)
    if lower_prefix.starts_with("global ")
        || (lower_prefix == "global" && line[..char_idx].ends_with(char::is_whitespace))
    {
        let global_items = get_global_declaration_completions(doc);
        return Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: global_items,
        }));
    }

    let active_segment = get_active_segment(doc, position.line as usize);
    let present_segments = get_present_segments(doc);
    let mut items = Vec::new();

    match active_segment {
        Some(SegmentKind::Text) => {
            // Check if cursor is inside an instruction operand context
            if let Some(operand_items) =
                get_instruction_operand_completions(doc, position, line_prefix)
            {
                items.extend(operand_items);
            } else {
                // At instruction position (start of line / indentation)
                if line_prefix.starts_with('%') {
                    items.extend(get_header_directives(&present_segments));
                } else {
                    items.extend(get_instruction_completions());
                    items.extend(get_text_directives(&present_segments));
                    items.extend(get_label_completions(doc, position));
                }
            }
        }
        Some(SegmentKind::Data) => {
            if line_prefix.starts_with('%') {
                items.extend(get_data_directives());
            } else {
                items.extend(get_data_directives());
                items.extend(get_data_segment_completions(&present_segments));
                items.extend(get_constant_and_len_completions(doc));
            }
        }
        Some(SegmentKind::Bss) => {
            items.extend(get_bss_segment_completions(&present_segments));
        }
        None => {
            // Before any segment declaration (file header)
            items.extend(get_header_directives(&present_segments));
        }
    }

    Some(CompletionResponse::List(CompletionList {
        is_incomplete: false,
        items,
    }))
}

fn get_include_path_completions(
    doc: &Document,
    _position: &Position,
    line: &str,
    char_idx: usize,
) -> Option<Vec<CompletionItem>> {
    let prefix = &line[..char_idx];
    let trimmed = prefix.trim_start();
    if !trimmed.starts_with("%include") {
        return None;
    }

    let quote_pos = prefix.find('"').or_else(|| prefix.find('\''))?;
    if char_idx <= quote_pos {
        return None;
    }

    let raw_rel = &prefix[quote_pos + 1..char_idx];
    let doc_path = doc.uri.to_file_path().ok()?;
    let base_dir = doc_path.parent()?;

    let (search_dir, typed_prefix) = if let Some(last_slash) = raw_rel.rfind('/') {
        let sub = &raw_rel[..last_slash];
        let pre = &raw_rel[last_slash + 1..];
        (base_dir.join(sub), pre)
    } else {
        (base_dir.to_path_buf(), raw_rel)
    };

    let entries = std::fs::read_dir(&search_dir).ok()?;
    let mut items = Vec::new();

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
                    insert_text: Some(format!("{file_name}/")),
                    ..Default::default()
                });
            } else if file_type.is_file() {
                if file_name.ends_with(".e8085") || file_name.ends_with(".inc") {
                    items.push(CompletionItem {
                        label: file_name.clone(),
                        kind: Some(CompletionItemKind::FILE),
                        detail: Some("8085 Assembly file".to_string()),
                        insert_text: Some(file_name),
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

        _ => None,
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

    // 4. Labels from %include imported files
    for line in &raw_lines {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.starts_with("%include") {
            if let Some(quote_char) = if code_line.contains('"') {
                Some('"')
            } else if code_line.contains('\'') {
                Some('\'')
            } else {
                None
            } {
                if let Some(start_q) = code_line.find(quote_char) {
                    if let Some(end_q) = code_line[start_q + 1..].find(quote_char) {
                        let rel_path = &code_line[start_q + 1..start_q + 1 + end_q];
                        if let Some(target_path) = resolve_relative_path(&doc.uri, rel_path) {
                            if let Ok(content) = std::fs::read_to_string(target_path) {
                                for inc_line in content.lines() {
                                    let inc_code = inc_line.split(';').next().unwrap_or("").trim();
                                    if let Some(rest) = inc_code.strip_suffix(':') {
                                        let clean =
                                            rest.strip_prefix("global ").unwrap_or(rest).trim();
                                        if !clean.starts_with('.') && !clean.is_empty() {
                                            items.push(CompletionItem {
                                                label: clean.to_string(),
                                                kind: Some(CompletionItemKind::FUNCTION),
                                                detail: Some(format!("Imported from {}", rel_path)),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
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

        if code_line.to_lowercase().starts_with("segment ") {
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

    // 2. Scan %include files
    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.to_lowercase().starts_with("%include") {
            if let Some(quote_char) = if code_line.contains('"') {
                Some('"')
            } else if code_line.contains('\'') {
                Some('\'')
            } else {
                None
            } {
                if let Some(start_q) = code_line.find(quote_char) {
                    if let Some(end_q) = code_line[start_q + 1..].find(quote_char) {
                        let rel_path = &code_line[start_q + 1..start_q + 1 + end_q];
                        if let Some(target_path) = resolve_relative_path(&doc.uri, rel_path) {
                            if let Ok(content) = std::fs::read_to_string(&target_path) {
                                let mut inc_seg: Option<SegmentKind> = None;
                                for inc_line in content.lines() {
                                    let inc_code = inc_line.split(';').next().unwrap_or("").trim();
                                    if inc_code.is_empty() {
                                        continue;
                                    }
                                    if inc_code.to_lowercase().starts_with("segment ") {
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
                                        let parts: Vec<&str> =
                                            inc_code.split_whitespace().collect();
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
                                    if is_reserved_keyword(first)
                                        || first.starts_with('%')
                                        || first.is_empty()
                                    {
                                        continue;
                                    }
                                    let lower_inc = inc_code.to_lowercase();
                                    match inc_seg {
                                        Some(SegmentKind::Data) => {
                                            vars.push(VariableInfo {
                                                name: first.to_string(),
                                                detail: format!("Data Variable from {}", rel_path),
                                            });
                                        }
                                        Some(SegmentKind::Bss) => {
                                            vars.push(VariableInfo {
                                                name: first.to_string(),
                                                detail: format!("BSS Buffer from {}", rel_path),
                                            });
                                        }
                                        _ => {
                                            if lower_inc.contains("byte")
                                                || lower_inc.contains("word")
                                                || lower_inc.contains('"')
                                            {
                                                vars.push(VariableInfo {
                                                    name: first.to_string(),
                                                    detail: format!("Variable from {}", rel_path),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
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

fn get_header_directives(present: &HashSet<SegmentKind>) -> Vec<CompletionItem> {
    let mut items = vec![
        make_snippet_item(
            "%define",
            "%define ${1:NAME} ${2:value}",
            "Define a constant value",
        ),
        make_snippet_item(
            "%include",
            "%include \"${1:path/to/file.e8085}\"",
            "Include an external source file",
        ),
    ];
    if !present.contains(&SegmentKind::Text) {
        items.push(make_snippet_item(
            "segment .text",
            "segment .text\n$0",
            "Executable code segment",
        ));
    }
    if !present.contains(&SegmentKind::Data) {
        items.push(make_snippet_item(
            "segment .data",
            "segment .data\n$0",
            "Initialized data segment",
        ));
    }
    if !present.contains(&SegmentKind::Bss) {
        items.push(make_snippet_item(
            "segment .bss",
            "segment .bss\n$0",
            "Uninitialized memory buffer segment",
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
    ];
    if !present.contains(&SegmentKind::Data) {
        items.push(make_snippet_item(
            "segment .data",
            "segment .data\n$0",
            "Switch to data segment",
        ));
    }
    if !present.contains(&SegmentKind::Bss) {
        items.push(make_snippet_item(
            "segment .bss",
            "segment .bss\n$0",
            "Switch to bss segment",
        ));
    }
    items
}

fn get_data_directives() -> Vec<CompletionItem> {
    vec![
        make_snippet_item(
            "%repeat",
            "%repeat ${1:count} ${2:value}",
            "Repeat a data element multiple times",
        ),
        make_snippet_item(
            "%len",
            "%len ${1:var_name}",
            "Compute byte length of a symbol",
        ),
    ]
}

fn get_data_segment_completions(present: &HashSet<SegmentKind>) -> Vec<CompletionItem> {
    let mut items = vec![
        make_snippet_item(
            "string variable",
            "${1:name} \"${2:Hello, World!\\n}\"",
            "Declare initialized string variable",
        ),
        make_snippet_item(
            "BYTE array",
            "${1:name} BYTE ${2:10, 20, 30}",
            "Declare initialized byte array",
        ),
        make_snippet_item(
            "WORD array",
            "${1:name} WORD ${2:0x1000, 0x2000}",
            "Declare initialized 16-bit word array",
        ),
    ];
    if !present.contains(&SegmentKind::Text) {
        items.push(make_snippet_item(
            "segment .text",
            "segment .text\n$0",
            "Switch to text segment",
        ));
    }
    if !present.contains(&SegmentKind::Bss) {
        items.push(make_snippet_item(
            "segment .bss",
            "segment .bss\n$0",
            "Switch to bss segment",
        ));
    }
    items
}

fn get_bss_segment_completions(present: &HashSet<SegmentKind>) -> Vec<CompletionItem> {
    let mut items = vec![
        make_snippet_item(
            "BYTE buffer",
            "${1:name} BYTE ${2:64}",
            "Allocate uninitialized byte buffer in BSS",
        ),
        make_snippet_item(
            "WORD buffer",
            "${1:name} WORD ${2:32}",
            "Allocate uninitialized word buffer in BSS",
        ),
    ];
    if !present.contains(&SegmentKind::Text) {
        items.push(make_snippet_item(
            "segment .text",
            "segment .text\n$0",
            "Switch to text segment",
        ));
    }
    if !present.contains(&SegmentKind::Data) {
        items.push(make_snippet_item(
            "segment .data",
            "segment .data\n$0",
            "Switch to data segment",
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

    // 2. Check %include files for global exports
    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.to_lowercase().starts_with("%include") {
            if let Some(quote_char) = if code_line.contains('"') {
                Some('"')
            } else if code_line.contains('\'') {
                Some('\'')
            } else {
                None
            } {
                if let Some(start_q) = code_line.find(quote_char) {
                    if let Some(end_q) = code_line[start_q + 1..].find(quote_char) {
                        let rel_path = &code_line[start_q + 1..start_q + 1 + end_q];
                        if let Some(target_path) = resolve_relative_path(&doc.uri, rel_path) {
                            if let Ok(content) = std::fs::read_to_string(&target_path) {
                                extract_exported_globals_from_str(
                                    &content,
                                    &format!("Imported from {rel_path}"),
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

    // 3. Scan workspace / library directories (programs/lib, programs/devices)
    if let Ok(doc_path) = doc.uri.to_file_path() {
        let project_root = crate::asm::include::find_project_root(&doc_path);
        let search_dirs = [
            project_root.join("programs").join("lib"),
            project_root.join("programs").join("devices"),
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
    fn test_no_instruction_completions_in_data_or_bss_segment() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "segment .data\n    ".to_string();
        let doc = Document::new(uri.clone(), 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 1,
                character: 4,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(!list.items.iter().any(|item| item.label == "mov"));
            assert!(!list.items.iter().any(|item| item.label == "lxi"));
            assert!(list.items.iter().any(|item| item.label == "BYTE array"));
        } else {
            panic!("expected list of completions");
        }

        let bss_text = "segment .bss\n    ".to_string();
        let bss_doc = Document::new(uri, 1, bss_text);
        let bss_res = get_completions(
            &bss_doc,
            &Position {
                line: 1,
                character: 4,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = bss_res {
            assert!(!list.items.iter().any(|item| item.label == "mov"));
            assert!(list.items.iter().any(|item| item.label == "BYTE buffer"));
        } else {
            panic!("expected list of completions");
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
    fn test_data_segment_repeat_and_directives() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "segment .data\n    arr BYTE 10 ".to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(
            &doc,
            &Position {
                line: 1,
                character: 17,
            },
        )
        .unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "%repeat"));
            assert!(list.items.iter().any(|item| item.label == "%len"));
        } else {
            panic!("expected list");
        }
    }
}
