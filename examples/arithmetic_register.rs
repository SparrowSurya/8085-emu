//! Ported from the reference project's `examples/arithmetic_register.py`.
//! Run with `cargo run --example arithmetic_register`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program, Reg16};

fn main() {
    let mut machine = Machine::create(16, 8);

    // Set up HL to point to memory location 0x0100
    machine.cpu.regs.set16(Reg16::HL, Addr(0x0100));
    machine.ram.write(Addr(0x0100), 0x05); // Memory operand M = 0x05

    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_A, Operand::byte(0x12)), // A = 0x12
        Instruction::with(Opcode::MVI_B, Operand::byte(0x0E)), // B = 0x0E
        Instruction::new(Opcode::ADD_B),                      // A = A + B = 0x12 + 0x0E = 0x20
        Instruction::new(Opcode::ADD_M),                      // A = A + M = 0x20 + 0x05 = 0x25
        Instruction::new(Opcode::STC),                         // CY = 1
        Instruction::new(Opcode::ADC_B),                      // A = A + B + CY = 0x25 + 0x0E + 1 = 0x34
        Instruction::new(Opcode::SUB_B),                      // A = A - B = 0x34 - 0x0E = 0x26
        Instruction::new(Opcode::STC),                         // CY = 1
        Instruction::new(Opcode::SBB_M),                      // A = A - M - CY = 0x26 - 0x05 - 1 = 0x20
        Instruction::new(Opcode::INR_B),                      // B = B + 1 = 0x0F
        Instruction::new(Opcode::INR_M),                      // Memory at HL [0x0100] = 0x06
        Instruction::new(Opcode::DCR_B),                      // B = B - 1 = 0x0E
        Instruction::new(Opcode::DCR_M),                      // Memory at HL [0x0100] = 0x05
        Instruction::new(Opcode::HLT),
    ]);

    machine.load(&program, Addr(0x0000)).expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    let ram = &machine.ram;
    println!("Arithmetic Register & Memory Example State:");
    println!("Accumulator (A): 0x{:02X} (Expected: 0x20)", cpu.regs.a);
    println!("Register B: 0x{:02X} (Expected: 0x0E)", cpu.regs.b);
    println!("Memory at 0x0100: 0x{:02X} (Expected: 0x05)", ram.read(Addr(0x0100)));
    println!("Flags: Z={}, CY={}, S={}", cpu.flags.zero, cpu.flags.carry, cpu.flags.sign);

    assert_eq!(cpu.regs.a, 0x20);
    assert_eq!(cpu.regs.b, 0x0E);
    assert_eq!(ram.read(Addr(0x0100)), 0x05);
}
