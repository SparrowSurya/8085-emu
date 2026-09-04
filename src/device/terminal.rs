//! A terminal peripheral driven by a length-prefixed buffer and a command register.
//!
//! It exposes two I/O ports — a *command* port and a *data* port — and owns a 256-byte
//! buffer whose first byte is the payload length: `buffer[0] = len`, `buffer[1..=len]`
//! is the payload (so up to 255 payload bytes). A single auto-advancing cursor walks the
//! buffer during data transfers, which makes writing and reading perfectly symmetric and
//! needs no callbacks — output lands in a public `output` vector, input is taken from a
//! public `input` queue.
//!
//! Commands (written to the command port):
//!
//! | byte  | command   | effect                                                    |
//! |-------|-----------|-----------------------------------------------------------|
//! | `0x00`| `WRITE`   | enter write mode, cursor → 0; the CPU then streams the     |
//! |       |           | length byte and the payload to the data port              |
//! | `0x01`| `DISPLAY` | copy `buffer[1..=len]` to the output                      |
//! | `0x02`| `READ`    | capture a line of input into the buffer, cursor → 0; the  |
//! |       |           | CPU then reads the length byte and payload from data      |
//!
//! Because the length is stored in `buffer[0]`, the device knows exactly when a transfer
//! is complete — no terminator byte is needed, and the payload may contain any byte
//! (including `0x00`).

use super::Device;
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;

/// Total buffer size: one length byte plus up to 255 payload bytes.
pub const BUFFER_LEN: usize = 256;

/// Command: enter write mode (CPU will stream length + payload to the data port).
pub const CMD_WRITE: u8 = 0x00;
/// Command: emit the current buffer payload to the output.
pub const CMD_DISPLAY: u8 = 0x01;
/// Command: capture a line of input into the buffer (CPU will then read it back).
pub const CMD_READ: u8 = 0x02;

/// What the device is currently doing to its buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Not mid-transfer; data-port accesses do nothing useful.
    Idle,
    /// `WRITE` issued: data-port writes fill the buffer.
    Writing,
    /// `READ` issued: data-port reads drain the buffer.
    Reading,
}

/// A console mapped to a command port and a data port, backed by a length-prefixed buffer.
pub struct TerminalDevice {
    cmd_port: u8,
    data_port: u8,
    buffer: [u8; BUFFER_LEN],
    cursor: usize,
    mode: Mode,
    input: VecDeque<u8>,
    output: Vec<u8>,
    rx: Option<Receiver<u8>>,
    callback: Option<Box<dyn FnMut(u8) + Send + 'static>>,
}

impl TerminalDevice {
    /// A terminal on the given command and data ports.
    pub fn new(cmd_port: u8, data_port: u8) -> Self {
        TerminalDevice {
            cmd_port,
            data_port,
            buffer: [0; BUFFER_LEN],
            cursor: 0,
            mode: Mode::Idle,
            input: VecDeque::new(),
            output: Vec::new(),
            rx: None,
            callback: None,
        }
    }

    /// Creates a terminal with input receiver and output callback.
    pub fn with_io<F>(data_port: u8, cmd_port: u8, rx: Receiver<u8>, on_display: F) -> Self
    where
        F: FnMut(u8) + Send + 'static,
    {
        TerminalDevice {
            cmd_port,
            data_port,
            buffer: [0; BUFFER_LEN],
            cursor: 0,
            mode: Mode::Idle,
            input: VecDeque::new(),
            output: Vec::new(),
            rx: Some(rx),
            callback: Some(Box::new(on_display)),
        }
    }

    /// Host side: make `s` available as input bytes for a future `READ`.
    pub fn feed_input(&mut self, s: &str) {
        self.input.extend(s.bytes());
    }

    /// Host side: queue `s` followed by a newline (one line for a single `READ`).
    pub fn feed_line(&mut self, s: &str) {
        self.input.extend(s.bytes());
        self.input.push_back(b'\n');
    }

    /// The displayed output decoded lossily as UTF-8.
    pub fn output_string(&self) -> String {
        String::from_utf8_lossy(&self.output).into_owned()
    }

    /// Current payload string stored in buffer.
    pub fn payload_string(&self) -> String {
        let len = self.buffer[0] as usize;
        String::from_utf8_lossy(&self.buffer[1..=len]).into_owned()
    }

    /// Current payload bytes stored in buffer.
    pub fn payload(&self) -> Vec<u8> {
        let len = self.buffer[0] as usize;
        self.buffer[1..=len].to_vec()
    }

    /// The current payload length stored in `buffer[0]`.
    pub fn payload_len(&self) -> u8 {
        self.buffer[0]
    }

    /// `DISPLAY`: copy the payload to the output.
    fn emit(&mut self) {
        let len = self.buffer[0] as usize;
        if len > 0 {
            let payload = self.payload();
            self.output.extend_from_slice(&payload);
            for &byte in &payload {
                if let Some(ref mut cb) = self.callback {
                    cb(byte);
                }
            }
        }
    }

