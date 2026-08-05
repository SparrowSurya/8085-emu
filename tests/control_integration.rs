//! Differential tests for stack, branch, subroutine, and direct-addressing instructions.
//! Each program is run to HLT on both emulators; final registers, SP, PC, PSW, probed
//! memory, and the T-state count must match the reference implementation exactly.

use emu8085::{Addr, Cpu, Memory, SystemBus};

include!("common/control_cases.rs");

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
    while !cpu.is_halt && cpu.fault.is_none() && ticks < 500_000 {
        cpu.process(&mut bus);
        mem.step(&mut bus);
        ticks += 1;
    }
    (cpu, mem, ticks)
}

#[test]
fn matches_reference_on_all_control_cases() {
    for case in CASES {
        let (cpu, mem, ticks) = run(case.prog, case.mem);
        assert!(cpu.fault.is_none(), "case `{}` faulted: {:?}", case.name, cpu.fault);
        let e = &case.exp;

        let regs = (
            cpu.regs.a, cpu.regs.b, cpu.regs.c, cpu.regs.d, cpu.regs.e, cpu.regs.h, cpu.regs.l,
        );
        assert_eq!(
            regs,
            (e.a, e.b, e.c, e.d, e.e, e.h, e.l),
            "case `{}`: register mismatch",
            case.name
        );
        assert_eq!(cpu.regs.sp.0, e.sp, "case `{}`: SP mismatch", case.name);
        assert_eq!(cpu.regs.pc.0, e.pc, "case `{}`: PC mismatch", case.name);
        assert_eq!(cpu.flags.to_psw(), e.psw, "case `{}`: PSW mismatch", case.name);
        assert_eq!(ticks, e.ticks, "case `{}`: T-state count mismatch", case.name);

        for (probe, &expected) in PROBES.iter().zip(e.probes.iter()) {
            assert_eq!(
                mem.read(Addr(*probe)),
                expected,
                "case `{}`: memory[{:#06X}] mismatch",
                case.name,
                probe
            );
        }
    }
}
