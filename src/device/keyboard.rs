//! A keyboard peripheral: buffers ASCII key presses and, optionally, supplies an
//! `RST n` restart opcode during an INTA cycle.

use super::Device;
use std::collections::VecDeque;

/// Captures ASCII key presses (0–127) into a FIFO that the CPU drains with `IN`. If
/// configured with an interrupt vector (0–7), it answers an INTA cycle with the matching
/// `RST n` opcode.
#[derive(Debug, Default)]
pub struct KeyboardDevice {
    buffer: VecDeque<u8>,
    /// Restart vector (0–7) emitted on INTA, if any.
    pub interrupt_vector: Option<u8>,
}

impl KeyboardDevice {
    /// A keyboard with no interrupt vector (polled via `IN`).
    pub fn new() -> Self {
        Self::default()
    }

    /// A keyboard that drives `RST vector` on INTA.
    pub fn with_vector(vector: u8) -> Self {
        KeyboardDevice {
            buffer: VecDeque::new(),
            interrupt_vector: Some(vector),
        }
    }

    /// Queue a key press. ASCII must be 0–127; the buffer holds up to 255 pending keys.
    pub fn press(&mut self, key: u8) -> Result<(), &'static str> {
        if key > 127 {
            return Err("ASCII code out of range (0-127)");
        }
        if self.buffer.len() >= 255 {
            return Err("keyboard buffer overflow");
        }
        self.buffer.push_back(key);
        Ok(())
    }

    /// Queue a character key press (convenience over [`press`](Self::press)).
    pub fn press_char(&mut self, c: char) -> Result<(), &'static str> {
        let code = c as u32;
        if code > 127 {
            return Err("ASCII code out of range (0-127)");
        }
        self.press(code as u8)
    }

    /// Whether a key is waiting to be read.
    pub fn has_key(&self) -> bool {
        !self.buffer.is_empty()
    }
}

impl Device for KeyboardDevice {
    fn name(&self) -> &str {
        "KeyboardDevice"
    }

    /// Return the next buffered key, or `0x00` when the buffer is empty.
    fn port_read(&mut self, _port: u8) -> u8 {
        self.buffer.pop_front().unwrap_or(0x00)
    }

    /// Emit `RST n` (`0xC7 | n << 3`) for a configured vector 0–7, else `0xFF`.
    fn on_inta(&mut self) -> u8 {
        match self.interrupt_vector {
            Some(v) if v <= 7 => 0xC7 | ((v & 0x07) << 3),
            _ => 0xFF,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_read_order_and_empty_sentinel() {
        let mut kb = KeyboardDevice::new();
        assert!(!kb.has_key());
        kb.press_char('A').unwrap();
        kb.press(66).unwrap();
        assert!(kb.has_key());
        assert_eq!(kb.port_read(1), 65);
        assert_eq!(kb.port_read(1), 66);
        assert_eq!(kb.port_read(1), 0x00); // drained
        assert!(!kb.has_key());
    }

    #[test]
    fn rejects_non_ascii() {
        let mut kb = KeyboardDevice::new();
        assert!(kb.press(128).is_err());
    }

    #[test]
    fn inta_returns_rst_opcode() {
        assert_eq!(KeyboardDevice::with_vector(1).on_inta(), 0xCF); // RST 1
        assert_eq!(KeyboardDevice::with_vector(3).on_inta(), 0xDF); // RST 3
        assert_eq!(KeyboardDevice::with_vector(2).on_inta(), 0xD7); // RST 2
        assert_eq!(KeyboardDevice::new().on_inta(), 0xFF); // no vector
    }
}
