//! Per-opcode execution: decode at fetch T4 and the execute-phase micro-operations.
//!
//! Single-machine-cycle ops act immediately at T4; multi-cycle ops bind operands and
//! run a fixed number of execute T-states from `EXEC_TSTATES` (extracted from the
//! reference, so timing is identical). Conditional CALL/RET end early via
//! `Micro::Done`, reproducing the reference's shorter not-taken timing. Flag effects
//! come from the pure `alu` functions.
//!
//! This now covers the full documented 8085 set: data transfer, arithmetic/logical,
//! stack/subroutine, branching, and machine/I-O control. (IN/OUT exchange bytes with
//! the device manager, wired up in the Machine step.)

use super::{alu, Cpu, MachineCycle};
use crate::bus::SystemBus;
use crate::cpu::registers::{Reg16, Reg8};
use crate::instruction::opcode::Opcode;
use crate::value::Addr;

/// Execute-phase T-state count per opcode byte (0 = single-cycle, effect at
/// fetch T4). Total instruction time is 4 (fetch M1) + this. Extracted verbatim
/// from the reference dispatch table so timing is bit-for-bit identical.
pub(crate) const EXEC_TSTATES: [u8; 256] = [
     0,  6,  3,  2,  0,  0,  3,  0,  0,  6,  3,  2,  0,  0,  3,  0, // 0X00
     0,  6,  3,  2,  0,  0,  3,  0,  0,  6,  3,  2,  0,  0,  3,  0, // 0X10
     0,  6, 12,  2,  0,  0,  3,  0,  0,  6, 12,  2,  0,  0,  3,  0, // 0X20
     0,  6,  9,  2,  6,  6,  6,  0,  0,  6,  9,  2,  0,  0,  3,  0, // 0X30
     0,  0,  0,  0,  0,  0,  3,  0,  0,  0,  0,  0,  0,  0,  3,  0, // 0X40
     0,  0,  0,  0,  0,  0,  3,  0,  0,  0,  0,  0,  0,  0,  3,  0, // 0X50
     0,  0,  0,  0,  0,  0,  3,  0,  0,  0,  0,  0,  0,  0,  3,  0, // 0X60
     3,  3,  3,  3,  3,  3,  0,  3,  0,  0,  0,  0,  0,  0,  3,  0, // 0X70
     0,  0,  0,  0,  0,  0,  3,  0,  0,  0,  0,  0,  0,  0,  3,  0, // 0X80
     0,  0,  0,  0,  0,  0,  3,  0,  0,  0,  0,  0,  0,  0,  3,  0, // 0X90
     0,  0,  0,  0,  0,  0,  3,  0,  0,  0,  0,  0,  0,  0,  3,  0, // 0XA0
     0,  0,  0,  0,  0,  0,  3,  0,  0,  0,  0,  0,  0,  0,  3,  0, // 0XB0
     8,  6,  6,  6, 14,  8,  3,  9,  8,  6,  6,  0, 14, 14,  3,  9, // 0XC0
     8,  6,  6,  6, 14,  8,  3,  9,  8,  0,  6,  6, 14,  0,  3,  9, // 0XD0
     8,  6,  6, 12, 14,  8,  3,  9,  8,  2,  6,  0, 14,  0,  3,  9, // 0XE0
     8,  6,  6,  0, 14,  8,  3,  9,  8,  2,  6,  0, 14,  0,  3,  9, // 0XF0
];


/// Result of running one execute-phase micro-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Micro {
    Continue,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecodeOutcome {
    Done,
    Execute,
}

/// The eight accumulator operations shared by the 0x80..0xBF block and the immediates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Aop {
    Add,
    Adc,
    Sub,
    Sbb,
    And,
    Or,
    Xor,
    Cmp,
}

impl Cpu {
    /// Execute-phase T-state count for the latched opcode.
    #[inline]
    pub(crate) fn exec_len(&self) -> u32 {
        EXEC_TSTATES[self.ireg as usize] as u32
    }

    /// Decode at fetch T4: complete single-cycle ops, or set up the execute phase.
    pub(crate) fn decode(&mut self, _bus: &mut SystemBus) {
        let opcode = match Opcode::from_byte(self.ireg) {
            Ok(op) => op,
            Err(e) => {
                self.fault = Some(e);
                self.is_halt = true;
                return;
            }
        };
        if opcode == Opcode::HLT {
            self.is_halt = true;
            self.t_state = 0;
            return;
        }
        self.dst8 = None;
        self.src8 = None;
        self.ptr16 = None;

        match self.decode_op(opcode) {
            DecodeOutcome::Done => {
                self.cycle = MachineCycle::Fetch;
                self.t_state = 1;
            }
            DecodeOutcome::Execute => {
                self.cycle = MachineCycle::Execute;
                self.t_state = 1;
            }
        }
    }

