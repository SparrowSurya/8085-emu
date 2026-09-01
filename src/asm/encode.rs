//! The encoder: resolves a mnemonic and its (already-resolved) operands into the exact
//! machine-code bytes for one instruction.
//!
//! Register-parameterised families (`ADD`, `INR`, `MOV`, `LXI`, …) are computed
//! arithmetically from the operand's register code; fixed-form instructions map straight
//! to their opcode byte. Every source instruction produces exactly one instruction's
//! worth of bytes (opcode byte plus any immediate/address bytes, little-endian).

use super::error::{AsmError, AsmErrorKind, Span};

/// An 8-bit register operand, including `M` (the byte at `[HL]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AReg8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    M,
}

/// A 16-bit register / register-pair operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AReg16 {
    BC,
    DE,
    HL,
    SP,
    PSW,
}

/// A fully-resolved instruction operand (symbols already resolved to numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// An 8-bit register (or `M`).
    Reg8(AReg8),
    /// A 16-bit register pair.
    Reg16(AReg16),
    /// A numeric immediate, address, port, or restart index.
    Imm(u32),
}

/// The 3-bit register code used inside opcode bytes: `B=0 … A=7`, with `M=6`.
fn r8_code(r: AReg8) -> u8 {
    match r {
        AReg8::B => 0,
        AReg8::C => 1,
        AReg8::D => 2,
        AReg8::E => 3,
        AReg8::H => 4,
        AReg8::L => 5,
        AReg8::M => 6,
        AReg8::A => 7,
    }
}

/// Pair index for `LXI`/`INX`/`DCX`/`DAD` (`BC=0, DE=1, HL=2, SP=3`; `PSW` invalid).
fn pair_idx_sp(r: AReg16) -> Option<u8> {
    match r {
        AReg16::BC => Some(0),
        AReg16::DE => Some(1),
        AReg16::HL => Some(2),
        AReg16::SP => Some(3),
        AReg16::PSW => None,
    }
}

/// Pair index for `PUSH`/`POP` (`BC=0, DE=1, HL=2, PSW=3`; `SP` invalid).
fn pair_idx_psw(r: AReg16) -> Option<u8> {
    match r {
        AReg16::BC => Some(0),
        AReg16::DE => Some(1),
        AReg16::HL => Some(2),
        AReg16::PSW => Some(3),
        AReg16::SP => None,
    }
}

/// Pair index for `LDAX`/`STAX` (`BC=0, DE=1` only).
fn pair_idx_bcde(r: AReg16) -> Option<u8> {
    match r {
        AReg16::BC => Some(0),
        AReg16::DE => Some(1),
        _ => None,
    }
}

