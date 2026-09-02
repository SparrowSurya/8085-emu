//! 8085 machine code disassembler.
//!
//! Translates raw binary byte streams or structured .8085.bin containers into
//! human-readable assembly instructions, symbolic references, and annotations.

use std::collections::HashMap;

use crate::asm::container::BinaryContainer;
use crate::asm::inspect::extract_strings;

pub const ANSI_RESET: &str = "\x1b[0m";
pub const ANSI_WHITE: &str = "\x1b[37m";
pub const ANSI_CYAN: &str = "\x1b[36m";
pub const ANSI_YELLOW: &str = "\x1b[33m";
pub const ANSI_MAGENTA: &str = "\x1b[35m";
pub const ANSI_BLUE: &str = "\x1b[34m";
pub const ANSI_GREEN: &str = "\x1b[32m";

/// Options for configuring disassembly output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisassembleOptions {
    pub color: bool,
    pub show_cycles: bool,
    pub show_vectors: bool,
    pub show_banners: bool,
}

impl Default for DisassembleOptions {
    fn default() -> Self {
        Self {
            color: false,
            show_cycles: false,
            show_vectors: false,
            show_banners: true,
        }
    }
}

/// A single disassembled output line (instruction or banner comment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyRow {
    /// Starting memory address.
    pub addr: u16,
    /// Raw machine code bytes (1 to 3 bytes, or empty for comments/banners).
    pub bytes: Vec<u8>,
    /// Human-readable assembly mnemonic, operands, or comments.
    pub mnemonic: String,
    /// Optional hardware cycle timing string (e.g. "10 T", "18/9 T").
    pub cycles: Option<&'static str>,
}

impl DisassemblyRow {
    /// Creates a comment or banner row without machine code bytes.
    pub fn banner(text: impl Into<String>) -> Self {
        Self {
            addr: 0,
            bytes: Vec::new(),
            mnemonic: text.into(),
            cycles: None,
        }
    }

    /// Formats the disassembly row with ANSI color codes.
    pub fn to_colored_string(&self) -> String {
        if self.bytes.is_empty() {
            // Banner or comment line
            return format!("{ANSI_WHITE}{}{ANSI_RESET}", self.mnemonic);
        }

        let hex_bytes = self
            .bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");

        let colored_addr = format!("{ANSI_WHITE}{:04X}{ANSI_RESET}", self.addr);
        let colored_bytes = format!("{ANSI_WHITE}{:<16}{ANSI_RESET}", hex_bytes);
        let colored_mnemonic = colorize_mnemonic(&self.mnemonic);

        if let Some(cyc) = self.cycles {
            let pad = 42usize.saturating_sub(self.mnemonic.len());
            let pad_str = " ".repeat(pad);
            format!(
                "{colored_addr}: {colored_bytes} {colored_mnemonic}{pad_str} {ANSI_WHITE}[{cyc}]{ANSI_RESET}"
            )
        } else {
            format!("{colored_addr}: {colored_bytes} {colored_mnemonic}")
        }
    }
}

impl std::fmt::Display for DisassemblyRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.bytes.is_empty() {
            return write!(f, "{}", self.mnemonic);
        }
        let hex_bytes = self
            .bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(cyc) = self.cycles {
            write!(
                f,
                "{:04X}: {:<16} {:<42} [{}]",
                self.addr, hex_bytes, self.mnemonic, cyc
            )
        } else {
            write!(f, "{:04X}: {:<16} {}", self.addr, hex_bytes, self.mnemonic)
        }
    }
}

fn is_register_token(w: &str) -> bool {
    matches!(
        w.to_ascii_uppercase().as_str(),
        "A" | "B" | "C" | "D" | "E" | "H" | "L" | "M" | "BC" | "DE" | "HL" | "SP" | "PSW"
    )
}

fn is_number_token(w: &str) -> bool {
    if w.starts_with("0x") || w.starts_with("0X") {
        w.len() > 2 && w[2..].chars().all(|c| c.is_ascii_hexdigit())
    } else {
        !w.is_empty() && w.chars().all(|c| c.is_ascii_digit())
    }
}

