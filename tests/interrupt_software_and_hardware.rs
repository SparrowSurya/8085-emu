//! Integration tests for hardware interrupts (TRAP on illegal opcode and illegal memory access)
//! and software interrupts (RST 1..7 with custom ISR subroutines).

use emu8085::asm::{assemble, load};
use emu8085::{Addr, Machine};

fn run_source(src: &str) -> Machine {
    let image = assemble(src).expect("assembles cleanly");
    let mut m = Machine::create(16, 8);
    load(&mut m, &image).expect("loads cleanly");
    m.run();
    m
}

#[test]
fn software_interrupt_rst1_and_rst2_execute_custom_isrs() {
    let src = r#"
segment .data
  counter BYTE 0x00

segment .text
main:
  lxi SP, 0xF000
  rst 1
  rst 2
  lda counter
  hlt

isr_rst1:
  inr A
  sta counter
  ret

isr_rst2:
  lda counter
  adi 5
  sta counter
  ret
"#;
    let m = run_source(src);
    assert_eq!(m.cpu.regs.a, 6); // 0 + 1 (RST 1) + 5 (RST 2) = 6
    assert!(m.cpu.is_halt);
}

#[test]
fn software_interrupt_rst7_preserves_registers_and_stack() {
    let src = r#"
segment .text
main:
  lxi SP, 0xE000
  mvi B, 0x10
  mvi C, 0x20
  rst 7
  mov A, B
  add C
  hlt

isr_rst7:
  inr B
  inr C
  ret
"#;
    let m = run_source(src);
    assert_eq!(m.cpu.regs.b, 0x11);
    assert_eq!(m.cpu.regs.c, 0x21);
    assert_eq!(m.cpu.regs.a, 0x32); // 0x11 + 0x21 = 0x32
    assert_eq!(m.cpu.regs.sp.0, 0xE000);
}

#[test]
fn hardware_interrupt_trap_handles_illegal_opcode() {
    // Inject undefined opcode 0x08 into instruction stream.
    // Fetching 0x08 triggers hardware TRAP, which vectors to 0x0024 (isr_trap).
    let src = r#"
segment .text
main:
  lxi SP, 0xF000
  mvi B, 0xAA
  nop
  mvi B, 0x55
  hlt

isr_trap:
  mvi A, 0x99
  hlt
"#;
    let image = assemble(src).expect("assembles cleanly");
    let mut m = Machine::create(16, 8);
    load(&mut m, &image).expect("loads cleanly");
    // Replace the 'nop' instruction (at main + 5 = 0x0045) with illegal opcode 0x08
    let nop_addr = Addr(image.entry + 5);
    m.ram.write(nop_addr, 0x08);
    m.run();

    assert_eq!(m.cpu.regs.b, 0xAA); // Execution redirected before mvi B, 0x55
    assert_eq!(m.cpu.regs.a, 0x99); // isr_trap ran successfully
    assert!(m.cpu.is_halt);
}

#[test]
fn hardware_interrupt_trap_handles_illegal_memory_access() {
    // When memory is restricted with a valid limit, accessing past the limit triggers TRAP.
    let src = r#"
segment .text
main:
  lxi SP, 0x0F00
  ; Read from memory address 0x2000 (which will be configured beyond the limit)
  lda 0x2000
  hlt

isr_trap:
  mvi A, 0xEE
  hlt
"#;
    let image = assemble(src).expect("assembles cleanly");
    let mut m = Machine::create(16, 8);
    // Set valid memory limit to 0x1000 (4KB). Address 0x2000 is illegal.
    m.ram.set_limit(0x1000);
    load(&mut m, &image).expect("loads cleanly");
    m.run();

    assert_eq!(m.cpu.regs.a, 0xEE); // isr_trap ran on memory fault
    assert!(m.cpu.is_halt);
}

#[test]
fn hardware_interrupt_rst55_executes_isr() {
    let src = r#"
segment .text
main:
  lxi SP, 0xF000
  ei
  nop
  nop
  hlt

isr_rst55:
  mvi A, 0x55
  ret
"#;
    let image = assemble(src).expect("assembles cleanly");
    let mut m = Machine::create(16, 8);
    load(&mut m, &image).expect("loads cleanly");
    // Trigger RST 5.5 hardware interrupt line
    m.cpu.rst_5_5 = true;
    m.run();

    assert_eq!(m.cpu.regs.a, 0x55);
    assert!(m.cpu.is_halt);
}
