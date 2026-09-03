use std::collections::HashMap;
use std::path::Path;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use super::document::Document;
use crate::asm::{assemble_with_options, parse};

/// Analyzes an `.e8085` source document and returns compiler error diagnostics.
pub fn compute_diagnostics(doc: &Document) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let base_dir = if let Ok(path) = doc.uri.to_file_path() {
        path.parent().map(|p| p.to_path_buf())
    } else {
        None
    };

    let base_ref = base_dir.as_deref().unwrap_or_else(|| Path::new("."));

    // Collect all declared `extern <name>` symbols so references to external functions do not produce false errors
    let mut extern_symbols = HashMap::new();
    if let Ok(tokens) = crate::asm::lex(&doc.text) {
        if let Ok(program) = parse(tokens) {
            for ext in &program.externs {
                extern_symbols.insert(ext.clone(), 0x8000);
            }
            for seg in &program.segments {
                if let crate::asm::Segment::Text(items) = seg {
                    for item in items {
                        if let crate::asm::TextItem::ExternDecl(name, _) = item {
                            extern_symbols.insert(name.clone(), 0x8000);
                        }
                    }
                }
            }
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

    // Run assembler front-end in analysis mode with declared extern symbols
    if let Err(err) = assemble_with_options(&doc.text, Some(base_ref), &extern_symbols) {
        let line = err.span.line.saturating_sub(1);
        let col = err.span.col.saturating_sub(1);

        let end_col = if let Some(doc_line) = doc.text.lines().nth(line as usize) {
            (col + 1).max(doc_line.len() as u32)
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
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("e8085".to_string()),
            message: err.kind.to_string(),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    diagnostics
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
        assert!(diags.is_empty());
    }

    #[test]
    fn test_extern_label_produces_no_diagnostic() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "extern my_label\nsegment .text\nmain:\n    call my_label\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        let diags = compute_diagnostics(&doc);
        assert!(diags.is_empty(), "declared extern symbol should not produce diagnostic errors: {:?}", diags);
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
}
