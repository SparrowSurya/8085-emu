use std::collections::HashMap;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Position, Range, TextEdit,
    WorkspaceEdit,
};

use super::document::Document;

/// Computes quick-fixes and linter code actions for the given range in the document.
pub fn get_code_actions(doc: &Document, params: &CodeActionParams) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();
    let start_line = params.range.start.line as usize;
    let end_line = params.range.end.line as usize;

    for (line_idx, line) in doc.text.lines().enumerate() {
        if line_idx < start_line || line_idx > end_line {
            continue;
        }

        let trimmed = line.trim();

        // 1. Optimize `mvi A, 0` -> `xra A`
        if let Some(action) = check_mvi_zero_optimization(doc, line_idx, line, trimmed) {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        // 2. Fix instruction mnemonic casing
        if let Some(action) = check_mnemonic_casing(doc, line_idx, line, trimmed) {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        // 3. Number base conversions (hex, dec, bin 4-bit, bin 8-bit, oct)
        for action in check_number_base_conversions(doc, line_idx, line, &params.range) {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }
    }

    actions
}

fn check_mvi_zero_optimization(
    doc: &Document,
    line_idx: usize,
    line: &str,
    trimmed: &str,
) -> Option<CodeAction> {
    let lower = trimmed.to_lowercase();
    let is_zero = if let Some(rest) = lower.strip_prefix("mvi a,") {
        let op = rest.split(';').next().unwrap_or(rest).trim();
        op == "0" || op == "0x00" || op == "00h" || op == "0x0" || op == "00"
    } else {
        false
    };

    if is_zero {
        let col_start = line.find(trimmed).unwrap_or(0);
        let matched_len = if let Some(idx) = trimmed.find(';') {
            trimmed[..idx].trim_end().len()
        } else {
            trimmed.len()
        };
        let col_end = col_start + matched_len;

        let mut changes = HashMap::new();
        changes.insert(
            doc.uri.clone(),
            vec![TextEdit {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: col_start as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: col_end as u32,
                    },
                },
                new_text: "xra A".to_string(),
            }],
        );

        Some(CodeAction {
            title: "Replace with 'xra A' (saves 1 byte and 3 T-states)".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        })
    } else {
        None
    }
}

fn check_mnemonic_casing(
    doc: &Document,
    line_idx: usize,
    line: &str,
    trimmed: &str,
) -> Option<CodeAction> {
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let first = tokens[0].trim_end_matches(':');
    if is_known_mnemonic(first)
        && first.chars().any(|c| c.is_ascii_uppercase())
        && first.chars().any(|c| c.is_ascii_lowercase())
    {
        // Mixed casing like `Mvi` or `lXi` -> convert to lowercase
        let col_start = line.find(first).unwrap_or(0);
        let col_end = col_start + first.len();

        let mut changes = HashMap::new();
        changes.insert(
            doc.uri.clone(),
            vec![TextEdit {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: col_start as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: col_end as u32,
                    },
                },
                new_text: first.to_lowercase(),
            }],
        );

        Some(CodeAction {
            title: format!("Convert mnemonic '{}' to lowercase", first),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    } else {
        None
    }
}

fn is_known_mnemonic(m: &str) -> bool {
    matches!(
        m.to_uppercase().as_str(),
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
    )
}