    /// Per-opcode decode. Single-cycle effects happen here; multi-cycle ops bind their
    /// operands and defer to `execute_micro`.
    fn decode_op(&mut self, opcode: Opcode) -> DecodeOutcome {
        let b = self.ireg;

        if opcode == Opcode::NOP {
            return DecodeOutcome::Done;
        }

        // MOV r1,r2 (register-register, 4T)
        if is_mov_rr(b) {
            let (dst, src) = (reg_from_code((b >> 3) & 7), reg_from_code(b & 7));
            self.regs.set8(dst, self.regs.get8(src));
            return DecodeOutcome::Done;
        }

        // Accumulator ops, register source (4T)
        if (0x80..=0xBF).contains(&b) && (b & 7) != 6 {
            let kind = aop_group(b & 0xF8);
            let operand = self.regs.get8(reg_from_code(b & 7));
            self.apply_aop(kind, operand);
            return DecodeOutcome::Done;
        }

        // INR r / DCR r (register, 4T)
        if is_inr_dcr(b) && ((b >> 3) & 7) != 6 {
            let reg = reg_from_code((b >> 3) & 7);
            let v = self.regs.get8(reg);
            let res = if b & 1 == 0 {
                alu::inr(&mut self.flags, v)
            } else {
                alu::dcr(&mut self.flags, v)
            };
            self.regs.set8(reg, res);
            return DecodeOutcome::Done;
        }

        // Rotates / accumulator & carry housekeeping / interrupt & exchange (4T, decode-time)
        match opcode {
            Opcode::RLC => return self.done_a(alu::rlc as fn(&mut _, u8) -> u8),
            Opcode::RRC => return self.done_a(alu::rrc),
            Opcode::RAL => return self.done_a(alu::ral),
            Opcode::RAR => return self.done_a(alu::rar),
            Opcode::CMA => {
                self.regs.a = alu::cma(self.regs.a);
                return DecodeOutcome::Done;
            }
            Opcode::DAA => {
                self.regs.a = alu::daa(&mut self.flags, self.regs.a);
                return DecodeOutcome::Done;
            }
            Opcode::STC => {
                self.flags.carry = true;
                return DecodeOutcome::Done;
            }
            Opcode::CMC => {
                self.flags.carry = !self.flags.carry;
                return DecodeOutcome::Done;
            }
            Opcode::XCHG => {
                self.exec_xchg();
                return DecodeOutcome::Done;
            }
            Opcode::EI => {
                self.inte = true;
                return DecodeOutcome::Done;
            }
            Opcode::DI => {
                self.inte = false;
                return DecodeOutcome::Done;
            }
            Opcode::RIM => {
                self.exec_rim();
                return DecodeOutcome::Done;
            }
            Opcode::SIM => {
                self.exec_sim();
                return DecodeOutcome::Done;
            }
            _ => {}
        }

        // Multi-cycle: bind operands the execute phase needs.
        if is_mvi_r(b) {
            self.dst8 = Some(reg_from_code((b >> 3) & 7));
        } else if matches!(b, 0x01 | 0x11 | 0x21 | 0x31) {
            self.ptr16 = Some(pair_from_code((b >> 4) & 3)); // LXI
        } else if matches!(b, 0x03 | 0x13 | 0x23 | 0x33 | 0x0B | 0x1B | 0x2B | 0x3B | 0x09 | 0x19 | 0x29 | 0x39)
        {
            self.ptr16 = Some(pair_from_code((b >> 4) & 3)); // INX/DCX/DAD
        } else if matches!(b, 0x0A | 0x1A) {
            self.ptr16 = Some(pair_from_code((b >> 4) & 3)); // LDAX
        } else if matches!(b, 0x02 | 0x12) {
            self.ptr16 = Some(pair_from_code((b >> 4) & 3)); // STAX
        } else if is_mov_from_m(b) {
            self.dst8 = Some(reg_from_code((b >> 3) & 7));
        } else if is_mov_to_m(b) {
            self.src8 = Some(reg_from_code(b & 7));
        } else if is_push(b) || is_pop(b) {
            if (b & 0x30) != 0x30 {
                self.ptr16 = Some(pair_from_code((b >> 4) & 3));
            }
        }

        if self.exec_len() == 0 {
            DecodeOutcome::Done
        } else {
            DecodeOutcome::Execute
        }
    }

