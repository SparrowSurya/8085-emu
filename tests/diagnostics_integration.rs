//! Integration tests for binary diagnostics, inspection tools, string extraction,
//! and multi-symbol disassembly annotations.

use emu8085::asm::container::BinaryContainer;
use emu8085::asm::{assemble, assemble_and_link, extract_strings, get_segments, inspect_container, InspectOptions};
use emu8085::disassemble_container;

#[test]
fn test_inspect_header_and_segments_on_library() {
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal library");
    let term_container = term_image.to_container();
    let encoded = term_container.encode();
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
    assert!(report.contains("print"));
    assert!(report.contains("input"));
    assert!(report.contains("putch"));
    assert!(report.contains("endl"));
}

#[test]
fn test_inspect_strings_extraction() {
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal helper");
    let term_container = term_image.to_container();

    let greet_src = include_str!("../programs/greet.e8085");
    let standalone_image = assemble_and_link(greet_src, None, &[term_container])
        .expect("statically links greet executable");

    let container = standalone_image.to_container();
    let extracted = extract_strings(&container, 4);

    assert!(
        extracted.iter().any(|s| s.content.contains("What is your name?")),
        "should find prompt string"
    );
    assert!(
        extracted.iter().any(|s| s.content.contains("Hello, ")),
        "should find greeting string"
    );
}

#[test]
fn test_disassemble_multi_symbol_annotations() {
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal helper");
    let term_container = term_image.to_container();

    let greet_src = include_str!("../programs/greet.e8085");
    let standalone_image = assemble_and_link(greet_src, None, &[term_container])
        .expect("statically links greet executable");

    let container = standalone_image.to_container();
    let rows = disassemble_container(&container);

    let printed = rows
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    // Subroutine banners
    assert!(printed.contains("Subroutine: print"), "has print subroutine banner");
    assert!(printed.contains("Function: main"), "has main function banner");

    // Symbolic call targets
    assert!(printed.contains("CALL print"), "replaces 0x0040 with CALL print");
    assert!(printed.contains("CALL input"), "replaces 0x0054 with CALL input");
    assert!(printed.contains("CALL endl"), "replaces 0x0082 with CALL endl");

    // String preview comment
    assert!(printed.contains("What is your name?"), "displays string literal preview");

    // Internal loop label
    assert!(printed.contains("loc_0047"), "generates internal loop label for JNZ target");
}

#[test]
fn test_inspect_options_priority_and_selective_filters() {
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal helper");
    let container = term_image.to_container();

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
    assert!(report2.contains("print"));
}

#[test]
fn test_disassemble_colored_output() {
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal helper");
    let term_container = term_image.to_container();

    let greet_src = include_str!("../programs/greet.e8085");
    let standalone_image = assemble_and_link(greet_src, None, &[term_container])
        .expect("statically links greet executable");

    let container = standalone_image.to_container();
    let rows = disassemble_container(&container);
    assert!(!rows.is_empty());

    let colored_output = rows
        .iter()
        .map(|r| r.to_colored_string())
        .collect::<Vec<_>>()
        .join("\n");

    // Check ANSI codes:
    // Cyan (\x1b[36m) for instructions
    assert!(colored_output.contains("\x1b[36m"), "should contain cyan for instructions");
    // Magenta (\x1b[35m) for registers
    assert!(colored_output.contains("\x1b[35m"), "should contain magenta for registers");
    // Yellow (\x1b[33m) for numbers
    assert!(colored_output.contains("\x1b[33m"), "should contain yellow for numbers");
    // Blue (\x1b[34m) for labels and symbols
    assert!(colored_output.contains("\x1b[34mprint\x1b[0m"), "should contain blue for print symbol");
    // White (\x1b[37m) for address and opcodes
    assert!(colored_output.contains("\x1b[37m"), "should contain white for address/opcodes");
}

#[test]
fn test_disassemble_cycles_and_vectors_options() {
    use emu8085::DisassembleOptions;
    let term_src = include_str!("../programs/terminal.e8085");
    let term_image = assemble(term_src).expect("assembles terminal helper");
    let term_container = term_image.to_container();

    let greet_src = include_str!("../programs/greet.e8085");
    let standalone_image = assemble_and_link(greet_src, None, &[term_container])
        .expect("statically links greet executable");

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
    assert!(text.contains("[4 T]"), "shows 4 T for MOV / INR / DCR");
}


