use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position, Range};

use super::document::Document;

/// Generates inline hardware cycle (T-state) hints for instructions.
pub fn get_inlay_hints(doc: &Document, range: &Range) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;

    for (line_idx, line) in doc.text.lines().enumerate() {
        if line_idx < start_line || line_idx > end_line {
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with(';')
            || trimmed.starts_with('%')
            || trimmed.starts_with("segment")
        {
            continue;
        }

        let clean_line = if let Some(idx) = trimmed.find(';') {
            &trimmed[..idx]
        } else {
            trimmed
        };

        let tokens: Vec<&str> = clean_line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let first = tokens[0].trim_end_matches(':');
        if let Some(t_states) = get_opcode_cycles(first) {
            let col = line.find(tokens[0]).unwrap_or(0) + tokens[0].len();
            hints.push(InlayHint {
                position: Position {
                    line: line_idx as u32,
                    character: col as u32,
                },
                label: InlayHintLabel::String(format!(" [{}T]", t_states)),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: Some(tower_lsp::lsp_types::InlayHintTooltip::String(format!(
                    "Hardware execution timing: {} T-states",
                    t_states
                ))),
                padding_left: Some(true),
                padding_right: None,
                data: None,
            });
        }
    }

    hints
}

fn get_opcode_cycles(mnemonic: &str) -> Option<u8> {
    match mnemonic.to_uppercase().as_str() {
        "NOP" | "MOV" | "ADD" | "ADC" | "SUB" | "SBB" | "INR" | "DCR" | "ANA" | "XRA" | "ORA"
        | "CMP" | "RLC" | "RRC" | "RAL" | "RAR" | "CMA" | "CMC" | "STC" | "DAA" | "EI" | "DI"
        | "RIM" | "SIM" | "XCHG" => Some(4),
        "HLT" => Some(5),
        "INX" | "DCX" | "PCHL" | "SPHL" => Some(6),
        "MVI" | "ADI" | "ACI" | "SUI" | "SBI" | "ANI" | "XRI" | "ORI" | "CPI" | "LDAX" | "STAX" => {
            Some(7)
        }
        "JMP" | "JZ" | "JNZ" | "JC" | "JNC" | "JP" | "JM" | "JPE" | "JPO" | "IN" | "OUT"
        | "DAD" | "POP" | "RET" => Some(10),
        "PUSH" | "RST" => Some(12),
        "LDA" | "STA" => Some(13),
        "LHLD" | "SHLD" | "XTHL" => Some(16),
        "CALL" | "CZ" | "CNZ" | "CC" | "CNC" | "CP" | "CM" | "CPE" | "CPO" => Some(18),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_inlay_hints_generation() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    mvi A, 0x05\n    call print\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 4,
                character: 0,
            },
        };

        let hints = get_inlay_hints(&doc, &range);
        assert_eq!(hints.len(), 3);
        if let InlayHintLabel::String(ref s) = hints[0].label {
            assert_eq!(s, " [7T]");
        }
        if let InlayHintLabel::String(ref s) = hints[1].label {
            assert_eq!(s, " [18T]");
        }
        if let InlayHintLabel::String(ref s) = hints[2].label {
            assert_eq!(s, " [5T]");
        }
    }
}
