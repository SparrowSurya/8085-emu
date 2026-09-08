//! Coverage for spec features that were implemented but not yet exercised by tests:
//! WORD data arrays, %repeat of a string, %define chaining, char-literal operands,
//! and multiple segments of the same kind.

use emu8085::Machine;
use emu8085::asm::{assemble, load};

fn image(src: &str) -> Vec<u8> {
    assemble(src).expect("assembles").bytes
}
fn run(src: &str) -> Machine {
    let img = assemble(src).expect("assembles");
    let mut m = Machine::create(16, 8);
    load(&mut m, &img).expect("loads");
    m.run();
    assert!(m.cpu.is_halt);
    m
}

#[test]
fn word_data_array_is_little_endian() {
    let b = image("segment .data\ntable WORD 0x1234 0xABCD\nsegment .text\nmain:\nhlt\n");
    // table at 0x0040: 0x1234 -> 34 12, 0xABCD -> CD AB
    assert_eq!(&b[0x40..0x44], &[0x34, 0x12, 0xCD, 0xAB]);
}

#[test]
fn repeat_of_a_string() {
    let b = image("segment .data\nr BYTE %repeat 3 \"ab\"\nsegment .text\nmain:\nhlt\n");
    assert_eq!(&b[0x40..0x46], b"ababab");
}

#[test]
fn define_chaining() {
    // one define references an earlier define
    let m =
        run("%define FIRST 5\n%define SECOND FIRST\nsegment .text\nmain:\nmvi A, SECOND\nhlt\n");
    assert_eq!(m.cpu.regs.a, 5);
}

#[test]
fn char_literal_operand() {
    let m = run("segment .text\nmain:\nmvi A, 'Z'\nhlt\n");
    assert_eq!(m.cpu.regs.a, 0x5A);
}

#[test]
fn duplicate_segment_in_single_file_is_rejected() {
    use emu8085::asm::AsmErrorKind;
    let src = "segment .data\nx BYTE 0x11\nsegment .data\ny BYTE 0x22\n\
               segment .text\nmain:\nhlt\n";
    let err = emu8085::asm::assemble(src).unwrap_err();
    assert!(matches!(err.kind, AsmErrorKind::DuplicateSegment(ref s) if s == "data"));
}

#[test]
fn len_of_a_bss_variable() {
    // %len reports a .bss reservation's byte size (WORD count x 2).
    let m = run("segment .bss\nbuf WORD 5\nsegment .text\nmain:\nmvi A, %len buf\nhlt\n");
    assert_eq!(m.cpu.regs.a, 10); // 5 words = 10 bytes
}

#[test]
fn forward_len_reference_is_rejected() {
    // %len must refer backward in source order; a forward reference is undefined.
    use emu8085::asm::AsmErrorKind;
    let err = assemble(
        "segment .data\nfirst BYTE %len second\nsecond BYTE 1 2 3\nsegment .text\nmain:\nhlt\n",
    )
    .unwrap_err();
    assert!(matches!(err.kind, AsmErrorKind::UndefinedName(_)));
}

#[test]
fn duplicate_global_declaration_is_rejected() {
    use emu8085::asm::AsmErrorKind;
    // Top-level duplicate
    let src1 = "global func\nglobal func\nsegment .text\nfunc:\nret\nmain:\nhlt\n";
    let err1 = assemble(src1).unwrap_err();
    assert!(matches!(err1.kind, AsmErrorKind::DuplicateGlobal(ref s) if s == "func"));

    // Inline + top-level duplicate
    let src2 = "global func\nsegment .text\nglobal func:\nret\nmain:\nhlt\n";
    let err2 = assemble(src2).unwrap_err();
    assert!(matches!(err2.kind, AsmErrorKind::DuplicateGlobal(ref s) if s == "func"));
}

#[test]
fn duplicate_extern_declaration_is_rejected() {
    use emu8085::asm::AsmErrorKind;
    let src = "extern helper\nextern helper\nsegment .text\nmain:\nhlt\n";
    let err = assemble(src).unwrap_err();
    assert!(matches!(err.kind, AsmErrorKind::DuplicateExtern(ref s) if s == "helper"));
}

#[test]
fn conflicting_global_and_extern_declaration_is_rejected() {
    use emu8085::asm::AsmErrorKind;
    let src1 = "global foo\nextern foo\nsegment .text\nfoo:\nret\nmain:\nhlt\n";
    let err1 = assemble(src1).unwrap_err();
    assert!(matches!(err1.kind, AsmErrorKind::ConflictingDeclaration(ref s) if s == "foo"));

    let src2 = "extern bar\nglobal bar\nsegment .text\nbar:\nret\nmain:\nhlt\n";
    let err2 = assemble(src2).unwrap_err();
    assert!(matches!(err2.kind, AsmErrorKind::ConflictingDeclaration(ref s) if s == "bar"));
}
