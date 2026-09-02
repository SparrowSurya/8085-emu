//! Command-line interface for the Intel 8085 emulator and assembler toolchain.

use std::io::Write;
use std::path::Path;
use std::sync::mpsc::channel;

use clap::{Parser, Subcommand};
use emu8085::asm::assemble_and_link;
use emu8085::asm::container::{BinaryContainer, CONTAINER_MAGIC};
use emu8085::asm::load;
use emu8085::{Addr, DisassembleOptions, InspectOptions, Machine, TerminalDevice};

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

        /// Additional library containers (.8085.bin) to link when running a source file
        #[arg(short = 'l', long = "link")]
        link: Vec<String>,
    },

    /// Compile a .e8085 assembly source file into a .8085.bin binary image
    Compile {
        /// Input assembly file (.e8085 or .asm)
        file: String,

        /// Additional library containers (.8085.bin) to link
        #[arg(short = 'l', long = "link")]
        link: Vec<String>,

        /// Output binary path (defaults to <name>.8085.bin)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Disassemble a .8085.bin binary image into 8085 assembly instructions
    Disassemble {
        /// Binary file to disassemble (.8085.bin or .bin)
        file: String,

        /// Enable ANSI colored disassembly output
        #[arg(long = "color")]
        color: bool,

        /// Display hardware T-state cycle counts
        #[arg(short = 'c', long = "cycles")]
        cycles: bool,

        /// Include interrupt vector table disassembly
        #[arg(short = 'V', long = "vectors")]
        vectors: bool,
    },

    /// Inspect a .8085.bin binary container (header, segments, symbols, strings)
    Inspect {
        /// Binary file to inspect (.8085.bin or .bin)
        file: String,

        /// Show all diagnostic sections (default behavior; overrides individual flags)
        #[arg(short = 'a', long = "all")]
        all: bool,

        /// Show container header information only
        #[arg(short = 'H', long = "header")]
        header: bool,

        /// Show segment layout, boundaries, and offsets only
        #[arg(short = 'S', long = "segments")]
        segments: bool,

        /// Show exported symbols and main entry point only
        #[arg(short = 's', long = "symbols")]
        symbols: bool,

        /// Extract and display embedded printable strings only
        #[arg(short = 't', long = "strings")]
        strings: bool,

        /// Minimum string length for extraction (default: 3)
        #[arg(short = 'n', long = "min-len", default_value = "3")]
        min_len: usize,
    },

    /// Extract printable ASCII strings from a binary container or file
    Strings {
        /// Binary file to inspect (.8085.bin or .bin)
        file: String,

        /// Minimum string length (default: 3)
        #[arg(short = 'n', long = "min-len", default_value = "3")]
        min_len: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, link } => run_file(&file, &link),
        Commands::Compile { file, link, output } => compile_file(&file, &link, output.as_deref()),
        Commands::Disassemble {
            file,
            color,
            cycles,
            vectors,
        } => disassemble_file(&file, color, cycles, vectors),
        Commands::Inspect {
            file,
            all,
            header,
            segments,
            symbols,
            strings,
            min_len,
        } => inspect_file(&file, all, header, segments, symbols, strings, min_len),
        Commands::Strings { file, min_len } => strings_file(&file, min_len),
    }
}

fn inspect_file(
    path: &str,
    all: bool,
    header: bool,
    segments: bool,
    symbols: bool,
    strings: bool,
    min_len: usize,
) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("cannot read binary file '{path}': {e}");
        std::process::exit(2);
    });

    if bytes.len() < 4 || &bytes[0..4] != CONTAINER_MAGIC {
        eprintln!("error: file '{path}' is not a valid .8085.bin binary container");
        std::process::exit(1);
    }

    let container = BinaryContainer::decode(&bytes).unwrap_or_else(|e| {
        eprintln!("error reading container '{path}': {e}");
        std::process::exit(1);
    });

    let is_selective = header || segments || symbols || strings;
    let show_all = all || !is_selective;

    let options = if show_all {
        InspectOptions {
            show_header: true,
            show_segments: true,
            show_symbols: true,
            show_strings: true,
            min_string_len: min_len,
        }
    } else {
        InspectOptions {
            show_header: header,
            show_segments: segments,
            show_symbols: symbols,
            show_strings: strings,
            min_string_len: min_len,
        }
    };

    let report = emu8085::inspect_container(&container, bytes.len(), &options);
    print!("{report}");
}

