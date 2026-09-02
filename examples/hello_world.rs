//! Prints "Hello, World!" by walking a null-terminated string in RAM and sending each
//! byte to a printer on I/O port 0x02. Run with `cargo run --example hello_world`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, PrinterDevice, Program};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    // A printer that echoes each character to stdout as it is written.
    let printed = Rc::new(RefCell::new(String::new()));
    let sink = printed.clone();
    let printer = PrinterDevice::with_callback(move |ch| {
        print!("{ch}");
        sink.borrow_mut().push(ch);
    });

    let mut machine = Machine::create(16, 8);
    machine.attach_device(Box::new(printer), &[0x02]);

    // Read chars from 0x0100 until NUL, printing each to port 0x02.
    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_HL, Operand::word(0x0100)), // HL -> string start
        Instruction::new(Opcode::MOV_AM).labeled("LOOP"),         // A = *HL
        Instruction::with(Opcode::CPI, Operand::byte(0x00)),      // NUL?
        Instruction::with(Opcode::JZ, Operand::label("END")),
        Instruction::with(Opcode::OUT, Operand::byte(0x02)), // print A
        Instruction::new(Opcode::INX_HL),                    // HL++
        Instruction::with(Opcode::JMP, Operand::label("LOOP")),
        Instruction::new(Opcode::HLT).labeled("END"),
    ]);

    machine
        .load(&program, Addr(0x0000))
        .expect("program compiles");

    // Place the message (NUL-terminated) at 0x0100.
    let message = b"Hello, World!\n";
    for (i, &b) in message.iter().enumerate() {
        machine.ram.write(Addr(0x0100 + i as u16), b);
    }
    machine.ram.write(Addr(0x0100 + message.len() as u16), 0x00);

    machine.run();
    assert_eq!(*printed.borrow(), "Hello, World!\n");
}
