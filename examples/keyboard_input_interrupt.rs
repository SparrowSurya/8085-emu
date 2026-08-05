//! Ported from the reference project's `examples/keyboard_input_interrupt.py`.
//! Run with `cargo run --example keyboard_input_interrupt`.

use emu8085::{Addr, Instruction, KeyboardDevice, Machine, Opcode, Program};

fn main() {
    // 1. Create a keyboard device that triggers interrupt vector 1 (RST 1 -> vector 0x0008)
    let mut kbd = KeyboardDevice::with_vector(1);

    // Trigger a keyboard keypress event
    kbd.press_char('K').unwrap();

    // 2. Create machine and attach keyboard device to port 0x01
    let mut machine = Machine::create(16, 8);
    machine.attach_device(Box::new(kbd), &[0x01]);
    machine.cpu.regs.sp = Addr(0x1000);

    // 3. Write ISR for RST 1 at 0x0008:
    // 0x0008: IN 0x01   - Read ASCII key value from keyboard port into A
    // 0x000A: MOV B, A  - Save input key in Register B
    // 0x000B: RET       - Return from interrupt handler
    machine.ram.write(Addr(0x0008), Opcode::IN as u8);
    machine.ram.write(Addr(0x0009), 0x01);
    machine.ram.write(Addr(0x000A), Opcode::MOV_BA as u8);
    machine.ram.write(Addr(0x000B), Opcode::RET as u8);

    // 4. Main Program: Enable interrupts and NOP in loop
    let program = Program::new(vec![
        Instruction::new(Opcode::EI),  // Enable Interrupts (inte = true)
        Instruction::new(Opcode::NOP),
        Instruction::new(Opcode::HLT),
    ]);

    machine.load(&program, Addr(0x00A0)).expect("program compiles");

    // Emulate the hardware interrupt request asserting INTR pin
    machine.cpu.intr = true;

    // 5. Run the machine
    machine.run();

    let cpu = &machine.cpu;
    println!("Keyboard Input & Interrupt Example State:");
    println!("Register B: 0x{:02X} (Expected: 0x4B - ASCII 'K')", cpu.regs.b);
    println!("Interrupts Enabled (inte): {}", cpu.inte);

    assert_eq!(cpu.regs.b, b'K');
}
