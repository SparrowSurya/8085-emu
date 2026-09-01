//! Integration tests verifying that all .e8085 programs in the programs/ directory
//! compile, load, and execute correctly on the 8085 emulator.

use std::cell::RefCell;
use std::rc::Rc;

use emu8085::asm::{assemble, load};
use emu8085::{Machine, PrinterDevice};

fn run_program(src: &str) -> Machine {
    let image = assemble(src).expect("program assembles cleanly");
    let mut m = Machine::create(16, 8);
    load(&mut m, &image).expect("program loads into RAM");
    m.run();
    assert!(m.cpu.is_halt, "program did not halt");
    m
}

fn run_program_with_printer(src: &str) -> (Machine, String) {
    let image = assemble(src).expect("program assembles cleanly");
    let printed = Rc::new(RefCell::new(String::new()));
    let sink = printed.clone();
    let mut m = Machine::create(16, 8);
    m.attach_device(
        Box::new(PrinterDevice::with_callback(move |c| {
            sink.borrow_mut().push(c)
        })),
        &[0x02],
    );
    load(&mut m, &image).expect("program loads into RAM");
    m.run();
    assert!(m.cpu.is_halt, "program did not halt");
    let out = printed.borrow().clone();
    (m, out)
}

#[test]
fn test_program_array_sum() {
    let src = include_str!("../programs/array_sum.e8085");
    let m = run_program(src);
    assert_eq!(m.cpu.regs.a, 10); // 1 + 2 + 3 + 4 = 10
}

#[test]
fn test_program_directives() {
    let src = include_str!("../programs/directives.e8085");
    let m = run_program(src);
    assert_eq!(m.cpu.regs.a, 4); // %len pattern = 4
    assert_eq!(m.cpu.regs.h, 0x00);
    assert_eq!(m.cpu.regs.l, 0x44); // scratch address in .bss
}

#[test]
fn test_program_print_stars() {
    let src = include_str!("../programs/print_stars.e8085");
    let (_m, out) = run_program_with_printer(src);
    assert_eq!(out, "*****");
}

#[test]
fn test_program_subroutine() {
    let src = include_str!("../programs/subroutine.e8085");
    let m = run_program(src);
    assert_eq!(m.cpu.regs.a, 8); // 5 + 3 = 8
    assert_eq!(m.cpu.regs.sp.0, 0xF000); // stack pointer balanced
}

#[test]
fn test_program_software_interrupts() {
    let src = include_str!("../programs/software_interrupts.e8085");
    let (m, out) = run_program_with_printer(src);
    assert_eq!(out, "6"); // 1 + 5 = 6 printed as ASCII '6'
    assert_eq!(m.cpu.regs.sp.0, 0xF000); // return stack balanced
}

#[test]
fn test_program_hardware_trap() {
    let src = include_str!("../programs/hardware_trap.e8085");
    let image = assemble(src).expect("assembles cleanly");
    let printed = Rc::new(RefCell::new(String::new()));
    let sink = printed.clone();
    let mut m = Machine::create(16, 8);
    m.attach_device(
        Box::new(PrinterDevice::with_callback(move |c| {
            sink.borrow_mut().push(c)
        })),
        &[0x02],
    );
    load(&mut m, &image).expect("loads cleanly");
    // Inject illegal opcode at main + 3 to trigger TRAP handler
    let trap_trigger_addr = emu8085::Addr(image.entry + 3);
    m.ram.write(trap_trigger_addr, 0x08); // 0x08 is invalid opcode
    m.run();

    assert_eq!(*printed.borrow(), "Trap handled!\n");
    assert!(m.cpu.is_halt);
}

