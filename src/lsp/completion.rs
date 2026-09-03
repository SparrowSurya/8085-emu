use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionResponse, InsertTextFormat,
    Position,
};

use super::document::{Document, resolve_relative_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentKind {
    Text,
    Data,
    Bss,
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

    let active_segment = get_active_segment(doc, position.line as usize);
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
                    items.extend(get_header_directives());
                } else {
                    items.extend(get_instruction_completions());
                    items.extend(get_text_directives());
                    items.extend(get_label_completions(doc, position));
                }
            }
        }
        Some(SegmentKind::Data) => {
            if line_prefix.starts_with('%') {
                items.extend(get_data_directives());
            } else {
                items.extend(get_data_directives());
                items.extend(get_data_segment_completions());
                items.extend(get_constant_and_len_completions(doc));
            }
        }
        Some(SegmentKind::Bss) => {
            items.extend(get_bss_segment_completions());
        }
        None => {
            // Before any segment declaration (file header)
            items.extend(get_header_directives());
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
    let code_line = line_prefix.split(';').next().unwrap_or("").trim_start();
    if code_line.is_empty() {
        return None;
    }

    // Split on whitespace or comma to detect mnemonic and operand position
    let parts: Vec<&str> = code_line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mnemonic = parts[0].to_uppercase();
    let has_comma = code_line.contains(',');

    match mnemonic.as_str() {
        // 1. Branch & Subroutine Call Target Labels
        "CALL" | "CZ" | "CNZ" | "CC" | "CNC" | "CP" | "CM" | "CPE" | "CPO" | "JMP" | "JZ"
        | "JNZ" | "JC" | "JNC" | "JP" | "JM" | "JPE" | "JPO" => {
            Some(get_label_completions(doc, position))
        }

        // 2. Direct Address instructions (LDA, STA, LHLD, SHLD)
        "LDA" | "STA" | "LHLD" | "SHLD" => Some(get_memory_symbol_completions(doc)),

        // 3. LXI: Operand 1 = Reg Pair, Operand 2 = Address / Symbol / Constant
        "LXI" => {
            if has_comma {
                Some(get_memory_symbol_completions(doc))
            } else {
                Some(vec![
                    make_reg_item("BC", "Register Pair BC (16-bit)"),
                    make_reg_item("DE", "Register Pair DE (16-bit)"),
                    make_reg_item("HL", "Register Pair HL (16-bit)"),
                    make_reg_item("SP", "Stack Pointer SP (16-bit)"),
                ])
            }
        }

        // 4. MOV: Both operands are 8-bit registers / M
        "MOV" => Some(get_8bit_registers()),

        // 5. MVI: Operand 1 = Reg, Operand 2 = Constants / Immediates
        "MVI" => {
            if has_comma {
                Some(get_constant_and_len_completions(doc))
            } else {
                Some(get_8bit_registers())
            }
        }

        // 6. Arithmetic / Logic register instructions
        "ADD" | "ADC" | "SUB" | "SBB" | "INR" | "DCR" | "ANA" | "XRA" | "ORA" | "CMP" => {
            Some(get_8bit_registers())
        }

        // 7. Register Pair (INX, DCX, DAD)
        "INX" | "DCX" | "DAD" => Some(vec![
            make_reg_item("BC", "Register Pair BC (16-bit)"),
            make_reg_item("DE", "Register Pair DE (16-bit)"),
            make_reg_item("HL", "Register Pair HL (16-bit)"),
            make_reg_item("SP", "Stack Pointer SP (16-bit)"),
        ]),

        // 8. Stack Pair (PUSH, POP)
        "PUSH" | "POP" => Some(vec![
            make_reg_item("BC", "Register Pair BC (16-bit)"),
            make_reg_item("DE", "Register Pair DE (16-bit)"),
            make_reg_item("HL", "Register Pair HL (16-bit)"),
            make_reg_item("PSW", "Program Status Word (A + Flags)"),
        ]),

        // 9. Indirect Load/Store (LDAX, STAX)
        "LDAX" | "STAX" => Some(vec![
            make_reg_item("BC", "Register Pair BC pointer"),
            make_reg_item("DE", "Register Pair DE pointer"),
        ]),

        // 10. Immediate Arithmetic (ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI)
        "ADI" | "ACI" | "SUI" | "SBI" | "ANI" | "XRI" | "ORI" | "CPI" => {
            Some(get_constant_and_len_completions(doc))
        }

        // 11. I/O Ports
        "IN" | "OUT" => Some(get_constant_and_len_completions(doc)),

        // 12. RST
        "RST" => Some(vec![
            make_snippet_item("0", "0", "RST 0 (0x0000)"),
            make_snippet_item("1", "1", "RST 1 (0x0008)"),
            make_snippet_item("2", "2", "RST 2 (0x0010)"),
            make_snippet_item("3", "3", "RST 3 (0x0018)"),
            make_snippet_item("4", "4", "RST 4 (0x0020)"),
            make_snippet_item("5", "5", "RST 5 (0x0028)"),
            make_snippet_item("6", "6", "RST 6 (0x0030)"),
            make_snippet_item("7", "7", "RST 7 (0x0038)"),
        ]),

        _ => None,
    }
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

fn get_memory_symbol_completions(doc: &Document) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = code_line.split_whitespace().collect();

        // 1. Data & BSS Variables
        if !parts.is_empty()
            && !is_reserved_keyword(parts[0])
            && !parts[0].starts_with('%')
            && !parts[0].ends_with(':')
            && (code_line.contains('"') || code_line.contains("BYTE") || code_line.contains("WORD"))
        {
            items.push(CompletionItem {
                label: parts[0].to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("Data Variable".to_string()),
                ..Default::default()
            });
        }

        // 2. Constants (%define)
        if code_line.starts_with("%define") && parts.len() >= 2 {
            items.push(CompletionItem {
                label: parts[1].to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Defined Constant".to_string()),
                ..Default::default()
            });
        }

        // 3. Global Labels
        if let Some(rest) = code_line.strip_suffix(':') {
            let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
            if !clean.starts_with('.') && !clean.is_empty() {
                items.push(CompletionItem {
                    label: clean.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("Label Address".to_string()),
                    ..Default::default()
                });
            }
        }
    }

    // 4. Imported symbols from %include
    for line in doc.text.lines() {
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
                                    let parts: Vec<&str> = inc_code.split_whitespace().collect();
                                    if !parts.is_empty()
                                        && !is_reserved_keyword(parts[0])
                                        && !parts[0].starts_with('%')
                                        && !parts[0].ends_with(':')
                                        && (inc_code.contains('"')
                                            || inc_code.contains("BYTE")
                                            || inc_code.contains("WORD"))
                                    {
                                        items.push(CompletionItem {
                                            label: parts[0].to_string(),
                                            kind: Some(CompletionItemKind::VARIABLE),
                                            detail: Some(format!("Variable from {}", rel_path)),
                                            ..Default::default()
                                        });
                                    }
                                    if inc_code.starts_with("%define") && parts.len() >= 2 {
                                        items.push(CompletionItem {
                                            label: parts[1].to_string(),
                                            kind: Some(CompletionItemKind::CONSTANT),
                                            detail: Some(format!("Constant from {}", rel_path)),
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

    items
}

fn get_constant_and_len_completions(doc: &Document) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if code_line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = code_line.split_whitespace().collect();

        if code_line.starts_with("%define") && parts.len() >= 2 {
            items.push(CompletionItem {
                label: parts[1].to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Defined Constant".to_string()),
                ..Default::default()
            });
        }

        // Variables eligible for %len
        if !parts.is_empty()
            && !is_reserved_keyword(parts[0])
            && !parts[0].starts_with('%')
            && !parts[0].ends_with(':')
            && (code_line.contains('"') || code_line.contains("BYTE") || code_line.contains("WORD"))
        {
            items.push(CompletionItem {
                label: format!("%len {}", parts[0]),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("Byte length of {}", parts[0])),
                insert_text: Some(format!("%len {}", parts[0])),
                ..Default::default()
            });
        }
    }

    items
}

