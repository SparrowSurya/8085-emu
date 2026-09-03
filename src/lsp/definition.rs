use std::path::Path;
use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};

use super::document::Document;

/// Resolves the definition location for a symbol or included file at the given cursor position.
pub fn get_definition(doc: &Document, position: &Position) -> Option<GotoDefinitionResponse> {
    // 1. Check if the line is an %include directive and the cursor is on the include path
    if let Some(loc) = get_include_definition(doc, position) {
        return Some(GotoDefinitionResponse::Scalar(loc));
    }

    let (word, _) = doc.get_word_at_position(position)?;

    // 2. Check local labels (starts with '.')
    if word.starts_with('.') {
        if let Some(loc) = find_local_label_definition(doc, position, &word) {
            return Some(GotoDefinitionResponse::Scalar(loc));
        }
    }

    // 3. Check global labels, variables, and %define in current document
    if let Some(loc) = find_symbol_definition_in_doc(doc, &word) {
        return Some(GotoDefinitionResponse::Scalar(loc));
    }

    // 4. Check %include files in the workspace for external symbols
    if let Some(loc) = find_symbol_in_included_files(doc, &word) {
        return Some(GotoDefinitionResponse::Scalar(loc));
    }

    None
}

fn get_include_definition(doc: &Document, position: &Position) -> Option<Location> {
    let line = doc.text.lines().nth(position.line as usize)?;
    let trimmed = line.trim();

    if trimmed.starts_with("%include") {
        if let Some(start_quote) = line.find('"') {
            if let Some(end_quote) = line[start_quote + 1..].find('"') {
                let end_quote_idx = start_quote + 1 + end_quote;
                let rel_path = &line[start_quote + 1..end_quote_idx];

                let target_path = resolve_relative_path(&doc.uri, rel_path)?;
                let target_uri = Url::from_file_path(target_path).ok()?;

                return Some(Location {
                    uri: target_uri,
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 0 },
                    },
                });
            }
        }
    }

    None
}

fn find_local_label_definition(doc: &Document, position: &Position, local_label: &str) -> Option<Location> {
    let lines: Vec<&str> = doc.text.lines().collect();
    let current_line = position.line as usize;

    // Find the enclosing parent label before the current cursor line
    let mut parent_start = 0;
    for i in (0..=current_line.min(lines.len().saturating_sub(1))).rev() {
        let line = lines[i].trim();
        if is_parent_label_def(line) {
            parent_start = i;
            break;
        }
    }

    // Find the next parent label to bound the local scope
    let mut parent_end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(parent_start + 1) {
        if is_parent_label_def(line.trim()) {
            parent_end = i;
            break;
        }
    }

    // Search for the local label definition within the bounded scope
    for (idx, line) in lines[parent_start..parent_end].iter().enumerate() {
        let line_num = parent_start + idx;
        let trimmed = line.trim();
        if let Some(ident) = extract_label_name(trimmed) {
            if ident == local_label {
                let char_offset = line.find(ident).unwrap_or(0) as u32;
                return Some(Location {
                    uri: doc.uri.clone(),
                    range: Range {
                        start: Position { line: line_num as u32, character: char_offset },
                        end: Position { line: line_num as u32, character: char_offset + ident.len() as u32 },
                    },
                });
            }
        }
    }

    None
}

fn find_symbol_definition_in_doc(doc: &Document, symbol: &str) -> Option<Location> {
    for (i, line) in doc.text.lines().enumerate() {
        let trimmed = line.trim();

        // 1. Check label definition (e.g. `main:`, `global print:`, `multiply:`)
        if let Some(label) = extract_label_name(trimmed) {
            if label == symbol {
                let char_offset = line.find(label).unwrap_or(0) as u32;
                return Some(Location {
                    uri: doc.uri.clone(),
                    range: Range {
                        start: Position { line: i as u32, character: char_offset },
                        end: Position { line: i as u32, character: char_offset + label.len() as u32 },
                    },
                });
            }
        }

        // 2. Check variable declaration (e.g. `prompt "Hello"`, `buffer BYTE 32`)
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if !parts.is_empty() && parts[0] == symbol && !is_reserved_keyword(parts[0]) {
            let char_offset = line.find(symbol).unwrap_or(0) as u32;
            return Some(Location {
                uri: doc.uri.clone(),
                range: Range {
                    start: Position { line: i as u32, character: char_offset },
                    end: Position { line: i as u32, character: char_offset + symbol.len() as u32 },
                },
            });
        }

        // 3. Check %define macro
        if trimmed.starts_with("%define") && parts.len() >= 2 && parts[1] == symbol {
            let char_offset = line.find(symbol).unwrap_or(0) as u32;
            return Some(Location {
                uri: doc.uri.clone(),
                range: Range {
                    start: Position { line: i as u32, character: char_offset },
                    end: Position { line: i as u32, character: char_offset + symbol.len() as u32 },
                },
            });
        }
    }

    None
}

