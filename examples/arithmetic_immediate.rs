//! Run with `cargo run --example arithmetic_immediate`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program};

fn main() {
    let mut machine = Machine::create(16, 8);

    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_A, Operand::byte(0x10)),
        Instruction::with(Opcode::ADI, Operand::byte(0x20)), // A = 0x30
        Instruction::with(Opcode::ACI, Operand::byte(0x05)), // A = 0x35
        Instruction::new(Opcode::STC),                       // Set CY = 1
        Instruction::with(Opcode::ACI, Operand::byte(0x05)), // A = 0x3B
        Instruction::with(Opcode::SUI, Operand::byte(0x10)), // A = 0x2B
        Instruction::new(Opcode::STC),                       // Set CY = 1
        Instruction::with(Opcode::SBI, Operand::byte(0x0A)), // A = 0x20
        Instruction::new(Opcode::HLT),
    ]);

    machine
        .load(&program, Addr(0x0000))
        .expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    println!("Arithmetic Immediate Example State:");
    println!("Accumulator (A): 0x{:02X} (Expected: 0x20)", cpu.regs.a);
    println!("Carry Flag (CY): {}", cpu.flags.carry);
    println!("Zero Flag (Z): {}", cpu.flags.zero);
    println!("Sign Flag (S): {}", cpu.flags.sign);

    assert_eq!(cpu.regs.a, 0x20);
}