fn check_number_base_conversions(
    doc: &Document,
    line_idx: usize,
    line: &str,
    range: &Range,
) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    let code_part = line.split(';').next().unwrap_or(line);

    let mut char_indices = code_part.char_indices().peekable();
    while let Some(&(i, ch)) = char_indices.peek() {
        if ch.is_ascii_alphanumeric() || ch == '\'' || ch == '_' {
            let start = i;
            let mut end = i + ch.len_utf8();
            char_indices.next();
            while let Some(&(j, c)) = char_indices.peek() {
                if c.is_ascii_alphanumeric() || c == '_' || c == '\'' {
                    end = j + c.len_utf8();
                    char_indices.next();
                } else {
                    break;
                }
            }
            let word = &code_part[start..end];
            let col_start = start;
            let col_end = end;
            let touches = if range.start.line as usize == line_idx {
                (range.start.character as usize <= col_end) && (range.end.character as usize >= col_start)
            } else {
                false
            };

            if touches {
                if let Some(val) = crate::lsp::hover::parse_numeric_literal(word) {
                    let hex_rep = format!("0x{:X}", val);
                    let dec_rep = format!("{}", val);
                    let bin_4_rep = crate::lsp::hover::format_binary_grouped(val, 4);
                    let bin_8_rep = crate::lsp::hover::format_binary_grouped(val, 8);
                    let oct_rep = format!("0o{:o}", val);

                    let mut options = vec![
                        (format!("Convert to Hexadecimal: {hex_rep}"), hex_rep),
                        (format!("Convert to Decimal: {dec_rep}"), dec_rep),
                        (format!("Convert to Binary (4-bit grouped): {bin_4_rep}"), bin_4_rep),
                    ];

                    if bin_8_rep != options[2].1 {
                        options.push((format!("Convert to Binary (8-bit grouped): {bin_8_rep}"), bin_8_rep));
                    }
                    options.push((format!("Convert to Octal: {oct_rep}"), oct_rep));

                    for (title, new_text) in options {
                        if word.eq_ignore_ascii_case(&new_text) {
                            continue;
                        }
                        let mut changes = HashMap::new();
                        changes.insert(
                            doc.uri.clone(),
                            vec![TextEdit {
                                range: Range {
                                    start: Position {
                                        line: line_idx as u32,
                                        character: col_start as u32,
                                    },
                                    end: Position {
                                        line: line_idx as u32,
                                        character: col_end as u32,
                                    },
                                },
                                new_text,
                            }],
                        );
                        actions.push(CodeAction {
                            title,
                            kind: Some(CodeActionKind::REFACTOR_REWRITE),
                            diagnostics: None,
                            edit: Some(WorkspaceEdit {
                                changes: Some(changes),
                                document_changes: None,
                                change_annotations: None,
                            }),
                            command: None,
                            is_preferred: Some(false),
                            disabled: None,
                            data: None,
                        });
                    }
                }
            }
        } else {
            char_indices.next();
        }
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::{CodeActionContext, TextDocumentIdentifier, Url};

    #[test]
    fn test_mvi_zero_quickfix() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    mvi A, 0\n    hlt\n".to_string();
        let doc = Document::new(uri.clone(), 1, text);

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range {
                start: Position {
                    line: 1,
                    character: 4,
                },
                end: Position {
                    line: 1,
                    character: 12,
                },
            },
            context: CodeActionContext::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let actions = get_code_actions(&doc, &params);
        let has_xra = actions.iter().any(|act| {
            if let CodeActionOrCommand::CodeAction(a) = act {
                a.title.contains("xra A")
            } else {
                false
            }
        });
        assert!(has_xra);
    }

    #[test]
    fn test_number_base_conversions() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    mvi A, 10\n    hlt\n".to_string();
        let doc = Document::new(uri.clone(), 1, text);

        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range {
                start: Position {
                    line: 1,
                    character: 11,
                },
                end: Position {
                    line: 1,
                    character: 13,
                },
            },
            context: CodeActionContext::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let actions = get_code_actions(&doc, &params);
        let titles: Vec<String> = actions
            .iter()
            .filter_map(|act| {
                if let CodeActionOrCommand::CodeAction(a) = act {
                    Some(a.title.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(titles.iter().any(|t| t.contains("Hexadecimal: 0xA")));
        assert!(titles.iter().any(|t| t.contains("Binary (4-bit grouped): 0b0000_1010")));
        assert!(titles.iter().any(|t| t.contains("Binary (8-bit grouped): 0b00001010")));
        assert!(titles.iter().any(|t| t.contains("Octal: 0o12")));
    }
}
