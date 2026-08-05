//! Ported from the reference project's `examples/branching_control.py`.
//! Run with `cargo run --example branching_control`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program};

fn main() {
    let mut machine = Machine::create(16, 8);
    machine.cpu.regs.sp = Addr(0x1000); // SP points to 0x1000

    // Subroutine at address 0x00A0:
    // 0x00A0: MVI B, 0xAA
    // 0x00A2: RET
    machine.ram.write(Addr(0x00A0), Opcode::MVI_B as u8);
    machine.ram.write(Addr(0x00A1), 0xAA);
    machine.ram.write(Addr(0x00A2), Opcode::RET as u8);

    // Conditional Jump Target at 0x00B0:
    // 0x00B0: MVI C, 0xBB
    // 0x00B2: HLT
    machine.ram.write(Addr(0x00B0), Opcode::MVI_C as u8);
    machine.ram.write(Addr(0x00B1), 0xBB);
    machine.ram.write(Addr(0x00B2), Opcode::HLT as u8);

    let program = Program::new(vec![
        Instruction::with(Opcode::CALL, Operand::word(0x00A0)),     // Call subroutine at 0x00A0
        Instruction::with(Opcode::CPI, Operand::byte(0x00)),         // CPI 0x00 (Sets Zero Flag since A=0)
        Instruction::with(Opcode::JZ, Operand::word(0x00B0)),       // Jump if Zero to 0x00B0 (will jump)
        Instruction::new(Opcode::HLT),                               // Should not be executed
    ]);

    machine.load(&program, Addr(0x0000)).expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    println!("Branching & Control Example State:");
    println!("Register B: 0x{:02X} (Expected: 0xAA from subroutine)", cpu.regs.b);
    println!("Register C: 0x{:02X} (Expected: 0xBB from conditional jump)", cpu.regs.c);
    println!("Program Counter (PC): 0x{:04X}", cpu.regs.pc.0);
    println!("Stack Pointer (SP): 0x{:04X}", cpu.regs.sp.0);

    assert_eq!(cpu.regs.b, 0xAA);
    assert_eq!(cpu.regs.c, 0xBB);
}
