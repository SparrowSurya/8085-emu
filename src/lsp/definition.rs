use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, LocationLink, Position, Range, Url};

use super::document::Document;

/// Resolves the definition location for a symbol or included file at the given cursor position.
pub fn get_definition(doc: &Document, position: &Position) -> Option<GotoDefinitionResponse> {
    // 1. Check if the line is an %include directive and the cursor is on the include path
    if let Some(link) = get_include_definition(doc, position) {
        return Some(GotoDefinitionResponse::Link(vec![link]));
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

fn get_include_definition(doc: &Document, position: &Position) -> Option<LocationLink> {
    let line = doc.text.lines().nth(position.line as usize)?;
    let trimmed = line.trim();

    if trimmed.starts_with("%include") {
        let char_col = position.character as usize;
        let (rel_path, is_system, start_col, end_col) = if let Some(start_angle) = line.find('<') {
            if let Some(end_angle_offset) = line[start_angle + 1..].find('>') {
                let end_angle = start_angle + 1 + end_angle_offset;
                if char_col >= start_angle && char_col <= end_angle {
                    let path = line[start_angle + 1..end_angle].trim();
                    (path, true, start_angle, end_angle)
                } else {
                    return None;
                }
            } else {
                return None;
            }
        } else if let Some(quote_char) = if line.contains('"') {
            Some('"')
        } else if line.contains('\'') {
            Some('\'')
        } else {
            None
        } {
            let start_quote = line.find(quote_char)?;
            let end_quote_offset = line[start_quote + 1..].find(quote_char)?;
            let end_quote = start_quote + 1 + end_quote_offset;
            if char_col >= start_quote && char_col <= end_quote {
                let path = &line[start_quote + 1..end_quote];
                (path, false, start_quote, end_quote)
            } else {
                return None;
            }
        } else {
            let inc_end = line.find("%include")? + "%include".len();
            let after_inc = &line[inc_end..];
            let spaces = after_inc.chars().take_while(|c| c.is_whitespace()).count();
            if spaces == 0 {
                return None;
            }
            let raw_start = inc_end + spaces;
            let raw_end = line[raw_start..]
                .find(|c: char| c.is_whitespace() || c == ';')
                .map(|off| raw_start + off)
                .unwrap_or(line.len());
            if char_col >= raw_start && char_col <= raw_end {
                let path = line[raw_start..raw_end].trim();
                (path, false, raw_start, raw_end.saturating_sub(1))
            } else {
                return None;
            }
        };

        let target_path = super::document::resolve_include_target(&doc.uri, rel_path, is_system)?;
        let target_uri = Url::from_file_path(target_path).ok()?;

        let origin_selection_range = Range {
            start: Position {
                line: position.line,
                character: start_col as u32,
            },
            end: Position {
                line: position.line,
                character: (end_col + 1) as u32,
            },
        };

        let target_range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        };

        return Some(LocationLink {
            origin_selection_range: Some(origin_selection_range),
            target_uri,
            target_range,
            target_selection_range: target_range,
        });
    }

    None
}

fn find_local_label_definition(
    doc: &Document,
    position: &Position,
    local_label: &str,
) -> Option<Location> {
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
                        start: Position {
                            line: line_num as u32,
                            character: char_offset,
                        },
                        end: Position {
                            line: line_num as u32,
                            character: char_offset + ident.len() as u32,
                        },
                    },
                });
            }
        }
    }

    None
}

