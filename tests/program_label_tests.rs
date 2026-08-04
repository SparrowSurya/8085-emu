//! Differential tests for the label compiler: the reference's own labeled example
//! programs must compile to identical bytes, and forward/backward/unresolved references
//! must behave as the reference does.

use emu8085::instruction::Opcode;
use emu8085::{Addr, Instruction, Operand, Program};

fn op(o: Opcode) -> Instruction {
    Instruction::new(o)
}
fn byte(o: Opcode, b: u8) -> Instruction {
    Instruction::with(o, Operand::byte(b))
}
fn word(o: Opcode, w: u16) -> Instruction {
    Instruction::with(o, Operand::word(w))
}
fn lbl(o: Opcode, name: &str) -> Instruction {
    Instruction::with(o, Operand::label(name))
}

#[test]
fn hello_world_labels_compiles_identically() {
    // Ported from examples/hello_world_labels.py.
    let prog = Program::new(vec![
        lbl(Opcode::MVI_HL, "STR_DATA"), // LXI H, STR_DATA
        op(Opcode::MOV_AM).labeled("LOOP"),
        byte(Opcode::CPI, 0x00),
        lbl(Opcode::JZ, "EXIT"),
        byte(Opcode::OUT, 0x02),
        op(Opcode::INX_HL),
        lbl(Opcode::JMP, "LOOP"),
        op(Opcode::HLT).labeled("EXIT"),
        op(Opcode::NOP).labeled("STR_DATA"),
    ]);
    let expected = [
        0x21, 0x10, 0x00, 0x7e, 0xfe, 0x00, 0xca, 0x0f, 0x00, 0xd3, 0x02, 0x23, 0xc3, 0x03, 0x00,
        0x76, 0x00,
    ];
    assert_eq!(prog.compile(Addr(0x0000)).unwrap(), expected);
}

#[test]
fn loop_multiplication_labels_compiles_identically() {
    // Ported from examples/loop_multiplication_labels.py.
    let prog = Program::new(vec![
        byte(Opcode::MVI_B, 7),
        byte(Opcode::MVI_C, 6),
        byte(Opcode::MVI_A, 0),
        op(Opcode::ADD_B).labeled("LOOP"),
        op(Opcode::DCR_C),
        lbl(Opcode::JNZ, "LOOP"),
        word(Opcode::STA, 0x0020),
        op(Opcode::HLT),
    ]);
    let expected = [
        0x06, 0x07, 0x0e, 0x06, 0x3e, 0x00, 0x80, 0x0d, 0xc2, 0x06, 0x00, 0x32, 0x20, 0x00, 0x76,
    ];
    assert_eq!(prog.compile(Addr(0x0000)).unwrap(), expected);
}