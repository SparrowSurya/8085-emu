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

#[test]
fn test_dap_macro_stepping_and_breakpoints() {
    use std::fs;
    use emu8085::dap::protocol::{LaunchRequestArguments, SourceBreakpoint};
    use emu8085::dap::session::DebugSession;

    let tmp_dir = std::env::temp_dir().join("emu8085_test_macro_dap");
    let _ = fs::create_dir_all(&tmp_dir);
    let src_path = tmp_dir.join("macro_dap_test.e8085");

    let src = r#"%macro mSetAB 2
    mvi A, %1
    mvi B, %2
%end

segment .text
main:
    mvi C, 99
    mSetAB 10, 20
    hlt
"#;
    fs::write(&src_path, src).unwrap();

    let args = LaunchRequestArguments {
        program: src_path.to_string_lossy().to_string(),
        stop_on_entry: Some(true),
        libraries: None,
        t_state_limit: None,
        step_mode: None,
        console: Some("internalConsole".to_string()),
        terminal_port: None,
    };

    // --- Case 1: Stepping Into Macro Body ---
    let mut session1 = DebugSession::new();
    session1.launch(&args).expect("DAP session launch should succeed");

    // 1. Initial stop on entry -> Line 8 (mvi C, 99)
    let frames0 = session1.get_stack_trace();
    assert_eq!(frames0[0].line, 8, "initial stop on entry should pause at Line 8");

    // 2. StepOver on Line 8 -> should land at Line 9 (macro call site: mSetAB)
    let step1 = session1.step_over().unwrap();
    assert!(step1.is_some());
    assert_eq!(session1.machine.cpu.regs.c, 99);
    let frames_call = session1.get_stack_trace();
    assert_eq!(frames_call[0].line, 9, "should pause at macro call site (Line 9)");

    // 3. StepIn on macro call site (Line 9) -> enters macro body at Line 2
    let step2 = session1.step_in().unwrap();
    assert!(step2.is_some());
    let frames_in1 = session1.get_stack_trace();
    assert_eq!(frames_in1[0].line, 2, "StepIn on macro call should enter macro body at Line 2");
    assert_eq!(frames_in1[1].line, 9, "Caller frame should be macro call site Line 9");

    // 4. StepIn on Line 2 -> executes mvi A, 10 and moves to Line 3 (mvi B, 20)
    let step3 = session1.step_in().unwrap();
    assert!(step3.is_some());
    assert_eq!(session1.machine.cpu.regs.a, 10);
    let frames_in2 = session1.get_stack_trace();
    assert_eq!(frames_in2[0].line, 3, "StepIn inside macro should advance to Line 3");
    assert_eq!(frames_in2[1].line, 9, "Caller frame should still be Line 9");

    // 5. StepIn on Line 3 -> executes mvi B, 20 and advances out to Line 10 (hlt)
    let step4 = session1.step_in().unwrap();
    assert!(step4.is_some());
    assert_eq!(session1.machine.cpu.regs.b, 20);
    let frames_out = session1.get_stack_trace();
    assert_eq!(frames_out[0].line, 10, "StepIn out of macro should land on Line 10 (hlt)");

    // 6. Test StepBack (time-travel debugging through macros)
    session1.step_back().unwrap();
    assert_eq!(session1.get_stack_trace()[0].line, 3);
    session1.step_back().unwrap();
    assert_eq!(session1.get_stack_trace()[0].line, 2);
    session1.step_back().unwrap();
    assert_eq!(session1.get_stack_trace()[0].line, 9, "StepBack should return to macro call site Line 9");
    session1.step_back().unwrap();
    assert_eq!(session1.get_stack_trace()[0].line, 8, "StepBack should return to initial Line 8");

    // --- Case 2: Stepping Over Macro Call ---
    let mut session2 = DebugSession::new();
    session2.launch(&args).unwrap();
    session2.step_over().unwrap(); // From Line 8 to Line 9 (macro call site)
    assert_eq!(session2.get_stack_trace()[0].line, 9);

    // StepOver on macro call site (Line 9) -> steps over entire macro to Line 10 (hlt)
    let step_over_macro = session2.step_over().unwrap();
    assert!(step_over_macro.is_some());
    assert_eq!(session2.machine.cpu.regs.a, 10);
    assert_eq!(session2.machine.cpu.regs.b, 20);
    let frames_stepped_over = session2.get_stack_trace();
    assert_eq!(frames_stepped_over[0].line, 10, "StepOver on macro call should skip to Line 10 (hlt)");

    // --- Case 3: Breakpoints ---
    let mut session_bp = DebugSession::new();
    session_bp.launch(&args).unwrap();
    let bps = session_bp.set_line_breakpoints(
        &src_path,
        &[SourceBreakpoint {
            line: 9,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }],
    );
    assert_eq!(bps.len(), 1);
    assert!(bps[0].verified, "breakpoint on macro call site should be verified");

    let bps_body = session_bp.set_line_breakpoints(
        &src_path,
        &[SourceBreakpoint {
            line: 3,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }],
    );
    assert_eq!(bps_body.len(), 1);
    assert!(bps_body[0].verified, "breakpoint inside macro body should be verified");

    let _ = fs::remove_dir_all(&tmp_dir);
}