/// Encode one instruction. `span` positions any error at the mnemonic.
pub fn encode(mnemonic: &str, span: Span, ops: &[Operand]) -> Result<Vec<u8>, AsmError> {
    let m = mnemonic.to_ascii_uppercase();
    let ctx = Ctx { m: &m, span, ops };

    // Fixed no-operand instructions.
    if let Some(byte) = fixed_no_operand(&m) {
        ctx.expect_count(0)?;
        return Ok(vec![byte]);
    }
    // Fixed instructions taking one 8-bit immediate (I/O port).
    if let Some(byte) = fixed_imm8(&m) {
        let imm = ctx.one_imm()?;
        return Ok(vec![byte, ctx.imm8(imm)?]);
    }
    // Fixed instructions taking one 16-bit address.
    if let Some(byte) = fixed_imm16(&m) {
        let imm = ctx.one_imm()?;
        let [lo, hi] = ctx.imm16(imm)?;
        return Ok(vec![byte, lo, hi]);
    }
    // Accumulator ops with an 8-bit immediate.
    if let Some(byte) = alu_imm(&m) {
        let imm = ctx.one_imm()?;
        return Ok(vec![byte, ctx.imm8(imm)?]);
    }

    match m.as_str() {
        // Accumulator ops with a register/memory source.
        "ADD" | "ADC" | "SUB" | "SBB" | "ANA" | "XRA" | "ORA" | "CMP" => {
            let base = alu_reg_base(&m).unwrap();
            let r = ctx.one_reg8()?;
            Ok(vec![base + r8_code(r)])
        }
        "INR" => Ok(vec![0x04 + (r8_code(ctx.one_reg8()?) << 3)]),
        "DCR" => Ok(vec![0x05 + (r8_code(ctx.one_reg8()?) << 3)]),

        // 16-bit pair arithmetic.
        "INX" => Ok(vec![0x03 + ctx.pair(pair_idx_sp)? * 0x10]),
        "DCX" => Ok(vec![0x0B + ctx.pair(pair_idx_sp)? * 0x10]),
        "DAD" => Ok(vec![0x09 + ctx.pair(pair_idx_sp)? * 0x10]),

        // Stack.
        "PUSH" => Ok(vec![0xC5 + ctx.pair(pair_idx_psw)? * 0x10]),
        "POP" => Ok(vec![0xC1 + ctx.pair(pair_idx_psw)? * 0x10]),

        // Indirect load/store through BC/DE.
        "LDAX" => Ok(vec![0x0A + ctx.pair(pair_idx_bcde)? * 0x10]),
        "STAX" => Ok(vec![0x02 + ctx.pair(pair_idx_bcde)? * 0x10]),

        // Load pair immediate.
        "LXI" => {
            let (rp, imm) = ctx.reg16_imm()?;
            let idx = ctx.check_pair(rp, pair_idx_sp)?;
            let [lo, hi] = ctx.imm16(imm)?;
            Ok(vec![0x01 + idx * 0x10, lo, hi])
        }

        // Move immediate into a register/memory.
        "MVI" => {
            let (dst, imm) = ctx.reg8_imm()?;
            Ok(vec![0x06 + (r8_code(dst) << 3), ctx.imm8(imm)?])
        }

        // Unified move: register↔register, or (dst, imm) which resolves to MVI.
        "MOV" => match ctx.ops {
            [Operand::Reg8(d), Operand::Reg8(s)] => {
                if *d == AReg8::M && *s == AReg8::M {
                    return Err(ctx.bad("MOV M, M is not a valid instruction (that byte is HLT)"));
                }
                Ok(vec![0x40 + (r8_code(*d) << 3) + r8_code(*s)])
            }
            [Operand::Reg8(d), Operand::Imm(v)] => {
                Ok(vec![0x06 + (r8_code(*d) << 3), ctx.imm8(*v)?])
            }
            _ => Err(ctx.bad("expected `MOV reg, reg` or `MOV reg, imm8`")),
        },

        // Restart.
        "RST" => {
            let n = ctx.one_imm()?;
            if n > 7 {
                return Err(AsmError::new(
                    span,
                    AsmErrorKind::ImmediateOutOfRange { value: n, max: 7 },
                ));
            }
            Ok(vec![0xC7 + (n as u8) * 8])
        }

        other => Err(AsmError::new(
            span,
            AsmErrorKind::UnknownMnemonic(other.to_string()),
        )),
    }
}

/// Per-call context, bundling the operand-shape checks with good error messages.
struct Ctx<'a> {
    m: &'a str,
    span: Span,
    ops: &'a [Operand],
}

