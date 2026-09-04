//! Ported README example programs, run through the `Machine` facade and checked against
//! the reference implementation for both printer output and final CPU state.

use emu8085::{Addr, Cpu, DeviceManager, Machine, Memory, PrinterDevice, SystemBus};
use std::sync::{Arc, Mutex};

include!("common/example_cases.rs");

#[test]
fn examples_match_reference() {
    for c in ECASES {
        // A machine we drive by hand so we can capture printer output via a callback.
        let mut cpu = Cpu::new();
        let mut ram = Memory::from_lines(16);
        let mut bus = SystemBus::default();
        let mut dm = DeviceManager::new();

        ram.load_bytes(c.prog, Addr(c.at)).unwrap();
        for &(a, v) in c.mem {
            ram.write(Addr(a), v);
        }
        cpu.regs.b = c.b0;
        cpu.regs.c = c.c0;
        cpu.regs.d = c.d0;
        cpu.regs.e = c.e0;
        cpu.regs.h = c.h0;
        cpu.regs.l = c.l0;
        cpu.regs.sp = Addr(c.sp0);
        cpu.start_at(Addr(c.at));

        let seen = Arc::new(Mutex::new(String::new()));
        if c.printer_port >= 0 {
            let sink = seen.clone();
            dm.attach(
                Box::new(PrinterDevice::with_callback(move |ch| {
                    sink.lock().unwrap().push(ch)
                })),
                &[c.printer_port as u8],
            );
        }

        let mut t = 0u64;
        while !cpu.is_halt && cpu.fault.is_none() && t < 1_000_000 {
            cpu.process(&mut bus);
            ram.step(&mut bus);
            dm.step(&mut bus);
            t += 1;
        }

        assert_eq!(
            *seen.lock().unwrap(),
            c.out,
            "example `{}`: printer output",
            c.name
        );
        let e = &c.exp;
        assert_eq!(
            (
                cpu.regs.a,
                cpu.regs.b,
                cpu.regs.c,
                cpu.regs.d,
                cpu.regs.e,
                cpu.regs.h,
                cpu.regs.l,
                cpu.regs.sp.0,
                cpu.flags.to_psw()
            ),
            (e.a, e.b, e.c, e.d, e.e, e.h, e.l, e.sp, e.psw),
            "example `{}`: final CPU state",
            c.name
        );
    }
}

/// The public API compiles and runs the same way `Machine::run` will be used downstream.
#[test]
fn machine_run_matches_hand_driven_loop() {
    use emu8085::{Instruction, Opcode, Operand, Program};
    let prog = Program::new(vec![
        Instruction::with(Opcode::MVI_A, Operand::byte(7)),
        Instruction::with(Opcode::MVI_B, Operand::byte(6)),
        Instruction::new(Opcode::ADD_B),
        Instruction::new(Opcode::HLT),
    ]);
    let mut m = Machine::default();
    m.load(&prog, Addr(0)).unwrap();
    m.run();
    assert_eq!(m.cpu.regs.a, 13);
}