#[test]
fn test_dap_macro_programs_file_scenario() {
    use std::path::PathBuf;
    use emu8085::dap::protocol::LaunchRequestArguments;
    use emu8085::dap::session::DebugSession;

    let macro_prog_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs/macro.e8085");
    assert!(macro_prog_path.exists(), "programs/macro.e8085 must exist");

    let args = LaunchRequestArguments {
        program: macro_prog_path.to_string_lossy().to_string(),
        stop_on_entry: Some(true),
        libraries: None,
        t_state_limit: None,
        step_mode: None,
        console: Some("internalConsole".to_string()),
        terminal_port: None,
    };

    // --- Scenario: Stop on Line 25 (mPrintIm) -> StepIn to Line 7 -> StepOver through macro -> Line 26 (mPrintMm) -> StepIn to Line 13 -> StepOut to Line 27 (hlt) ---
    let mut session = DebugSession::new();
    session.launch(&args).expect("launch should succeed");

    // 1. Initial stop on entry -> Line 25 (mPrintIm macro call site)
    let frames0 = session.get_stack_trace();
    assert_eq!(frames0[0].line, 25, "Initial pause at line 25");

    // 2. StepIn on line 25 -> jumps into macro body at line 7 (lxi HL, %1)
    let step_in_macro = session.step_in().unwrap();
    assert!(step_in_macro.is_some());
    let frames_in = session.get_stack_trace();
    assert_eq!(frames_in[0].line, 7, "StepIn jumps to line 7 inside macro body");
    assert!(frames_in[0].name.contains("mPrintIm(msg, %len msg)"), "Frame 0 should contain macro name and args: {}", frames_in[0].name);
    assert_eq!(frames_in[1].line, 25, "Frame 1 points to line 25 caller");

    // Inspect Macro Arguments Scope while inside mPrintIm
    let scopes_m1 = session.get_scopes(0);
    assert_eq!(scopes_m1[0].name, "Macro Arguments (mPrintIm)");
    let vars_m1 = session.get_variables(scopes_m1[0].variables_reference);
    assert_eq!(vars_m1.len(), 2);
    assert_eq!(vars_m1[0].name, "%1");
    assert!(vars_m1[0].value.contains("msg"), "value should contain msg: {}", vars_m1[0].value);
    assert!(vars_m1[0].value.contains("0x0040"), "value should contain msg address: {}", vars_m1[0].value);
    assert_eq!(vars_m1[1].name, "%2");
    assert!(vars_m1[1].value.contains("%len msg"), "value should contain %len msg: {}", vars_m1[1].value);
    assert!(vars_m1[1].value.contains("6"), "value should contain length 6: {}", vars_m1[1].value);

    // Evaluate macro arguments %1 and %2 in debug console / watch / hover
    let eval1 = session.evaluate("%1").unwrap();
    assert_eq!(eval1.raw_value, Some(b'H' as u16), "evaluating %1 (msg) evaluates to byte at msg");
    let eval2 = session.evaluate("%2").unwrap();
    assert_eq!(eval2.raw_value, Some(6), "evaluating %2 (%len msg) resolves to len 6");

    // 3. StepOver on line 7 -> advances to line 8 (mvi B, %2)
    session.step_over().unwrap();
    let frames_in_l8 = session.get_stack_trace();
    assert_eq!(frames_in_l8[0].line, 8, "Line 8 inside macro body");

    // 4. StepOver on line 8 -> advances to line 9 (call print)
    session.step_over().unwrap();
    let frames_in_l9 = session.get_stack_trace();
    assert_eq!(frames_in_l9[0].line, 9, "Line 9 inside macro body");

    // 5. StepOver on line 9 (call print) -> steps over subroutine call and advances to line 26 (mPrintMm macro call site)
    session.step_over().unwrap();
    let frames_macro2 = session.get_stack_trace();
    assert_eq!(frames_macro2[0].line, 26, "Pauses at line 26 (mPrintMm macro call site)");

    // StepIn to mPrintMm macro at line 13
    session.step_in().unwrap();
    let frames_in_m2 = session.get_stack_trace();
    assert_eq!(frames_in_m2[0].line, 13, "Inside mPrintMm at line 13");
    assert!(frames_in_m2[0].name.contains("mPrintMm(msg, msg_len)"), "Frame 0 should contain mPrintMm args: {}", frames_in_m2[0].name);

    let scopes_m2 = session.get_scopes(0);
    assert_eq!(scopes_m2[0].name, "Macro Arguments (mPrintMm)");
    let vars_m2 = session.get_variables(scopes_m2[0].variables_reference);
    assert_eq!(vars_m2.len(), 2);
    assert_eq!(vars_m2[0].name, "%1");
    assert!(vars_m2[0].value.contains("msg"));
    assert_eq!(vars_m2[1].name, "%2");
    assert!(vars_m2[1].value.contains("msg_len"));

    // 6. StepOut on rest of macro to line 27 (hlt)
    session.step_out().unwrap();
    let frames_hlt = session.get_stack_trace();
    assert_eq!(frames_hlt[0].line, 27, "Pauses at line 27 (hlt)");
}

