//! Run with `cargo run --example bcd_arithmetic`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program};

fn main() {
    let mut machine = Machine::create(16, 8);

    // Test BCD addition:
    // 0x38 + 0x45 = 0x7D (binary sum)
    // Applying DAA (Decimal Adjust Accumulator) corrects 0x7D to BCD 0x83
    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_A, Operand::byte(0x38)), // Load BCD 38 into A
        Instruction::with(Opcode::ADI, Operand::byte(0x45)),   // Add BCD 45 -> binary sum 0x7D
        Instruction::new(Opcode::DAA),                         // Adjust to BCD -> 0x83
        Instruction::new(Opcode::HLT),
    ]);

    machine.load(&program, Addr(0x0000)).expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    println!("BCD Arithmetic & DAA Example State:");
    println!("Accumulator (A): 0x{:02X} (Expected BCD result: 0x83)", cpu.regs.a);
    println!("Flags: CY={}, Z={}, S={}", cpu.flags.carry, cpu.flags.zero, cpu.flags.sign);

    assert_eq!(cpu.regs.a, 0x83);
}
