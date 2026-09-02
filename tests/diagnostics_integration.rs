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

    assert!(printed.contains("<print>"), "disassembly contains <print> symbol");
    assert!(printed.contains("<input>"), "disassembly contains <input> symbol");
    assert!(printed.contains("<putch>"), "disassembly contains <putch> symbol");
    assert!(printed.contains("<endl>"), "disassembly contains <endl> symbol");
    assert!(printed.contains("<main>"), "disassembly contains <main> entry point");
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
    // Blue (\x1b[34m) for labels
    assert!(colored_output.contains("\x1b[34m<print>"), "should contain blue for <print> label");
    assert!(colored_output.contains("\x1b[34m<main>"), "should contain blue for <main> label");
    // White (\x1b[37m) for address and opcodes
    assert!(colored_output.contains("\x1b[37m"), "should contain white for address/opcodes");
}

