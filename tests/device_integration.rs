//! End-to-end device tests through the full CPU + RAM + DeviceManager loop, matching the
//! reference: keyboard `IN`, printer `OUT`, and an INTR/INTA vector fetch.

use emu8085::{Addr, Cpu, DeviceManager, KeyboardDevice, Memory, PrinterDevice, SystemBus};
use std::cell::RefCell;
use std::rc::Rc;

/// Tick CPU + RAM + devices together (the same order the Machine will use) until HLT.
fn run(cpu: &mut Cpu, ram: &mut Memory, dm: &mut DeviceManager) {
    let mut bus = SystemBus::default();
    let mut t = 0;
    while !cpu.is_halt && cpu.fault.is_none() && t < 100_000 {
        cpu.process(&mut bus);
        ram.step(&mut bus);
        dm.step(&mut bus);
        t += 1;
    }
}

#[test]
fn keyboard_in_reads_buffered_key_then_zero() {
    let mut kbd = KeyboardDevice::new();
    kbd.press_char('X').unwrap();
    let mut dm = DeviceManager::new();
    dm.attach(Box::new(kbd), &[0x01]);

    let mut cpu = Cpu::new();
    let mut ram = Memory::from_lines(16);
    // IN 1 ; MOV B,A ; IN 1 ; MOV C,A ; HLT
    ram.load_bytes(&[0xDB, 0x01, 0x47, 0xDB, 0x01, 0x4F, 0x76], Addr(0x00A0))
        .unwrap();
    cpu.start_at(Addr(0x00A0));
    run(&mut cpu, &mut ram, &mut dm);

    assert_eq!(cpu.regs.b, b'X'); // first IN drained the key
    assert_eq!(cpu.regs.c, 0x00); // buffer empty on the second IN
}

#[test]
fn printer_out_streams_characters() {
    let seen = Rc::new(RefCell::new(String::new()));
    let sink = seen.clone();
    let printer = PrinterDevice::with_callback(move |c| sink.borrow_mut().push(c));
    let mut dm = DeviceManager::new();
    dm.attach(Box::new(printer), &[0x02]);

    let mut cpu = Cpu::new();
    let mut ram = Memory::from_lines(16);
    // MVI A,'H' ; OUT 2 ; MVI A,'i' ; OUT 2 ; HLT
    ram.load_bytes(
        &[0x3E, b'H', 0xD3, 0x02, 0x3E, b'i', 0xD3, 0x02, 0x76],
        Addr(0x00A0),
    )
    .unwrap();
    cpu.start_at(Addr(0x00A0));
    run(&mut cpu, &mut ram, &mut dm);

    assert_eq!(*seen.borrow(), "Hi");
}

#[test]
fn intr_inta_fetches_rst_vector_from_device() {
    // Keyboard configured to answer INTA with RST 2 (0xD7).
    let kbd = KeyboardDevice::with_vector(2);
    let mut dm = DeviceManager::new();
    dm.attach(Box::new(kbd), &[0x01]);

    let mut cpu = Cpu::new();
    let mut ram = Memory::from_lines(16);
    cpu.regs.sp = Addr(0x1000);
    // ISR at RST 2 vector (0x0010): MVI A,0x99 ; RET
    for (a, b) in [(0x10u16, 0x3Eu8), (0x11, 0x99), (0x12, 0xC9)] {
        ram.write(Addr(a), b);
    }
    // EI ; NOP ; HLT
    ram.load_bytes(&[0xFB, 0x00, 0x76], Addr(0x00A0)).unwrap();
    cpu.start_at(Addr(0x00A0));
    cpu.intr = true;
    run(&mut cpu, &mut ram, &mut dm);

    assert_eq!(cpu.regs.a, 0x99); // ISR ran via the device-supplied restart opcode
}
