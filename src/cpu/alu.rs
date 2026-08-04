//! The arithmetic-logic unit as pure functions over `u8` and [`Flags`].
//!
//! Every operation here is a direct port of the corresponding Python `_ts_exec_*`
//! method, kept side-effect-free so it can be unit-tested and differentially checked
//! against the reference (see the `alu_vectors` replay test). The execution unit calls
//! these; it never recomputes flags itself.

use super::flags::{even_parity, Flags};

/// Add `b` (+ optional carry-in) to `a`. Sets all five flags; returns the 8-bit result.
/// Used by `ADD`/`ADI` (carry_in = false) and `ADC`/`ACI` (carry_in = current carry).
pub fn add(f: &mut Flags, a: u8, b: u8, carry_in: bool) -> u8 {
    let cin = carry_in as u16;
    let res = a as u16 + b as u16 + cin;
    let res8 = (res & 0xFF) as u8;
    let res4 = (a & 0x0F) as u16 + (b & 0x0F) as u16 + cin;

    f.carry = (res >> 8) & 1 == 1;
    f.aux_carry = (res4 >> 4) & 1 == 1;
    set_szp(f, res8);
    res8
}

/// Subtract `b` (+ optional borrow-in) from `a`, borrow convention: carry set on borrow.
/// Used by `SUB`/`SUI` (borrow_in = false) and `SBB`/`SBI` (borrow_in = current carry).
pub fn sub(f: &mut Flags, a: u8, b: u8, borrow_in: bool) -> u8 {
    let bin = borrow_in as i32;
    let res = a as i32 - b as i32 - bin;
    let res8 = (res & 0xFF) as u8;
    let res4 = (a & 0x0F) as i32 - (b & 0x0F) as i32 - bin;

    f.carry = res < 0;
    f.aux_carry = res4 < 0;
    set_szp(f, res8);
    res8
}

/// Compare `a` with `b`: the flags of `a - b`, but no result is stored (`CMP`/`CPI`).
pub fn cmp(f: &mut Flags, a: u8, b: u8) {
    let _ = sub(f, a, b, false);
}

/// Bitwise AND. Clears carry and (per 8085) *sets* the auxiliary carry.
pub fn and(f: &mut Flags, a: u8, b: u8) -> u8 {
    let res = a & b;
    f.carry = false;
    f.aux_carry = true;
    set_szp(f, res);
    res
}

/// Bitwise OR. Clears carry and auxiliary carry.
pub fn or(f: &mut Flags, a: u8, b: u8) -> u8 {
    let res = a | b;
    f.carry = false;
    f.aux_carry = false;
    set_szp(f, res);
    res
}

/// Bitwise XOR. Clears carry and auxiliary carry.
pub fn xor(f: &mut Flags, a: u8, b: u8) -> u8 {
    let res = a ^ b;
    f.carry = false;
    f.aux_carry = false;
    set_szp(f, res);
    res
}

/// Increment. Affects S/Z/AC/P but *not* carry (which is preserved).
pub fn inr(f: &mut Flags, v: u8) -> u8 {
    let res = v.wrapping_add(1);
    let res4 = (v & 0x0F) as u16 + 1;
    f.aux_carry = (res4 >> 4) & 1 == 1;
    set_szp(f, res);
    res
}

/// Decrement. Affects S/Z/AC/P but *not* carry (which is preserved).
pub fn dcr(f: &mut Flags, v: u8) -> u8 {
    let res = v.wrapping_sub(1);
    let res4 = (v & 0x0F) as i32 - 1;
    f.aux_carry = res4 < 0;
    set_szp(f, res);
    res
}

/// Rotate accumulator left circular; carry receives the bit rotated out (old bit 7).
pub fn rlc(f: &mut Flags, a: u8) -> u8 {
    let bit7 = (a >> 7) & 1;
    f.carry = bit7 == 1;
    ((a << 1) & 0xFE) | bit7
}

/// Rotate accumulator right circular; carry receives old bit 0.
pub fn rrc(f: &mut Flags, a: u8) -> u8 {
    let bit0 = a & 1;
    f.carry = bit0 == 1;
    ((a >> 1) & 0x7F) | (bit0 << 7)
}

/// Rotate accumulator left through carry; carry receives old bit 7.
pub fn ral(f: &mut Flags, a: u8) -> u8 {
    let cin = f.carry as u8;
    let bit7 = (a >> 7) & 1;
    f.carry = bit7 == 1;
    ((a << 1) & 0xFE) | cin
}

/// Rotate accumulator right through carry; carry receives old bit 0.
pub fn rar(f: &mut Flags, a: u8) -> u8 {
    let cin = f.carry as u8;
    let bit0 = a & 1;
    f.carry = bit0 == 1;
    ((a >> 1) & 0x7F) | (cin << 7)
}