fn is_mnemonic_token(w: &str) -> bool {
    matches!(
        w.to_ascii_uppercase().as_str(),
        "MOV"
            | "MVI"
            | "LXI"
            | "LDA"
            | "STA"
            | "LHLD"
            | "SHLD"
            | "LDAX"
            | "STAX"
            | "XCHG"
            | "ADD"
            | "ADI"
            | "ADC"
            | "ACI"
            | "SUB"
            | "SUI"
            | "SBB"
            | "SBI"
            | "INR"
            | "DCR"
            | "INX"
            | "DCX"
            | "DAD"
            | "DAA"
            | "ANA"
            | "ANI"
            | "ORA"
            | "ORI"
            | "XRA"
            | "XRI"
            | "CMP"
            | "CPI"
            | "RLC"
            | "RRC"
            | "RAL"
            | "RAR"
            | "CMA"
            | "CMC"
            | "STC"
            | "JMP"
            | "JNZ"
            | "JZ"
            | "JNC"
            | "JC"
            | "JPO"
            | "JPE"
            | "JP"
            | "JM"
            | "PCHL"
            | "CALL"
            | "CNZ"
            | "CZ"
            | "CNC"
            | "CC"
            | "CPO"
            | "CPE"
            | "CP"
            | "CM"
            | "RET"
            | "RNZ"
            | "RZ"
            | "RNC"
            | "RC"
            | "RPO"
            | "RPE"
            | "RP"
            | "RM"
            | "RST"
            | "PUSH"
            | "POP"
            | "XTHL"
            | "SPHL"
            | "IN"
            | "OUT"
            | "EI"
            | "DI"
            | "SIM"
            | "RIM"
            | "NOP"
            | "HLT"
            | "DB"
    )
}

fn colorize_code_part(code: &str) -> String {
    let mut out = String::new();
    let mut chars = code.chars().peekable();
    let mut is_first_token = true;

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            out.push(chars.next().unwrap());
        } else if ch == ',' || ch == ':' {
            out.push_str(ANSI_WHITE);
            out.push(chars.next().unwrap());
            out.push_str(ANSI_RESET);
        } else {
            // Read identifier or literal
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == ',' || c == ':' || c == ';' {
                    break;
                }
                word.push(chars.next().unwrap());
            }

            if is_first_token && is_mnemonic_token(&word) {
                out.push_str(&format!("{ANSI_CYAN}{word}{ANSI_RESET}"));
                is_first_token = false;
            } else if is_register_token(&word) {
                out.push_str(&format!("{ANSI_MAGENTA}{word}{ANSI_RESET}"));
            } else if is_number_token(&word) {
                out.push_str(&format!("{ANSI_YELLOW}{word}{ANSI_RESET}"));
            } else {
                // Symbol / label name (e.g. print, input, loc_0047)
                out.push_str(&format!("{ANSI_BLUE}{word}{ANSI_RESET}"));
            }
        }
    }

    out
}

fn colorize_comment_part(comment: &str) -> String {
    let mut out = String::new();
    let mut chars = comment.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut label = String::from("<");
            while let Some(&c) = chars.peek() {
                chars.next();
                label.push(c);
                if c == '>' {
                    break;
                }
            }
            out.push_str(&format!("{ANSI_BLUE}{label}{ANSI_RESET}"));
        } else if ch == '"' {
            let mut str_lit = String::from("\"");
            while let Some(&c) = chars.peek() {
                chars.next();
                str_lit.push(c);
                if c == '"' {
                    break;
                }
            }
            out.push_str(&format!("{ANSI_GREEN}{str_lit}{ANSI_RESET}"));
        } else {
            out.push(ch);
        }
    }
    out
}

fn colorize_mnemonic(text: &str) -> String {
    if let Some((code_part, comment_part)) = text.split_once(" ; ") {
        let colored_code = colorize_code_part(code_part);
        let colored_comment = colorize_comment_part(comment_part);
        format!("{colored_code} {ANSI_WHITE};{ANSI_RESET} {colored_comment}")
    } else {
        colorize_code_part(text)
    }
}

