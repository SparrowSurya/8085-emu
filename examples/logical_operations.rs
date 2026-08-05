//! Run with `cargo run --example logical_operations`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program, Reg16};

fn main() {
    let mut machine = Machine::create(16, 8);

    // Set up HL pointing to memory location 0x0100
    machine.cpu.regs.set16(Reg16::HL, Addr(0x0100));
    machine.ram.write(Addr(0x0100), 0x55); // M = 0x55

    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_A, Operand::byte(0xFF)), // A = 0xFF
        Instruction::with(Opcode::MVI_B, Operand::byte(0x0A)), // B = 0x0A
        Instruction::new(Opcode::ANA_B),                      // A = A & B = 0x0A
        Instruction::with(Opcode::ANI, Operand::byte(0x0F)),   // A = A & 0x0F = 0x0A
        Instruction::new(Opcode::ORA_M),                      // A = A | M = 0x0A | 0x55 = 0x5F
        Instruction::with(Opcode::ORI, Operand::byte(0x80)),   // A = A | 0x80 = 0xDF
        Instruction::new(Opcode::XRA_B),                      // A = A ^ B = 0xDF ^ 0x0A = 0xD5
        Instruction::with(Opcode::XRI, Operand::byte(0xD5)),   // A = A ^ 0xD5 = 0x00 (Zero flag set to 1)
        Instruction::with(Opcode::MVI_A, Operand::byte(0x10)), // A = 0x10
        Instruction::new(Opcode::CMP_B),                      // Compare A with B (0x10 > 0x0A -> Carry clear, Zero clear)
        Instruction::with(Opcode::CPI, Operand::byte(0x10)),   // Compare A with 0x10 (0x10 == 0x10 -> Zero set, Carry clear)
        Instruction::new(Opcode::CMA),                         // Complement A (A = ~0x10 = 0xEF)
        Instruction::new(Opcode::HLT),
    ]);

    machine.load(&program, Addr(0x0000)).expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    println!("Logical Operations Example State:");
    println!("Accumulator (A): 0x{:02X} (Expected: 0xEF)", cpu.regs.a);
    println!("Zero Flag (Z): {}", cpu.flags.zero);
    println!("Carry Flag (CY): {}", cpu.flags.carry);
    println!("Sign Flag (S): {}", cpu.flags.sign);
    println!("Parity Flag (P): {}", cpu.flags.parity);

    assert_eq!(cpu.regs.a, 0xEF);
}
