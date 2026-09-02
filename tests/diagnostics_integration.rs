//! Integration tests for binary diagnostics, inspection tools, string extraction,
//! and multi-symbol disassembly annotations.

use emu8085::asm::container::BinaryContainer;
use emu8085::asm::{assemble, assemble_and_link, extract_strings, get_segments, inspect_container, InspectOptions};
use emu8085::disassemble_container;

const TEST_LIB_SRC: &str = r#"
segment .text
    global foo
    global bar

foo:
    mvi A, 0x05
.loop:
    dcr A
    jnz .loop
    ret

bar:
    mvi A, 0x0A
    out 0x01
    ret
"#;

const TEST_PROG_SRC: &str = r#"
segment .data
    msg "Hello Diagnostics"

segment .text
    global main
    extern foo
    extern bar

main:
    lxi HL, msg
    call foo
    call bar
    hlt
"#;

#[test]
fn test_inspect_header_and_segments_on_library() {
    let lib_image = assemble(TEST_LIB_SRC).expect("assembles library");
    let lib_container = lib_image.to_container();
    let encoded = lib_container.encode();
    let decoded = BinaryContainer::decode(&encoded).expect("decodes container");

    let segments = get_segments(&decoded);
    assert!(segments.iter().any(|s| s.name == ".header"));
    assert!(segments.iter().any(|s| s.name == ".text"));
    assert!(segments.iter().any(|s| s.name == ".symtab"));

    let opts = InspectOptions {
        show_header: true,
        show_segments: true,
        show_symbols: true,
        show_strings: false,
        min_string_len: 3,
    };

    let report = inspect_container(&decoded, encoded.len(), &opts);
    assert!(report.contains("CONTAINER HEADER"));
    assert!(report.contains("SEGMENT TABLE"));
    assert!(report.contains("SYMBOL TABLE & ENTRY"));
    assert!(report.contains("Pure Subroutine Library"));
    assert!(report.contains("foo"));
    assert!(report.contains("bar"));
}

#[test]
fn test_inspect_strings_extraction() {
    let lib_image = assemble(TEST_LIB_SRC).expect("assembles library");
    let lib_container = lib_image.to_container();

    let standalone_image = assemble_and_link(TEST_PROG_SRC, None, &[lib_container])
        .expect("statically links executable");

    let container = standalone_image.to_container();
    let extracted = extract_strings(&container, 4);

    assert!(
        extracted.iter().any(|s| s.content.contains("Hello Diagnostics")),
        "should find greeting string"
    );
}

#[test]
fn test_disassemble_multi_symbol_annotations() {
    let lib_image = assemble(TEST_LIB_SRC).expect("assembles library");
    let lib_container = lib_image.to_container();

    let standalone_image = assemble_and_link(TEST_PROG_SRC, None, &[lib_container])
        .expect("statically links executable");

    let container = standalone_image.to_container();
    let rows = disassemble_container(&container);

    let printed = rows
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    // Subroutine banners
    assert!(printed.contains("Subroutine: foo"), "has foo subroutine banner");
    assert!(printed.contains("Function: main"), "has main function banner");

    // Symbolic call targets
    assert!(printed.contains("CALL foo"), "replaces address with CALL foo");
    assert!(printed.contains("CALL bar"), "replaces address with CALL bar");

    // String preview comment
    assert!(printed.contains("Hello Diagnostics"), "displays string literal preview");

    // Internal loop label
    assert!(printed.contains("loc_0042"), "generates internal loop label for JNZ target");
}

#[test]
fn test_inspect_options_priority_and_selective_filters() {
    let lib_image = assemble(TEST_LIB_SRC).expect("assembles library");
    let container = lib_image.to_container();

    // 1. Header only
    let header_only = InspectOptions {
        show_header: true,
        show_segments: false,
        show_symbols: false,
        show_strings: false,
        min_string_len: 3,
    };
    let report1 = inspect_container(&container, 100, &header_only);
    assert!(report1.contains("CONTAINER HEADER"));
    assert!(!report1.contains("SEGMENT TABLE"));
    assert!(!report1.contains("SYMBOL TABLE"));

    // 2. Symbols only
    let symbols_only = InspectOptions {
        show_header: false,
        show_segments: false,
        show_symbols: true,
        show_strings: false,
        min_string_len: 3,
    };
    let report2 = inspect_container(&container, 100, &symbols_only);
    assert!(!report2.contains("CONTAINER HEADER"));
    assert!(report2.contains("SYMBOL TABLE & ENTRY"));
    assert!(report2.contains("foo"));
}

#[test]
fn test_disassemble_colored_output() {
    let lib_image = assemble(TEST_LIB_SRC).expect("assembles library");
    let lib_container = lib_image.to_container();

    let standalone_image = assemble_and_link(TEST_PROG_SRC, None, &[lib_container])
        .expect("statically links executable");

    let container = standalone_image.to_container();
    let rows = disassemble_container(&container);
    assert!(!rows.is_empty());

    let colored_output = rows
        .iter()
        .map(|r| r.to_colored_string())
        .collect::<Vec<_>>()
        .join("\n");

    // Cyan (\x1b[36m) for instructions
    assert!(colored_output.contains("\x1b[36m"), "should contain cyan for instructions");
    // Magenta (\x1b[35m) for registers
    assert!(colored_output.contains("\x1b[35m"), "should contain magenta for registers");
    // Yellow (\x1b[33m) for numbers
    assert!(colored_output.contains("\x1b[33m"), "should contain yellow for numbers");
    // Blue (\x1b[34m) for labels and symbols
    assert!(colored_output.contains("\x1b[34mfoo\x1b[0m"), "should contain blue for foo symbol");
    // White (\x1b[37m) for address and opcodes
    assert!(colored_output.contains("\x1b[37m"), "should contain white for address/opcodes");
}

#[test]
fn test_disassemble_cycles_and_vectors_options() {
    use emu8085::DisassembleOptions;
    let lib_image = assemble(TEST_LIB_SRC).expect("assembles library");
    let lib_container = lib_image.to_container();

    let standalone_image = assemble_and_link(TEST_PROG_SRC, None, &[lib_container])
        .expect("statically links executable");

    let container = standalone_image.to_container();
    let opts = DisassembleOptions {
        color: false,
        show_cycles: true,
        show_vectors: true,
        show_banners: true,
    };
    let rows = emu8085::disassemble_container_with_options(&container, &opts);

    let text = rows
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    // Vector Table
    assert!(text.contains("Section: .vec"), "contains vector table banner");
    assert!(text.contains("RST 0 / Reset Vector"), "annotates reset vector");

    // T-State cycle counts
    assert!(text.contains("[18 T]"), "shows 18 T for CALL");
    assert!(text.contains("[10 T]"), "shows 10 T for OUT / LXI");
    assert!(text.contains("[4 T]"), "shows 4 T for DCR / NOP");
}



