//! The processor: its architectural + internal state and the one-T-state-at-a-time
//! execution loop.
//!
//! [`Cpu::process`] advances exactly one T-state per call, following the same priority
//! ladder as the Python `process`: hardware reset, then the HOLD/HLDA DMA handshake,
//! then READY wait-states, then (at fetch T1) interrupt sampling, then the current
//! machine cycle. The per-opcode work lives in [`execute`]; this file owns only the
//! *loop* and the shared state it walks.

pub mod alu;
pub(crate) mod alu_vectors;
pub mod execute;
pub mod interrupts;
pub mod flags;
pub mod registers;

use crate::bus::SystemBus;
use crate::error::EmuError;
use crate::value::Addr;
use flags::Flags;
use registers::{Reg16, Reg8, RegisterFile};

/// Which machine cycle the CPU is in. Fetch always spans 4 T-states (the M1 opcode
/// fetch); Execute spans a per-opcode number of T-states; Hold means the bus has been
/// surrendered to a DMA master.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineCycle {
    Fetch,
    Execute,
    Hold,
}

/// The 8085 core. Holds the register file and flags, the fetch/execute sequencing
/// state, and the interrupt/handshake pins. Wire it to a [`SystemBus`] and step it with
/// [`Cpu::process`].
#[derive(Debug, Clone, PartialEq)]
pub struct Cpu {
    /// Architectural + internal registers (A-L, W/Z, SP, PC).
    pub regs: RegisterFile,
    /// Condition flags.
    pub flags: Flags,

    /// Latched opcode byte currently being executed (the instruction register).
    pub(crate) ireg: u8,
    pub cycle: MachineCycle,
    pub t_state: u32,

    /// Set by `HLT` (and cleared by a serviceable interrupt or reset).
    pub is_halt: bool,
    /// Interrupt enable flip-flop (`EI`/`DI`).
    pub inte: bool,

    /// A decoded but not-yet-handled fatal condition (e.g. an undefined opcode). The
    /// loop halts and records it here rather than panicking.
    pub fault: Option<EmuError>,

    // --- interrupt masks / pending / pins (serviced in the Step 6 controller) ---
    pub mask_5_5: bool,
    pub mask_6_5: bool,
    pub mask_7_5: bool,
    pub pending_7_5: bool,
    pub trap: bool,
    pub rst_5_5: bool,
    pub rst_6_5: bool,
    pub rst_7_5: bool,
    pub intr: bool,
    pub(crate) is_inta_cycle: bool,

    // --- operand binding set at decode, consumed by execute micro-ops ---
    pub(crate) dst8: Option<Reg8>,
    pub(crate) src8: Option<Reg8>,
    pub(crate) ptr16: Option<Reg16>,
    pub(crate) work: Addr,
    /// 8-bit scratch latched mid-instruction (operand byte for RMW/MVI-M sequences).
    pub(crate) work8: u8,
    /// In-progress interrupt PC push: `(sp_after, pc_low, vector)`. The push spans two
    /// T-states (high byte, then low byte + jump); `Some` means the second half is due.
    pub(crate) int_push: Option<(Addr, u8, Addr)>,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu {
            regs: RegisterFile::new(),
            flags: Flags::new(),
            ireg: 0,
            cycle: MachineCycle::Fetch,
            t_state: 1,
            is_halt: true, // powers up halted; a load/run releases it (matches Python)
            inte: false,
            fault: None,
            mask_5_5: false,
            mask_6_5: false,
            mask_7_5: false,
            pending_7_5: false,
            trap: false,
            rst_5_5: false,
            rst_6_5: false,
            rst_7_5: false,
            intr: false,
            is_inta_cycle: false,
            dst8: None,
            src8: None,
            ptr16: None,
            work: Addr(0),
            work8: 0,
            int_push: None,
        }
    }
}

impl Cpu {
    /// A fresh, powered-on CPU (halted, interrupts disabled, SP = 0xFFFF).
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience for callers/tests that want to begin execution at `addr`.
    pub fn start_at(&mut self, addr: Addr) {
        self.regs.pc = addr;
        self.is_halt = false;
        self.cycle = MachineCycle::Fetch;
        self.t_state = 1;
    }

    /// Whether the CPU has surrendered the bus to a DMA master (in a HOLD cycle).
    pub fn in_hold(&self) -> bool {
        self.cycle == MachineCycle::Hold
    }

    /// Advance the CPU by one T-state, reading and driving `bus`.
    pub fn process(&mut self, bus: &mut SystemBus) {
        // 1. Hardware reset dominates everything.
        if bus.lines.reset_in {
            self.regs.pc = Addr(0x0000);
            self.inte = false;
            self.is_halt = false;
            self.cycle = MachineCycle::Fetch;
            self.t_state = 1;
            bus.lines.reset_out = true;
            return;
        }
        bus.lines.reset_out = false;

        // 2. DMA HOLD/HLDA handshake: surrender the bus while HOLD is asserted.
        if bus.lines.hold {
            bus.lines.hlda = true;
            self.cycle = MachineCycle::Hold;
            bus.lines.mr = false;
            bus.lines.mw = false;
            bus.lines.ior = false;
            bus.lines.iow = false;
            return;
        } else if self.cycle == MachineCycle::Hold {
            bus.lines.hlda = false;
            self.cycle = MachineCycle::Fetch;
            self.t_state = 1;
        }

        // 3. READY low inserts wait states.
        if !bus.lines.ready {
            return;
        }

        // 4. Interrupt push completion (Step 6) hooks in here.
        if self.resume_interrupt_push(bus) {
            return;
        }

        // 5. A halted CPU only wakes for TRAP or an enabled maskable interrupt.
        if self.is_halt
            && (self.trap
                || (self.inte && (self.rst_7_5 || self.rst_6_5 || self.rst_5_5 || self.intr)))
        {
            self.is_halt = false;
        }
        if self.is_halt {
            return;
        }

        // 6. Sample hardware interrupts at the start of a fetch.
        if self.cycle == MachineCycle::Fetch
            && self.t_state <= 1
            && self.check_hardware_interrupts(bus)
        {
            return;
        }

        // 7. Run the current machine cycle.
        match self.cycle {
            MachineCycle::Fetch => self.fetch(bus),
            MachineCycle::Execute => self.execute(bus),
            MachineCycle::Hold => {}
        }
    }