/// Complement the accumulator. No flags change.
pub fn cma(a: u8) -> u8 {
    !a
}

/// Decimal-adjust the accumulator after a BCD addition. Sets all five flags.
pub fn daa(f: &mut Flags, a: u8) -> u8 {
    let mut inc: u16 = 0;
    let mut carry = f.carry;
    let mut aux = false;

    if (a & 0x0F) > 9 || f.aux_carry {
        inc += 0x06;
        aux = true;
    }
    if a > 0x99 || f.carry {
        inc += 0x60;
        carry = true;
    }

    let res = ((a as u16 + inc) & 0xFF) as u8;
    f.carry = carry;
    f.aux_carry = aux;
    set_szp(f, res);
    res
}

/// Add a register pair to HL (`DAD`); only carry (out of bit 15) is affected.
pub fn dad(f: &mut Flags, hl: u16, rp: u16) -> u16 {
    let res = hl as u32 + rp as u32;
    f.carry = (res >> 16) & 1 == 1;
    (res & 0xFFFF) as u16
}

/// Set Sign, Zero, and Parity from a result byte (Carry/AuxCarry handled per-op).
fn set_szp(f: &mut Flags, res: u8) {
    f.sign = res & 0x80 != 0;
    f.zero = res == 0;
    f.parity = even_parity(res);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::alu_vectors::ALU_VECTORS;

    fn flags_with(carry: bool, aux: bool) -> Flags {
        let mut f = Flags::new();
        f.carry = carry;
        f.aux_carry = aux;
        f
    }

    /// Replay all 1,160 vectors captured from the Python reference and require an exact
    /// match on both the result byte and the full packed PSW.
    #[test]
    fn matches_python_reference_exactly() {
        for &(op, a, b, cin, exp_res, exp_psw) in ALU_VECTORS {
            let cin_b = cin == 1;
            let (mut f, res) = match op {
                0 => run(a, b, false, |f| add(f, a, b, false)),
                1 => run(a, b, cin_b, |f| add(f, a, b, cin_b)),
                2 => run(a, b, false, |f| sub(f, a, b, false)),
                3 => run(a, b, cin_b, |f| sub(f, a, b, cin_b)),
                4 => run(a, b, false, |f| and(f, a, b)),
                5 => run(a, b, false, |f| or(f, a, b)),
                6 => run(a, b, false, |f| xor(f, a, b)),
                7 => {
                    let mut f = flags_with(false, false);
                    cmp(&mut f, a, b);
                    (f, a) // CMP leaves A unchanged
                }
                8 => run_pre(cin_b, |f| inr(f, a)),
                9 => run_pre(cin_b, |f| dcr(f, a)),
                10 => run_pre(cin_b, |f| rlc(f, a)),
                11 => run_pre(cin_b, |f| rrc(f, a)),
                12 => run_pre(cin_b, |f| ral(f, a)),
                13 => run_pre(cin_b, |f| rar(f, a)),
                14 => {
                    // DAA: b column carries aux-carry-in.
                    let mut f = flags_with(cin_b, b == 1);
                    let r = daa(&mut f, a);
                    (f, r)
                }
                other => panic!("unknown op id {other}"),
            };
            let got_psw = f.to_psw();
            assert_eq!(
                (res, got_psw),
                (exp_res, exp_psw),
                "op={op} a={a:#04x} b={b:#04x} cin={cin}: got (res={res:#04x}, psw={got_psw:#04x}) expected (res={exp_res:#04x}, psw={exp_psw:#04x})"
            );
            let _ = &mut f;
        }
    }

    fn run(_a: u8, _b: u8, carry_in: bool, op: impl FnOnce(&mut Flags) -> u8) -> (Flags, u8) {
        let mut f = flags_with(carry_in, false);
        let r = op(&mut f);
        (f, r)
    }

    fn run_pre(carry_in: bool, op: impl FnOnce(&mut Flags) -> u8) -> (Flags, u8) {
        let mut f = flags_with(carry_in, false);
        let r = op(&mut f);
        (f, r)
    }

    #[test]
    fn add_sets_carry_and_aux() {
        let mut f = Flags::new();
        assert_eq!(add(&mut f, 0xFF, 0x01, false), 0x00);
        assert!(f.carry);
        assert!(f.aux_carry);
        assert!(f.zero);
    }

    #[test]
    fn sub_sets_borrow() {
        let mut f = Flags::new();
        assert_eq!(sub(&mut f, 0x00, 0x01, false), 0xFF);
        assert!(f.carry); // borrow
        assert!(f.sign);
    }

    #[test]
    fn ana_sets_aux_carry() {
        let mut f = Flags::new();
        and(&mut f, 0x0F, 0xF0);
        assert!(f.aux_carry); // 8085 quirk: ANA always sets AC
        assert!(f.zero);
        assert!(!f.carry);
    }
}