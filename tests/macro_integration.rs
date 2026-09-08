use std::collections::HashMap;
use tower_lsp::lsp_types::Position;

use emu8085::asm::{AsmErrorKind, assemble, assemble_with_symbols, load};
use emu8085::cpu::registers::Reg16;
use emu8085::lsp::completion::get_completions;
use emu8085::lsp::definition::get_definition;
use emu8085::lsp::diagnostics::compute_diagnostics;
use emu8085::lsp::document::Document;
use emu8085::lsp::hover::get_hover;
use emu8085::Machine;

fn run_source(src: &str) -> Machine {
    let image = assemble(src).expect("assembly should succeed");
    let mut m = Machine::create(16, 8);
    load(&mut m, &image).expect("loads");
    m.run();
    m
}

#[test]
fn test_basic_macro_definition_and_execution() {
    let src = r#"
%macro mSetA 1
    mvi A, %1
%end

segment .text
main:
    mSetA 0x42
    hlt
"#;
    let m = run_source(src);
    assert!(m.cpu.is_halt);
    assert_eq!(m.cpu.regs.a, 0x42);
}

#[test]
fn test_macro_multi_args_with_variables_and_len() {
    let src = r#"
%macro mCopyLen 2
    lxi HL, %1
    mvi B, %2
%end

segment .data
msg "Hello"

segment .text
main:
    mCopyLen msg, %len msg
    hlt
"#;
    let (image, symbols) = emu8085::asm::assemble_with_symbols(src).expect("assembly should succeed");
    let mut m = Machine::create(16, 8);
    load(&mut m, &image).unwrap();
    m.run();
    assert!(m.cpu.is_halt);
    assert_eq!(m.cpu.regs.b, 5);
    let msg_addr = symbols["msg"];
    assert_eq!(m.cpu.regs.get16(Reg16::HL).0, msg_addr);
}

#[test]
fn test_macro_defined_inside_text_segment() {
    let src = r#"
segment .text
%macro mAddConst 1
    adi %1
%end

main:
    mvi A, 10
    mAddConst 25
    hlt
"#;
    let m = run_source(src);
    assert!(m.cpu.is_halt);
    assert_eq!(m.cpu.regs.a, 35);
}

#[test]
fn test_nested_macro_expansion() {
    let src = r#"
%macro mLoad 1
    mvi A, %1
%end

%macro mDouble 1
    mLoad %1
    add A
%end

segment .text
main:
    mDouble 20
    hlt
"#;
    let m = run_source(src);
    assert!(m.cpu.is_halt);
    assert_eq!(m.cpu.regs.a, 40);
}

#[test]
fn test_macro_with_constants_and_literals() {
    let src = r#"
%define FACTOR 3

%macro mScale 2
    mvi A, %1
    mvi C, %2
%end

segment .text
main:
    mScale FACTOR, 'Z'
    hlt
"#;
    let m = run_source(src);
    assert!(m.cpu.is_halt);
    assert_eq!(m.cpu.regs.a, 3);
    assert_eq!(m.cpu.regs.c, b'Z');
}

#[test]
fn test_macro_recursion_limit_error() {
    let src = r#"
%macro mLoop 1
    mLoop %1
%end

segment .text
main:
    mLoop 1
    hlt
"#;
    let err = assemble(src).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::MacroRecursionLimitExceeded(_)),
        "expected recursion error, got: {:?}",
        err.kind
    );
}

#[test]
fn test_macro_arg_count_non_literal_rejected() {
    let src = r#"
%define COUNT 2
%macro mFoo COUNT
    nop
%end

segment .text
main:
    mFoo
    hlt
"#;
    let err = assemble(src).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::InvalidMacroArgCount(_)),
        "expected InvalidMacroArgCount, got: {:?}",
        err.kind
    );
}

#[test]
fn test_macro_arg_count_mismatch_error() {
    let src = r#"
%macro mTest 2
    mvi A, %1
    mvi B, %2
%end

segment .text
main:
    mTest 1
    hlt
"#;
    let err = assemble(src).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::MacroArgCountMismatch { expected: 2, found: 1, .. }),
        "expected arg count mismatch, got: {:?}",
        err.kind
    );
}