#[test]
fn test_program_hello_world() {
    let src = include_str!("../programs/hello_world.e8085");
    let image = assemble(src).expect("program assembles cleanly");
    let printed = Rc::new(RefCell::new(String::new()));
    let sink = printed.clone();
    let mut m = Machine::create(16, 8);
    let (_tx, rx) = std::sync::mpsc::channel();
    let terminal = emu8085::TerminalDevice::with_io(0x01, 0x02, rx, move |b| {
        sink.borrow_mut().push(b as char);
    });
    m.attach_device(Box::new(terminal), &[0x01, 0x02]);
    load(&mut m, &image).expect("program loads into RAM");
    m.run();
    assert_eq!(*printed.borrow(), "Hello World!\n");
}

#[test]
fn test_program_demo_assembles_and_loads() {
    let src = include_str!("../programs/demo.e8085");
    let image = assemble(src).expect("demo.e8085 assembles cleanly");
    let mut m = Machine::create(16, 8);
    load(&mut m, &image).expect("demo.e8085 loads cleanly");
    assert!(image.bytes.len() > 0x40);
}

#[test]
fn test_binary_image_execution_matches_source() {
    let src = include_str!("../programs/array_sum.e8085");
    let image = assemble(src).expect("assembles cleanly");
    let raw_bytes = image.bytes.clone();

    // Execute raw binary bytes loaded into machine RAM directly
    let mut m = Machine::create(16, 8);
    for (i, &b) in raw_bytes.iter().enumerate() {
        m.ram.write(emu8085::Addr(i as u16), b);
    }
    m.cpu.regs.pc = emu8085::Addr(0x0000);
    m.run();

    assert_eq!(m.cpu.regs.a, 10);
}

#[test]
fn test_container_encoding_and_disassembly() {
    let src = include_str!("../programs/hello_world.e8085");
    let image = assemble(src).expect("assembles cleanly");
    let container = image.to_container();
    let container_bytes = container.encode();

    // Decode container
    let decoded = emu8085::asm::container::BinaryContainer::decode(&container_bytes)
        .expect("container decodes cleanly");
    assert_eq!(decoded.header.entry_pc, image.entry);
    assert_eq!(decoded.header.text_size, image.text_size);
    assert_eq!(decoded.header.data_size, image.data_size);

    // Disassemble container (strictly .text segment)
    let rows = emu8085::disassemble_bytes(&container_bytes)
        .expect("disassembles container cleanly");
    assert!(!rows.is_empty());

    let mnemonics: Vec<String> = rows.iter().map(|r| r.mnemonic.clone()).collect();
    assert!(mnemonics.iter().any(|m| m.contains("MVI A, 0x00")));
    assert!(mnemonics.iter().any(|m| m.contains("OUT 0x02")));
    assert!(mnemonics.iter().any(|m| m.contains("HLT")));
    // Must NOT contain vector table or data
    assert!(!mnemonics.iter().any(|m| m.contains("JMP 0x004D")));
    assert!(!mnemonics.iter().any(|m| m.contains("Hello World!")));
}

#[test]
fn test_disassembler_decodes_container_subroutine() {
    let src = include_str!("../programs/subroutine.e8085");
    let image = assemble(src).expect("assembles cleanly");
    let container_bytes = image.to_container().encode();

    let rows = emu8085::disassemble_bytes(&container_bytes)
        .expect("disassembles cleanly");
    assert!(!rows.is_empty());

    let mnemonics: Vec<String> = rows.iter().map(|r| r.mnemonic.clone()).collect();
    assert!(mnemonics.iter().any(|m| m.contains("LXI SP, 0xF000")));
    assert!(mnemonics.iter().any(|m| m.contains("MVI A, 0x05")));
    assert!(mnemonics.iter().any(|m| m.contains("MVI B, 0x03")));
    assert!(mnemonics.iter().any(|m| m.contains("ADD B")));
    assert!(mnemonics.iter().any(|m| m.contains("RET")));
    assert!(mnemonics.iter().any(|m| m.contains("HLT")));
}

#[test]
fn test_disassembler_rejects_non_container_bytes() {
    let raw_bytes = vec![0xC3, 0x00, 0x00, 0x76];
    assert!(emu8085::disassemble_bytes(&raw_bytes).is_err());
}
