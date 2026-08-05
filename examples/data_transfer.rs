//! Run with `cargo run --example data_transfer`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program, Reg16};

fn main() {
    let mut machine = Machine::create(16, 8);
    let cpu = &mut machine.cpu;
    let ram = &mut machine.ram;

    // Set initial register values
    cpu.regs.a = 0xAA;
    cpu.regs.b = 0x11;
    cpu.regs.c = 0x22;
    cpu.regs.d = 0x33;
    cpu.regs.e = 0x44;
    cpu.regs.set16(Reg16::HL, Addr(0x0100)); // HL points to 0x0100

    // Write a test byte to memory 0x0100
    ram.write(Addr(0x0100), 0x99);

    // Assembly Program covering various data transfers
    let program = Program::new(vec![
        Instruction::new(Opcode::MOV_BA),            // MOV B, A (B becomes 0xAA)
        Instruction::new(Opcode::MOV_AM),            // MOV A, M (A reads from [HL] = 0x99)
        Instruction::with(Opcode::MVI_C, Operand::byte(0x55)), // MVI C, 0x55
        Instruction::new(Opcode::MOV_MC),            // MOV M, C (memory at [HL] becomes 0x55)
        Instruction::new(Opcode::LDA_DE),             // LDAX D (Load Accumulator from memory at DE [0x3344])
        Instruction::new(Opcode::STA_BC),             // STAX B (Store Accumulator to memory at BC [0xAA22])
        Instruction::with(Opcode::LHLD, Operand::word(0x0200)), // LHLD 0x0200 (Load HL from memory 0x0200)
        Instruction::with(Opcode::SHLD, Operand::word(0x0300)), // SHLD 0x0300 (Store HL to memory 0x0300)
        Instruction::new(Opcode::XCHG),               // XCHG (Exchange HL and DE)
        Instruction::new(Opcode::HLT),
    ]);

    // Write source data to memory locations
    ram.write(Addr(0x3344), 0x77); // DE pointer source
    ram.write(Addr(0x0200), 0xEF); // L value
    ram.write(Addr(0x0201), 0xBE); // H value (HL = 0xBEEF)

    // Load and run program
    machine.load(&program, Addr(0x0000)).expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    let ram = &machine.ram;
    println!("Data Transfer Example State:");
    println!("Register A: 0x{:02X}", cpu.regs.a);
    println!("Register B: 0x{:02X}", cpu.regs.b);
    println!("Register C: 0x{:02X}", cpu.regs.c);
    println!("Register D: 0x{:02X}", cpu.regs.d);
    println!("Register E: 0x{:02X}", cpu.regs.e);
    println!("Register H: 0x{:02X}", cpu.regs.h);
    println!("Register L: 0x{:02X}", cpu.regs.l);
    println!("Memory at 0x0100: 0x{:02X}", ram.read(Addr(0x0100)));
    println!("Memory at 0xAA22: 0x{:02X}", ram.read(Addr(0xAA22)));
    println!("Memory at 0x0300: 0x{:02X}", ram.read(Addr(0x0300)));
    println!("Memory at 0x0301: 0x{:02X}", ram.read(Addr(0x0301)));

    assert_eq!(cpu.regs.b, 0xAA);
    assert_eq!(cpu.regs.a, 0x77);
    assert_eq!(cpu.regs.c, 0x55);
    assert_eq!(ram.read(Addr(0x0100)), 0x55);
    assert_eq!(ram.read(Addr(0xAA22)), 0x77);
    assert_eq!(ram.read(Addr(0x0300)), 0xEF);
    assert_eq!(ram.read(Addr(0x0301)), 0xBE);
}
