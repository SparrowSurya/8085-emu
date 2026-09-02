//! 8085 machine code disassembler.
//!
//! Translates raw binary byte streams or structured .8085.bin containers into
//! human-readable assembly instructions and symbol annotations.

use std::collections::HashMap;

use crate::asm::container::BinaryContainer;

/// A single disassembled instruction row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisassemblyRow {
    /// Starting memory address.
    pub addr: u16,
    /// Raw machine code bytes (1 to 3 bytes).
    pub bytes: Vec<u8>,
    /// Human-readable assembly mnemonic, operands, or comments.
    pub mnemonic: String,
}

impl std::fmt::Display for DisassemblyRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex_bytes = self
            .bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        write!(f, "{:04X}: {:<16} {}", self.addr, hex_bytes, self.mnemonic)
    }
}

const REG_NAMES: [&str; 8] = ["B", "C", "D", "E", "H", "L", "M", "A"];
const ALU_OPS: [&str; 8] = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];

/// Disassembles the `.text` segment of a `.8085.bin` container image.
///
/// Returns an error if the input bytes do not contain a valid `.8085.bin` container.
pub fn disassemble_bytes(bytes: &[u8]) -> Result<Vec<DisassemblyRow>, String> {
    let container = BinaryContainer::decode(bytes)?;
    Ok(disassemble_container(&container))
}

/// Disassembles the .text segment of a structured `.8085.bin` container.
pub fn disassemble_container(container: &BinaryContainer) -> Vec<DisassemblyRow> {
    let mut rows = Vec::new();

    // Map address -> list of symbol names
    let mut symbol_map: HashMap<u16, Vec<String>> = HashMap::new();
    for (sym, addr) in &container.export_symbols {
        symbol_map.entry(*addr).or_default().push(sym.clone());
    }

    // Executable Code Section (.text) only
    if container.header.text_size > 0 && !container.text_bytes.is_empty() {
        let code_rows = disassemble_linear(&container.text_bytes, container.header.text_addr);
        for mut r in code_rows {
            let mut labels = Vec::new();

            // Check for exported global symbols at this address
            if let Some(syms) = symbol_map.get(&r.addr) {
                for sym in syms {
                    labels.push(format!("<{sym}>"));
                }
            }

            // Check if this address is the main entry point
            if r.addr == container.header.entry_pc && container.header.entry_pc != 0 {
                let main_tag = "<main>".to_string();
                if !labels.contains(&main_tag) {
                    labels.push(main_tag);
                }
            }

            if !labels.is_empty() {
                r.mnemonic = format!("{:<20} ; {}", r.mnemonic, labels.join(" "));
            }
            rows.push(r);
        }
    }

    rows
}

/// Linearly decodes instructions from `bytes` starting at `base_addr`.
fn disassemble_linear(bytes: &[u8], base_addr: u16) -> Vec<DisassemblyRow> {
    let mut rows = Vec::new();
    let mut offset = 0;

    while offset < bytes.len() {
        let addr = base_addr.wrapping_add(offset as u16);
        let (size, mnemonic) = decode_instruction(bytes, offset);
        let safe_size = size.min(bytes.len() - offset);
        let row_bytes = bytes[offset..offset + safe_size].to_vec();

        rows.push(DisassemblyRow {
            addr,
            bytes: row_bytes,
            mnemonic,
        });

        offset += safe_size;
    }

    rows
}

fn decode_instruction(bytes: &[u8], offset: usize) -> (usize, String) {
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
            (3, format!("{name} 0x{addr:04X}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disassemble_linear_known_bytes() {
        let bytes = vec![
            0xC3, 0x06, 0x00, // JMP 0x0006
            0x3E, 0x42,       // MVI A, 0x42
            0x06, 0x10,       // MVI B, 0x10
            0x80,             // ADD B
            0x76,             // HLT
        ];

        let rows = disassemble_linear(&bytes, 0);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].mnemonic, "JMP 0x0006");
        assert_eq!(rows[1].mnemonic, "MVI A, 0x42");
        assert_eq!(rows[2].mnemonic, "MVI B, 0x10");
        assert_eq!(rows[3].mnemonic, "ADD B");
        assert_eq!(rows[4].mnemonic, "HLT");
    }

    #[test]
    fn test_disassemble_container_text_only() {
        use crate::asm::container::{BinaryContainer, ContainerHeader, CONTAINER_MAGIC, CONTAINER_VERSION};

        let header = ContainerHeader {
            magic: CONTAINER_MAGIC,
            version: CONTAINER_VERSION,
            flags: 0,
            entry_pc: 0x0040,
            sp_init: 0xFFFF,
            text_addr: 0x0040,
            text_size: 5,
            data_addr: 0x0000,
            data_size: 0,
            bss_addr: 0x0000,
            bss_size: 0,
            vec_size: 0,
            sym_size: 0,
            reserved: [0u8; 6],
        };

        let container = BinaryContainer {
            header,
            vec_bytes: Vec::new(),
            data_bytes: Vec::new(),
            text_bytes: vec![0x3E, 0x42, 0x06, 0x10, 0x76], // MVI A, 0x42; MVI B, 0x10; HLT
            export_symbols: Vec::new(),
        };

        let container_bytes = container.encode();
        let rows = disassemble_bytes(&container_bytes).expect("disassembles container cleanly");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].addr, 0x0040);
        assert!(rows[0].mnemonic.contains("MVI A, 0x42"));
        assert!(rows[0].mnemonic.contains("<main>"));
        assert_eq!(rows[1].addr, 0x0042);
        assert_eq!(rows[1].mnemonic, "MVI B, 0x10");
        assert_eq!(rows[2].addr, 0x0044);
        assert_eq!(rows[2].mnemonic, "HLT");
    }
}