    fn done_a(&mut self, f: fn(&mut super::flags::Flags, u8) -> u8) -> DecodeOutcome {
        self.regs.a = f(&mut self.flags, self.regs.a);
        DecodeOutcome::Done
    }

    fn apply_aop(&mut self, kind: Aop, operand: u8) {
        let a = self.regs.a;
        let carry = self.flags.carry;
        let res = match kind {
            Aop::Add => alu::add(&mut self.flags, a, operand, false),
            Aop::Adc => alu::add(&mut self.flags, a, operand, carry),
            Aop::Sub => alu::sub(&mut self.flags, a, operand, false),
            Aop::Sbb => alu::sub(&mut self.flags, a, operand, carry),
            Aop::And => alu::and(&mut self.flags, a, operand),
            Aop::Or => alu::or(&mut self.flags, a, operand),
            Aop::Xor => alu::xor(&mut self.flags, a, operand),
            Aop::Cmp => {
                alu::cmp(&mut self.flags, a, operand);
                return;
            }
        };
        self.regs.a = res;
    }

    fn exec_xchg(&mut self) {
        std::mem::swap(&mut self.regs.d, &mut self.regs.h);
        std::mem::swap(&mut self.regs.e, &mut self.regs.l);
    }

    fn exec_rim(&mut self) {
        let mut v = 0u8;
        v |= (self.mask_5_5 as u8) << 0;
        v |= (self.mask_6_5 as u8) << 1;
        v |= (self.mask_7_5 as u8) << 2;
        v |= (self.inte as u8) << 3;
        // pending_5_5 / pending_6_5 mirror the level pins; pending_7_5 latched.
        v |= ((self.rst_5_5) as u8) << 4;
        v |= ((self.rst_6_5) as u8) << 5;
        v |= (self.pending_7_5 as u8) << 6;
        self.regs.a = v;
    }

    fn exec_sim(&mut self) {
        let v = self.regs.a;
        if v & (1 << 3) != 0 {
            self.mask_5_5 = v & (1 << 0) != 0;
            self.mask_6_5 = v & (1 << 1) != 0;
            self.mask_7_5 = v & (1 << 2) != 0;
        }
        if v & (1 << 4) != 0 {
            self.pending_7_5 = false;
        }
        // SOD (bits 6/7) has no observable sink here.
    }

    /// Drive one execute-phase T-state, then decide whether the phase is over.
    pub(crate) fn execute(&mut self, bus: &mut SystemBus) {
        let n = self.exec_len();
        let idx = self.t_state - 1;
        if idx < n {
            match self.execute_micro(bus, idx) {
                Micro::Continue => self.t_state += 1,
                Micro::Done => {
                    self.end_execute();
                    return;
                }
            }
        }
        if self.t_state.saturating_sub(1) >= n {
            self.end_execute();
        }
    }

    fn end_execute(&mut self) {
        self.cycle = MachineCycle::Fetch;
        self.t_state = 1;
    }