const REG_NAMES: [&str; 8] = ["B", "C", "D", "E", "H", "L", "M", "A"];
const ALU_OPS: [&str; 8] = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];

/// Disassembles the `.text` segment of a `.8085.bin` container image with default options.
pub fn disassemble_bytes(bytes: &[u8]) -> Result<Vec<DisassemblyRow>, String> {
    let container = BinaryContainer::decode(bytes)?;
    Ok(disassemble_container_with_options(
        &container,
        &DisassembleOptions::default(),
    ))
}

/// Disassembles a structured `.8085.bin` container with default options.
pub fn disassemble_container(container: &BinaryContainer) -> Vec<DisassemblyRow> {
    disassemble_container_with_options(container, &DisassembleOptions::default())
}

/// Disassembles a structured `.8085.bin` container with custom options.
pub fn disassemble_container_with_options(
    container: &BinaryContainer,
    options: &DisassembleOptions,
) -> Vec<DisassemblyRow> {
    let mut rows = Vec::new();

    // Map address -> exported symbol name
    let mut symbol_map: HashMap<u16, String> = HashMap::new();
    for (sym, addr) in &container.export_symbols {
        symbol_map.insert(*addr, sym.clone());
    }
    if container.header.entry_pc != 0 {
        symbol_map.insert(container.header.entry_pc, "main".to_string());
    }

    // Map address -> data string preview
    let mut data_previews: HashMap<u16, String> = HashMap::new();
    for s in extract_strings(container, 2) {
        if s.segment == ".data" {
            let truncated = if s.content.len() > 24 {
                format!("{}...", &s.content[..21])
            } else {
                s.content.clone()
            };
            data_previews.insert(s.ram_addr, format!("\"{truncated}\""));
        }
    }

    // Pass 1: Collect internal branch/jump targets in .text
    let mut branch_targets: HashMap<u16, String> = HashMap::new();
    let text_start = container.header.text_addr;
    let text_end = text_start.wrapping_add(container.header.text_size);

    let mut offset = 0;
    while offset < container.text_bytes.len() {
        let (size, target_addr) = extract_branch_target(&container.text_bytes, offset);
        if let Some(target) = target_addr {
            if target >= text_start && target < text_end && !symbol_map.contains_key(&target) {
                branch_targets
                    .entry(target)
                    .or_insert_with(|| format!("loc_{:04X}", target));
            }
        }
        offset += size.max(1);
    }

    // Optional Vector Table Disassembly
    if options.show_vectors && !container.vec_bytes.is_empty() {
        rows.push(DisassemblyRow::banner(
            "; ==============================================================================",
        ));
        rows.push(DisassemblyRow::banner(
            "; Section: .vec (Interrupt Vector Table, 64 Bytes)",
        ));
        rows.push(DisassemblyRow::banner(
            "; ==============================================================================",
        ));

        let vec_rows = disassemble_linear(
            &container.vec_bytes,
            0x0000,
            &symbol_map,
            &branch_targets,
            &data_previews,
            options.show_cycles,
        );
        for mut r in vec_rows {
            let vec_desc = match r.addr {
                0x0000 => "; RST 0 / Reset Vector",
                0x0008 => "; RST 1 Vector (0x0008)",
                0x0010 => "; RST 2 Vector (0x0010)",
                0x0018 => "; RST 3 Vector (0x0018)",
                0x0020 => "; RST 4 Vector (0x0020)",
                0x0024 => "; TRAP Vector (0x0024)",
                0x0028 => "; RST 5 Vector (0x0028)",
                0x002C => "; RST 5.5 Vector (0x002C)",
                0x0030 => "; RST 6 Vector (0x0030)",
                0x0034 => "; RST 6.5 Vector (0x0034)",
                0x0038 => "; RST 7 Vector (0x0038)",
                0x003C => "; RST 7.5 Vector (0x003C)",
                _ => "",
            };
            if !vec_desc.is_empty() {
                r.mnemonic = format!("{:<20} {}", r.mnemonic, vec_desc);
            }
            rows.push(r);
        }
        rows.push(DisassemblyRow::banner(""));
    }

    // Pass 2: Executable Code Section (.text)
    if container.header.text_size > 0 && !container.text_bytes.is_empty() {
        let code_rows = disassemble_linear(
            &container.text_bytes,
            container.header.text_addr,
            &symbol_map,
            &branch_targets,
            &data_previews,
            options.show_cycles,
        );

        let mut is_first = true;
        for mut r in code_rows {
            let has_global_symbol = symbol_map.get(&r.addr);
            let has_branch_label = branch_targets.get(&r.addr);

            if let Some(sym_name) = has_global_symbol {
                if options.show_banners {
                    if !is_first {
                        rows.push(DisassemblyRow::banner(""));
                    }
                    let tag = if r.addr == container.header.entry_pc {
                        format!("Function: {sym_name} (Entry Point: 0x{:04X})", r.addr)
                    } else {
                        format!("Subroutine: {sym_name} (Address: 0x{:04X})", r.addr)
                    };
                    rows.push(DisassemblyRow::banner("; =============================================================================="));
                    rows.push(DisassemblyRow::banner(format!("; {tag}")));
                    rows.push(DisassemblyRow::banner("; =============================================================================="));
                }
            } else if let Some(lbl) = has_branch_label {
                r.mnemonic = format!("{:<20} ; {lbl}:", r.mnemonic);
            }

            rows.push(r);
            is_first = false;
        }
    }

    rows
}