    /// `READ`: read the payload from input. The payload is stored from `buffer[1]` and
    /// does not store newline. Input is truncated if payload length exceeds 255 bytes.
    fn capture(&mut self) {
        if !self.input.contains(&b'\n') {
            if let Some(ref rx) = self.rx {
                while let Ok(byte) = rx.recv() {
                    let is_nl = byte == b'\n';
                    self.input.push_back(byte);
                    if is_nl {
                        break;
                    }
                }
            }
        }

        let mut line = Vec::new();
        while let Some(b) = self.input.pop_front() {
            if b == b'\n' {
                break;
            }
            if b == 0x08 || b == 0x7f {
                line.pop();
            } else {
                line.push(b);
            }
        }

        let len = line.len().min(BUFFER_LEN - 1);
        for i in 0..len {
            self.buffer[1 + i] = line[i];
        }
        self.buffer[0] = len as u8;
        self.cursor = 0;
        self.mode = Mode::Reading;
    }

    /// Returns a read-only view of the terminal's internal buffer.
    ///
    /// The returned slice borrows the buffer and does not copy its contents.
    /// Any modifications to the buffer must be performed through the terminal's
    /// other APIs.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }
}

impl Device for TerminalDevice {
    fn name(&self) -> &str {
        "TerminalDevice"
    }

    fn port_write(&mut self, port: u8, data: u8) {
        if port == self.cmd_port {
            match data {
                CMD_WRITE => {
                    self.mode = Mode::Writing;
                    self.cursor = 0;
                }
                CMD_DISPLAY => self.emit(),
                CMD_READ => self.capture(),
                _ => {}
            }
        } else if port == self.data_port && self.mode == Mode::Writing {
            if self.cursor < BUFFER_LEN {
                self.buffer[self.cursor] = data;
                self.cursor += 1;
            }
            // First byte set the length; once length+1 bytes are in, the transfer is done.
            let len = self.buffer[0] as usize;
            if self.cursor >= len + 1 {
                self.mode = Mode::Idle;
            }
        }
    }

    fn port_read(&mut self, port: u8) -> u8 {
        if port == self.data_port && self.mode == Mode::Reading {
            let byte = self.buffer.get(self.cursor).copied().unwrap_or(0);
            self.cursor += 1;
            let len = self.buffer[0] as usize;
            if self.cursor >= len + 1 {
                self.mode = Mode::Idle;
            }
            byte
        } else if port == self.cmd_port {
            0x00 // command port reads back as idle/ready
        } else {
            0x00
        }
    }

    fn tick(&mut self, _bus: &mut crate::bus::SystemBus) {
        if let Some(ref rx) = self.rx {
            while let Ok(byte) = rx.try_recv() {
                self.input.push_back(byte);
            }
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

    const CMD: u8 = 0x08;
    const DATA: u8 = 0x09;

    #[test]
    fn write_then_display_round_trips() {
        let mut t = TerminalDevice::new(CMD, DATA);
        t.port_write(CMD, CMD_WRITE);
        t.port_write(DATA, 2); // length
        t.port_write(DATA, b'H');
        t.port_write(DATA, b'I');
        t.port_write(CMD, CMD_DISPLAY);
        assert_eq!(t.payload_string(), "HI");
    }

    #[test]
    fn write_auto_completes_at_length_and_ignores_extra() {
        let mut t = TerminalDevice::new(CMD, DATA);
        t.port_write(CMD, CMD_WRITE);
        t.port_write(DATA, 1); // length 1
        t.port_write(DATA, b'X'); // payload -> transfer complete
        t.port_write(DATA, b'Y'); // stray write after completion: ignored
        t.port_write(CMD, CMD_DISPLAY);
        assert_eq!(t.payload_string(), "X");
    }

    #[test]
    fn empty_payload_displays_nothing() {
        let mut t = TerminalDevice::new(CMD, DATA);
        t.port_write(CMD, CMD_WRITE);
        t.port_write(DATA, 0); // length 0 -> immediately complete
        t.port_write(CMD, CMD_DISPLAY);
        assert!(t.payload_string().is_empty());
    }

    #[test]
    fn payload_is_binary_safe_including_nul() {
        let mut t = TerminalDevice::new(CMD, DATA);
        t.port_write(CMD, CMD_WRITE);
        for b in [3u8, 0x00, 0xFF, 0x01] {
            t.port_write(DATA, b);
        }
        t.port_write(CMD, CMD_DISPLAY);
        assert_eq!(t.payload(), vec![0x00, 0xFF, 0x01]);
    }

    #[test]
    fn read_fills_buffer_and_data_reads_length_then_payload() {
        let mut t = TerminalDevice::new(CMD, DATA);
        t.feed_line("OK");
        t.port_write(CMD, CMD_READ);
        assert_eq!(t.payload_len(), 2);
        assert_eq!(t.port_read(DATA), 2); // length first
        assert_eq!(t.port_read(DATA), b'O');
        assert_eq!(t.port_read(DATA), b'K');
        assert_eq!(t.port_read(DATA), 0); // past end -> idle, returns 0
    }

    #[test]
    fn read_then_display_echoes_without_touching_data_port() {
        // Because READ leaves the line in the buffer, DISPLAY alone echoes it.
        let mut t = TerminalDevice::new(CMD, DATA);
        t.feed_line("echo me");
        t.port_write(CMD, CMD_READ);
        t.port_write(CMD, CMD_DISPLAY);
        assert_eq!(t.payload_string(), "echo me");
    }
}
