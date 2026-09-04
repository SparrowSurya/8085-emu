//! End-to-end assembler tests: assemble source text, load the image into a machine, run
//! it, and check the result — the full lex → parse → resolve → encode → execute path.

use emu8085::asm::{assemble, load};
use emu8085::{Machine, PrinterDevice};
use std::sync::{Arc, Mutex};

fn run_source(src: &str) -> Machine {
    let image = assemble(src).expect("assembles");
    let mut m = Machine::create(16, 8);
    load(&mut m, &image).expect("loads");
    m.run();
    m
}

#[test]
fn add_two_registers() {
    let m = run_source(
        "segment .text\n\
         main:\n\
         mvi A, 0x07\n\
         mvi B, 0x03\n\
         add B\n\
         hlt\n",
    );
    assert_eq!(m.cpu.regs.a, 0x0A);
    assert!(m.cpu.is_halt);
}

#[test]
fn multiply_by_repeated_addition() {
    // 7 * 6 = 42, via a dcr/jnz loop with a backward label.
    let m = run_source(
        "segment .text\n\
         main:\n\
         mvi B, 7\n\
         mvi C, 6\n\
         mvi A, 0\n\
         loop:\n\
         add B\n\
         dcr C\n\
         jnz loop\n\
         hlt\n",
    );
    assert_eq!(m.cpu.regs.a, 42);
}

#[test]
fn stack_roundtrip_with_lxi_sp() {
    // Exercises LXI SP (the added 0x31), PUSH/POP, and register moves.
    let m = run_source(
        "segment .text\n\
         main:\n\
         lxi SP, 0xF000\n\
         mvi B, 0xAB\n\
         mvi C, 0xCD\n\
         push BC\n\
         pop DE\n\
         hlt\n",
    );
    assert_eq!(m.cpu.regs.d, 0xAB);
    assert_eq!(m.cpu.regs.e, 0xCD);
    assert_eq!(m.cpu.regs.sp.0, 0xF000);
}

#[test]
fn print_a_data_string_through_a_port() {
    // Walk a NUL-terminated string in .data, sending each byte to a printer on port 2.
    // Exercises data layout, `lxi HL, symbol`, `M`, comparison, and branches.
    let src = "segment .data\n\
               msg BYTE \"Hi 8085\" 0x00\n\
               segment .text\n\
               main:\n\
               lxi HL, msg\n\
               next:\n\
               mov A, M\n\
               cpi 0x00\n\
               jz done\n\
               out 0x02\n\
               inx HL\n\
               jmp next\n\
               done:\n\
               hlt\n";
    let image = assemble(src).expect("assembles");

    let printed = Arc::new(Mutex::new(String::new()));
    let sink = printed.clone();
    let mut m = Machine::create(16, 8);
    m.attach_device(
        Box::new(PrinterDevice::with_callback(move |c| {
            sink.lock().unwrap().push(c)
        })),
        &[0x02],
    );
    load(&mut m, &image).expect("loads");
    m.run();

    assert_eq!(*printed.lock().unwrap(), "Hi 8085");
}