fn extract_branch_target(bytes: &[u8], offset: usize) -> (usize, Option<u16>) {
    let b = bytes[offset];
    let remaining = bytes.len() - offset;

    match b {
        // 3-byte jump/call/load instructions
        0x01 | 0x11 | 0x21 | 0x22 | 0x2A | 0x31 | 0x32 | 0x3A | 0xC2 | 0xC3 | 0xC4 | 0xCA
        | 0xCC | 0xCD | 0xD2 | 0xD4 | 0xDA | 0xDC | 0xE2 | 0xE4 | 0xEA | 0xEC | 0xF2 | 0xF4
        | 0xFA | 0xFC => {
            if remaining >= 3 {
                let addr = (bytes[offset + 1] as u16) | ((bytes[offset + 2] as u16) << 8);
                (3, Some(addr))
            } else {
                (1, None)
            }
        }
        // 2-byte instructions
        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E | 0xC6 | 0xCE | 0xD3 | 0xD6
        | 0xDB | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => (2.min(remaining), None),
        // 1-byte instructions
        _ => (1, None),
    }
}

/// Linearly decodes instructions from `bytes` starting at `base_addr`.
fn disassemble_linear(
    bytes: &[u8],
    base_addr: u16,
    symbol_map: &HashMap<u16, String>,
    branch_targets: &HashMap<u16, String>,
    data_previews: &HashMap<u16, String>,
    show_cycles: bool,
) -> Vec<DisassemblyRow> {
    let mut rows = Vec::new();
    let mut offset = 0;

    while offset < bytes.len() {
        let addr = base_addr.wrapping_add(offset as u16);
        let (size, mnemonic) = decode_instruction_with_symbols(
            bytes,
            offset,
            symbol_map,
            branch_targets,
            data_previews,
        );
        let safe_size = size.min(bytes.len() - offset);
        let row_bytes = bytes[offset..offset + safe_size].to_vec();
        let cycles = if show_cycles {
            Some(opcode_t_states(bytes[offset]))
        } else {
            None
        };

        rows.push(DisassemblyRow {
            addr,
            bytes: row_bytes,
            mnemonic,
            cycles,
        });

        offset += safe_size;
    }

    rows
}

