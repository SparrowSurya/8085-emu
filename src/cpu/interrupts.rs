//! The 8085 interrupt controller: sampling, priority resolution, and vectoring.
//!
//! Sampled once per instruction (at fetch T1). Priority, highest first: TRAP
//! (non-maskable, always serviced), then RST 7.5 / 6.5 / 5.5 (each needs interrupts
//! enabled *and* its mask clear), then INTR (which does not vector directly but starts
//! an interrupt-acknowledge fetch — the external device supplies a restart opcode).
//!
//! A vectored interrupt pushes the return address over two T-states, mirroring the
//! reference's split (high byte, then low byte + jump). This preserves cycle timing and
//! lets memory latch each pushed byte through the normal `Memory::step`.

use crate::bus::SystemBus;
use crate::cpu::Cpu;
use crate::value::Addr;

/// Fixed vector addresses (also re-exported at the crate root).
pub const VEC_TRAP: Addr = Addr(0x0024);
pub const VEC_RST_5_5: Addr = Addr(0x002C);
pub const VEC_RST_6_5: Addr = Addr(0x0034);
pub const VEC_RST_7_5: Addr = Addr(0x003C);

impl Cpu {
    /// Sample hardware interrupts in priority order. Returns `true` if a fetch was
    /// pre-empted to begin servicing a vectored interrupt (TRAP/RST x.5). INTR returns
    /// `false` because it lets the (now interrupt-acknowledge) fetch proceed.
    pub(crate) fn check_hardware_interrupts(&mut self, bus: &mut SystemBus) -> bool {
        if self.trap {
            self.trap = false;
            self.trigger_vector(bus, VEC_TRAP);
            return true;
        }

        if !self.inte {
            return false;
        }

        if self.rst_7_5 && !self.mask_7_5 {
            self.rst_7_5 = false;
            self.pending_7_5 = false;
            self.trigger_vector(bus, VEC_RST_7_5);
            return true;
        }
        if self.rst_6_5 && !self.mask_6_5 {
            self.rst_6_5 = false;
            self.trigger_vector(bus, VEC_RST_6_5);
            return true;
        }
        if self.rst_5_5 && !self.mask_5_5 {
            self.rst_5_5 = false;
            self.trigger_vector(bus, VEC_RST_5_5);
            return true;
        }

        if self.intr {
            // INTR: acknowledge and let the next fetch run as an INTA cycle, during which
            // the device drives a restart opcode onto the bus.
            self.intr = false;
            self.inte = false;
            self.is_inta_cycle = true;
            return false;
        }

        false
    }

    /// Begin a vectored interrupt: disable further interrupts, push the PC high byte now,
    /// and stage the low-byte push + jump for the next T-state.
    pub(crate) fn trigger_vector(&mut self, bus: &mut SystemBus, vector: Addr) {
        self.inte = false;
        let pc = self.regs.pc;
        let sp1 = self.regs.sp.wrapping_sub(1);
        bus.set_address(sp1);
        bus.set_data(pc.high());
        bus.lines.mw = true;
        self.int_push = Some((sp1.wrapping_sub(1), pc.low(), vector));
    }