    /// One execute-phase micro-op for the latched opcode at step `idx`.
    fn execute_micro(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        let b = self.ireg;
        let opcode = Opcode::from_byte(b).ok();

        // ---- data transfer ----
        if is_mvi_r(b) {
            return self.read_byte_seq(bus, idx, Src::Pc, |c, byte| {
                if let Some(d) = c.dst8 {
                    c.regs.set8(d, byte);
                }
            });
        }
        if matches!(opcode, Some(Opcode::LDA_BC) | Some(Opcode::LDA_DE)) {
            let ptr = self.regs.get16(self.ptr16.unwrap());
            return self.read_byte_seq(bus, idx, Src::Addr(ptr), |c, byte| c.regs.a = byte);
        }
        if matches!(opcode, Some(Opcode::STA_BC) | Some(Opcode::STA_DE)) {
            let ptr = self.regs.get16(self.ptr16.unwrap());
            return self.write_byte_seq(bus, idx, ptr, self.regs.a);
        }
        if is_mov_from_m(b) {
            let hl = self.regs.get16(Reg16::HL);
            return self.read_byte_seq(bus, idx, Src::Addr(hl), |c, byte| {
                if let Some(d) = c.dst8 {
                    c.regs.set8(d, byte);
                }
            });
        }
        if is_mov_to_m(b) {
            let val = self.src8.map(|r| self.regs.get8(r)).unwrap_or(0);
            return self.write_byte_seq(bus, idx, self.regs.get16(Reg16::HL), val);
        }
        if opcode == Some(Opcode::MVI_M) {
            return self.mvi_m_seq(bus, idx); // MVI M,d8
        }
        if matches!(opcode, Some(Opcode::MVI_BC) | Some(Opcode::MVI_DE) | Some(Opcode::MVI_HL) | Some(Opcode::LXI_SP)) {
            return self.lxi_seq(bus, idx);
        }
        if opcode == Some(Opcode::LDA) {
            return self.lda_seq(bus, idx); // LDA
        }
        if opcode == Some(Opcode::STA) {
            return self.sta_seq(bus, idx); // STA
        }
        if opcode == Some(Opcode::LHLD) {
            return self.lhld_seq(bus, idx); // LHLD
        }
        if opcode == Some(Opcode::SHLD) {
            return self.shld_seq(bus, idx); // SHLD
        }
        if opcode == Some(Opcode::XTHL) {
            return self.xthl_seq(bus, idx); // XTHL
        }
        if opcode == Some(Opcode::SPHL) {
            // SPHL
            if idx == 1 {
                self.regs.sp = self.regs.get16(Reg16::HL);
            }
            return Micro::Continue;
        }
        if opcode == Some(Opcode::PCHL) {
            // PCHL
            if idx == 1 {
                self.regs.pc = self.regs.get16(Reg16::HL);
            }
            return Micro::Continue;
        }

        // ---- arithmetic / logical (memory + immediate) ----
        if let Some(kind) = aop_immediate(b) {
            return self.read_byte_seq(bus, idx, Src::Pc, move |c, byte| c.apply_aop(kind, byte));
        }
        if (0x80..=0xBF).contains(&b) && (b & 7) == 6 {
            let kind = aop_group(b & 0xF8);
            let hl = self.regs.get16(Reg16::HL);
            return self.read_byte_seq(bus, idx, Src::Addr(hl), move |c, byte| c.apply_aop(kind, byte));
        }
        if opcode == Some(Opcode::INR_M) || opcode == Some(Opcode::DCR_M) {
            return self.rmw_hl_seq(bus, idx, |c, v| {
                if opcode == Some(Opcode::INR_M) {
                    alu::inr(&mut c.flags, v)
                } else {
                    alu::dcr(&mut c.flags, v)
                }
            });
        }
        if matches!(b, 0x03 | 0x13 | 0x23 | 0x33 | 0x0B | 0x1B | 0x2B | 0x3B) {
            if idx == 1 {
                let rp = self.ptr16.unwrap();
                let cur = self.regs.get16(rp);
                let next = if b & 0x08 == 0 {
                    cur.wrapping_add(1)
                } else {
                    cur.wrapping_sub(1)
                };
                self.regs.set16(rp, next);
            }
            return Micro::Continue;
        }
        if matches!(b, 0x09 | 0x19 | 0x29 | 0x39) {
            if idx == self.exec_len() - 1 {
                let rp = self.ptr16.unwrap();
                let hl = self.regs.get16(Reg16::HL).0;
                let rpv = self.regs.get16(rp).0;
                let res = alu::dad(&mut self.flags, hl, rpv);
                self.regs.set16(Reg16::HL, Addr(res));
            }
            return Micro::Continue;
        }

        // ---- stack ----
        if is_push(b) {
            return self.push_seq(bus, idx);
        }
        if is_pop(b) {
            return self.pop_seq(bus, idx);
        }

        // ---- branch / subroutine ----
        if opcode == Some(Opcode::JMP) || is_cond_jump(b) {
            return self.jmp_seq(bus, idx, cond_of(b));
        }
        if opcode == Some(Opcode::CALL) || is_cond_call(b) {
            return self.call_seq(bus, idx, cond_of(b));
        }
        if opcode == Some(Opcode::RET) || is_cond_ret(b) {
            return self.ret_seq(bus, idx, cond_of(b));
        }
        if is_rst(b) {
            return self.rst_seq(bus, idx);
        }

        // ---- machine / I-O control ----
        if opcode == Some(Opcode::IN) {
            return self.in_seq(bus, idx);
        }
        if opcode == Some(Opcode::OUT) {
            return self.out_seq(bus, idx);
        }

        Micro::Continue
    }

    // ---------- shared byte-transfer sequences ----------