impl Ctx<'_> {
    fn bad(&self, detail: &str) -> AsmError {
        AsmError::new(
            self.span,
            AsmErrorKind::BadOperand {
                mnemonic: self.m.to_string(),
                detail: detail.to_string(),
            },
        )
    }

    fn expect_count(&self, n: usize) -> Result<(), AsmError> {
        if self.ops.len() == n {
            Ok(())
        } else {
            Err(AsmError::new(
                self.span,
                AsmErrorKind::OperandCount {
                    mnemonic: self.m.to_string(),
                    expected: n,
                    found: self.ops.len(),
                },
            ))
        }
    }

    fn one_imm(&self) -> Result<u32, AsmError> {
        self.expect_count(1)?;
        match self.ops[0] {
            Operand::Imm(v) => Ok(v),
            _ => Err(self.bad("expected a numeric operand")),
        }
    }

    fn one_reg8(&self) -> Result<AReg8, AsmError> {
        self.expect_count(1)?;
        match self.ops[0] {
            Operand::Reg8(r) => Ok(r),
            _ => Err(self.bad("expected an 8-bit register (A, B, C, D, E, H, L, or M)")),
        }
    }

    fn pair(&self, idx: fn(AReg16) -> Option<u8>) -> Result<u8, AsmError> {
        self.expect_count(1)?;
        match self.ops[0] {
            Operand::Reg16(rp) => self.check_pair(rp, idx),
            _ => Err(self.bad("expected a register pair")),
        }
    }

    fn check_pair(&self, rp: AReg16, idx: fn(AReg16) -> Option<u8>) -> Result<u8, AsmError> {
        idx(rp).ok_or_else(|| self.bad("register pair not allowed for this instruction"))
    }

    fn reg16_imm(&self) -> Result<(AReg16, u32), AsmError> {
        self.expect_count(2)?;
        match (self.ops[0], self.ops[1]) {
            (Operand::Reg16(rp), Operand::Imm(v)) => Ok((rp, v)),
            _ => Err(self.bad("expected `pair, imm16`")),
        }
    }

    fn reg8_imm(&self) -> Result<(AReg8, u32), AsmError> {
        self.expect_count(2)?;
        match (self.ops[0], self.ops[1]) {
            (Operand::Reg8(r), Operand::Imm(v)) => Ok((r, v)),
            _ => Err(self.bad("expected `reg, imm8`")),
        }
    }

    fn imm8(&self, v: u32) -> Result<u8, AsmError> {
        if v <= 0xFF {
            Ok(v as u8)
        } else {
            Err(AsmError::new(
                self.span,
                AsmErrorKind::ImmediateOutOfRange {
                    value: v,
                    max: 0xFF,
                },
            ))
        }
    }

    fn imm16(&self, v: u32) -> Result<[u8; 2], AsmError> {
        if v <= 0xFFFF {
            Ok([(v & 0xFF) as u8, (v >> 8) as u8])
        } else {
            Err(AsmError::new(
                self.span,
                AsmErrorKind::ImmediateOutOfRange {
                    value: v,
                    max: 0xFFFF,
                },
            ))
        }
    }
}

fn alu_reg_base(m: &str) -> Option<u8> {
    Some(match m {
        "ADD" => 0x80,
        "ADC" => 0x88,
        "SUB" => 0x90,
        "SBB" => 0x98,
        "ANA" => 0xA0,
        "XRA" => 0xA8,
        "ORA" => 0xB0,
        "CMP" => 0xB8,
        _ => return None,
    })
}

fn alu_imm(m: &str) -> Option<u8> {
    Some(match m {
        "ADI" => 0xC6,
        "ACI" => 0xCE,
        "SUI" => 0xD6,
        "SBI" => 0xDE,
        "ANI" => 0xE6,
        "XRI" => 0xEE,
        "ORI" => 0xF6,
        "CPI" => 0xFE,
        _ => return None,
    })
}

fn fixed_imm8(m: &str) -> Option<u8> {
    Some(match m {
        "IN" => 0xDB,
        "OUT" => 0xD3,
        _ => return None,
    })
}

fn fixed_imm16(m: &str) -> Option<u8> {
    Some(match m {
        "LDA" => 0x3A,
        "STA" => 0x32,
        "LHLD" => 0x2A,
        "SHLD" => 0x22,
        "JMP" => 0xC3,
        "JZ" => 0xCA,
        "JNZ" => 0xC2,
        "JC" => 0xDA,
        "JNC" => 0xD2,
        "JP" => 0xF2,
        "JM" => 0xFA,
        "JPE" => 0xEA,
        "JPO" => 0xE2,
        "CALL" => 0xCD,
        "CZ" => 0xCC,
        "CNZ" => 0xC4,
        "CC" => 0xDC,
        "CNC" => 0xD4,
        "CP" => 0xF4,
        "CM" => 0xFC,
        "CPE" => 0xEC,
        "CPO" => 0xE4,
        _ => return None,
    })
}

