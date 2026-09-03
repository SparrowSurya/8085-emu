use std::collections::HashMap;
use tower_lsp::lsp_types::{Position, PrepareRenameResponse, Range, TextEdit, WorkspaceEdit};

use super::document::Document;

/// Prepares and validates if the symbol at cursor position can be renamed.
pub fn prepare_rename(doc: &Document, position: &Position) -> Option<PrepareRenameResponse> {
    let (word, range) = doc.get_word_at_position(position)?;

    if is_reserved_keyword(&word) || is_register(&word) || word.starts_with('%') {
        return None;
    }

    Some(PrepareRenameResponse::Range(range))
}

/// Computes workspace text edits to rename all occurrences of a symbol.
pub fn rename(doc: &Document, position: &Position, new_name: &str) -> Option<WorkspaceEdit> {
    let (old_name, _) = doc.get_word_at_position(position)?;

    if is_reserved_keyword(&old_name) || is_register(&old_name) {
        return None;
    }

    // Validate new name
    if !is_valid_identifier(new_name) || is_reserved_keyword(new_name) || is_register(new_name) {
        return None;
    }

    let mut edits = Vec::new();

    if old_name.starts_with('.') {
        // Scoped local label renaming
        edits.extend(collect_local_label_edits(
            doc, position, &old_name, new_name,
        ));
    } else {
        // Global / File-level symbol renaming
        edits.extend(collect_global_symbol_edits(doc, &old_name, new_name));
    }

    if edits.is_empty() {
        return None;
    }

    let mut changes = HashMap::new();
    changes.insert(doc.uri.clone(), edits);

    Some(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

fn collect_local_label_edits(
    doc: &Document,
    position: &Position,
    old_name: &str,
    new_name: &str,
) -> Vec<TextEdit> {
    let lines: Vec<&str> = doc.text.lines().collect();
    let current_line = position.line as usize;

    // Find parent scope
    let mut parent_start = 0;
    for i in (0..=current_line.min(lines.len().saturating_sub(1))).rev() {
        if is_parent_label_def(lines[i].trim()) {
            parent_start = i;
            break;
        }
    }

    let mut parent_end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(parent_start + 1) {
        if is_parent_label_def(line.trim()) {
            parent_end = i;
            break;
        }
    }

    let mut edits = Vec::new();

    for (idx, line) in lines[parent_start..parent_end].iter().enumerate() {
        let line_num = parent_start + idx;
        for (col_idx, _) in find_ident_occurrences_in_line(line, old_name) {
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: line_num as u32,
                        character: col_idx as u32,
                    },
                    end: Position {
                        line: line_num as u32,
                        character: (col_idx + old_name.len()) as u32,
                    },
                },
                new_text: new_name.to_string(),
            });
        }
    }

    edits
}

fn collect_global_symbol_edits(doc: &Document, old_name: &str, new_name: &str) -> Vec<TextEdit> {
    let mut edits = Vec::new();

    for (line_num, line) in doc.text.lines().enumerate() {
        for (col_idx, _) in find_ident_occurrences_in_line(line, old_name) {
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: line_num as u32,
                        character: col_idx as u32,
                    },
                    end: Position {
                        line: line_num as u32,
                        character: (col_idx + old_name.len()) as u32,
                    },
                },
                new_text: new_name.to_string(),
            });
        }
    }

    edits
}

fn find_ident_occurrences_in_line(line: &str, ident: &str) -> Vec<(usize, usize)> {
    let mut results = Vec::new();
    let bytes = line.as_bytes();
    let ident_bytes = ident.as_bytes();

    if bytes.is_empty() || ident_bytes.is_empty() || bytes.len() < ident_bytes.len() {
        return results;
    }

    let mut i = 0;
    while i + ident_bytes.len() <= bytes.len() {
        // Stop if in comment
        if bytes[i] == b';' {
            break;
        }

        if &bytes[i..i + ident_bytes.len()] == ident_bytes {
            let prev_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let next_ok = (i + ident_bytes.len() >= bytes.len())
                || !is_ident_char(bytes[i + ident_bytes.len()]);

            if prev_ok && next_ok {
                results.push((i, i + ident_bytes.len()));
                i += ident_bytes.len();
                continue;
            }
        }
        i += 1;
    }

    results
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let clean = if let Some(local) = name.strip_prefix('.') {
        if local.is_empty() {
            return false;
        }
        local
    } else {
        name
    };

    let first = clean.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    clean.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_parent_label_def(trimmed_line: &str) -> bool {
    if let Some(rest) = trimmed_line.strip_suffix(':') {
        let label = rest.strip_prefix("global ").unwrap_or(rest).trim();
        !label.starts_with('.') && !label.is_empty()
    } else {
        false
    }
}

fn is_register(word: &str) -> bool {
    matches!(
        word.to_uppercase().as_str(),
        "A" | "B" | "C" | "D" | "E" | "H" | "L" | "M" | "BC" | "DE" | "HL" | "SP" | "PSW"
    )
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
    fn test_rename_global_label() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "\
main:
    call compute
    jmp main

compute:
    ret
"
        .to_string();
        let doc = Document::new(uri.clone(), 1, text);

        let edit = rename(
            &doc,
            &Position {
                line: 1,
                character: 11,
            },
            "calculate",
        )
        .unwrap();
        let changes = edit.changes.unwrap();
        let file_edits = changes.get(&uri).unwrap();

        assert_eq!(file_edits.len(), 2);
        assert_eq!(file_edits[0].new_text, "calculate");
        assert_eq!(file_edits[1].new_text, "calculate");
    }

    #[test]
    fn test_rename_scoped_local_label() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "\
func_a:
.loop:
    dcr A
    jnz .loop
    ret

func_b:
.loop:
    dcr B
    jnz .loop
    ret
"
        .to_string();
        let doc = Document::new(uri.clone(), 1, text);

        // Rename .loop in func_b (line 9: "    jnz .loop")
        let edit = rename(
            &doc,
            &Position {
                line: 9,
                character: 9,
            },
            ".repeat",
        )
        .unwrap();
        let changes = edit.changes.unwrap();
        let file_edits = changes.get(&uri).unwrap();

        // Should ONLY rename occurrences in func_b (lines 7 & 9)
        assert_eq!(file_edits.len(), 2);
        assert_eq!(file_edits[0].range.start.line, 7);
        assert_eq!(file_edits[1].range.start.line, 9);
    }
}
