//! A printer peripheral: each `OUT` to its port appends a character to an output stream.

use super::Device;

/// Collects characters written by the CPU. Every byte is recorded in [`history`](Self::history);
/// an optional callback receives each character live (the Rust stand-in for the Python
/// `output_callback`, which defaulted to `print`).
#[derive(Default)]
pub struct PrinterDevice {
    /// Everything printed so far, in order.
    pub history: String,
    callback: Option<Box<dyn FnMut(char)>>,
}

impl PrinterDevice {
    /// A printer that only records to `history`.
    pub fn new() -> Self {
        Self::default()
    }

    /// A printer that also invokes `f` for each character as it is printed.
    pub fn with_callback(f: impl FnMut(char) + 'static) -> Self {
        PrinterDevice {
            history: String::new(),
            callback: Some(Box::new(f)),
        }
    }
}

impl Device for PrinterDevice {
    fn name(&self) -> &str {
        "PrinterDevice"
    }

    /// Append the written byte (as a Latin-1 character) to the output stream.
    fn port_write(&mut self, _port: u8, data: u8) {
        let ch = data as char;
        self.history.push(ch);
        if let Some(cb) = self.callback.as_mut() {
            cb(ch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn records_history_in_order() {
        let mut p = PrinterDevice::new();
        for b in b"Hi!" {
            p.port_write(0x02, *b);
        }
        assert_eq!(p.history, "Hi!");
    }

    #[test]
    fn callback_sees_each_char() {
        let seen = Rc::new(RefCell::new(String::new()));
        let sink = seen.clone();
        let mut p = PrinterDevice::with_callback(move |c| sink.borrow_mut().push(c));
        p.port_write(0, b'O');
        p.port_write(0, b'K');
        assert_eq!(*seen.borrow(), "OK");
        assert_eq!(p.history, "OK");
    }
}
