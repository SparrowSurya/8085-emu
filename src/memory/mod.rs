//! Flat, byte-addressable RAM. Memory only ever moves data in response to the bus:
//! [`Memory::step`] honours the memory-read/write strobes the CPU (or a DMA device)
//! has driven, matching the reference behavioral specification.

use crate::bus::SystemBus;
use crate::error::EmuError;
use crate::value::Addr;

/// A block of RAM sized to a number of address lines (2^lines bytes).
///
/// Bus-driven access ([`read`](Memory::read)/[`write`](Memory::write)) wraps at the
/// memory boundary, mirroring real address-line aliasing and keeping the hot path
/// panic-free. Bulk loading ([`load_bytes`](Memory::load_bytes)) is bounds-checked and
/// returns [`EmuError::AddressOutOfBounds`] so a program that overruns RAM is reported
/// rather than silently wrapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    data: Vec<u8>,
    addr_mask: usize,
    /// Optional upper valid address limit (exclusive). Accesses at or beyond this
    /// limit trigger an illegal memory access hardware TRAP.
    pub valid_limit: Option<usize>,
}

impl Memory {
    /// Allocate RAM for `lines` address lines: 2^lines bytes, all zeroed.
    pub fn from_lines(lines: u32) -> Self {
        let size = 1usize << lines;
        Memory {
            data: vec![0; size],
            addr_mask: size - 1,
            valid_limit: if lines < 16 { Some(size) } else { None },
        }
    }

    /// Sets an upper valid address bound (exclusive) for memory access.
    pub fn set_limit(&mut self, limit: usize) {
        self.valid_limit = Some(limit);
    }

    /// Whether an address is valid within addressable RAM and any configured limit.
    #[inline]
    pub fn is_valid_address(&self, addr: Addr) -> bool {
        let raw = usize::from(addr);
        if raw >= self.data.len() {
            return false;
        }
        if let Some(limit) = self.valid_limit {
            if raw >= limit {
                return false;
            }
        }
        true
    }

    /// Total addressable bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the RAM is empty (never true for a `from_lines` memory, but clippy asks).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Read a byte, wrapping the address at the memory boundary.
    #[inline]
    pub fn read(&self, addr: Addr) -> u8 {
        self.data[usize::from(addr) & self.addr_mask]
    }

    /// Write a byte, wrapping the address at the memory boundary.
    #[inline]
    pub fn write(&mut self, addr: Addr, val: u8) {
        let idx = usize::from(addr) & self.addr_mask;
        self.data[idx] = val;
    }

    /// Copy a byte slice into memory starting at `start`, returning how many bytes were
    /// written. Errors if the slice would run past the end of RAM. The program compiler
    /// yields a flat little-endian byte stream, so there is nothing width-tagged to unpack.
    pub fn load_bytes(&mut self, bytes: &[u8], start: Addr) -> Result<usize, EmuError> {
        let end = usize::from(start) + bytes.len();
        if end > self.len() {
            return Err(EmuError::AddressOutOfBounds {
                addr: (end.saturating_sub(1)) as u16,
                size: self.len(),
            });
        }
        let base = usize::from(start);
        self.data[base..end].copy_from_slice(bytes);
        Ok(bytes.len())
    }

    /// Service one bus transaction: on a memory-read strobe, drive the addressed byte
    /// onto the data bus; on a memory-write strobe, store the data bus byte.
    /// Accesses to illegal or out-of-bounds addresses assert `bus.lines.trap = true`.
    pub fn step(&mut self, bus: &mut SystemBus) {
        let addr = bus.address();
        if bus.lines.mr {
            if !self.is_valid_address(addr) {
                bus.lines.trap = true;
            } else {
                let byte = self.read(addr);
                bus.set_data(byte);
            }
        } else if bus.lines.mw {
            if !self.is_valid_address(addr) {
                bus.lines.trap = true;
            } else {
                self.write(addr, bus.data());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_lines_sizes_to_power_of_two() {
        assert_eq!(Memory::from_lines(16).len(), 65536);
        assert_eq!(Memory::from_lines(12).len(), 4096);
    }

    #[test]
    fn read_write_roundtrip() {
        let mut m = Memory::from_lines(16);
        m.write(Addr(0x2000), 0x42);
        assert_eq!(m.read(Addr(0x2000)), 0x42);
    }

    #[test]
    fn access_wraps_at_boundary() {
        let mut m = Memory::from_lines(12); // 4K
        m.write(Addr(0x0001), 0xAB);
        // 0x1001 aliases to 0x0001 in a 4K space.
        assert_eq!(m.read(Addr(0x1001)), 0xAB);
    }

    #[test]
    fn load_bytes_reports_overflow() {
        let mut m = Memory::from_lines(8); // 256 bytes
        let err = m.load_bytes(&[0u8; 4], Addr(0x00FE)).unwrap_err();
        assert!(matches!(err, EmuError::AddressOutOfBounds { .. }));
        // A slice that just fits is fine.
        assert_eq!(m.load_bytes(&[1, 2], Addr(0x00FE)).unwrap(), 2);
        assert_eq!(m.read(Addr(0x00FF)), 2);
    }

    #[test]
    fn step_honours_mr_and_mw() {
        let mut m = Memory::from_lines(16);
        let mut bus = SystemBus::default();

        // Write path: CPU drives address, data, and MW.
        bus.set_address(Addr(0x0100));
        bus.set_data(0x7E);
        bus.lines.mw = true;
        m.step(&mut bus);
        assert_eq!(m.read(Addr(0x0100)), 0x7E);

        // Read path: CPU drives address and MR, expects data back on the bus.
        bus.lines.mw = false;
        bus.lines.mr = true;
        bus.set_data(0x00);
        m.step(&mut bus);
        assert_eq!(bus.data(), 0x7E);
    }
}