fn find_symbol_definition_in_doc(doc: &Document, symbol: &str) -> Option<Location> {
    let sym_lower = symbol.to_lowercase();
    let mut curr_seg: Option<super::document::SegmentKind> = None;
    for (i, line) in doc.text.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.to_lowercase().starts_with("segment ") || trimmed.to_lowercase().starts_with("section ") {
            let seg_name = trimmed[8..].trim().to_lowercase();
            if seg_name == ".text" || seg_name == "text" {
                curr_seg = Some(super::document::SegmentKind::Text);
            } else if seg_name == ".data" || seg_name == "data" {
                curr_seg = Some(super::document::SegmentKind::Data);
            } else if seg_name == ".bss" || seg_name == "bss" {
                curr_seg = Some(super::document::SegmentKind::Bss);
            }
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        // 1. Check label definition (e.g. `main:`, `global print:`, `multiply:`)
        if let Some(label) = extract_label_name(trimmed) {
            if label.eq_ignore_ascii_case(symbol) {
                let char_offset = line
                    .to_lowercase()
                    .find(&sym_lower)
                    .unwrap_or(0) as u32;
                return Some(Location {
                    uri: doc.uri.clone(),
                    range: Range {
                        start: Position {
                            line: i as u32,
                            character: char_offset,
                        },
                        end: Position {
                            line: i as u32,
                            character: char_offset + label.len() as u32,
                        },
                    },
                });
            }
        }

        // 2. Check variable declaration (e.g. `prompt "Hello"`, `buffer BYTE 32`)
        if curr_seg != Some(super::document::SegmentKind::Text) {
            if !parts.is_empty() && parts[0].eq_ignore_ascii_case(symbol) && !is_reserved_keyword(parts[0]) {
                let is_var_decl = match curr_seg {
                    Some(super::document::SegmentKind::Data) | Some(super::document::SegmentKind::Bss) => true,
                    _ => {
                        if parts.len() >= 2 {
                            let second = parts[1].to_lowercase();
                            second == "byte"
                                || second == "word"
                                || second.starts_with('"')
                                || second.starts_with('\'')
                                || second.starts_with("%repeat")
                                || second.starts_with("%len")
                        } else {
                            false
                        }
                    }
                };
                if is_var_decl {
                    let char_offset = line
                        .to_lowercase()
                        .find(&sym_lower)
                        .unwrap_or(0) as u32;
                    return Some(Location {
                        uri: doc.uri.clone(),
                        range: Range {
                            start: Position {
                                line: i as u32,
                                character: char_offset,
                            },
                            end: Position {
                                line: i as u32,
                                character: char_offset + symbol.len() as u32,
                            },
                        },
                    });
                }
            }
        }

        // 3. Check %define macro
        if trimmed.starts_with("%define") && parts.len() >= 2 && parts[1].eq_ignore_ascii_case(symbol) {
            let char_offset = line
                .to_lowercase()
                .find(&sym_lower)
                .unwrap_or(0) as u32;
            return Some(Location {
                uri: doc.uri.clone(),
                range: Range {
                    start: Position {
                        line: i as u32,
                        character: char_offset,
                    },
                    end: Position {
                        line: i as u32,
                        character: char_offset + symbol.len() as u32,
                    },
                },
            });
        }

        // 4. Check %macro definition
        if trimmed.starts_with("%macro") && parts.len() >= 2 && parts[1].eq_ignore_ascii_case(symbol) {
            let char_offset = line
                .to_lowercase()
                .find(&sym_lower)
                .unwrap_or(0) as u32;
            return Some(Location {
                uri: doc.uri.clone(),
                range: Range {
                    start: Position {
                        line: i as u32,
                        character: char_offset,
                    },
                    end: Position {
                        line: i as u32,
                        character: char_offset + symbol.len() as u32,
                    },
                },
            });
        }
    }

    None
}

fn find_symbol_in_included_files(doc: &Document, symbol: &str) -> Option<Location> {
    let included_docs = super::document::collect_all_included_documents(doc);
    for inc_doc in &included_docs {
        let inc_lsp_doc = Document::new(inc_doc.uri.clone(), 1, inc_doc.content.clone());
        if let Some(loc) = find_symbol_definition_in_doc(&inc_lsp_doc, symbol) {
            return Some(loc);
        }
    }

    None
}

