//! Integration tests for modular assembly: global/export, extern, local labels, %include,
//! and -l binary library linking.

use std::collections::HashMap;

use emu8085::asm::container::BinaryContainer;
use emu8085::asm::{assemble, assemble_and_link, assemble_with_options, load};
use emu8085::{Addr, Machine};

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
    assert!(
        err.to_string()
            .contains("local label .orphan has no preceding parent label")
    );
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
    let temp_dir = std::env::temp_dir().join("emu8085_test_include");
    std::fs::create_dir_all(&temp_dir).unwrap();
    std::fs::write(
        temp_dir.join("sub.e8085"),
        "segment .text\nadd_ten:\n    adi 10\n    ret\n",
    )
    .unwrap();

    let src = r#"
%include "sub.e8085"

segment .text
main:
    mvi A, 5
    call add_ten
    hlt
"#;

    let image = assemble_with_options(src, Some(&temp_dir), &HashMap::new())
        .expect("assembles cleanly with %include");

    let mut machine = Machine::default();
    load(&mut machine, &image).expect("loads image cleanly");
    machine.run();

    assert_eq!(machine.cpu.regs.a, 15);
    assert!(machine.cpu.is_halt);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_extern_linking_with_precompiled_binary_library() {
    let lib_src = r#"
segment .text
    global add_five
add_five:
    adi 5
    ret
"#;
    let lib_image = assemble(lib_src).expect("assembles library");
    let lib_container = lib_image.to_container();
    let lib_bytes = lib_container.encode();

    let decoded_lib = BinaryContainer::decode(&lib_bytes).expect("decodes lib container");
    let mut ext_symbols = HashMap::new();
    for (sym, addr) in &decoded_lib.export_symbols {
        ext_symbols.insert(sym.clone(), *addr);
    }
    assert_eq!(ext_symbols.get("add_five"), Some(&0x0040));

    let prog_src = r#"
segment .text
    extern add_five

main:
    mvi A, 10
    call add_five
    hlt
"#;

    let prog_image =
        assemble_with_options(prog_src, None, &ext_symbols).expect("assembles program with extern");

    let mut machine = Machine::default();
    for (i, &b) in decoded_lib.text_bytes.iter().enumerate() {
        machine
            .ram
            .write(Addr(decoded_lib.header.text_addr.wrapping_add(i as u16)), b);
    }
    load(&mut machine, &prog_image).expect("loads main program");
    machine.run();

    assert_eq!(machine.cpu.regs.a, 15);
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
    assert!(
        err.to_string()
            .contains("undefined name \"missing_function\"")
    );
}

#[test]
fn test_static_standalone_binary_compilation_and_execution() {
    let lib_src = r#"
segment .text
    global compute_answer
compute_answer:
    mvi A, 42
    ret
"#;
    let lib_image = assemble(lib_src).expect("assembles library");
    let lib_container = lib_image.to_container();
    assert_eq!(
        lib_container.header.entry_pc, 0,
        "library without main has entry_pc == 0"
    );

    let main_src = r#"
segment .text
    global main
    extern compute_answer

main:
    call compute_answer
    hlt
"#;

    let standalone_image = assemble_and_link(main_src, None, &[lib_container])
        .expect("statically links executable with library container");

    assert_ne!(
        standalone_image.entry, 0,
        "standalone executable has main entry point"
    );

    let standalone_container = standalone_image.to_container();
    let encoded = standalone_container.encode();
    let decoded = BinaryContainer::decode(&encoded).expect("decodes standalone container");
    assert_ne!(decoded.header.entry_pc, 0);

    let mut machine = Machine::default();
    for (i, &b) in decoded.text_bytes.iter().enumerate() {
        machine
            .ram
            .write(Addr(decoded.header.text_addr.wrapping_add(i as u16)), b);
    }
    machine.cpu.regs.pc = Addr(decoded.header.entry_pc);
    machine.cpu.regs.sp = Addr(decoded.header.sp_init);
    machine.run();

    assert_eq!(machine.cpu.regs.a, 42);
    assert!(machine.cpu.is_halt);
}

#[test]
fn test_library_without_main_has_entry_pc_zero() {
    let lib_src = r#"
segment .text
    global func_a
    global func_b
func_a:
    ret
func_b:
    ret
"#;
    let lib_image = assemble(lib_src).expect("assembles library");
    assert_eq!(lib_image.entry, 0);

    let container = lib_image.to_container();
    assert_eq!(container.header.entry_pc, 0);
    assert_eq!(container.export_symbols.len(), 2);
}
