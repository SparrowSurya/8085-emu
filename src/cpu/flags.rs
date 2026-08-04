//! The 8085's flag register: Sign, Zero, Auxiliary Carry, Parity, and Carry.
//! Only these five bits are meaningful; the remaining bits exist so the whole
//! register can be pushed/popped as the low byte of the PSW.

/// The five architectural condition flags, stored as plain bools rather than a packed
/// byte so instruction logic reads clearly. Pack to / unpack from the PSW byte with
/// [`Flags::to_psw`] / [`Flags::from_psw`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Flags {
    /// Set when bit 7 of the result is 1 (result is negative in two's complement).
    pub sign: bool,
    /// Set when the result is zero.
    pub zero: bool,
    /// Half-carry out of bit 3 (used by `DAA`).
    pub aux_carry: bool,
    /// Set when the low byte of the result has *even* parity.
    pub parity: bool,
    /// Carry/borrow out of bit 7.
    pub carry: bool,
    /// The PSW's unused bits (1, 3, 5). The reference stores the flag register as a raw
    /// byte, so a value loaded by `POP PSW` keeps its unused bits until the next
    /// `POP PSW` — flag-setting instructions never touch them. Carrying them here keeps
    /// `PUSH PSW` byte-identical to the spec. (Real hardware instead forces bit 1 high
    /// and bits 3/5 low; the spec does neither, so neither do we.)
    unused: u8,
}

// PSW bit positions, matching the Python `FlagRegister` layout (which is also the
// real 8085 layout): CY=0, P=2, AC=4, Z=6, S=7. Bits 1/3/5 are unused.
const BIT_CARRY: u8 = 0;
const BIT_PARITY: u8 = 2;
const BIT_AUX: u8 = 4;
const BIT_ZERO: u8 = 6;
const BIT_SIGN: u8 = 7;

/// Mask of the PSW's unused bits (1, 3, 5).
const PSW_UNUSED_MASK: u8 = 0b0010_1010;

impl Flags {
    /// All flags clear.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pack the five flags into the PSW low byte, restoring any preserved unused bits.
    pub fn to_psw(self) -> u8 {
        (u8::from(self.carry) << BIT_CARRY)
            | (u8::from(self.parity) << BIT_PARITY)
            | (u8::from(self.aux_carry) << BIT_AUX)
            | (u8::from(self.zero) << BIT_ZERO)
            | (u8::from(self.sign) << BIT_SIGN)
            | (self.unused & PSW_UNUSED_MASK)
    }

    /// Unpack the PSW low byte into the five flags, retaining the unused bits verbatim.
    pub fn from_psw(byte: u8) -> Self {
        Flags {
            carry: (byte >> BIT_CARRY) & 1 == 1,
            parity: (byte >> BIT_PARITY) & 1 == 1,
            aux_carry: (byte >> BIT_AUX) & 1 == 1,
            zero: (byte >> BIT_ZERO) & 1 == 1,
            sign: (byte >> BIT_SIGN) & 1 == 1,
            unused: byte & PSW_UNUSED_MASK,
        }
    }

    /// Set Sign, Zero, and Parity from an 8-bit result. Carry and Aux-Carry are left
    /// untouched because they depend on the specific operation, not just the result.
    pub fn set_szp(&mut self, result: u8) {
        self.sign = result & 0x80 != 0;
        self.zero = result == 0;
        self.parity = even_parity(result);
    }
}

/// True when `byte` has an even number of set bits — the 8085's parity convention.
#[inline]
pub fn even_parity(byte: u8) -> bool {
    byte.count_ones() % 2 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parity_is_even_bit_count() {
        assert!(even_parity(0b0000_0000)); // 0 ones
        assert!(even_parity(0b0000_0011)); // 2 ones
        assert!(!even_parity(0b0000_0001)); // 1 one
        assert!(!even_parity(0b0000_0111)); // 3 ones
    }

    #[test]
    fn set_szp_flags_a_zero_result() {
        let mut f = Flags::new();
        f.set_szp(0x00);
        assert!(f.zero);
        assert!(!f.sign);
        assert!(f.parity); // zero has even (0) parity
    }

    #[test]
    fn set_szp_flags_a_negative_result() {
        let mut f = Flags::new();
        f.set_szp(0x80);
        assert!(f.sign);
        assert!(!f.zero);
        assert!(!f.parity); // single bit set -> odd
    }

    #[test]
    fn psw_roundtrips_through_the_five_flags() {
        let f = Flags {
            sign: true,
            zero: false,
            aux_carry: true,
            parity: false,
            carry: true,
            ..Flags::new()
        };
        assert_eq!(Flags::from_psw(f.to_psw()), f);
    }

    #[test]
    fn psw_preserves_unused_bits_across_pop_push() {
        // A byte loaded by POP PSW keeps its unused bits (1,3,5) through PUSH PSW,
        // matching the reference's raw-byte flag register.
        let loaded = Flags::from_psw(0b0010_1010); // only unused bits set
        assert_eq!(loaded.to_psw() & 0b0010_1010, 0b0010_1010);
        // ...and clearing the five meaningful flags doesn't disturb them.
        let mut f = Flags::from_psw(0xFF);
        f.set_szp(0x00); // recompute S/Z/P from a zero result
        assert_eq!(f.to_psw() & 0b0010_1010, 0b0010_1010);
    }

    #[test]
    fn psw_bit_positions_match_the_spec() {
        let only_carry = Flags {
            carry: true,
            ..Flags::new()
        };
        assert_eq!(only_carry.to_psw(), 0b0000_0001);

        let only_sign = Flags {
            sign: true,
            ..Flags::new()
        };
        assert_eq!(only_sign.to_psw(), 0b1000_0000);

        let only_zero = Flags {
            zero: true,
            ..Flags::new()
        };
        assert_eq!(only_zero.to_psw(), 0b0100_0000);
    }
}