fn fixed_no_operand(m: &str) -> Option<u8> {
    Some(match m {
        "NOP" => 0x00,
        "HLT" => 0x76,
        "RET" => 0xC9,
        "RZ" => 0xC8,
        "RNZ" => 0xC0,
        "RC" => 0xD8,
        "RNC" => 0xD0,
        "RP" => 0xF0,
        "RM" => 0xF8,
        "RPE" => 0xE8,
        "RPO" => 0xE0,
        "EI" => 0xFB,
        "DI" => 0xF3,
        "RIM" => 0x20,
        "SIM" => 0x30,
        "XCHG" => 0xEB,
        "XTHL" => 0xE3,
        "SPHL" => 0xF9,
        "PCHL" => 0xE9,
        "DAA" => 0x27,
        "CMA" => 0x2F,
        "CMC" => 0x3F,
        "STC" => 0x37,
        "RLC" => 0x07,
        "RRC" => 0x0F,
        "RAL" => 0x17,
        "RAR" => 0x1F,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use AReg16::*;
    use AReg8::*;

    fn enc(m: &str, ops: &[Operand]) -> Vec<u8> {
        encode(m, Span::new(1, 1), ops).unwrap()
    }
    fn err(m: &str, ops: &[Operand]) -> AsmErrorKind {
        encode(m, Span::new(1, 1), ops).unwrap_err().kind
    }
    fn r8(r: AReg8) -> Operand {
        Operand::Reg8(r)
    }
    fn r16(r: AReg16) -> Operand {
        Operand::Reg16(r)
    }
    fn imm(v: u32) -> Operand {
        Operand::Imm(v)
    }

    #[test]
    fn mov_matrix() {
        assert_eq!(enc("mov", &[r8(A), r8(B)]), vec![0x78]);
        assert_eq!(enc("MOV", &[r8(B), r8(C)]), vec![0x41]);
        assert_eq!(enc("mov", &[r8(M), r8(A)]), vec![0x77]);
        assert_eq!(enc("mov", &[r8(A), r8(A)]), vec![0x7F]); // MOV A,A
        assert_eq!(enc("mov", &[r8(A), r8(M)]), vec![0x7E]);
        // mov with an immediate resolves to MVI
        assert_eq!(enc("mov", &[r8(M), imm(0xFF)]), vec![0x36, 0xFF]);
        // MOV M,M is HLT's byte and is rejected
        assert!(matches!(
            err("mov", &[r8(M), r8(M)]),
            AsmErrorKind::BadOperand { .. }
        ));
    }

    #[test]
    fn mvi_lxi_ldax_stax() {
        assert_eq!(enc("mvi", &[r8(A), imm(0x2A)]), vec![0x3E, 0x2A]);
        assert_eq!(enc("mvi", &[r8(M), imm(0x00)]), vec![0x36, 0x00]);
        assert_eq!(enc("lxi", &[r16(BC), imm(0x1234)]), vec![0x01, 0x34, 0x12]);
        assert_eq!(enc("lxi", &[r16(SP), imm(0xF000)]), vec![0x31, 0x00, 0xF0]); // LXI SP
        assert!(matches!(
            err("lxi", &[r16(PSW), imm(1)]),
            AsmErrorKind::BadOperand { .. }
        ));
        assert_eq!(enc("ldax", &[r16(BC)]), vec![0x0A]);
        assert_eq!(enc("stax", &[r16(DE)]), vec![0x12]);
        assert!(matches!(
            err("ldax", &[r16(HL)]),
            AsmErrorKind::BadOperand { .. }
        ));
    }

    #[test]
    fn arithmetic_logical_and_pairs() {
        assert_eq!(enc("add", &[r8(B)]), vec![0x80]);
        assert_eq!(enc("cmp", &[r8(M)]), vec![0xBE]);
        assert_eq!(enc("adi", &[imm(0x10)]), vec![0xC6, 0x10]);
        assert_eq!(enc("cpi", &[imm(0x50)]), vec![0xFE, 0x50]);
        assert_eq!(enc("inr", &[r8(A)]), vec![0x3C]);
        assert_eq!(enc("dcr", &[r8(M)]), vec![0x35]);
        assert_eq!(enc("inx", &[r16(SP)]), vec![0x33]);
        assert_eq!(enc("dad", &[r16(HL)]), vec![0x29]);
        assert_eq!(enc("push", &[r16(PSW)]), vec![0xF5]);
        assert_eq!(enc("pop", &[r16(BC)]), vec![0xC1]);
        assert!(matches!(
            err("push", &[r16(SP)]),
            AsmErrorKind::BadOperand { .. }
        ));
    }

    #[test]
    fn branch_io_rst_and_no_operand() {
        assert_eq!(enc("jmp", &[imm(0x00A0)]), vec![0xC3, 0xA0, 0x00]);
        assert_eq!(enc("jz", &[imm(0x1234)]), vec![0xCA, 0x34, 0x12]);
        assert_eq!(enc("call", &[imm(0x0008)]), vec![0xCD, 0x08, 0x00]);
        assert_eq!(enc("in", &[imm(0x02)]), vec![0xDB, 0x02]);
        assert_eq!(enc("out", &[imm(0x05)]), vec![0xD3, 0x05]);
        assert_eq!(enc("rst", &[imm(0)]), vec![0xC7]);
        assert_eq!(enc("rst", &[imm(7)]), vec![0xFF]);
        assert_eq!(enc("nop", &[]), vec![0x00]);
        assert_eq!(enc("hlt", &[]), vec![0x76]);
        assert_eq!(enc("ret", &[]), vec![0xC9]);
        assert_eq!(enc("xchg", &[]), vec![0xEB]);
    }

    #[test]
    fn operand_and_range_errors() {
        assert!(matches!(
            err("nop", &[imm(1)]),
            AsmErrorKind::OperandCount { .. }
        ));
        assert!(matches!(
            err("mvi", &[r8(A)]),
            AsmErrorKind::OperandCount { .. }
        ));
        assert!(matches!(
            err("frobnicate", &[]),
            AsmErrorKind::UnknownMnemonic(_)
        ));
        assert!(matches!(
            err("mvi", &[r8(A), imm(0x100)]),
            AsmErrorKind::ImmediateOutOfRange { max: 0xFF, .. }
        ));
        assert!(matches!(
            err("jmp", &[imm(0x10000)]),
            AsmErrorKind::ImmediateOutOfRange { max: 0xFFFF, .. }
        ));
        assert!(matches!(
            err("rst", &[imm(8)]),
            AsmErrorKind::ImmediateOutOfRange { max: 7, .. }
        ));
    }

    /// Every register/pair-parameterised byte the encoder computes must be a real opcode
    /// in the emulator's enum — this ties the assembler to the verified instruction set.
    #[test]
    fn every_encoded_byte_is_a_real_opcode() {
        use crate::instruction::opcode::Opcode;
        let r8s = [A, B, C, D, E, H, L, M];
        for &d in &r8s {
            for &s in &r8s {
                if d == M && s == M {
                    continue;
                }
                let bytes = enc("mov", &[r8(d), r8(s)]);
                assert!(
                    Opcode::from_byte(bytes[0]).is_ok(),
                    "MOV {d:?},{s:?} -> {:#04X}",
                    bytes[0]
                );
            }
        }
        for m in [
            "ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP", "INR", "DCR",
        ] {
            for &r in &r8s {
                let b = enc(m, &[r8(r)])[0];
                assert!(Opcode::from_byte(b).is_ok(), "{m} {r:?} -> {b:#04X}");
            }
        }
        for m in ["INX", "DCX", "DAD"] {
            for &rp in &[BC, DE, HL, SP] {
                let b = enc(m, &[r16(rp)])[0];
                assert!(Opcode::from_byte(b).is_ok(), "{m} {rp:?} -> {b:#04X}");
            }
        }
        for n in 0..=7u32 {
            let b = enc("rst", &[imm(n)])[0];
            assert!(Opcode::from_byte(b).is_ok(), "RST {n} -> {b:#04X}");
        }
    }
}
