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
    if lower.starts_with("mvi a, 0")
        || lower.starts_with("mvi a, 0x00")
        || lower.starts_with("mvi a, 00h")
    {
        let col_start = line.find(trimmed).unwrap_or(0);
        let col_end = col_start + trimmed.len();

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
        assert_eq!(actions.len(), 1);
        if let CodeActionOrCommand::CodeAction(ref a) = actions[0] {
            assert!(a.title.contains("xra A"));
        }
    }
}