    fn read_byte_seq(&mut self, bus: &mut SystemBus, idx: u32, src: Src, act: impl FnOnce(&mut Cpu, u8)) -> Micro {
        match idx {
            0 => {
                let a = match src {
                    Src::Pc => {
                        let a = self.regs.pc;
                        self.regs.pc = self.regs.pc.wrapping_add(1);
                        a
                    }
                    Src::Addr(a) => a,
                };
                bus.set_address(a);
            }
            1 => bus.lines.mr = true,
            _ => {
                let byte = bus.data();
                bus.lines.mr = false;
                act(self, byte);
            }
        }
        Micro::Continue
    }

    fn write_byte_seq(&mut self, bus: &mut SystemBus, idx: u32, addr: Addr, val: u8) -> Micro {
        match idx {
            0 => bus.set_address(addr),
            1 => {
                bus.set_address(addr);
                bus.set_data(val);
                bus.lines.mw = true;
            }
            _ => bus.lines.mw = false,
        }
        Micro::Continue
    }

    fn rmw_hl_seq(&mut self, bus: &mut SystemBus, idx: u32, f: impl FnOnce(&mut Cpu, u8) -> u8) -> Micro {
        let hl = self.regs.get16(Reg16::HL);
        match idx {
            0 => bus.set_address(hl),
            1 => bus.lines.mr = true,
            2 => {
                let v = bus.data();
                bus.lines.mr = false;
                self.work8 = f(self, v);
            }
            3 => bus.set_address(hl),
            4 => {
                bus.set_address(hl);
                bus.set_data(self.work8);
                bus.lines.mw = true;
            }
            _ => bus.lines.mw = false,
        }
        Micro::Continue
    }

