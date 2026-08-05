//! Differential interrupt tests: TRAP / RST 7.5-6.5-5.5 vectoring, masking, priority,
//! and nested software RSTs, each checked against the reference implementation including ticks.
use emu8085::{Addr, Cpu, Memory, SystemBus};
include!("common/interrupt_cases.rs");

fn run(c: &ICase) -> (Cpu, u64) {
    let mut cpu = Cpu::new();
    let mut mem = Memory::from_lines(16);
    let mut bus = SystemBus::default();
    mem.load_bytes(c.prog, Addr(c.at)).unwrap();
    for &(a, v) in c.isr {
        mem.write(Addr(a), v);
    }
    cpu.regs.sp = Addr(c.sp);
    cpu.start_at(Addr(c.at));
    cpu.trap = c.trap;
    cpu.inte = c.inte;
    cpu.rst_7_5 = c.r75;
    cpu.rst_6_5 = c.r65;
    cpu.rst_5_5 = c.r55;
    cpu.mask_6_5 = c.m65;
    let mut t = 0u64;
    while !cpu.is_halt && cpu.fault.is_none() && t < 200_000 {
        cpu.process(&mut bus);
        mem.step(&mut bus);
        t += 1;
    }
    (cpu, t)
}

#[test]
fn interrupts_match_reference() {
    for case in ICASES {
        let (cpu, ticks) = run(case);
        let e = &case.exp;
        assert_eq!(
            (cpu.regs.a, cpu.regs.b, cpu.regs.c, cpu.regs.sp.0, ticks),
            (e.a, e.b, e.c, e.sp, e.ticks),
            "interrupt case `{}` diverged",
            case.name
        );
    }
}
