use emu8085::lsp::completion::get_completions;
use emu8085::lsp::diagnostics::compute_diagnostics;
use emu8085::lsp::document::Document;
use emu8085::lsp::hover::get_hover;
use tower_lsp::lsp_types::*;

#[test]
fn test_lsp_dead_code_dimming_for_statically_false_conditionals() {
    let uri = Url::parse("file:///conditional_dimming.e8085").unwrap();
    let text = r#"%define DEBUG 0
section .text
main:
%if DEBUG
    mvi a, 1
    mvi b, 2
%else
    mvi a, 3
%end
    hlt
"#;
    let doc = Document::new(uri, 1, text.to_string());
    let diags = compute_diagnostics(&doc);

    // Filter diagnostics that have UNNECESSARY tag (dead code dimming)
    let unnecessary_diags: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| {
            d.tags
                .as_ref()
                .map(|tags| tags.contains(&DiagnosticTag::UNNECESSARY))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !unnecessary_diags.is_empty(),
        "Expected at least one dead-code dimming diagnostic"
    );

    let dead_code = unnecessary_diags[0];
    assert_eq!(dead_code.severity, Some(DiagnosticSeverity::HINT));
    assert!(dead_code.message.contains("inactive"));
    // Lines 4 and 5 (0-indexed lines 4 and 5, which are lines 5 and 6 in 1-indexed) are dead code
    assert_eq!(dead_code.range.start.line, 4);
    assert_eq!(dead_code.range.end.line, 5);
}

#[test]
fn test_lsp_valueless_define_hover_and_branch_activation() {
    let uri = Url::parse("file:///valueless_hover.e8085").unwrap();
    let text = r#"%define FEATURE_ENABLED
section .text
main:
%if FEATURE_ENABLED
    mvi a, 10
%else
    mvi a, 20
%end
    hlt
"#;
    let doc = Document::new(uri, 1, text.to_string());

    // 1. Hover on FEATURE_ENABLED on line 0
    let hover = get_hover(&doc, &Position { line: 0, character: 10 }).expect("Expected hover for FEATURE_ENABLED");
    if let HoverContents::Markup(m) = hover.contents {
        assert_eq!(m.value, "```\n%define FEATURE_ENABLED 1\n```");
    } else {
        panic!("Expected markdown hover content");
    }

    // 2. Dead code dimming: FEATURE_ENABLED is 1, so the %else branch (line 6) is dimmed
    let diags = compute_diagnostics(&doc);
    let dead_code: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| {
            d.tags
                .as_ref()
                .map(|tags| tags.contains(&DiagnosticTag::UNNECESSARY))
                .unwrap_or(false)
                && d.message.contains("inactive")
        })
        .collect();

    assert_eq!(dead_code.len(), 1);
    assert_eq!(dead_code[0].range.start.line, 6);
    assert_eq!(dead_code[0].range.end.line, 6);
}

#[test]
fn test_lsp_completions_for_conditionals_and_constants() {
    let uri = Url::parse("file:///cond_completions.e8085").unwrap();
    let text = r#"%define CONFIG_OPTIMIZE
%define MAX_LIMIT 256
section .text
main:
%if 
%
"#;
    let doc = Document::new(uri, 1, text.to_string());

    // 1. Completions after `%if ` suggest defined constants
    let comp_if = get_completions(&doc, &Position { line: 4, character: 4 }).unwrap();
    if let CompletionResponse::List(list) = comp_if {
        assert!(list.items.iter().any(|i| i.label == "CONFIG_OPTIMIZE"));
        assert!(list.items.iter().any(|i| i.label == "MAX_LIMIT"));
    } else {
        panic!("Expected completion list");
    }

    // 2. Completions after `%` suggest conditional directives
    let comp_pct = get_completions(&doc, &Position { line: 5, character: 1 }).unwrap();
    if let CompletionResponse::List(list) = comp_pct {
        assert!(list.items.iter().any(|i| i.label == "if"));
        assert!(list.items.iter().any(|i| i.label == "elif"));
        assert!(list.items.iter().any(|i| i.label == "else"));
        assert!(list.items.iter().any(|i| i.label == "endif"));
        assert!(list.items.iter().any(|i| i.label == "ifdef"));
        assert!(list.items.iter().any(|i| i.label == "ifndef"));
    } else {
        panic!("Expected completion list");
    }
}