    fn mvi_m_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        match idx {
            0 => {
                bus.set_address(self.regs.pc);
                self.regs.pc = self.regs.pc.wrapping_add(1);
            }
            1 => bus.lines.mr = true,
            2 => {
                self.work8 = bus.data();
                bus.lines.mr = false;
            }
            3 => bus.set_address(self.regs.get16(Reg16::HL)),
            4 => {
                bus.set_address(self.regs.get16(Reg16::HL));
                bus.set_data(self.work8);
                bus.lines.mw = true;
            }
            _ => bus.lines.mw = false,
        }
        Micro::Continue
    }

    fn lxi_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        match idx {
            0 => self.pc_out(bus),
            1 => bus.lines.mr = true,
            2 => {
                self.work8 = bus.data();
                bus.lines.mr = false;
            }
            3 => self.pc_out(bus),
            4 => bus.lines.mr = true,
            _ => {
                let high = bus.data();
                bus.lines.mr = false;
                if let Some(rp) = self.ptr16 {
                    self.regs.set16(rp, Addr::from_le(self.work8, high));
                }
            }
        }
        Micro::Continue
    }

    /// Steps 0..5: fetch a 16-bit direct address from PC into `self.work`.
    fn fetch_addr16(&mut self, bus: &mut SystemBus, idx: u32) {
        match idx {
            0 => self.pc_out(bus),
            1 => bus.lines.mr = true,
            2 => {
                self.work8 = bus.data();
                bus.lines.mr = false;
            }
            3 => self.pc_out(bus),
            4 => bus.lines.mr = true,
            5 => {
                let high = bus.data();
                bus.lines.mr = false;
                self.work = Addr::from_le(self.work8, high);
            }
            _ => {}
        }
    }

    fn lda_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        if idx <= 5 {
            self.fetch_addr16(bus, idx);
        } else {
            match idx {
                6 => bus.set_address(self.work),
                7 => bus.lines.mr = true,
                _ => {
                    self.regs.a = bus.data();
                    bus.lines.mr = false;
                }
            }
        }
        Micro::Continue
    }

    fn sta_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        if idx <= 5 {
            self.fetch_addr16(bus, idx);
        } else {
            match idx {
                6 => bus.set_address(self.work),
                7 => {
                    bus.set_address(self.work);
                    bus.set_data(self.regs.a);
                    bus.lines.mw = true;
                }
                _ => bus.lines.mw = false,
            }
        }
        Micro::Continue
    }

    fn lhld_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        if idx <= 5 {
            self.fetch_addr16(bus, idx);
        } else {
            match idx {
                6 => bus.set_address(self.work),
                7 => bus.lines.mr = true,
                8 => {
                    self.regs.l = bus.data();
                    bus.lines.mr = false;
                }
                9 => bus.set_address(self.work.wrapping_add(1)),
                10 => bus.lines.mr = true,
                _ => {
                    self.regs.h = bus.data();
                    bus.lines.mr = false;
                }
            }
        }
        Micro::Continue
    }

    fn shld_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        if idx <= 5 {
            self.fetch_addr16(bus, idx);
        } else {
            match idx {
                6 => bus.set_address(self.work),
                7 => {
                    bus.set_address(self.work);
                    bus.set_data(self.regs.l);
                    bus.lines.mw = true;
                }
                8 => bus.lines.mw = false,
                9 => bus.set_address(self.work.wrapping_add(1)),
                10 => {
                    bus.set_address(self.work.wrapping_add(1));
                    bus.set_data(self.regs.h);
                    bus.lines.mw = true;
                }
                _ => bus.lines.mw = false,
            }
        }
        Micro::Continue
    }

    fn xthl_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        let sp = self.regs.sp;
        match idx {
            0 => bus.set_address(sp),
            1 => bus.lines.mr = true,
            2 => {
                let old_l = self.regs.l;
                self.regs.l = bus.data();
                bus.lines.mr = false;
                bus.set_data(old_l);
            }
            3 => {}
            4 => bus.lines.mw = true,
            5 => bus.lines.mw = false,
            6 => bus.set_address(sp.wrapping_add(1)),
            7 => bus.lines.mr = true,
            8 => {
                let old_h = self.regs.h;
                self.regs.h = bus.data();
                bus.lines.mr = false;
                bus.set_data(old_h);
            }
            9 => bus.set_address(sp.wrapping_add(1)),
            10 => bus.lines.mw = true,
            _ => bus.lines.mw = false,
        }
        Micro::Continue
    }

    // ---------- stack ----------

    /// (high, low) byte pair a PUSH stores / a POP loads for the current opcode.
    fn stack_pair_bytes(&self) -> (u8, u8) {
        match self.ireg & 0x30 {
            0x00 => (self.regs.b, self.regs.c),
            0x10 => (self.regs.d, self.regs.e),
            0x20 => (self.regs.h, self.regs.l),
            _ => (self.regs.a, self.flags.to_psw()), // PSW
        }
    }

    fn push_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        let (high, low) = self.stack_pair_bytes();
        match idx {
            0 => {}
            1 => self.push_step(bus),
            2 => bus.set_data(high),
            3 => bus.lines.mw = true,
            4 => self.push_step(bus),
            5 => bus.set_data(low),
            6 => bus.lines.mw = true,
            _ => bus.lines.mw = false,
        }
        Micro::Continue
    }

    fn pop_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        match idx {
            0 => bus.set_address(self.regs.sp),
            1 => bus.lines.mr = true,
            2 => {
                let low = bus.data();
                bus.lines.mr = false;
                self.store_stack_low(low);
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
            3 => bus.set_address(self.regs.sp),
            4 => bus.lines.mr = true,
            _ => {
                let high = bus.data();
                bus.lines.mr = false;
                self.store_stack_high(high);
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
        }
        Micro::Continue
    }

    fn store_stack_low(&mut self, v: u8) {
        match self.ireg & 0x30 {
            0x00 => self.regs.c = v,
            0x10 => self.regs.e = v,
            0x20 => self.regs.l = v,
            _ => self.flags = super::flags::Flags::from_psw(v),
        }
    }
    fn store_stack_high(&mut self, v: u8) {
        match self.ireg & 0x30 {
            0x00 => self.regs.b = v,
            0x10 => self.regs.d = v,
            0x20 => self.regs.h = v,
            _ => self.regs.a = v,
        }
    }

    /// Decrement SP and drive it onto the address bus (start of a stack write).
    fn push_step(&mut self, bus: &mut SystemBus) {
        bus.lines.mw = false;
        self.regs.sp = self.regs.sp.wrapping_sub(1);
        bus.set_address(self.regs.sp);
    }

    // ---------- branch / subroutine ----------

    fn jmp_seq(&mut self, bus: &mut SystemBus, idx: u32, cond: Option<Cond>) -> Micro {
        if idx <= 4 {
            self.fetch_addr16(bus, idx);
        } else {
            // idx == 5: high byte just latched by fetch_addr16 at idx 5? No: fetch spans 0..5.
        }
        if idx == 5 {
            self.fetch_addr16(bus, 5);
            if cond.map(|c| self.cond_met(c)).unwrap_or(true) {
                self.regs.pc = self.work;
            }
        }
        Micro::Continue
    }

    fn call_seq(&mut self, bus: &mut SystemBus, idx: u32, cond: Option<Cond>) -> Micro {
        match idx {
            0 | 1 => {}
            2 => self.pc_out(bus),
            3 => bus.lines.mr = true,
            4 => {
                self.work8 = bus.data();
                bus.lines.mr = false;
            }
            5 => self.pc_out(bus),
            6 => bus.lines.mr = true,
            7 => {
                let high = bus.data();
                bus.lines.mr = false;
                self.work = Addr::from_le(self.work8, high);
                if let Some(c) = cond {
                    if !self.cond_met(c) {
                        return Micro::Done; // conditional call not taken: end early
                    }
                }
            }
            8 => self.push_step(bus),
            9 => bus.set_data(self.regs.pc.high()),
            10 => bus.lines.mw = true,
            11 => self.push_step(bus),
            12 => bus.set_data(self.regs.pc.low()),
            _ => {
                bus.lines.mw = true;
                self.regs.pc = self.work;
            }
        }
        Micro::Continue
    }

    fn ret_seq(&mut self, bus: &mut SystemBus, idx: u32, cond: Option<Cond>) -> Micro {
        // Unconditional RET has no leading delay/check; conditional RET has 2 extra steps.
        let conditional = cond.is_some();
        if conditional {
            match idx {
                0 => return Micro::Continue, // internal delay
                1 => {
                    let c = cond.unwrap();
                    if !self.cond_met(c) {
                        return Micro::Done; // not taken: end early
                    }
                    return Micro::Continue;
                }
                _ => return self.ret_body(bus, idx - 2),
            }
        }
        self.ret_body(bus, idx)
    }

    fn ret_body(&mut self, bus: &mut SystemBus, k: u32) -> Micro {
        match k {
            0 => bus.set_address(self.regs.sp),
            1 => bus.lines.mr = true,
            2 => {
                self.work8 = bus.data(); // low
                bus.lines.mr = false;
                self.regs.sp = self.regs.sp.wrapping_add(1);
            }
            3 => bus.set_address(self.regs.sp),
            4 => bus.lines.mr = true,
            _ => {
                let high = bus.data();
                bus.lines.mr = false;
                self.regs.sp = self.regs.sp.wrapping_add(1);
                self.regs.pc = Addr::from_le(self.work8, high);
            }
        }
        Micro::Continue
    }

    fn rst_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        match idx {
            0 | 1 => {}
            2 => self.push_step(bus),
            3 => bus.set_data(self.regs.pc.high()),
            4 => bus.lines.mw = true,
            5 => self.push_step(bus),
            6 => bus.set_data(self.regs.pc.low()),
            7 => bus.lines.mw = true,
            _ => {
                bus.lines.mw = false;
                let n = (self.ireg >> 3) & 7;
                self.regs.pc = Addr(u16::from(n) * 8);
            }
        }
        Micro::Continue
    }

    // ---------- machine / I-O ----------

    fn in_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        match idx {
            0 => self.pc_out(bus),
            1 => bus.lines.mr = true,
            2 => {
                self.work8 = bus.data(); // port
                bus.lines.mr = false;
            }
            3 => {
                bus.lines.mr = false;
                let p = self.work8;
                bus.set_address(Addr((u16::from(p) << 8) | u16::from(p)));
            }
            4 => bus.lines.ior = true,
            _ => {
                self.regs.a = bus.data();
                bus.lines.ior = false;
            }
        }
        Micro::Continue
    }

    fn out_seq(&mut self, bus: &mut SystemBus, idx: u32) -> Micro {
        match idx {
            0 => self.pc_out(bus),
            1 => bus.lines.mr = true,
            2 => {
                self.work8 = bus.data(); // port
                bus.lines.mr = false;
            }
            3 => {
                let p = self.work8;
                bus.set_address(Addr((u16::from(p) << 8) | u16::from(p)));
            }
            4 => bus.set_data(self.regs.a),
            _ => bus.lines.iow = true,
        }
        Micro::Continue
    }

    // ---------- small helpers ----------

    fn pc_out(&mut self, bus: &mut SystemBus) {
        bus.set_address(self.regs.pc);
        self.regs.pc = self.regs.pc.wrapping_add(1);
    }

    fn cond_met(&self, c: Cond) -> bool {
        match c {
            Cond::Nz => !self.flags.zero,
            Cond::Z => self.flags.zero,
            Cond::Nc => !self.flags.carry,
            Cond::C => self.flags.carry,
            Cond::Po => !self.flags.parity,
            Cond::Pe => self.flags.parity,
            Cond::P => !self.flags.sign,
            Cond::M => self.flags.sign,
        }
    }
}

