//! Prints "Hi Labels!" by walking a label-referenced string in RAM and sending each
//! byte to a printer on I/O port 0x02. Run with `cargo run --example hello_world_labels`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, PrinterDevice, Program};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let printed = Rc::new(RefCell::new(String::new()));
    let sink = printed.clone();
    let printer = PrinterDevice::with_callback(move |ch| {
        print!("{ch}");
        sink.borrow_mut().push(ch);
    });

    let mut machine = Machine::create(16, 8);
    machine.attach_device(Box::new(printer), &[0x02]);

    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_HL, Operand::label("STR_DATA")), // LXI H, STR_DATA
        Instruction::new(Opcode::MOV_AM).labeled("LOOP"),              // A = *HL
        Instruction::with(Opcode::CPI, Operand::byte(0x00)),          // NUL?
        Instruction::with(Opcode::JZ, Operand::label("EXIT")),
        Instruction::with(Opcode::OUT, Operand::byte(0x02)),          // print A
        Instruction::new(Opcode::INX_HL),                              // HL++
        Instruction::with(Opcode::JMP, Operand::label("LOOP")),
        Instruction::new(Opcode::HLT).labeled("EXIT"),
        Instruction::new(Opcode::NOP).labeled("STR_DATA"),
    ]);

    machine.load(&program, Addr(0x0000)).expect("program compiles");

    // Place the message (NUL-terminated) at STR_DATA (0x0010).
    let message = b"Hi Labels!\n";
    for (i, &b) in message.iter().enumerate() {
        machine.ram.write(Addr(0x0010 + i as u16), b);
    }
    machine.ram.write(Addr(0x0010 + message.len() as u16), 0x00);

    machine.run();
    assert_eq!(*printed.borrow(), "Hi Labels!\n");
}