fn get_header_directives() -> Vec<CompletionItem> {
    vec![
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
        make_snippet_item(
            "segment .text",
            "segment .text\n$0",
            "Executable code segment",
        ),
        make_snippet_item(
            "segment .data",
            "segment .data\n$0",
            "Initialized data segment",
        ),
        make_snippet_item(
            "segment .bss",
            "segment .bss\n$0",
            "Uninitialized memory buffer segment",
        ),
    ]
}

fn get_text_directives() -> Vec<CompletionItem> {
    vec![
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
            "segment .data",
            "segment .data\n$0",
            "Switch to data segment",
        ),
        make_snippet_item("segment .bss", "segment .bss\n$0", "Switch to bss segment"),
    ]
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

fn get_data_segment_completions() -> Vec<CompletionItem> {
    vec![
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
        make_snippet_item(
            "segment .text",
            "segment .text\n$0",
            "Switch to text segment",
        ),
        make_snippet_item("segment .bss", "segment .bss\n$0", "Switch to bss segment"),
    ]
}

fn get_bss_segment_completions() -> Vec<CompletionItem> {
    vec![
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
        make_snippet_item(
            "segment .text",
            "segment .text\n$0",
            "Switch to text segment",
        ),
        make_snippet_item(
            "segment .data",
            "segment .data\n$0",
            "Switch to data segment",
        ),
    ]
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
buffer BYTE 64

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

        // 1. mvi A, <cursor> -> constants
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

        // 3. lda <cursor> -> variables
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
            assert!(
                !list
                    .items
                    .iter()
                    .any(|item| item.label.contains("comment_var"))
            );
        } else {
            panic!("expected list");
        }

        // 4. lxi H, <cursor> -> variables & constants
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