fn extract_label_name(trimmed_line: &str) -> Option<&str> {
    if let Some(rest) = trimmed_line.strip_suffix(':') {
        let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
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

pub(crate) fn is_reserved_keyword(word: &str) -> bool {
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

    #[test]
    fn test_goto_include_file_link() {
        let temp_dir = std::env::temp_dir();
        let target_file = temp_dir.join("lib_helper.e8085");
        std::fs::write(&target_file, "helper:\n    ret\n").unwrap();

        let source_file = temp_dir.join("main.e8085");
        let text = "%include \"lib_helper.e8085\"\nmain:\n    call helper\n".to_string();
        let uri = Url::from_file_path(&source_file).unwrap();
        let doc = Document::new(uri, 1, text);

        let def_res = get_definition(
            &doc,
            &Position {
                line: 0,
                character: 12,
            },
        )
        .unwrap();
        if let GotoDefinitionResponse::Link(links) = def_res {
            assert_eq!(links.len(), 1);
            let link = &links[0];
            let origin = link.origin_selection_range.as_ref().unwrap();
            assert_eq!(origin.start.character, 9); // Opening quote "
            assert_eq!(origin.end.character, 27); // Closing quote "
        } else {
            panic!("expected Link with origin_selection_range");
        }

        let _ = std::fs::remove_file(target_file);
    }

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
"
        .to_string();
        let doc = Document::new(uri.clone(), 1, text);

        // Jump to 'helper' from line 6 ("    call helper")
        let def_helper = get_definition(
            &doc,
            &Position {
                line: 6,
                character: 10,
            },
        )
        .unwrap();
        if let GotoDefinitionResponse::Scalar(loc) = def_helper {
            assert_eq!(loc.range.start.line, 9);
        } else {
            panic!("expected scalar location");
        }

        // Jump to 'greeting' from line 5 ("    lxi HL, greeting")
        let def_var = get_definition(
            &doc,
            &Position {
                line: 5,
                character: 13,
            },
        )
        .unwrap();
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
"
        .to_string();
        let doc = Document::new(uri, 1, text);

        // Jump to .loop inside func_b (line 9: "    jnz .loop") -> should jump to line 7 (".loop:"), NOT line 1
        let def_local = get_definition(
            &doc,
            &Position {
                line: 9,
                character: 9,
            },
        )
        .unwrap();
        if let GotoDefinitionResponse::Scalar(loc) = def_local {
            assert_eq!(loc.range.start.line, 7);
        } else {
            panic!("expected scalar location in func_b scope");
        }
    }

    #[test]
    fn test_goto_system_include_and_imported_symbol() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let main_file = manifest.join("main_test.e8085");
        let uri = Url::from_file_path(&main_file).unwrap();
        let text = "%include <math>\nsegment .text\nmain:\n    call divide\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        // 1. Goto definition on <math>
        let def_math = get_definition(&doc, &Position { line: 0, character: 12 }).unwrap();
        if let GotoDefinitionResponse::Link(links) = def_math {
            assert_eq!(links.len(), 1);
            assert!(links[0].target_uri.path().ends_with("math.e8085"));
        } else {
            panic!("expected Link for <math>");
        }

        // 2. Goto definition on symbol `divide`
        let def_divide = get_definition(&doc, &Position { line: 3, character: 11 }).unwrap();
        if let GotoDefinitionResponse::Scalar(loc) = def_divide {
            assert!(loc.uri.path().ends_with("math.e8085"));
        } else {
            panic!("expected Scalar location for divide");
        }
    }

    #[test]
    fn test_goto_raw_unquoted_include_file_link() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let main_file = manifest.join("main_raw_test.e8085");
        let uri = Url::from_file_path(&main_file).unwrap();
        let text = "%include stdlib/math.e8085\nsegment .text\nmain:\n    call divide\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        // 1. Goto definition on raw path stdlib/math.e8085
        let def_math = get_definition(&doc, &Position { line: 0, character: 15 }).unwrap();
        if let GotoDefinitionResponse::Link(links) = def_math {
            assert_eq!(links.len(), 1);
            assert!(links[0].target_uri.path().ends_with("math.e8085"));
        } else {
            panic!("expected Link for raw include path");
        }

        // 2. Goto definition on symbol `divide` imported via raw include
        let def_divide = get_definition(&doc, &Position { line: 3, character: 11 }).unwrap();
        if let GotoDefinitionResponse::Scalar(loc) = def_divide {
            assert!(loc.uri.path().ends_with("math.e8085"));
        } else {
            panic!("expected Scalar location for divide");
        }
    }

    #[test]
    fn test_goto_definition_transitive_and_case_insensitive() {
        let temp_dir = std::env::temp_dir().join("emu8085_test_transitive_def");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let sub2 = temp_dir.join("sub2.e8085");
        let sub1 = temp_dir.join("sub1.e8085");
        let main = temp_dir.join("main.e8085");

        std::fs::write(&sub2, "%macro TargetMacro 1\n    nop\n%end\n").unwrap();
        std::fs::write(&sub1, "%include \"sub2.e8085\"\n").unwrap();
        std::fs::write(&main, "%include \"sub1.e8085\"\nsegment .text\nmain:\n    targetmacro 5\n    hlt\n").unwrap();

        let doc = Document::new(
            Url::from_file_path(&main).unwrap(),
            1,
            std::fs::read_to_string(&main).unwrap(),
        );

        // Jump from `targetmacro` (case-insensitive) at line 3
        let def = get_definition(&doc, &Position { line: 3, character: 8 }).unwrap();
        if let GotoDefinitionResponse::Scalar(loc) = def {
            assert!(loc.uri.path().ends_with("sub2.e8085"));
            assert_eq!(loc.range.start.line, 0);
        } else {
            panic!("expected scalar location for transitive macro definition");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
