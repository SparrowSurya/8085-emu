//! Ported from the reference project's `examples/usb_dma_transfer.py`.
//! Run with `cargo run --example usb_dma_transfer`.

use emu8085::{Addr, Instruction, Machine, Opcode, Program, USBDevice};

fn main() {
    // 1. Create a USB device (supporting DMA transfers)
    let mut usb = USBDevice::new();

    // 2. Initialize the machine with the USB device attached to port 0x10
    let mut machine = Machine::create(16, 8);
    machine.attach_device(Box::new(USBDevice::new()), &[0x10]);

    // Set up some program in memory
    let program = Program::new(vec![
        Instruction::new(Opcode::NOP),
        Instruction::new(Opcode::HLT),
    ]);
    machine.load(&program, Addr(0x00A0)).expect("program compiles");

    // 3. Simulate high-speed USB writing data directly to RAM via DMA protocol
    // The DMA protocol asserts HOLD -> CPU grants HLDA -> USB writes directly -> Release HOLD
    let write_data = b"USB_DMA_PACKET";
    println!("USB writing to memory at 0x0200 via DMA: {:?}", std::str::from_utf8(write_data).unwrap());
    machine.dma_write(&mut usb, 0x0200, write_data);

    // 4. Simulate high-speed USB reading data back from RAM via DMA protocol
    let read_data = machine.dma_read(&mut usb, 0x0200, write_data.len());
    println!("USB read back from memory at 0x0200 via DMA: {:?}", std::str::from_utf8(&read_data).unwrap());

    // Verify that CPU yields control and memory is updated
    println!("\nUSB DMA Example State:");
    println!("Read Match: {}", read_data == write_data);

    let mut mem_read_back = Vec::new();
    for i in 0..write_data.len() {
        mem_read_back.push(machine.ram.read(Addr(0x0200 + i as u16)));
    }
    println!("Memory at 0x0200: {:?}", std::str::from_utf8(&mem_read_back).unwrap());

    assert_eq!(read_data, write_data);
    assert_eq!(mem_read_back, write_data);
}
