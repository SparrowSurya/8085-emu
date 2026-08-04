use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program};

fn main() {
    let mut machine = Machine::default();
    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_A, Operand::Byte(0x03)),
        Instruction::new(Opcode::DCR_A).labeled("loop"),
        Instruction::with(Opcode::JNZ, Operand::Label(String::from("loop"))),
        Instruction::new(Opcode::HLT),
    ]);

    machine.load(&program, Addr::from_le(0xA0, 0x00)).unwrap();
    machine.run();
}