fn decode_instruction_with_symbols(
    bytes: &[u8],
    offset: usize,
    symbol_map: &HashMap<u16, String>,
    branch_targets: &HashMap<u16, String>,
    data_previews: &HashMap<u16, String>,
) -> (usize, String) {
    let b = bytes[offset];
    let remaining = bytes.len() - offset;

    let d8 = |name: &str| -> (usize, String) {
        if remaining >= 2 {
            (2, format!("{name} 0x{:02X}", bytes[offset + 1]))
        } else {
            (1, format!("DB 0x{b:02X}"))
        }
    };

    let a16 = |name: &str| -> (usize, String) {
        if remaining >= 3 {
            let addr = (bytes[offset + 1] as u16) | ((bytes[offset + 2] as u16) << 8);
            let target_name = if let Some(sym) = symbol_map.get(&addr) {
                sym.clone()
            } else if let Some(lbl) = branch_targets.get(&addr) {
                lbl.clone()
            } else {
                format!("0x{addr:04X}")
            };

            let preview = data_previews
                .get(&addr)
                .map(|p| format!("       ; -> {p}"))
                .unwrap_or_default();

            (3, format!("{name} {target_name}{preview}"))
        } else {
            (1, format!("DB 0x{b:02X}"))
        }
    };

    match b {
        0x00 => (1, "NOP".to_string()),
        0x01 => a16("LXI BC,"),
        0x02 => (1, "STAX BC".to_string()),
        0x03 => (1, "INX BC".to_string()),
        0x04 => (1, "INR B".to_string()),
        0x05 => (1, "DCR B".to_string()),
        0x06 => d8("MVI B,"),
        0x07 => (1, "RLC".to_string()),
        0x09 => (1, "DAD BC".to_string()),
        0x0A => (1, "LDAX BC".to_string()),
        0x0B => (1, "DCX BC".to_string()),
        0x0C => (1, "INR C".to_string()),
        0x0D => (1, "DCR C".to_string()),
        0x0E => d8("MVI C,"),
        0x0F => (1, "RRC".to_string()),

        0x11 => a16("LXI DE,"),
        0x12 => (1, "STAX DE".to_string()),
        0x13 => (1, "INX DE".to_string()),
        0x14 => (1, "INR D".to_string()),
        0x15 => (1, "DCR D".to_string()),
        0x16 => d8("MVI D,"),
        0x17 => (1, "RAL".to_string()),
        0x19 => (1, "DAD DE".to_string()),
        0x1A => (1, "LDAX DE".to_string()),
        0x1B => (1, "DCX DE".to_string()),
        0x1C => (1, "INR E".to_string()),
        0x1D => (1, "DCR E".to_string()),
        0x1E => d8("MVI E,"),
        0x1F => (1, "RAR".to_string()),

        0x20 => (1, "RIM".to_string()),
        0x21 => a16("LXI HL,"),
        0x22 => a16("SHLD"),
        0x23 => (1, "INX HL".to_string()),
        0x24 => (1, "INR H".to_string()),
        0x25 => (1, "DCR H".to_string()),
        0x26 => d8("MVI H,"),
        0x27 => (1, "DAA".to_string()),
        0x29 => (1, "DAD HL".to_string()),
        0x2A => a16("LHLD"),
        0x2B => (1, "DCX HL".to_string()),
        0x2C => (1, "INR L".to_string()),
        0x2D => (1, "DCR L".to_string()),
        0x2E => d8("MVI L,"),
        0x2F => (1, "CMA".to_string()),

        0x30 => (1, "SIM".to_string()),
        0x31 => a16("LXI SP,"),
        0x32 => a16("STA"),
        0x33 => (1, "INX SP".to_string()),
        0x34 => (1, "INR M".to_string()),
        0x35 => (1, "DCR M".to_string()),
        0x36 => d8("MVI M,"),
        0x37 => (1, "STC".to_string()),
        0x39 => (1, "DAD SP".to_string()),
        0x3A => a16("LDA"),
        0x3B => (1, "DCX SP".to_string()),
        0x3C => (1, "INR A".to_string()),
        0x3D => (1, "DCR A".to_string()),
        0x3E => d8("MVI A,"),
        0x3F => (1, "CMC".to_string()),

        // 0x40..=0x7F: MOV and HLT
        0x40..=0x7F => {
            if b == 0x76 {
                (1, "HLT".to_string())
            } else {
                let dst = REG_NAMES[((b >> 3) & 7) as usize];
                let src = REG_NAMES[(b & 7) as usize];
                (1, format!("MOV {dst}, {src}"))
            }
        }

        // 0x80..=0xBF: ALU operations
        0x80..=0xBF => {
            let op = ALU_OPS[((b >> 3) & 7) as usize];
            let r = REG_NAMES[(b & 7) as usize];
            (1, format!("{op} {r}"))
        }

        0xC0 => (1, "RNZ".to_string()),
        0xC1 => (1, "POP BC".to_string()),
        0xC2 => a16("JNZ"),
        0xC3 => a16("JMP"),
        0xC4 => a16("CNZ"),
        0xC5 => (1, "PUSH BC".to_string()),
        0xC6 => d8("ADI"),
        0xC7 => (1, "RST 0".to_string()),
        0xC8 => (1, "RZ".to_string()),
        0xC9 => (1, "RET".to_string()),
        0xCA => a16("JZ"),
        0xCC => a16("CZ"),
        0xCD => a16("CALL"),
        0xCE => d8("ACI"),
        0xCF => (1, "RST 1".to_string()),

        0xD0 => (1, "RNC".to_string()),
        0xD1 => (1, "POP DE".to_string()),
        0xD2 => a16("JNC"),
        0xD3 => d8("OUT"),
        0xD4 => a16("CNC"),
        0xD5 => (1, "PUSH DE".to_string()),
        0xD6 => d8("SUI"),
        0xD7 => (1, "RST 2".to_string()),
        0xD8 => (1, "RC".to_string()),
        0xDA => a16("JC"),
        0xDB => d8("IN"),
        0xDC => a16("CC"),
        0xDE => d8("SBI"),
        0xDF => (1, "RST 3".to_string()),

        0xE0 => (1, "RPO".to_string()),
        0xE1 => (1, "POP HL".to_string()),
        0xE2 => a16("JPO"),
        0xE3 => (1, "XTHL".to_string()),
        0xE4 => a16("CPO"),
        0xE5 => (1, "PUSH HL".to_string()),
        0xE6 => d8("ANI"),
        0xE7 => (1, "RST 4".to_string()),
        0xE8 => (1, "RPE".to_string()),
        0xE9 => (1, "PCHL".to_string()),
        0xEA => a16("JPE"),
        0xEB => (1, "XCHG".to_string()),
        0xEC => a16("CPE"),
        0xEE => d8("XRI"),
        0xEF => (1, "RST 5".to_string()),

        0xF0 => (1, "RP".to_string()),
        0xF1 => (1, "POP PSW".to_string()),
        0xF2 => a16("JP"),
        0xF3 => (1, "DI".to_string()),
        0xF4 => a16("CP"),
        0xF5 => (1, "PUSH PSW".to_string()),
        0xF6 => d8("ORI"),
        0xF7 => (1, "RST 6".to_string()),
        0xF8 => (1, "RM".to_string()),
        0xF9 => (1, "SPHL".to_string()),
        0xFA => a16("JM"),
        0xFB => (1, "EI".to_string()),
        0xFC => a16("CM"),
        0xFE => d8("CPI"),
        0xFF => (1, "RST 7".to_string()),

        _ => (1, format!("DB 0x{b:02X}")),
    }
}