/// Where a single-byte read gets its address.
#[derive(Clone, Copy)]
enum Src {
    Pc,
    Addr(Addr),
}

/// The eight condition codes encoded in bits 5..3 of conditional opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cond {
    Nz,
    Z,
    Nc,
    C,
    Po,
    Pe,
    P,
    M,
}

fn cond_of(b: u8) -> Option<Cond> {
    // Unconditional forms (JMP/CALL/RET) carry no condition.
    if matches!(b, 0xC3 | 0xCD | 0xC9) {
        return None;
    }
    Some(match (b >> 3) & 7 {
        0 => Cond::Nz,
        1 => Cond::Z,
        2 => Cond::Nc,
        3 => Cond::C,
        4 => Cond::Po,
        5 => Cond::Pe,
        6 => Cond::P,
        _ => Cond::M,
    })
}

fn aop_group(base: u8) -> Aop {
    match base {
        0x80 => Aop::Add,
        0x88 => Aop::Adc,
        0x90 => Aop::Sub,
        0x98 => Aop::Sbb,
        0xA0 => Aop::And,
        0xA8 => Aop::Xor,
        0xB0 => Aop::Or,
        0xB8 => Aop::Cmp,
        other => unreachable!("not an A-op group: {other:#04x}"),
    }
}

fn aop_immediate(b: u8) -> Option<Aop> {
    Some(match b {
        0xC6 => Aop::Add,
        0xCE => Aop::Adc,
        0xD6 => Aop::Sub,
        0xDE => Aop::Sbb,
        0xE6 => Aop::And,
        0xEE => Aop::Xor,
        0xF6 => Aop::Or,
        0xFE => Aop::Cmp,
        _ => return None,
    })
}