#[test]
fn test_dap_macro_breakpoint_inside_macro_body_hit_on_continue() {
    use std::path::PathBuf;
    use emu8085::dap::protocol::{LaunchRequestArguments, SourceBreakpoint};
    use emu8085::dap::session::{DebugSession, StopReason};

    let macro_prog_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("programs/macro.e8085");
    assert!(macro_prog_path.exists(), "programs/macro.e8085 must exist");

    let args = LaunchRequestArguments {
        program: macro_prog_path.to_string_lossy().to_string(),
        stop_on_entry: Some(true),
        libraries: None,
        t_state_limit: None,
        step_mode: None,
        console: Some("internalConsole".to_string()),
        terminal_port: None,
    };

    // Test 1: Breakpoint on Line 7 (first instruction inside mPrintIm)
    {
        let mut session = DebugSession::new();
        session.launch(&args).expect("launch should succeed");
        let bps = session.set_line_breakpoints(
            &macro_prog_path,
            &[SourceBreakpoint {
                line: 7,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }],
        );
        assert!(bps[0].verified);

        let stop = session.continue_exec().unwrap();
        assert_eq!(stop, Some(StopReason::Breakpoint(bps[0].id.unwrap())));
        let frames = session.get_stack_trace();
        assert_eq!(frames[0].line, 7, "Paused at Line 7 inside macro body");
        assert_eq!(frames[1].line, 25, "Caller frame is Line 25");
        let scopes = session.get_scopes(0);
        assert_eq!(scopes[0].name, "Macro Arguments (mPrintIm)");
    }

    // Test 2: Breakpoint on Line 8 (second instruction inside mPrintIm)
    {
        let mut session = DebugSession::new();
        session.launch(&args).expect("launch should succeed");
        let bps = session.set_line_breakpoints(
            &macro_prog_path,
            &[SourceBreakpoint {
                line: 8,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }],
        );
        assert!(bps[0].verified);

        let stop = session.continue_exec().unwrap();
        assert_eq!(stop, Some(StopReason::Breakpoint(bps[0].id.unwrap())));
        let frames = session.get_stack_trace();
        assert_eq!(frames[0].line, 8, "Paused at Line 8 inside macro body");
        assert_eq!(frames[1].line, 25, "Caller frame is Line 25");
    }

    // Test 3: Breakpoint on Line 14 (inside second macro mPrintMm)
    {
        let mut session = DebugSession::new();
        session.launch(&args).expect("launch should succeed");
        let bps = session.set_line_breakpoints(
            &macro_prog_path,
            &[SourceBreakpoint {
                line: 14,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }],
        );
        assert!(bps[0].verified);

        let stop = session.continue_exec().unwrap();
        assert_eq!(stop, Some(StopReason::Breakpoint(bps[0].id.unwrap())));
        let frames = session.get_stack_trace();
        assert_eq!(frames[0].line, 14, "Paused at Line 14 inside mPrintMm");
        assert_eq!(frames[1].line, 26, "Caller frame is Line 26");
        let scopes = session.get_scopes(0);
        assert_eq!(scopes[0].name, "Macro Arguments (mPrintMm)");
    }

    // Test 4: Breakpoints on Line 7, Line 8, and Line 14 hit sequentially
    {
        let mut session = DebugSession::new();
        session.launch(&args).expect("launch should succeed");
        let bps = session.set_line_breakpoints(
            &macro_prog_path,
            &[
                SourceBreakpoint { line: 7, column: None, condition: None, hit_condition: None, log_message: None },
                SourceBreakpoint { line: 8, column: None, condition: None, hit_condition: None, log_message: None },
                SourceBreakpoint { line: 14, column: None, condition: None, hit_condition: None, log_message: None },
            ],
        );
        assert!(bps.iter().all(|b| b.verified));

        // Hit 1 -> Line 7
        let stop1 = session.continue_exec().unwrap();
        assert_eq!(stop1, Some(StopReason::Breakpoint(bps[0].id.unwrap())));
        assert_eq!(session.get_stack_trace()[0].line, 7);

        // Hit 2 -> Line 8
        let stop2 = session.continue_exec().unwrap();
        assert_eq!(stop2, Some(StopReason::Breakpoint(bps[1].id.unwrap())));
        assert_eq!(session.get_stack_trace()[0].line, 8);

        // Hit 3 -> Line 14
        let stop3 = session.continue_exec().unwrap();
        assert_eq!(stop3, Some(StopReason::Breakpoint(bps[2].id.unwrap())));
        assert_eq!(session.get_stack_trace()[0].line, 14);

        // Hit 4 -> Program termination (Halt)
        let stop4 = session.continue_exec().unwrap();
        assert_eq!(stop4, Some(StopReason::Halt));
    }
}

