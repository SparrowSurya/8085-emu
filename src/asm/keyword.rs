//! Classification of identifiers against the reserved keyword table.
//!
//! The lexer emits every word as an identifier; these helpers decide, case-insensitively,
//! whether a word is a register, a size specifier, or otherwise reserved — so the parser
//! can turn it into the right node and reject reserved words used as names.

use super::ast::Size;
use super::encode::{AReg16, AReg8};

/// The 8-bit register named by `word`, if any (case-insensitive).
pub fn reg8(word: &str) -> Option<AReg8> {
    Some(match word.to_ascii_uppercase().as_str() {
        "A" => AReg8::A,
        "B" => AReg8::B,
        "C" => AReg8::C,
        "D" => AReg8::D,
        "E" => AReg8::E,
        "H" => AReg8::H,
        "L" => AReg8::L,
        "M" => AReg8::M,
        _ => return None,
    })
}

/// The 16-bit register/pair named by `word`, if any (case-insensitive).
pub fn reg16(word: &str) -> Option<AReg16> {
    Some(match word.to_ascii_uppercase().as_str() {
        "BC" => AReg16::BC,
        "DE" => AReg16::DE,
        "HL" => AReg16::HL,
        "SP" => AReg16::SP,
        "PSW" => AReg16::PSW,
        _ => return None,
    })
}

/// The size specifier named by `word`, if any (case-insensitive).
pub fn size(word: &str) -> Option<Size> {
    match word.to_ascii_uppercase().as_str() {
        "BYTE" => Some(Size::Byte),
        "WORD" => Some(Size::Word),
        _ => None,
    }
}

/// Whether `word` is any reserved keyword (and therefore illegal as a user-chosen name).
pub fn is_reserved(word: &str) -> bool {
    RESERVED.contains(&word.to_ascii_uppercase().as_str())
}

/// Every reserved keyword (uppercased): registers, size specifiers, segment/directive
/// words, and all instruction mnemonics.
const RESERVED: &[&str] = &[
    // sizes
    "BYTE", "WORD", // registers
    "A", "B", "C", "D", "E", "H", "L", "M", "BC", "DE", "HL", "SP", "PSW",
    // segment & directive words
    "SEGMENT", "DATA", "BSS", "TEXT", "DEFINE", "REPEAT", "LEN", // data transfer
    "MOV", "MVI", "LXI", "LDA", "STA", "LDAX", "STAX", "LHLD", "SHLD", "XCHG", "XTHL", "SPHL",
    "PCHL", // arithmetic
    "ADD", "ADI", "ADC", "ACI", "SUB", "SUI", "SBB", "SBI", "INR", "DCR", "INX", "DCX", "DAD",
    "DAA", // logical
    "ANA", "ANI", "XRA", "XRI", "ORA", "ORI", "CMP", "CPI", "CMA", "CMC", "STC", "RLC", "RRC",
    "RAL", "RAR", // branch / control
    "JMP", "JZ", "JNZ", "JC", "JNC", "JP", "JM", "JPE", "JPO", "CALL", "CZ", "CNZ", "CC", "CNC",
    "CP", "CM", "CPE", "CPO", "RET", "RZ", "RNZ", "RC", "RNC", "RP", "RM", "RPE", "RPO", "RST",
    // stack / io / machine control
    "PUSH", "POP", "IN", "OUT", "NOP", "HLT", "EI", "DI", "RIM", "SIM",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_sizes() {
        assert_eq!(reg8("a"), Some(AReg8::A));
        assert_eq!(reg8("M"), Some(AReg8::M));
        assert_eq!(reg8("BC"), None);
        assert_eq!(reg16("hl"), Some(AReg16::HL));
        assert_eq!(reg16("PSW"), Some(AReg16::PSW));
        assert_eq!(size("byte"), Some(Size::Byte));
        assert_eq!(size("Word"), Some(Size::Word));
    }

    #[test]
    fn reserved_detection_is_case_insensitive() {
        assert!(is_reserved("mov"));
        assert!(is_reserved("HLT"));
        assert!(is_reserved("Segment"));
        assert!(is_reserved("psw"));
        assert!(!is_reserved("loop"));
        assert!(!is_reserved("my_var"));
        assert!(!is_reserved("main"));
    }
}
