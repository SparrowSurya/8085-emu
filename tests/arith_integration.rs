//! Differential integration tests for the arithmetic & logical category: run programs
//! captured from the reference implementation and require identical final state and T-state count.

use emu8085::{Addr, Cpu, Memory, SystemBus};

include!("common/arith_cases.rs");

/// Load `prog` (and any memory presets) at 0x0000, run to HLT, return the CPU/RAM/ticks.
fn run(prog: &[u8], mem_pre: &[(u16, u8)]) -> (Cpu, Memory, u64) {
    let mut cpu = Cpu::new();
    let mut mem = Memory::from_lines(16);
    let mut bus = SystemBus::default();
    mem.load_bytes(prog, Addr(0)).unwrap();
    for &(a, v) in mem_pre {
        mem.write(Addr(a), v);
    }
    cpu.start_at(Addr(0));
    let mut ticks = 0u64;
    while !cpu.is_halt && cpu.fault.is_none() && ticks < 100_000 {
        cpu.process(&mut bus);
        mem.step(&mut bus);
        ticks += 1;
    }
    (cpu, mem, ticks)
}

#[test]
fn matches_reference_on_all_cases() {
    for case in CASES {
        let (cpu, mem, ticks) = run(case.prog, case.mem);
        let e = &case.exp;
        let got = (
            cpu.regs.a, cpu.regs.b, cpu.regs.c, cpu.regs.d, cpu.regs.e, cpu.regs.h, cpu.regs.l,
            cpu.regs.sp.0, cpu.flags.to_psw(), ticks, mem.read(Addr(0x50)),
        );
        let exp = (e.a, e.b, e.c, e.d, e.e, e.h, e.l, e.sp, e.psw, e.ticks, e.m50);
        assert_eq!(got, exp, "case `{}` diverged from the reference", case.name);
    }
}
