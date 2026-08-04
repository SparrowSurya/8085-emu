//! The system bus: the shared address bus, data bus, and control/handshake lines
//! that connect the CPU, memory, and devices.
//!
//! The 8085 multiplexes address and data onto shared pins, but for emulation it is
//! clearer to model the 16-bit address value, the 8-bit data value, and the
//! [`ControlLines`] as separate typed fields. Sub-width configurations (fewer address
//! or data lines than 16/8) are honored by masking on write, matching the Python
//! `SystemBus.create(address_lines, data_lines)`.

pub mod lines;

pub use lines::ControlLines;

use crate::value::Addr;

/// The wires between components. Components read the lines they care about and drive
/// the ones they own; nothing here decides *what* a transaction means — that is the
/// job of the CPU, memory, and device manager that share it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemBus {
    address: u16,
    data: u8,
    address_mask: u16,
    data_mask: u8,
    /// The control, status, and handshake lines.
    pub lines: ControlLines,
}

impl Default for SystemBus {
    fn default() -> Self {
        SystemBus::new(16, 8)
    }
}

impl SystemBus {
    /// Create a bus with the given number of address and data lines (16/8 for a stock
    /// 8085). `ready` idles high. Widths beyond the type sizes are clamped.
    pub fn new(address_lines: u32, data_lines: u32) -> Self {
        let address_mask = mask_u16(address_lines);
        let data_mask = mask_u8(data_lines);
        SystemBus {
            address: 0,
            data: 0,
            address_mask,
            data_mask,
            lines: ControlLines::new(),
        }
    }

    /// The address currently on the bus.
    #[inline]
    pub fn address(&self) -> Addr {
        Addr(self.address)
    }

    /// Drive an address onto the bus (masked to the configured address width).
    #[inline]
    pub fn set_address(&mut self, addr: Addr) {
        self.address = addr.0 & self.address_mask;
    }

    /// The data byte currently on the bus.
    #[inline]
    pub fn data(&self) -> u8 {
        self.data
    }

    /// Drive a data byte onto the bus (masked to the configured data width).
    #[inline]
    pub fn set_data(&mut self, data: u8) {
        self.data = data & self.data_mask;
    }
}

/// A `count`-bit mask, saturating at all 16 bits.
fn mask_u16(count: u32) -> u16 {
    if count >= 16 {
        u16::MAX
    } else {
        (1u16 << count) - 1
    }
}

/// A `count`-bit mask, saturating at all 8 bits.
fn mask_u8(count: u32) -> u8 {
    if count >= 8 {
        u8::MAX
    } else {
        (1u8 << count) - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_idles_high() {
        assert!(SystemBus::default().lines.ready);
    }

    #[test]
    fn address_and_data_masked_to_width() {
        // 12 address lines, 4 data lines: high bits should be dropped.
        let mut bus = SystemBus::new(12, 4);
        bus.set_address(Addr(0xFFFF));
        assert_eq!(bus.address(), Addr(0x0FFF));
        bus.set_data(0xFF);
        assert_eq!(bus.data(), 0x0F);
    }

    #[test]
    fn reset_clears_strobes_but_not_handshakes() {
        let mut bus = SystemBus::default();
        bus.lines.mr = true;
        bus.lines.inta = true;
        bus.lines.hold = true;
        bus.lines.reset_out = true;
        bus.lines.reset();
        assert!(!bus.lines.mr);
        assert!(!bus.lines.inta);
        assert!(bus.lines.hold); // handshake line untouched
        assert!(bus.lines.reset_out); // reset line untouched
    }
}