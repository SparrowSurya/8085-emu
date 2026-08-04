//! The 8085 register file and the enums used to name registers.
//!
//! The Python code addressed registers by string name and juggled 8-/16-bit views
//! through a `RegisterRef` shim. Here the closed sets are enums ([`Reg8`], [`Reg16`])
//! and the file exposes typed `get`/`set` plus register-pair accessors.

use crate::value::Addr;

/// The 8-bit registers, including the hidden internal `W`/`Z` scratch registers the
/// 8085 uses to stage 16-bit operands mid-instruction. `M` is deliberately absent —
/// "the byte HL points at" is memory, resolved by the execution unit, not a register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reg8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    /// Hidden internal temporary (high half of the WZ pair).
    W,
    /// Hidden internal temporary (low half of the WZ pair).
    Z,
}

/// The 16-bit registers and register pairs: the real `PC`/`SP` plus the `BC`/`DE`/`HL`
/// pairs and the internal `WZ` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum Reg16 {
    BC,
    DE,
    HL,
    SP,
    PC,
    WZ,
}

/// The complete architectural + internal register state (everything except the flags,
/// which live in [`super::flags::Flags`]). `SP` powers up to `0xFFFF` like real hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterFile {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub w: u8,
    pub z: u8,
    pub sp: Addr,
    pub pc: Addr,
}

impl Default for RegisterFile {
    fn default() -> Self {
        RegisterFile {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            w: 0,
            z: 0,
            sp: Addr(0xFFFF),
            pc: Addr(0x0000),
        }
    }
}

impl RegisterFile {
    /// Fresh register file in power-on state (`SP = 0xFFFF`, everything else zero).
    pub fn new() -> Self {
        Self::default()
    }

    /// Read an 8-bit register.
    pub fn get8(&self, r: Reg8) -> u8 {
        match r {
            Reg8::A => self.a,
            Reg8::B => self.b,
            Reg8::C => self.c,
            Reg8::D => self.d,
            Reg8::E => self.e,
            Reg8::H => self.h,
            Reg8::L => self.l,
            Reg8::W => self.w,
            Reg8::Z => self.z,
        }
    }

    /// Write an 8-bit register.
    pub fn set8(&mut self, r: Reg8, val: u8) {
        match r {
            Reg8::A => self.a = val,
            Reg8::B => self.b = val,
            Reg8::C => self.c = val,
            Reg8::D => self.d = val,
            Reg8::E => self.e = val,
            Reg8::H => self.h = val,
            Reg8::L => self.l = val,
            Reg8::W => self.w = val,
            Reg8::Z => self.z = val,
        }
    }

    /// Read a 16-bit register or register pair (high register is the MSB).
    pub fn get16(&self, r: Reg16) -> Addr {
        match r {
            Reg16::BC => Addr::from_le(self.c, self.b),
            Reg16::DE => Addr::from_le(self.e, self.d),
            Reg16::HL => Addr::from_le(self.l, self.h),
            Reg16::WZ => Addr::from_le(self.z, self.w),
            Reg16::SP => self.sp,
            Reg16::PC => self.pc,
        }
    }

    /// Write a 16-bit register or register pair, splitting across the two 8-bit halves.
    pub fn set16(&mut self, r: Reg16, val: Addr) {
        match r {
            Reg16::BC => {
                self.b = val.high();
                self.c = val.low();
            }
            Reg16::DE => {
                self.d = val.high();
                self.e = val.low();
            }
            Reg16::HL => {
                self.h = val.high();
                self.l = val.low();
            }
            Reg16::WZ => {
                self.w = val.high();
                self.z = val.low();
            }
            Reg16::SP => self.sp = val,
            Reg16::PC => self.pc = val,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_on_sp_is_ffff() {
        assert_eq!(RegisterFile::new().sp, Addr(0xFFFF));
    }

    #[test]
    fn pair_write_splits_high_and_low() {
        let mut rf = RegisterFile::new();
        rf.set16(Reg16::HL, Addr(0x1234));
        assert_eq!(rf.h, 0x12);
        assert_eq!(rf.l, 0x34);
        assert_eq!(rf.get16(Reg16::HL), Addr(0x1234));
    }

    #[test]
    fn eight_bit_get_set_roundtrip() {
        let mut rf = RegisterFile::new();
        for r in [Reg8::A, Reg8::B, Reg8::W, Reg8::Z] {
            rf.set8(r, 0xAB);
            assert_eq!(rf.get8(r), 0xAB);
        }
    }
}
