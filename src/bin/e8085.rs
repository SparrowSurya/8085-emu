//! Command-line interface for the Intel 8085 emulator and assembler toolchain.

use std::io::Write;
use std::sync::mpsc::channel;

use clap::{Parser, Subcommand};
use emu8085::asm::container::{BinaryContainer, CONTAINER_MAGIC};
use emu8085::asm::{assemble, load};
use emu8085::{Addr, Machine, TerminalDevice};

/// Maximum 8085 RAM capacity (64 KB).
const MAX_RAM_SIZE: usize = 65536;

const TERM_DATA_PORT: u8 = 0x01;
const TERM_CMD_PORT: u8 = 0x02;

#[derive(Parser)]
#[command(name = "e8085")]
#[command(author, version, about = "Cycle-accurate Intel 8085 microprocessor emulator & assembler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a .e8085 assembly source file or .8085.bin binary image
    Run {
        /// Input file to execute (.e8085 or .8085.bin)
        file: String,
    },

    /// Compile a .e8085 assembly source file into a .8085.bin binary image
    Compile {
        /// Input assembly file (.e8085 or .asm)
        file: String,

        /// Output binary file path (default: <filename>.8085.bin)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Disassemble a .8085.bin binary image into 8085 assembly instructions
    Disassemble {
        /// Binary file to disassemble (.8085.bin or .bin)
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => run_file(&file),
        Commands::Compile { file, output } => compile_file(&file, output.as_deref()),
        Commands::Disassemble { file } => disassemble_file(&file),
    }
}

fn disassemble_file(path: &str) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("cannot read binary file '{path}': {e}");
        std::process::exit(2);
    });

    let rows = emu8085::disassemble_bytes(&bytes).unwrap_or_else(|e| {
        eprintln!("error disassembling '{path}': {e}");
        std::process::exit(1);
    });

    for r in rows {
        println!("{r}");
    }
}

fn run_file(path: &str) {
    let mut machine = Machine::default();

    if path.ends_with(".bin") || path.ends_with(".8085.bin") {
        let bytes = std::fs::read(path).unwrap_or_else(|e| {
            eprintln!("cannot read binary file '{path}': {e}");
            std::process::exit(2);
        });

        if bytes.len() >= 4 && &bytes[0..4] == &CONTAINER_MAGIC {
            let container = BinaryContainer::decode(&bytes).unwrap_or_else(|e| {
                eprintln!("error reading container '{path}': {e}");
                std::process::exit(1);
            });

            // Load vector table if present
            if !container.vec_bytes.is_empty() {
                for (i, &b) in container.vec_bytes.iter().enumerate() {
                    machine.ram.write(Addr(i as u16), b);
                }
            }

            // Load .data
            for (i, &b) in container.data_bytes.iter().enumerate() {
                machine
                    .ram
                    .write(Addr(container.header.data_addr.wrapping_add(i as u16)), b);
            }

            // Zero .bss
            for i in 0..container.header.bss_size {
                machine
                    .ram
                    .write(Addr(container.header.bss_addr.wrapping_add(i)), 0);
            }

            // Load .text
            for (i, &b) in container.text_bytes.iter().enumerate() {
                machine
                    .ram
                    .write(Addr(container.header.text_addr.wrapping_add(i as u16)), b);
            }

            machine.cpu.regs.pc = Addr(container.header.entry_pc);
            machine.cpu.regs.sp = Addr(container.header.sp_init);
        } else {
            // Flat binary fallback
            if bytes.len() > MAX_RAM_SIZE {
                eprintln!(
                    "error: binary size ({} bytes) exceeds 64KB RAM capacity ({} bytes)",
                    bytes.len(),
                    MAX_RAM_SIZE
                );
                std::process::exit(1);
            }

            for (i, &b) in bytes.iter().enumerate() {
                machine.ram.write(Addr(i as u16), b);
            }
            machine.cpu.regs.pc = Addr(0x0000);
            machine.cpu.regs.sp = Addr(0xFFFF);
        }
    } else {
        let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("cannot read assembly file '{path}': {e}");
            std::process::exit(2);
        });

        let image = match assemble(&src) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("{path}:{}: error: {}", e.span, e.kind);
                std::process::exit(1);
            }
        };

        if image.bytes.len() > MAX_RAM_SIZE {
            eprintln!(
                "error: assembled image size ({} bytes) exceeds 64KB RAM capacity ({} bytes)",
                image.bytes.len(),
                MAX_RAM_SIZE
            );
            std::process::exit(1);
        }

        if let Err(e) = load(&mut machine, &image) {
            eprintln!("load error: {e}");
            std::process::exit(1);
        }
    }

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

    let terminal = TerminalDevice::with_io(TERM_DATA_PORT, TERM_CMD_PORT, rx, |b| {
        print!("{}", b as char);
        let _ = std::io::stdout().flush();
    });

    machine.attach_device(Box::new(terminal), &[TERM_DATA_PORT, TERM_CMD_PORT]);
    machine.run();

    if let Some(fault) = &machine.cpu.fault {
        eprintln!("\ncpu fault: {fault}");
        std::process::exit(1);
    }
}

fn compile_file(input_path: &str, output_path: Option<&str>) {
    let src = std::fs::read_to_string(input_path).unwrap_or_else(|e| {
        eprintln!("cannot read assembly file '{input_path}': {e}");
        std::process::exit(2);
    });

    let image = match assemble(&src) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("{input_path}:{}: error: {}", e.span, e.kind);
            std::process::exit(1);
        }
    };

    if image.bytes.len() > MAX_RAM_SIZE {
        eprintln!(
            "error: assembled image size ({} bytes) exceeds 64KB RAM capacity ({} bytes)",
            image.bytes.len(),
            MAX_RAM_SIZE
        );
        std::process::exit(1);
    }

    let container = image.to_container();
    let container_bytes = container.encode();

    let out_path = match output_path {
        Some(p) => p.to_string(),
        None => {
            if let Some(stripped) = input_path.strip_suffix(".e8085") {
                format!("{stripped}.8085.bin")
            } else if let Some(stripped) = input_path.strip_suffix(".asm") {
                format!("{stripped}.8085.bin")
            } else {
                format!("{input_path}.8085.bin")
            }
        }
    };

    std::fs::write(&out_path, &container_bytes).unwrap_or_else(|e| {
        eprintln!("cannot write binary output file '{out_path}': {e}");
        std::process::exit(2);
    });

    println!(
        "Compiled '{input_path}' -> '{out_path}' ({} bytes, text: {}B, data: {}B)",
        container_bytes.len(),
        container.header.text_size,
        container.header.data_size
    );
}