#[test]
fn test_dap_macro_multi_call_site_breakpoints() {
    use std::fs;
    use emu8085::dap::protocol::{LaunchRequestArguments, SourceBreakpoint};
    use emu8085::dap::session::{DebugSession, StopReason};

    let tmp_dir = std::env::temp_dir().join("emu8085_test_macro_multi_bp");
    let _ = fs::create_dir_all(&tmp_dir);
    let src_path = tmp_dir.join("multi_bp.e8085");

    let src = r#"%macro mIncA 1
    adi %1
%end

segment .text
main:
    mvi A, 0
    mIncA 5
    mIncA 10
    hlt
"#;
    fs::write(&src_path, src).unwrap();

    let args = LaunchRequestArguments {
        program: src_path.to_string_lossy().to_string(),
        stop_on_entry: Some(true),
        libraries: None,
        t_state_limit: None,
        step_mode: None,
        console: Some("internalConsole".to_string()),
        terminal_port: None,
    };

    let mut session = DebugSession::new();
    session.launch(&args).unwrap();

    // Set breakpoint on Line 2 inside mIncA (adi %1)
    let bps = session.set_line_breakpoints(
        &src_path,
        &[SourceBreakpoint {
            line: 2,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }],
    );
    assert!(bps[0].verified);

    // Hit 1: First invocation of mIncA at line 8 (adding 5)
    let stop1 = session.continue_exec().unwrap();
    assert_eq!(stop1, Some(StopReason::Breakpoint(bps[0].id.unwrap())));
    let frames1 = session.get_stack_trace();
    assert_eq!(frames1[0].line, 2);
    assert_eq!(frames1[1].line, 8, "Caller frame is first call site at Line 8");
    assert_eq!(session.machine.cpu.regs.a, 0);

    // Hit 2: Second invocation of mIncA at line 9 (adding 10)
    let stop2 = session.continue_exec().unwrap();
    assert_eq!(stop2, Some(StopReason::Breakpoint(bps[0].id.unwrap())));
    let frames2 = session.get_stack_trace();
    assert_eq!(frames2[0].line, 2);
    assert_eq!(frames2[1].line, 9, "Caller frame is second call site at Line 9");
    assert_eq!(session.machine.cpu.regs.a, 5);

    // Hit 3: Halt
    let stop3 = session.continue_exec().unwrap();
    assert_eq!(stop3, Some(StopReason::Halt));
    assert_eq!(session.machine.cpu.regs.a, 15);

    let _ = fs::remove_dir_all(&tmp_dir);
}

