//! Run with `cargo run --example loop_multiplication_labels`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program};

fn main() {
    let mut machine = Machine::create(16, 8);

    // Program to multiply B * C using successive addition loop
    // B = 7 (multiplicand)
    // C = 6 (multiplier)
    // Result accumulated in A, then stored in memory at address 0x0020
    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_B, Operand::byte(7)),
        Instruction::with(Opcode::MVI_C, Operand::byte(6)),
        Instruction::with(Opcode::MVI_A, Operand::byte(0)), // Initialize accumulator A = 0
        // LOOP:
        Instruction::new(Opcode::ADD_B).labeled("LOOP"), // A = A + B
        Instruction::new(Opcode::DCR_C),                 // Decrement C
        Instruction::with(Opcode::JNZ, Operand::label("LOOP")), // If C != 0, JMP back to LOOP
        // Store result and halt
        Instruction::with(Opcode::STA, Operand::word(0x0020)),
        Instruction::new(Opcode::HLT),
    ]);

    machine
        .load(&program, Addr(0x0000))
        .expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    let ram = &machine.ram;
    println!("Multiplication Example State:");
    println!("Multiplicand B: {}", cpu.regs.b);
    println!("Multiplier   C: {}", cpu.regs.c);
    println!("Product      A: {} (Expected: 42)", cpu.regs.a);
    println!("Value at 0x0020: {} (Expected: 42)", ram.read(Addr(0x0020)));

    assert_eq!(cpu.regs.a, 42);
    assert_eq!(ram.read(Addr(0x0020)), 42);
}