fn strings_file(path: &str, min_len: usize) {
    inspect_file(path, false, false, false, false, true, min_len);
}

fn disassemble_file(path: &str, color: bool, cycles: bool, vectors: bool) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("cannot read binary file '{path}': {e}");
        std::process::exit(2);
    });

    if bytes.len() < 4 || &bytes[0..4] != CONTAINER_MAGIC {
        eprintln!("error: file '{path}' is not a valid .8085.bin binary container");
        std::process::exit(1);
    }

    let container = BinaryContainer::decode(&bytes).unwrap_or_else(|e| {
        eprintln!("error reading container '{path}': {e}");
        std::process::exit(1);
    });

    let options = DisassembleOptions {
        color,
        show_cycles: cycles,
        show_vectors: vectors,
        show_banners: true,
    };

    let rows = emu8085::disassemble_container_with_options(&container, &options);

    for r in rows {
        if color {
            println!("{}", r.to_colored_string());
        } else {
            println!("{r}");
        }
    }
}

fn load_external_symbols(link_paths: &[String]) -> Vec<BinaryContainer> {
    let mut containers = Vec::new();

    for path in link_paths {
        if !path.ends_with(".8085.bin") && !path.ends_with(".bin") {
            eprintln!(
                "error: linked library '{path}' must be a compiled .8085.bin binary container"
            );
            std::process::exit(1);
        }

        let bytes = std::fs::read(path).unwrap_or_else(|e| {
            eprintln!("cannot read library file '{path}': {e}");
            std::process::exit(2);
        });

        if bytes.len() < 4 || bytes[0..4] != CONTAINER_MAGIC {
            eprintln!("error: linked library '{path}' is not a valid .8085.bin binary container");
            std::process::exit(1);
        }

        let container = BinaryContainer::decode(&bytes).unwrap_or_else(|e| {
            eprintln!("error reading library container '{path}': {e}");
            std::process::exit(1);
        });

        containers.push(container);
    }

    containers
}

fn load_container_into_machine(machine: &mut Machine, container: &BinaryContainer) {
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
}

fn run_file(path: &str, link: &[String]) {
    let mut machine = Machine::default();

    if path.ends_with(".bin") || path.ends_with(".8085.bin") {
        let bytes = std::fs::read(path).unwrap_or_else(|e| {
            eprintln!("cannot read binary file '{path}': {e}");
            std::process::exit(2);
        });

        if bytes.len() >= 4 && bytes[0..4] == CONTAINER_MAGIC {
            let container = BinaryContainer::decode(&bytes).unwrap_or_else(|e| {
                eprintln!("error reading container '{path}': {e}");
                std::process::exit(1);
            });

            if container.header.entry_pc == 0 {
                eprintln!("error: no entry point specified (main label missing in binary)");
                std::process::exit(1);
            }

            load_container_into_machine(&mut machine, &container);
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

        let link_containers = load_external_symbols(link);
        let base_dir = Path::new(path).parent();
        let image = match assemble_and_link(&src, base_dir, &link_containers) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("{path}:{}: error: {}", e.span, e.kind);
                std::process::exit(1);
            }
        };

        if image.entry == 0 {
            eprintln!("error: no entry point specified (main label missing in source)");
            std::process::exit(1);
        }

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

fn compile_file(input_path: &str, link: &[String], output_path: Option<&str>) {
    let src = std::fs::read_to_string(input_path).unwrap_or_else(|e| {
        eprintln!("cannot read assembly file '{input_path}': {e}");
        std::process::exit(2);
    });

    let link_containers = load_external_symbols(link);
    let base_dir = Path::new(input_path).parent();

    let image = match assemble_and_link(&src, base_dir, &link_containers) {
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
        "Compiled '{input_path}' -> '{out_path}' ({} bytes, text: {}B, data: {}B, export symbols: {})",
        container_bytes.len(),
        container.header.text_size,
        container.header.data_size,
        container.export_symbols.len()
    );
}
