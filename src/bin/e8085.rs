//! Command-line front-end for running .e8085 programs.
//!
//! Usage:
//! ```text
//! cargo run --bin e8085 <file.e8085>
//! ```

use std::io::Write;
use std::sync::mpsc::channel;

use emu8085::asm::{assemble, load};
use emu8085::{Machine, TerminalDevice};

fn main() {
    let mut path: Option<String> = None;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!("usage: e8085 <file.e8085>");
                return;
            }
            file => path = Some(file.to_string()),
        }
    }

    let Some(path) = path else {
        eprintln!("usage: e8085 <file.e8085>");
        std::process::exit(2);
    };

    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("cannot read {path}: {e}");
        std::process::exit(2);
    });

    let image = match assemble(&src) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("{path}:{}: error: {}", e.span, e.kind);
            std::process::exit(1);
        }
    };

    let (tx, rx) = channel::<u8>();

    std::thread::spawn(move || {
        use std::io::{self, Read};
        let mut buf = [0u8; 1];
        while io::stdin().read_exact(&mut buf).is_ok() {
            if tx.send(buf[0]).is_err() {
                break;
            }
        }
    });

    let mut machine = Machine::default();

    let term_data_port: u8 = 0x01;
    let term_cmd_port: u8 = 0x02;
    let terminal = TerminalDevice::with_io(term_data_port, term_cmd_port, rx, |b| {
        print!("{}", b as char);
        let _ = std::io::stdout().flush();
    });

    machine.attach_device(Box::new(terminal), &[term_data_port, term_cmd_port]);

    if let Err(e) = load(&mut machine, &image) {
        eprintln!("load error: {e}");
        std::process::exit(1);
    }

    machine.run();

    if let Some(fault) = &machine.cpu.fault {
        eprintln!("\ncpu fault: {fault}");
        std::process::exit(1);
    }
}
