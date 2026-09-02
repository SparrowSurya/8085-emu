//! Integration tests for modular assembly: global/export, extern, local labels, %include,
//! and -l binary library linking.

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::channel;

use emu8085::asm::container::BinaryContainer;
use emu8085::asm::{assemble, assemble_and_link, assemble_with_options, load};
use emu8085::{Addr, Machine, TerminalDevice};

#[test]
fn test_local_labels_scoping_no_collision() {
    let src = r#"
segment .text

func_a:
    mvi B, 0x05
.loop:
    dcr B
    jnz .loop
    ret

func_b:
    mvi C, 0x0A
.loop:
    dcr C
    jnz .loop
    ret

main:
    call func_a
    call func_b
    hlt
"#;

    let image = assemble(src).expect("assembles cleanly without label collision");
    let mut machine = Machine::default();
    load(&mut machine, &image).expect("loads cleanly");
    machine.run();

    assert_eq!(machine.cpu.regs.get8(emu8085::Reg8::B), 0);
    assert_eq!(machine.cpu.regs.get8(emu8085::Reg8::C), 0);
    assert!(machine.cpu.is_halt);
}

#[test]
fn test_local_label_without_parent_is_rejected() {
    let src = r#"
segment .text
.orphan:
    nop
    hlt
"#;

    let err = assemble(src).expect_err("should reject local label without parent");
    assert!(err.to_string().contains("local label .orphan has no preceding parent label"));
}

#[test]
fn test_global_label_and_symbol_table_export() {
    let src = r#"
segment .text

global helper:
    mvi A, 0x42
    ret

export add_one:
    inr A
    ret

main:
    call helper
    call add_one
    hlt
"#;

    let image = assemble(src).expect("assembles cleanly");
    let container = image.to_container();

    assert_eq!(container.lookup_symbol("helper"), Some(0x0040));
    assert_eq!(container.lookup_symbol("add_one"), Some(0x0043));
    // 'main' should never be exported
    assert_eq!(container.lookup_symbol("main"), None);

    let container_bytes = container.encode();
    let decoded = BinaryContainer::decode(&container_bytes).expect("decodes container cleanly");
    assert_eq!(decoded.lookup_symbol("helper"), Some(0x0040));
    assert_eq!(decoded.lookup_symbol("add_one"), Some(0x0043));
    assert_eq!(decoded.lookup_symbol("main"), None);
}

#[test]
fn test_include_source_directive() {
    let src = r#"
%include "terminal.e8085"

segment .data
prompt "Hello from %include!", 0x0A

segment .text
main:
    lxi HL, prompt
    mvi B, %len prompt
    call print
    hlt
"#;
    let base_dir = Path::new("programs");

    let image = assemble_with_options(src, Some(base_dir), &HashMap::new())
        .expect("assembles cleanly with %include terminal.e8085");

    let mut machine = Machine::default();
    load(&mut machine, &image).expect("loads image cleanly");

    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let output_clone = output.clone();
    let (_tx, rx) = channel();
    let terminal = TerminalDevice::with_io(0x01, 0x02, rx, move |b| {
        output_clone.lock().unwrap().push(b);
    });

    machine.attach_device(Box::new(terminal), &[0x01, 0x02]);
    machine.run();

    assert_eq!(*output.lock().unwrap(), b"Hello from %include!\n");
    assert!(machine.cpu.is_halt);
}

#[test]
fn test_terminal_input_subroutine() {
    let src = r#"
%include "terminal.e8085"

segment .bss
buffer BYTE 32

segment .text
main:
    lxi HL, buffer
    mvi B, %len buffer
    call input

    ; Print back what was read
    lxi HL, buffer
    ; B already contains the length of the string from input
    call print
    hlt
"#;
    let base_dir = Path::new("programs");
    let image = assemble_with_options(src, Some(base_dir), &HashMap::new())
        .expect("assembles cleanly with %include terminal.e8085");

    let mut machine = Machine::default();
    load(&mut machine, &image).expect("loads image cleanly");

    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let output_clone = output.clone();
    let (_tx, rx) = channel();
    let mut terminal = TerminalDevice::with_io(0x01, 0x02, rx, move |b| {
        output_clone.lock().unwrap().push(b);
    });

    terminal.feed_line("Antigravity");
    machine.attach_device(Box::new(terminal), &[0x01, 0x02]);
    machine.run();

    assert_eq!(*output.lock().unwrap(), b"Antigravity");
    assert!(machine.cpu.is_halt);
}

#[test]
fn test_terminal_putch_subroutine() {
    let src = r#"
%include "terminal.e8085"

segment .text
main:
    mvi A, 'O'
    call putch
    mvi A, 'K'
    call putch
    call endl
    hlt
"#;
    let base_dir = Path::new("programs");
    let image = assemble_with_options(src, Some(base_dir), &HashMap::new())
        .expect("assembles cleanly with %include terminal.e8085");

    let mut machine = Machine::default();
    load(&mut machine, &image).expect("loads image cleanly");

    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let output_clone = output.clone();
    let (_tx, rx) = channel();
    let terminal = TerminalDevice::with_io(0x01, 0x02, rx, move |b| {
        output_clone.lock().unwrap().push(b);
    });

    machine.attach_device(Box::new(terminal), &[0x01, 0x02]);
    machine.run();

    assert_eq!(*output.lock().unwrap(), b"OK\n");
    assert!(machine.cpu.is_halt);
}