    /// Complete a staged interrupt push: write the PC low byte, update SP, and jump to
    /// the vector. Returns `true` when it fired (so the caller returns for this T-state).
    pub(crate) fn resume_interrupt_push(&mut self, bus: &mut SystemBus) -> bool {
        if let Some((sp_after, low, vector)) = self.int_push.take() {
            bus.set_address(sp_after);
            bus.set_data(low);
            bus.lines.mw = true;
            self.regs.sp = sp_after;
            self.regs.pc = vector;
            self.cycle = crate::cpu::MachineCycle::Fetch;
            self.t_state = 1;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;

    /// Run a machine (CPU + RAM) to HLT, returning A and the tick count.
    fn run(prog: &[u8], at: u16, sp: u16, isr: &[(u16, u8)], setup: impl FnOnce(&mut Cpu)) -> (Cpu, u64) {
        let mut cpu = Cpu::new();
        let mut mem = Memory::from_lines(16);
        let mut bus = SystemBus::default();
        mem.load_bytes(prog, Addr(at)).unwrap();
        for &(a, v) in isr {
            mem.write(Addr(a), v);
        }
        cpu.regs.sp = Addr(sp);
        cpu.start_at(Addr(at));
        setup(&mut cpu);
        let mut ticks = 0;
        while !cpu.is_halt && cpu.fault.is_none() && ticks < 100_000 {
            cpu.process(&mut bus);
            mem.step(&mut bus);
            ticks += 1;
        }
        (cpu, ticks)
    }

    // ISR at `vec`: MVI A, val ; RET
    fn isr_set(vec: u16, val: u8) -> Vec<(u16, u8)> {
        vec![(vec, 0x3E), (vec + 1, val), (vec + 2, 0xC9)]
    }

    #[test]
    fn trap_is_serviced_even_with_interrupts_disabled() {
        // NOP ; HLT at 0x00A0; TRAP pending. inte stays false — TRAP ignores it.
        let (cpu, _t) = run(&[0x00, 0x76], 0x00A0, 0x1000, &isr_set(0x0024, 0x88), |c| {
            c.trap = true;
        });
        assert_eq!(cpu.regs.a, 0x88);
        assert_eq!(cpu.regs.sp.0, 0x1000); // RET restored SP
    }

    #[test]
    fn rst_7_5_6_5_5_5_vector_when_enabled_and_unmasked() {
        let cases: [(u16, u8, fn(&mut Cpu)); 3] = [
            (0x003C, 0x75, |c: &mut Cpu| c.rst_7_5 = true),
            (0x0034, 0x65, |c: &mut Cpu| c.rst_6_5 = true),
            (0x002C, 0x55, |c: &mut Cpu| c.rst_5_5 = true),
        ];
        for (vec, val, set) in cases {
            let (cpu, _t) = run(&[0x00, 0x76], 0x00A0, 0x1000, &isr_set(vec, val), |c| {
                c.inte = true;
                set(c);
            });
            assert_eq!(cpu.regs.a, val, "vector {vec:#06X}");
            assert_eq!(cpu.regs.sp.0, 0x1000);
        }
    }

    #[test]
    fn masked_interrupt_is_ignored() {
        // RST 6.5 pending but masked -> ISR never runs, A stays 0.
        let (cpu, _t) = run(&[0x00, 0x76], 0x00A0, 0x1000, &isr_set(0x0034, 0x65), |c| {
            c.inte = true;
            c.rst_6_5 = true;
            c.mask_6_5 = true;
        });
        assert_eq!(cpu.regs.a, 0x00);
    }

    #[test]
    fn priority_services_trap_over_lower_sources() {
        // TRAP and RST 5.5 both pending: TRAP (0x88) wins.
        let mut isr = isr_set(0x0024, 0x88);
        isr.extend(isr_set(0x002C, 0x55));
        let (cpu, _t) = run(&[0x00, 0x76], 0x00A0, 0x1000, &isr, |c| {
            c.inte = true;
            c.trap = true;
            c.rst_5_5 = true;
        });
        assert_eq!(cpu.regs.a, 0x88);
    }

    #[test]
    fn sim_sets_masks_and_rim_reads_them() {
        // MVI A, 0b0000_1101 (MSE=1, mask 5.5 and 7.5) ; SIM ; RIM ; HLT
        // Then A reflects the mask bits back (5.5=1, 6.5=0, 7.5=1) plus INTE state.
        let (cpu, _t) = run(&[0x3E, 0b0000_1101, 0x30, 0x20, 0x76], 0x0000, 0x1000, &[], |_| {});
        assert!(cpu.mask_5_5);
        assert!(!cpu.mask_6_5);
        assert!(cpu.mask_7_5);
        // RIM low three bits mirror the masks.
        assert_eq!(cpu.regs.a & 0b0000_0111, 0b0000_0101);
    }
}