/// Returns hardware T-state timing for a given opcode byte.
pub fn opcode_t_states(b: u8) -> &'static str {
    match b {
        0x00 => "4 T",
        0x01 => "10 T",
        0x02 => "7 T",
        0x03 => "6 T",
        0x04 => "4 T",
        0x05 => "4 T",
        0x06 => "7 T",
        0x07 => "4 T",
        0x08 => "4 T",
        0x09 => "10 T",
        0x0A => "7 T",
        0x0B => "6 T",
        0x0C => "4 T",
        0x0D => "4 T",
        0x0E => "7 T",
        0x0F => "4 T",

        0x10 => "4 T",
        0x11 => "10 T",
        0x12 => "7 T",
        0x13 => "6 T",
        0x14 => "4 T",
        0x15 => "4 T",
        0x16 => "7 T",
        0x17 => "4 T",
        0x18 => "4 T",
        0x19 => "10 T",
        0x1A => "7 T",
        0x1B => "6 T",
        0x1C => "4 T",
        0x1D => "4 T",
        0x1E => "7 T",
        0x1F => "4 T",

        0x20 => "4 T",
        0x21 => "10 T",
        0x22 => "16 T",
        0x23 => "6 T",
        0x24 => "4 T",
        0x25 => "4 T",
        0x26 => "7 T",
        0x27 => "4 T",
        0x28 => "4 T",
        0x29 => "10 T",
        0x2A => "16 T",
        0x2B => "6 T",
        0x2C => "4 T",
        0x2D => "4 T",
        0x2E => "7 T",
        0x2F => "4 T",

        0x30 => "4 T",
        0x31 => "10 T",
        0x32 => "13 T",
        0x33 => "6 T",
        0x34 => "10 T",
        0x35 => "10 T",
        0x36 => "10 T",
        0x37 => "4 T",
        0x38 => "4 T",
        0x39 => "10 T",
        0x3A => "13 T",
        0x3B => "6 T",
        0x3C => "4 T",
        0x3D => "4 T",
        0x3E => "7 T",
        0x3F => "4 T",

        0x76 => "5 T",
        0x46 | 0x4E | 0x56 | 0x5E | 0x66 | 0x6E | 0x7E => "7 T",
        0x70..=0x75 | 0x77 => "7 T",
        0x40..=0x7F => "4 T",

        0x86 | 0x8E | 0x96 | 0x9E | 0xA6 | 0xAE | 0xB6 | 0xBE => "7 T",
        0x80..=0xBF => "4 T",

        0xC0 => "12/6 T",
        0xC1 => "10 T",
        0xC2 => "10/7 T",
        0xC3 => "10 T",
        0xC4 => "18/9 T",
        0xC5 => "12 T",
        0xC6 => "7 T",
        0xC7 => "12 T",
        0xC8 => "12/6 T",
        0xC9 => "10 T",
        0xCA => "10/7 T",
        0xCB => "10 T",
        0xCC => "18/9 T",
        0xCD => "18 T",
        0xCE => "7 T",
        0xCF => "12 T",

        0xD0 => "12/6 T",
        0xD1 => "10 T",
        0xD2 => "10/7 T",
        0xD3 => "10 T",
        0xD4 => "18/9 T",
        0xD5 => "12 T",
        0xD6 => "7 T",
        0xD7 => "12 T",
        0xD8 => "12/6 T",
        0xD9 => "10 T",
        0xDA => "10/7 T",
        0xDB => "10 T",
        0xDC => "18/9 T",
        0xDD => "18 T",
        0xDE => "7 T",
        0xDF => "12 T",

        0xE0 => "12/6 T",
        0xE1 => "10 T",
        0xE2 => "10/7 T",
        0xE3 => "16 T",
        0xE4 => "18/9 T",
        0xE5 => "12 T",
        0xE6 => "7 T",
        0xE7 => "12 T",
        0xE8 => "12/6 T",
        0xE9 => "6 T",
        0xEA => "10/7 T",
        0xEB => "4 T",
        0xEC => "18/9 T",
        0xED => "18 T",
        0xEE => "7 T",
        0xEF => "12 T",

        0xF0 => "12/6 T",
        0xF1 => "10 T",
        0xF2 => "10/7 T",
        0xF3 => "4 T",
        0xF4 => "18/9 T",
        0xF5 => "12 T",
        0xF6 => "7 T",
        0xF7 => "12 T",
        0xF8 => "12/6 T",
        0xF9 => "6 T",
        0xFA => "10/7 T",
        0xFB => "4 T",
        0xFC => "18/9 T",
        0xFD => "18 T",
        0xFE => "7 T",
        0xFF => "12 T",
    }
}