fn find_symbol_in_included_files(doc: &Document, symbol: &str) -> Option<Location> {
    for line in doc.text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("%include") {
            if let Some(start_quote) = line.find('"') {
                if let Some(end_quote) = line[start_quote + 1..].find('"') {
                    let rel_path = &line[start_quote + 1..start_quote + 1 + end_quote];
                    if let Some(full_path) = resolve_relative_path(&doc.uri, rel_path) {
                        if let Ok(content) = std::fs::read_to_string(&full_path) {
                            if let Ok(inc_uri) = Url::from_file_path(&full_path) {
                                let inc_doc = Document::new(inc_uri, 1, content);
                                if let Some(loc) = find_symbol_definition_in_doc(&inc_doc, symbol) {
                                    return Some(loc);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn extract_label_name(trimmed_line: &str) -> Option<&str> {
    if let Some(rest) = trimmed_line.strip_suffix(':') {
        let clean = rest
            .strip_prefix("global ")
            .or_else(|| rest.strip_prefix("export "))
            .unwrap_or(rest)
            .trim();
        if !clean.is_empty() {
            return Some(clean);
        }
    }
    None
}

fn is_parent_label_def(trimmed_line: &str) -> bool {
    if let Some(label) = extract_label_name(trimmed_line) {
        !label.starts_with('.')
    } else {
        false
    }
}

fn is_reserved_keyword(word: &str) -> bool {
    matches!(
        word.to_uppercase().as_str(),
        "MOV" | "MVI" | "LXI" | "LDA" | "STA" | "LHLD" | "SHLD" | "LDAX" | "STAX" |
        "XCHG" | "XTHL" | "SPHL" | "PCHL" | "ADD" | "ADI" | "ADC" | "ACI" | "SUB" |
        "SUI" | "SBB" | "SBI" | "INR" | "DCR" | "INX" | "DCX" | "DAD" | "DAA" |
        "ANA" | "ANI" | "XRA" | "XRI" | "ORA" | "ORI" | "CMP" | "CPI" | "CMA" |
        "CMC" | "STC" | "RLC" | "RRC" | "RAL" | "RAR" | "PUSH" | "POP" | "IN" |
        "OUT" | "NOP" | "HLT" | "EI" | "DI" | "RIM" | "SIM" | "JMP" | "JZ" | "JNZ" |
        "JC" | "JNC" | "JP" | "JM" | "JPE" | "JPO" | "CALL" | "CZ" | "CNZ" | "CC" |
        "CNC" | "CP" | "CM" | "CPE" | "CPO" | "RET" | "RZ" | "RNZ" | "RC" | "RNC" |
        "RP" | "RM" | "RPE" | "RPO" | "RST" | "BYTE" | "WORD" | "SEGMENT" | "GLOBAL" |
        "EXPORT" | "EXTERN"
    )
}

fn resolve_relative_path(doc_uri: &Url, rel_path: &str) -> Option<std::path::PathBuf> {
    if let Ok(doc_path) = doc_uri.to_file_path() {
        if let Some(parent) = doc_path.parent() {
            let candidate = parent.join(rel_path);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // Try relative to workspace root (current directory)
    let candidate = Path::new(rel_path);
    if candidate.exists() {
        return Some(candidate.to_path_buf());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goto_global_label_and_variable() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "\
segment .data
    greeting \"Hello\"

segment .text
main:
    lxi HL, greeting
    call helper
    hlt

helper:
    ret
".to_string();
        let doc = Document::new(uri.clone(), 1, text);

        // Jump to 'helper' from line 6 ("    call helper")
        let def_helper = get_definition(&doc, &Position { line: 6, character: 10 }).unwrap();
        if let GotoDefinitionResponse::Scalar(loc) = def_helper {
            assert_eq!(loc.range.start.line, 9);
        } else {
            panic!("expected scalar location");
        }

        // Jump to 'greeting' from line 5 ("    lxi HL, greeting")
        let def_var = get_definition(&doc, &Position { line: 5, character: 13 }).unwrap();
        if let GotoDefinitionResponse::Scalar(loc) = def_var {
            assert_eq!(loc.range.start.line, 1);
        } else {
            panic!("expected scalar location");
        }
    }

    #[test]
    fn test_goto_local_label() {
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
".to_string();
        let doc = Document::new(uri, 1, text);

        // Jump to .loop inside func_b (line 9: "    jnz .loop") -> should jump to line 7 (".loop:"), NOT line 1
        let def_local = get_definition(&doc, &Position { line: 9, character: 9 }).unwrap();
        if let GotoDefinitionResponse::Scalar(loc) = def_local {
            assert_eq!(loc.range.start.line, 7);
        } else {
            panic!("expected scalar location in func_b scope");
        }
    }
}