fn is_mvi_r(b: u8) -> bool {
    matches!(b, 0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E)
}
fn is_inr_dcr(b: u8) -> bool {
    (b & 0xC7) == 0x04 || (b & 0xC7) == 0x05
}
fn is_mov_rr(b: u8) -> bool {
    (0x40..=0x7F).contains(&b) && b != 0x76 && (b & 0x07) != 0x06 && (b & 0x38) != 0x30
}
fn is_mov_from_m(b: u8) -> bool {
    (0x40..=0x7F).contains(&b) && (b & 0x07) == 0x06 && b != 0x76
}
fn is_mov_to_m(b: u8) -> bool {
    (0x70..=0x77).contains(&b) && b != 0x76
}
fn is_push(b: u8) -> bool {
    matches!(b, 0xC5 | 0xD5 | 0xE5 | 0xF5)
}
fn is_pop(b: u8) -> bool {
    matches!(b, 0xC1 | 0xD1 | 0xE1 | 0xF1)
}
fn is_cond_jump(b: u8) -> bool {
    matches!(b, 0xC2 | 0xCA | 0xD2 | 0xDA | 0xE2 | 0xEA | 0xF2 | 0xFA)
}
fn is_cond_call(b: u8) -> bool {
    matches!(b, 0xC4 | 0xCC | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC)
}
fn is_cond_ret(b: u8) -> bool {
    matches!(b, 0xC0 | 0xC8 | 0xD0 | 0xD8 | 0xE0 | 0xE8 | 0xF0 | 0xF8)
}
fn is_rst(b: u8) -> bool {
    matches!(b, 0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF)
}

fn reg_from_code(code: u8) -> Reg8 {
    match code {
        0 => Reg8::B,
        1 => Reg8::C,
        2 => Reg8::D,
        3 => Reg8::E,
        4 => Reg8::H,
        5 => Reg8::L,
        7 => Reg8::A,
        other => unreachable!("invalid register code {other}"),
    }
}
fn pair_from_code(code: u8) -> Reg16 {
    match code {
        0 => Reg16::BC,
        1 => Reg16::DE,
        2 => Reg16::HL,
        3 => Reg16::SP,
        other => unreachable!("invalid pair code {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_table_matches_known_opcodes() {
        assert_eq!(EXEC_TSTATES[0x00], 0);
        assert_eq!(EXEC_TSTATES[0xCD], 14); // CALL
        assert_eq!(EXEC_TSTATES[0xC9], 6); // RET
        assert_eq!(EXEC_TSTATES[0xC5], 8); // PUSH B
        assert_eq!(EXEC_TSTATES[0xE3], 12); // XTHL
    }

    #[test]
    fn condition_decode() {
        assert_eq!(cond_of(0xC3), None); // JMP
        assert_eq!(cond_of(0xCA), Some(Cond::Z)); // JZ
        assert_eq!(cond_of(0xD2), Some(Cond::Nc)); // JNC
        assert_eq!(cond_of(0xFC), Some(Cond::M)); // CM
    }
}
