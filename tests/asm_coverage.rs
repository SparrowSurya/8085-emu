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
fn multiple_segments_of_same_kind_concatenate() {
    let src = "segment .data\nx BYTE 0x11\nsegment .data\ny BYTE 0x22\n\
               segment .text\nmain:\nlda x\nmov B, A\nlda y\nhlt\n";
    let b = image(src);
    // x at 0x0040, y at 0x0041
    assert_eq!(b[0x40], 0x11);
    assert_eq!(b[0x41], 0x22);
    let m = run(src);
    assert_eq!(m.cpu.regs.b, 0x11); // lda x
    assert_eq!(m.cpu.regs.a, 0x22); // lda y
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