    /// The 4-T-state opcode fetch (M1). T1 drives the address (or `INTA` during an
    /// interrupt-acknowledge fetch), T2 strobes memory read, T3 latches the opcode, and
    /// T4 hands off to the decoder.
    fn fetch(&mut self, bus: &mut SystemBus) {
        match self.t_state {
            1 => {
                bus.lines.reset();
                if self.is_inta_cycle {
                    bus.lines.inta = true;
                } else {
                    bus.set_address(self.regs.pc);
                    self.regs.pc = self.regs.pc.wrapping_add(1);
                }
                self.t_state = 2;
            }
            2 => {
                if !self.is_inta_cycle {
                    bus.lines.mr = true;
                }
                self.t_state = 3;
            }
            3 => {
                self.ireg = bus.data();
                if self.is_inta_cycle {
                    bus.lines.inta = false;
                    self.is_inta_cycle = false;
                } else {
                    bus.lines.mr = false;
                }
                self.t_state = 4;
            }
            4 => self.decode(bus),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;

    /// Minimal stand-in for the (not-yet-built) Machine: tick CPU + RAM together until
    /// halt, returning the number of T-states consumed.
    fn run(program: &[u8]) -> (Cpu, Memory, u64) {
        let mut cpu = Cpu::new();
        let mut mem = Memory::from_lines(16);
        let mut bus = SystemBus::default();
        mem.load_bytes(program, Addr(0)).unwrap();
        cpu.start_at(Addr(0));

        let mut ticks = 0u64;
        while !cpu.is_halt && cpu.fault.is_none() && ticks < 10_000 {
            cpu.process(&mut bus);
            mem.step(&mut bus);
            ticks += 1;
        }
        (cpu, mem, ticks)
    }

    #[test]
    fn nop_then_hlt_takes_eight_tstates() {
        let (cpu, _m, ticks) = run(&[0x00, 0x76]);
        assert!(cpu.is_halt);
        assert_eq!(ticks, 8);
    }

    #[test]
    fn mvi_a_loads_immediate_in_seven_tstates() {
        // MVI A,0x05 (7T) then HLT (4T) = 11 T-states; A == 5.
        let (cpu, _m, ticks) = run(&[0x3E, 0x05, 0x76]);
        assert_eq!(cpu.regs.a, 0x05);
        assert_eq!(ticks, 11);
    }

    #[test]
    fn mov_copies_register_to_register() {
        // MVI B,0x2A ; MOV A,B ; HLT  -> A == 0x2A
        let (cpu, _m, _t) = run(&[0x06, 0x2A, 0x78, 0x76]);
        assert_eq!(cpu.regs.a, 0x2A);
        assert_eq!(cpu.regs.b, 0x2A);
    }

    #[test]
    fn reset_in_clears_pc_and_disables_interrupts() {
        let mut cpu = Cpu::new();
        cpu.regs.pc = Addr(0x1234);
        cpu.inte = true;
        let mut bus = SystemBus::default();
        bus.lines.reset_in = true;
        cpu.process(&mut bus);
        assert_eq!(cpu.regs.pc, Addr(0x0000));
        assert!(!cpu.inte);
        assert!(bus.lines.reset_out);
    }

    #[test]
    fn hold_asserts_hlda_and_parks_the_cpu() {
        let mut cpu = Cpu::new();
        cpu.start_at(Addr(0));
        let mut bus = SystemBus::default();
        bus.lines.hold = true;
        cpu.process(&mut bus);
        assert!(bus.lines.hlda);
        assert_eq!(cpu.cycle, MachineCycle::Hold);
        bus.lines.hold = false;
        cpu.process(&mut bus);
        assert!(!bus.lines.hlda);
    }

    #[test]
    fn ready_low_inserts_wait_states() {
        let mut cpu = Cpu::new();
        cpu.start_at(Addr(0x0100));
        let mut bus = SystemBus::default();
        bus.lines.ready = false;
        let pc_before = cpu.regs.pc;
        for _ in 0..5 {
            cpu.process(&mut bus);
        }
        assert_eq!(cpu.regs.pc, pc_before);
    }

    #[test]
    fn lxi_sp_loads_stack_pointer_immediate() {
        // LXI SP, 0x1234 (10T) ; HLT (4T) -> SP == 0x1234, 14 T-states total
        let (cpu, _m, ticks) = run(&[0x31, 0x34, 0x12, 0x76]);
        assert_eq!(cpu.regs.sp.0, 0x1234);
        assert_eq!(ticks, 14);
    }

    #[test]
    fn mov_aa_is_noop_in_four_tstates() {
        // MVI A, 0x5A (7T) ; MOV A, A (4T) ; HLT (4T) -> A unchanged, 15 T-states
        let (cpu, _m, ticks) = run(&[0x3E, 0x5A, 0x7F, 0x76]);
        assert_eq!(cpu.regs.a, 0x5A);
        assert_eq!(ticks, 15);
    }
}