#[test]
fn test_invalid_macro_arg_ref_error() {
    let src = r#"
%macro mBadRef 2
    mvi A, %0
%end

segment .text
main:
    mBadRef 1, 2
    hlt
"#;
    let err = assemble(src).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::InvalidMacroArgRef { index: 0, max: 2 }),
        "expected InvalidMacroArgRef %0, got: {:?}",
        err.kind
    );

    let src2 = r#"
%macro mBadRef2 2
    mvi A, %3
%end

segment .text
main:
    mBadRef2 1, 2
    hlt
"#;
    let err2 = assemble(src2).unwrap_err();
    assert!(
        matches!(err2.kind, AsmErrorKind::InvalidMacroArgRef { index: 3, max: 2 }),
        "expected InvalidMacroArgRef %3, got: {:?}",
        err2.kind
    );
}

#[test]
fn test_invalid_macro_arg_kind_registers_rejected() {
    let src = r#"
%macro mPass 1
    mvi B, %1
%end

segment .text
main:
    mPass A
    hlt
"#;
    let err = assemble(src).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::InvalidMacroArg(_)),
        "expected InvalidMacroArg, got: {:?}",
        err.kind
    );
}

#[test]
fn test_macro_collisions() {
    // Collision with instruction
    let src_instr = r#"
%macro mov 1
    nop
%end
segment .text
main:
    hlt
"#;
    let err = assemble(src_instr).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::MacroCollision { ref collision_with, .. } if collision_with == "instruction" || collision_with == "label"),
        "expected collision with instruction, got: {:?}",
        err.kind
    );

    // Collision with constant define
    let src_def = r#"
%define MY_CONST 10
%macro MY_CONST 1
    nop
%end
segment .text
main:
    hlt
"#;
    let err = assemble(src_def).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::MacroCollision { ref collision_with, .. } if collision_with == "constant"),
        "expected collision with constant, got: {:?}",
        err.kind
    );

    // Collision with data variable
    let src_var = r#"
%macro my_var 1
    nop
%end
segment .data
my_var BYTE 10
segment .text
main:
    hlt
"#;
    let err = assemble(src_var).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::MacroCollision { ref collision_with, .. } if collision_with == "variable"),
        "expected collision with variable, got: {:?}",
        err.kind
    );

    // Collision with label
    let src_lbl = r#"
%macro my_label 1
    nop
%end
segment .text
main:
    hlt
my_label:
    ret
"#;
    let err = assemble(src_lbl).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::MacroCollision { ref collision_with, .. } if collision_with == "label"),
        "expected collision with label, got: {:?}",
        err.kind
    );

    // Duplicate macro definition
    let src_dup = r#"
%macro my_macro 1
    nop
%end
%macro my_macro 1
    hlt
%end
segment .text
main:
    hlt
"#;
    let err = assemble(src_dup).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::DuplicateMacro(_)),
        "expected DuplicateMacro, got: {:?}",
        err.kind
    );
}

#[test]
fn test_unterminated_macro_error() {
    let src = r#"
%macro unterminated 1
    mvi A, %1

segment .text
main:
    hlt
"#;
    let err = assemble(src).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::UnterminatedMacro(_)),
        "expected UnterminatedMacro, got: {:?}",
        err.kind
    );
}

#[test]
fn test_macro_included_from_another_file() {
    let temp_dir = std::env::temp_dir().join("emu8085_test_macro_inc");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let lib_file = temp_dir.join("macros.e8085");
    std::fs::write(
        &lib_file,
        r#"
%macro mAddReg 2
    mvi A, %1
    mvi B, %2
    add B
%end
"#,
    )
    .unwrap();

    let main_file = temp_dir.join("main.e8085");
    let main_src = r#"
%include "macros.e8085"

segment .text
main:
    mAddReg 12, 30
    hlt
"#;
    std::fs::write(&main_file, main_src).unwrap();

    let image = emu8085::asm::assemble_with_full_context(
        main_src,
        Some(&main_file),
        Some(&temp_dir),
        Some(&temp_dir),
        &HashMap::new(),
    )
    .expect("multi-file macro assembly should succeed");

    let mut m = Machine::create(16, 8);
    load(&mut m, &image).unwrap();
    m.run();
    assert!(m.cpu.is_halt);
    assert_eq!(m.cpu.regs.a, 42);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_lsp_macro_hover_and_definition() {
    let uri = tower_lsp::lsp_types::Url::parse("file:///test_macro.e8085").unwrap();
    let text = r#"%macro mCustom 2
    lxi HL, %1
    mvi B, %2
%end

segment .text
main:
    mCustom 0x1000, 5
    hlt
"#
    .to_string();
    let doc = Document::new(uri.clone(), 1, text);

    // Hover over mCustom call at line 7
    let hover = get_hover(&doc, &Position { line: 7, character: 6 }).expect("hover should succeed");
    let hover_str = match hover.contents {
        tower_lsp::lsp_types::HoverContents::Markup(m) => m.value,
        _ => panic!("expected markup"),
    };
    assert!(hover_str.contains("%macro mCustom 2"), "hover: {hover_str}");

    // Go to definition from mCustom call at line 7
    let def = get_definition(&doc, &Position { line: 7, character: 6 }).expect("def should succeed");
    match def {
        tower_lsp::lsp_types::GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.range.start.line, 0);
        }
        _ => panic!("expected scalar definition location"),
    }
}

