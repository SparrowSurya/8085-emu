//! Run with `cargo run --example stack_operations`.

use emu8085::{Addr, Instruction, Machine, Opcode, Program};

fn main() {
    let mut machine = Machine::create(16, 8);
    let cpu = &mut machine.cpu;

    // Set SP = 0x1000
    cpu.regs.sp = Addr(0x1000);

    // Initialize registers
    cpu.regs.b = 0x11;
    cpu.regs.c = 0x22;
    cpu.regs.h = 0xAA;
    cpu.regs.l = 0xBB;
    cpu.regs.a = 0x55;

    // Clear CY, set Z, S, P flags in PSW
    cpu.flags.carry = false;
    cpu.flags.zero = true;
    cpu.flags.sign = true;
    cpu.flags.parity = true;

    let program = Program::new(vec![
        Instruction::new(Opcode::PUSH_BC), // Stack [0x0FFE, 0x0FFF] = [0x22, 0x11], SP = 0x0FFE
        Instruction::new(Opcode::PUSH_PSW), // Stack [0x0FFC, 0x0FFD] = [Flags, 0x55], SP = 0x0FFC
        Instruction::new(Opcode::POP_DE),  // Pop DE from PSW. D = 0x55, E = Flags, SP = 0x0FFE
        Instruction::new(Opcode::XTHL), // Swap HL with top of stack (0x0FFE -> BC value [0x1234] or 0x1122?)
        // HL becomes 0x1122, Stack at 0x0FFE becomes 0xAABB
        Instruction::new(Opcode::SPHL), // SP = HL = 0x1122
        Instruction::new(Opcode::HLT),
    ]);

    machine
        .load(&program, Addr(0x0000))
        .expect("program compiles");
    machine.run();

    let cpu = &machine.cpu;
    let ram = &machine.ram;
    println!("Stack Operations Example State:");
    println!(
        "Stack Pointer (SP): 0x{:04X} (Expected: 0x1122)",
        cpu.regs.sp.0
    );
    println!("Register D: 0x{:02X} (Expected: 0x55)", cpu.regs.d);
    println!("Register E: 0x{:02X}", cpu.regs.e);
    println!("Register H: 0x{:02X} (Expected: 0x11)", cpu.regs.h);
    println!("Register L: 0x{:02X} (Expected: 0x22)", cpu.regs.l);
    println!(
        "Stack memory at 0x0FFE: 0x{:02X} (Expected: 0xBB)",
        ram.read(Addr(0x0FFE))
    );
    println!(
        "Stack memory at 0x0FFF: 0x{:02X} (Expected: 0xAA)",
        ram.read(Addr(0x0FFF))
    );

    assert_eq!(cpu.regs.sp.0, 0x1122);
    assert_eq!(cpu.regs.d, 0x55);
    assert_eq!(cpu.regs.h, 0x11);
    assert_eq!(cpu.regs.l, 0x22);
    assert_eq!(ram.read(Addr(0x0FFE)), 0xBB);
    assert_eq!(ram.read(Addr(0x0FFF)), 0xAA);
}
