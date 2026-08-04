//! Small newtype wrappers that keep address-like and port-like values from being
//! mixed up with plain data bytes. These replace the Python `Mem` newtype and the
//! role played by the `Data`/`Mem` wrapper classes, without dragging in an
//! arbitrary-width value type — 8085 data is just `u8`, addresses just `u16`.

use std::fmt;

/// A 16-bit memory address. Wrapping is deliberate: 8085 pointer arithmetic is
/// modulo 2^16, so `Addr` add/inc/dec never overflow-panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Addr(pub u16);

impl Addr {
    /// The address one past this one, wrapping at 0xFFFF (used for PC/SP stepping).
    #[inline]
    pub fn wrapping_add(self, rhs: u16) -> Addr {
        Addr(self.0.wrapping_add(rhs))
    }

    /// The address one before this one, wrapping at 0x0000.
    #[inline]
    pub fn wrapping_sub(self, rhs: u16) -> Addr {
        Addr(self.0.wrapping_sub(rhs))
    }

    /// The low byte (A0..A7), as placed on the multiplexed address/data bus.
    #[inline]
    pub fn low(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// The high byte (A8..A15).
    #[inline]
    pub fn high(self) -> u8 {
        (self.0 >> 8) as u8
    }

    /// Build an address from a low/high byte pair (8085 little-endian order).
    #[inline]
    pub fn from_le(low: u8, high: u8) -> Addr {
        Addr(u16::from(low) | (u16::from(high) << 8))
    }
}

impl From<u16> for Addr {
    fn from(v: u16) -> Self {
        Addr(v)
    }
}

impl From<Addr> for u16 {
    fn from(a: Addr) -> Self {
        a.0
    }
}

impl From<Addr> for usize {
    fn from(a: Addr) -> Self {
        a.0 as usize
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#06X}", self.0)
    }
}

/// An 8-bit I/O port number, used by `IN`/`OUT` and the device port map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Port(pub u8);

impl From<u8> for Port {
    fn from(v: u8) -> Self {
        Port(v)
    }
}

impl From<Port> for u8 {
    fn from(p: Port) -> Self {
        p.0
    }
}

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#04X}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addr_byte_split_and_join_roundtrip() {
        let a = Addr(0xBEEF);
        assert_eq!(a.low(), 0xEF);
        assert_eq!(a.high(), 0xBE);
        assert_eq!(Addr::from_le(a.low(), a.high()), a);
    }

    #[test]
    fn addr_arithmetic_wraps_modulo_16bit() {
        assert_eq!(Addr(0xFFFF).wrapping_add(1), Addr(0x0000));
        assert_eq!(Addr(0x0000).wrapping_sub(1), Addr(0xFFFF));
    }
}
