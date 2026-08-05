//! Ported from the reference project's `examples/printer_output.py`.
//! Run with `cargo run --example printer_output`.

use emu8085::{Addr, Instruction, Machine, Opcode, Operand, PrinterDevice, Program};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    // 1. Create a printer device with a custom logging callback
    let printed_chars = Rc::new(RefCell::new(Vec::new()));
    let sink = printed_chars.clone();
    let printer = PrinterDevice::with_callback(move |char| {
        sink.borrow_mut().push(char);
        println!("[Printer Output]: '{char}'");
    });

    // 2. Attach printer to port 0x05
    let mut machine = Machine::create(16, 8);
    machine.attach_device(Box::new(printer), &[0x05]);

    // 3. Create a program that outputs the letters 'A', 'B', 'C' to the printer
    let program = Program::new(vec![
        Instruction::with(Opcode::MVI_A, Operand::byte(b'A')),
        Instruction::with(Opcode::OUT, Operand::byte(0x05)),
        Instruction::with(Opcode::MVI_A, Operand::byte(b'B')),
        Instruction::with(Opcode::OUT, Operand::byte(0x05)),
        Instruction::with(Opcode::MVI_A, Operand::byte(b'C')),
        Instruction::with(Opcode::OUT, Operand::byte(0x05)),
        Instruction::new(Opcode::HLT),
    ]);

    machine.load(&program, Addr(0x0000)).expect("program compiles");
    machine.run();

    let term = machine.devices.device_ref::<PrinterDevice>(0).unwrap();
    println!("\nPrinter Output Example State:");
    println!("Printed character buffer: {:?}", printed_chars.borrow());
    println!("Printer device history: {}", term.history);

    assert_eq!(*printed_chars.borrow(), vec!['A', 'B', 'C']);
}