#[test]
fn test_lsp_macro_completions() {
    let uri = tower_lsp::lsp_types::Url::parse("file:///test_macro_comp.e8085").unwrap();
    let text = r#"%macro mDisplay 2
    nop
%end

segment .data
msg "Test"

segment .text
main:

"#
    .to_string();
    let doc = Document::new(uri.clone(), 1, text);

    // In text segment at empty line, should suggest mDisplay
    let res = get_completions(&doc, &Position { line: 9, character: 4 }).unwrap();
    let items = match res {
        tower_lsp::lsp_types::CompletionResponse::List(l) => l.items,
        tower_lsp::lsp_types::CompletionResponse::Array(a) => a,
    };
    assert!(
        items.iter().any(|i| i.label == "mDisplay"),
        "mDisplay should be suggested"
    );

    // Typing % should suggest %macro and %end
    let text_pct = "segment .text\n    %".to_string();
    let doc_pct = Document::new(uri.clone(), 1, text_pct);
    let res_pct = get_completions(&doc_pct, &Position { line: 1, character: 5 }).unwrap();
    let items_pct = match res_pct {
        tower_lsp::lsp_types::CompletionResponse::List(l) => l.items,
        tower_lsp::lsp_types::CompletionResponse::Array(a) => a,
    };
    assert!(items_pct.iter().any(|i| i.label == "macro" || i.label == "%macro"));
    assert!(items_pct.iter().any(|i| i.label == "end" || i.label == "%end"));
}

#[test]
fn test_macro_16bit_and_8bit_operand_validation() {
    // Valid case: 16-bit variable passed to 16-bit instruction (LXI), and 8-bit %len passed to 8-bit instruction (MVI)
    let valid_src = r#"
%macro foo 2
    lxi HL, %1
    mvi B, %2
%end

segment .data
    message byte "Hello"
    message_len byte 5

segment .text
main:
    foo message, %len message
    hlt
"#;
    let (_image, symtab) = assemble_with_symbols(valid_src).expect("valid macro usage should compile");
    assert_eq!(*symtab.get("message").unwrap(), 0x0040);

    // Invalid case: 16-bit variable message_len passed where 8-bit immediate is required
    let invalid_src = r#"
%macro foo 2
    lxi HL, %1
    mvi B, %2
%end

segment .data
    message byte "Hello"
    message_len byte 5

segment .text
main:
    foo message, message_len
    hlt
"#;
    let err = assemble(invalid_src).unwrap_err();
    assert!(
        matches!(err.kind, AsmErrorKind::BadOperand { ref mnemonic, ref detail }
            if mnemonic.eq_ignore_ascii_case("MVI") && detail.contains("expected 8-bit immediate value, found 16-bit variable 'message_len'")),
        "expected BadOperand for 16-bit variable in 8-bit immediate, got: {:?}",
        err.kind
    );

    // Also verify that LSP diagnostics reports this error at the macro call line
    let uri = tower_lsp::lsp_types::Url::parse("file:///test_macro_diag.e8085").unwrap();
    let doc = Document::new(uri, 1, invalid_src.to_string());
    let diagnostics = compute_diagnostics(&doc);
    assert!(!diagnostics.is_empty(), "LSP should produce diagnostics for invalid argument size");
    assert!(
        diagnostics.iter().any(|d| d.message.contains("expected 8-bit immediate value, found 16-bit variable 'message_len'")),
        "LSP diagnostics: {:?}",
        diagnostics
    );
}
