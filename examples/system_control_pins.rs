//! Run with `cargo run --example system_control_pins`.

use emu8085::{Addr, Instruction, Machine, Opcode, Program};

fn main() {
    // 1. Initialize machine
    let mut machine = Machine::create(16, 8);

    // Load a NOP and HLT program
    let program = Program::new(vec![
        Instruction::new(Opcode::NOP),
        Instruction::new(Opcode::HLT),
    ]);
    machine.load(&program, Addr(0x00A0)).expect("program compiles");

    // Set some initial non-zero PC value to check hardware reset
    machine.cpu.regs.pc = Addr(0x1234);
    machine.cpu.inte = true;

    // 2. Assert RESET_IN hardware reset pin on system bus
    println!("Asserting RESET_IN = true");
    machine.bus.lines.reset_in = true;
    machine.tick(); // Tick once to trigger hardware reset latching

    println!("Checking hardware reset state:");
    println!("Program Counter (PC): 0x{:04X} (Expected: 0x0000)", machine.cpu.regs.pc.0);
    println!("Interrupts Enabled (inte): {} (Expected: false)", machine.cpu.inte);
    println!("RESET_OUT Pin on bus: {} (Expected: true)", machine.bus.lines.reset_out);

    assert_eq!(machine.cpu.regs.pc.0, 0x0000);
    assert!(!machine.cpu.inte);
    assert!(machine.bus.lines.reset_out);

    // De-assert reset
    machine.bus.lines.reset_in = false;
    machine.tick();
    println!("RESET_OUT Pin after releasing RESET_IN: {} (Expected: false)", machine.bus.lines.reset_out);
    assert!(!machine.bus.lines.reset_out);

    // Restore PC
    machine.cpu.regs.pc = Addr(0x00A0);

    // 3. Demonstrate READY signal wait states insertion
    println!("\nDemonstrating READY pin wait states:");
    machine.bus.lines.ready = false; // Peripherals not ready, drive READY = false

    let current_pc = machine.cpu.regs.pc;
    machine.tick(); // Tick the machine cycle

    println!("Did PC advance when READY=false? {} (Expected: true - PC does not advance)", machine.cpu.regs.pc == current_pc);
    println!("CPU Cycle State: {:?}, T-state: {}", machine.cpu.cycle, machine.cpu.t_state);

    assert_eq!(machine.cpu.regs.pc, current_pc);

    // Set READY back to true
    machine.bus.lines.ready = true;
    machine.tick();
    println!("Did PC advance after asserting READY=true? {} (Expected: true)", machine.cpu.regs.pc != current_pc);

    assert_ne!(machine.cpu.regs.pc, current_pc);
}
