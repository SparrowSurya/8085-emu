//! Ported from the reference project's `examples/register_pair_arithmetic.py`.
//! Run with `cargo run --example register_pair_arithmetic`.

use emu8085::{Addr, Instruction, Machine, Opcode, Program, Reg16};

fn main() {
    let mut machine = Machine::create(16, 8);
    let cpu = &mut machine.cpu;

    // Initialize register pairs:
    // BC = 0x1234
    // DE = 0x000F
    // HL = 0x1000
    cpu.regs.set16(Reg16::BC, Addr(0x1234));
    cpu.regs.set16(Reg16::DE, Addr(0x000F));
    cpu.regs.set16(Reg16::HL, Addr(0x1000));

    let program = Program::new(vec![
        Instruction::new(Opcode::INX_BC),  // BC = 0x1235
        Instruction::new(Opcode::DCX_DE),  // DE = 0x000E
        Instruction::new(Opcode::DAD_BC),  // HL = HL + BC = 0x1000 + 0x1235 = 0x2235
        Instruction::new(Opcode::DAD_DE),  // HL = HL + DE = 0x2235 + 0x000E = 0x2243
        Instruction::new(Opcode::HLT),
    ]);

    machine.load(&program, Addr(0x0000)).expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    let bc_val = cpu.regs.get16(Reg16::BC);
    let de_val = cpu.regs.get16(Reg16::DE);
    let hl_val = cpu.regs.get16(Reg16::HL);

    println!("16-Bit Register Pair Arithmetic Example State:");
    println!("Register Pair BC: 0x{:04X} (Expected: 0x1235)", bc_val.0);
    println!("Register Pair DE: 0x{:04X} (Expected: 0x000E)", de_val.0);
    println!("Register Pair HL: 0x{:04X} (Expected: 0x2243)", hl_val.0);
    println!("Carry Flag (CY): {}", cpu.flags.carry);

    assert_eq!(bc_val.0, 0x1235);
    assert_eq!(de_val.0, 0x000E);
    assert_eq!(hl_val.0, 0x2243);
}