#[test]
fn test_extern_linking_with_precompiled_binary_library() {
    // 1. Assemble library (terminal helper) into a container
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal helper");
    let term_container = term_image.to_container();
    let term_bytes = term_container.encode();

    // Decode library container and extract export symbols
    let decoded_lib = BinaryContainer::decode(&term_bytes).expect("decodes lib container");
    let mut ext_symbols = HashMap::new();
    for (sym, addr) in &decoded_lib.export_symbols {
        ext_symbols.insert(sym.clone(), *addr);
    }
    assert_eq!(ext_symbols.get("print"), Some(&0x0040));

    // 2. Program that uses `extern print`
    let prog_src = r#"
segment .data
msg BYTE "Hi", 0x0A

segment .text

extern print

main:
    lxi HL, msg
    mvi B, %len msg
    call print
    hlt
"#;

    let prog_image = assemble_with_options(prog_src, None, &ext_symbols)
        .expect("assembles program with extern print");

    // 3. Load both into machine
    let mut machine = Machine::default();

    // Load library container
    for (i, &b) in decoded_lib.text_bytes.iter().enumerate() {
        machine.ram.write(Addr(decoded_lib.header.text_addr.wrapping_add(i as u16)), b);
    }

    // Load main program
    load(&mut machine, &prog_image).expect("loads main program");

    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let output_clone = output.clone();
    let (_tx, rx) = channel();
    let terminal = TerminalDevice::with_io(0x01, 0x02, rx, move |b| {
        output_clone.lock().unwrap().push(b);
    });

    machine.attach_device(Box::new(terminal), &[0x01, 0x02]);
    machine.run();

    assert_eq!(*output.lock().unwrap(), b"Hi\n");
    assert!(machine.cpu.is_halt);
}

#[test]
fn test_unresolved_extern_error() {
    let prog_src = r#"
segment .text

extern missing_function

main:
    call missing_function
    hlt
"#;

    let err = assemble_with_options(prog_src, None, &HashMap::new())
        .expect_err("should fail with undefined name / unresolved extern");
    assert!(err.to_string().contains("undefined name \"missing_function\""));
}

#[test]
fn test_static_standalone_binary_compilation_and_execution() {
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal helper");
    let term_container = term_image.to_container();
    assert_eq!(term_container.header.entry_pc, 0, "library without main has entry_pc == 0");

    let greet_src = include_str!("../programs/greet.e8085");
    // greet.e8085 uses extern print, input, endl
    let standalone_image = assemble_and_link(greet_src, None, &[term_container])
        .expect("statically links greet with terminal binary container");

    assert_ne!(standalone_image.entry, 0, "standalone executable has main entry point");

    let standalone_container = standalone_image.to_container();
    let encoded = standalone_container.encode();
    let decoded = BinaryContainer::decode(&encoded).expect("decodes standalone container");
    assert_ne!(decoded.header.entry_pc, 0);

    // Load ONLY the standalone binary into machine (no extra library loading required!)
    let mut machine = Machine::default();
    
    // Load vector table
    if !decoded.vec_bytes.is_empty() {
        for (i, &b) in decoded.vec_bytes.iter().enumerate() {
            machine.ram.write(Addr(i as u16), b);
        }
    }
    // Load .data
    for (i, &b) in decoded.data_bytes.iter().enumerate() {
        machine.ram.write(Addr(decoded.header.data_addr.wrapping_add(i as u16)), b);
    }
    // Zero .bss
    for i in 0..decoded.header.bss_size {
        machine.ram.write(Addr(decoded.header.bss_addr.wrapping_add(i)), 0);
    }
    // Load .text
    for (i, &b) in decoded.text_bytes.iter().enumerate() {
        machine.ram.write(Addr(decoded.header.text_addr.wrapping_add(i as u16)), b);
    }
    machine.cpu.regs.pc = Addr(decoded.header.entry_pc);
    machine.cpu.regs.sp = Addr(decoded.header.sp_init);

    let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let output_clone = output.clone();
    let (_tx, rx) = channel();
    let mut terminal = TerminalDevice::with_io(0x01, 0x02, rx, move |b| {
        output_clone.lock().unwrap().push(b);
    });

    terminal.feed_line("Surya");
    machine.attach_device(Box::new(terminal), &[0x01, 0x02]);
    machine.run();

    assert_eq!(*output.lock().unwrap(), b"What is your name? Hello, Surya\n");
    assert!(machine.cpu.is_halt);
}

#[test]
fn test_library_without_main_has_entry_pc_zero() {
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal helper");
    assert_eq!(term_image.entry, 0);

    let container = term_image.to_container();
    assert_eq!(container.header.entry_pc, 0);
    assert_eq!(container.export_symbols.len(), 4);
}


