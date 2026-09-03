use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range, Url};

use super::document::{Document, resolve_relative_path};

/// Provides rich hover documentation for 8085 instructions, registers, directives, and symbols.
pub fn get_hover(doc: &Document, position: &Position) -> Option<Hover> {
    // 0. Check %include Directive & Module documentation
    if let Some(hover) = get_include_hover(doc, position) {
        return Some(hover);
    }

    let (word, range) = doc.get_word_at_position(position)?;
    let upper_word = word.to_uppercase();

    // 1. Check numeric literals (decimal, hex, binary, octal, ascii)
    if let Some(num_val) = parse_numeric_literal(&word) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format_number_hover(num_val),
            }),
            range: Some(range),
        });
    }

    // 2. Check 8085 Instruction Mnemonics
    if let Some(doc_str) = get_instruction_hover(&upper_word) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_str.to_string(),
            }),
            range: Some(range),
        });
    }

    // 3. Check 8085 Registers
    if let Some(doc_str) = get_register_hover(&upper_word) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_str.to_string(),
            }),
            range: Some(range),
        });
    }

    // 4. Check Directives, Keywords, & Segments
    if let Some(doc_str) = get_directive_hover(&word) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_str.to_string(),
            }),
            range: Some(range),
        });
    }

    // 5. Check User-defined Symbol (label, variable, constant, or extern in source)
    if let Some(doc_str) = get_user_symbol_hover(doc, &word, position) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_str,
            }),
            range: Some(range),
        });
    }

    None
}

fn parse_numeric_literal(word: &str) -> Option<u32> {
    let clean = word.trim();
    if clean.is_empty() {
        return None;
    }

    if let Some(hex) = clean.strip_prefix("0x").or_else(|| clean.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16).ok();
    }
    if let Some(bin) = clean.strip_prefix("0b").or_else(|| clean.strip_prefix("0B")) {
        return u32::from_str_radix(bin, 2).ok();
    }
    if let Some(oct) = clean.strip_prefix("0o").or_else(|| clean.strip_prefix("0O")) {
        return u32::from_str_radix(oct, 8).ok();
    }
    if clean.chars().all(|c| c.is_ascii_digit()) {
        return clean.parse::<u32>().ok();
    }
    None
}

fn format_number_hover(val: u32) -> String {
    let dec = format!("{}", val);
    let hex = format!("0x{:X}", val);
    let bin = format!("0b{:b}", val);
    let oct = format!("0o{:o}", val);
    let ascii = if val <= 0x7F {
        match val as u8 {
            b'\n' => "'\\n'".to_string(),
            b'\r' => "'\\r'".to_string(),
            b'\t' => "'\\t'".to_string(),
            b'\0' => "'\\0'".to_string(),
            b'\\' => "'\\\\'".to_string(),
            b'\'' => "'\\''".to_string(),
            32..=126 => format!("'{}'", (val as u8) as char),
            _ => format!("'\\x{:02X}'", val),
        }
    } else {
        "-".to_string()
    };

    format!(
        "```\n\
         Decimal:     {}\n\
         Hexadecimal: {}\n\
         Binary:      {}\n\
         Octal:       {}\n\
         ASCII:       {}\n\
         ```",
        dec, hex, bin, oct, ascii
    )
}

fn get_instruction_hover(mnemonic: &str) -> Option<&'static str> {
    match mnemonic {
        "MOV" => Some(
            "**Instruction** `MOV dest, src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: None\n\n\
             Copies data from the source register/memory to the destination register/memory.\n\n\
             **Example:**\n\
             ```e8085\n\
             mov A, B\n\
             ```",
        ),
        "MVI" => Some(
            "**Instruction** `MVI dest, data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states (10 T for M)\n\
             - **Flags affected**: None\n\n\
             Loads an 8-bit immediate value into the specified register or memory location.\n\n\
             **Example:**\n\
             ```e8085\n\
             mvi A, 0x42\n\
             ```",
        ),
        "LXI" => Some(
            "**Instruction** `LXI pair, data16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states\n\
             - **Flags affected**: None\n\n\
             Loads a 16-bit constant address or value into register pair (BC, DE, HL, SP).\n\n\
             **Example:**\n\
             ```e8085\n\
             lxi HL, 0x1000\n\
             ```",
        ),
        "LDA" => Some(
            "**Instruction** `LDA addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 13 T-states\n\
             - **Flags affected**: None\n\n\
             Copies the byte at the specified 16-bit memory address into Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             lda 0x2000\n\
             ```",
        ),
        "STA" => Some(
            "**Instruction** `STA addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 13 T-states\n\
             - **Flags affected**: None\n\n\
             Stores the contents of Accumulator A into the specified 16-bit memory address.\n\n\
             **Example:**\n\
             ```e8085\n\
             sta 0x2000\n\
             ```",
        ),
        "LHLD" => Some(
            "**Instruction** `LHLD addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 16 T-states\n\
             - **Flags affected**: None\n\n\
             Loads register pair HL from the specified 16-bit memory address.\n\n\
             **Example:**\n\
             ```e8085\n\
             lhld 0x2000\n\
             ```",
        ),
        "SHLD" => Some(
            "**Instruction** `SHLD addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 16 T-states\n\
             - **Flags affected**: None\n\n\
             Stores register pair HL into the specified 16-bit memory address.\n\n\
             **Example:**\n\
             ```e8085\n\
             shld 0x2000\n\
             ```",
        ),
        "LDAX" => Some(
            "**Instruction** `LDAX pair`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: None\n\n\
             Copies the byte at the memory address pointed to by BC or DE into Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             ldax BC\n\
             ```",
        ),
        "STAX" => Some(
            "**Instruction** `STAX pair`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: None\n\n\
             Stores the byte in Accumulator A into the memory address pointed to by BC or DE.\n\n\
             **Example:**\n\
             ```e8085\n\
             stax DE\n\
             ```",
        ),
        "XCHG" => Some(
            "**Instruction** `XCHG`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: None\n\n\
             Swaps the contents of register pair HL with register pair DE.\n\n\
             **Example:**\n\
             ```e8085\n\
             xchg\n\
             ```",
        ),
        "XTHL" => Some(
            "**Instruction** `XTHL`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 16 T-states\n\
             - **Flags affected**: None\n\n\
             Exchanges the contents of the top of the stack with register pair HL.\n\n\
             **Example:**\n\
             ```e8085\n\
             xthl\n\
             ```",
        ),
        "SPHL" => Some(
            "**Instruction** `SPHL`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 6 T-states\n\
             - **Flags affected**: None\n\n\
             Copies the contents of register pair HL into the Stack Pointer SP.\n\n\
             **Example:**\n\
             ```e8085\n\
             sphl\n\
             ```",
        ),
        "PCHL" => Some(
            "**Instruction** `PCHL`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 6 T-states\n\
             - **Flags affected**: None\n\n\
             Jumps to the memory address contained in register pair HL.\n\n\
             **Example:**\n\
             ```e8085\n\
             pchl\n\
             ```",
        ),
        "ADD" => Some(
            "**Instruction** `ADD src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Adds the contents of the specified register or memory to Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             add B\n\
             ```",
        ),
        "ADI" => Some(
            "**Instruction** `ADI data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Adds the 8-bit immediate byte to Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             adi 0x05\n\
             ```",
        ),
        "ADC" => Some(
            "**Instruction** `ADC src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Adds the specified register/memory and the Carry flag to Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             adc C\n\
             ```",
        ),
        "ACI" => Some(
            "**Instruction** `ACI data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Adds the immediate byte and the Carry flag to Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             aci 0x01\n\
             ```",
        ),
        "SUB" => Some(
            "**Instruction** `SUB src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Subtracts the specified register or memory byte from Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             sub D\n\
             ```",
        ),
        "SUI" => Some(
            "**Instruction** `SUI data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Subtracts the immediate byte from Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             sui 0x0A\n\
             ```",
        ),
        "SBB" => Some(
            "**Instruction** `SBB src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Subtracts the specified register/memory byte and Borrow (Carry) from Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             sbb E\n\
             ```",
        ),
        "SBI" => Some(
            "**Instruction** `SBI data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Subtracts the immediate byte and Borrow (Carry) from Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             sbi 0x01\n\
             ```",
        ),
        "INR" => Some(
            "**Instruction** `INR dest`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (10 T for M)\n\
             - **Flags affected**: Z, S, P, AC\n\n\
             Increments the specified register or memory byte by 1 (Carry flag is unaffected).\n\n\
             **Example:**\n\
             ```e8085\n\
             inr A\n\
             ```",
        ),
        "DCR" => Some(
            "**Instruction** `DCR dest`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (10 T for M)\n\
             - **Flags affected**: Z, S, P, AC\n\n\
             Decrements the specified register or memory byte by 1 (Carry flag is unaffected).\n\n\
             **Example:**\n\
             ```e8085\n\
             dcr B\n\
             ```",
        ),
        "INX" => Some(
            "**Instruction** `INX pair`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 6 T-states\n\
             - **Flags affected**: None\n\n\
             Increments the specified 16-bit register pair by 1.\n\n\
             **Example:**\n\
             ```e8085\n\
             inx HL\n\
             ```",
        ),
        "DCX" => Some(
            "**Instruction** `DCX pair`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 6 T-states\n\
             - **Flags affected**: None\n\n\
             Decrements the specified 16-bit register pair by 1.\n\n\
             **Example:**\n\
             ```e8085\n\
             dcx DE\n\
             ```",
        ),
        "DAD" => Some(
            "**Instruction** `DAD pair`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 10 T-states\n\
             - **Flags affected**: CY\n\n\
             Adds the 16-bit contents of the specified register pair to HL.\n\n\
             **Example:**\n\
             ```e8085\n\
             dad BC\n\
             ```",
        ),
        "DAA" => Some(
            "**Instruction** `DAA`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Adjusts the eight-bit value in Accumulator A to form two 4-bit packed BCD digits.\n\n\
             **Example:**\n\
             ```e8085\n\
             daa\n\
             ```",
        ),
        "ANA" => Some(
            "**Instruction** `ANA src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: Z, S, P, CY=0, AC=1\n\n\
             Performs logical AND between Accumulator A and the specified register or memory.\n\n\
             **Example:**\n\
             ```e8085\n\
             ana B\n\
             ```",
        ),
        "ANI" => Some(
            "**Instruction** `ANI data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: Z, S, P, CY=0, AC=1\n\n\
             Performs logical AND between Accumulator A and the 8-bit immediate byte.\n\n\
             **Example:**\n\
             ```e8085\n\
             ani 0x0F\n\
             ```",
        ),
        "XRA" => Some(
            "**Instruction** `XRA src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: Z, S, P, CY=0, AC=0\n\n\
             Performs logical XOR between Accumulator A and the specified register or memory.\n\n\
             **Example:**\n\
             ```e8085\n\
             xra A\n\
             ```",
        ),
        "XRI" => Some(
            "**Instruction** `XRI data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: Z, S, P, CY=0, AC=0\n\n\
             Performs logical XOR between Accumulator A and the 8-bit immediate byte.\n\n\
             **Example:**\n\
             ```e8085\n\
             xri 0xFF\n\
             ```",
        ),
        "ORA" => Some(
            "**Instruction** `ORA src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: Z, S, P, CY=0, AC=0\n\n\
             Performs logical OR between Accumulator A and the specified register or memory.\n\n\
             **Example:**\n\
             ```e8085\n\
             ora C\n\
             ```",
        ),
        "ORI" => Some(
            "**Instruction** `ORI data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: Z, S, P, CY=0, AC=0\n\n\
             Performs logical OR between Accumulator A and the 8-bit immediate byte.\n\n\
             **Example:**\n\
             ```e8085\n\
             ori 0x80\n\
             ```",
        ),
        "CMP" => Some(
            "**Instruction** `CMP src`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for M)\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Compares Accumulator A with the specified register or memory byte without modifying A.\n\n\
             **Example:**\n\
             ```e8085\n\
             cmp D\n\
             ```",
        ),
        "CPI" => Some(
            "**Instruction** `CPI data8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags affected**: Z, S, P, CY, AC\n\n\
             Compares Accumulator A with the 8-bit immediate byte without modifying A.\n\n\
             **Example:**\n\
             ```e8085\n\
             cpi 0x00\n\
             ```",
        ),
        "RLC" => Some(
            "**Instruction** `RLC`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: CY\n\n\
             Rotates Accumulator A left by 1 bit. Bit 7 moves to Carry and Bit 0.\n\n\
             **Example:**\n\
             ```e8085\n\
             rlc\n\
             ```",
        ),
        "RRC" => Some(
            "**Instruction** `RRC`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: CY\n\n\
             Rotates Accumulator A right by 1 bit. Bit 0 moves to Carry and Bit 7.\n\n\
             **Example:**\n\
             ```e8085\n\
             rrc\n\
             ```",
        ),
        "RAL" => Some(
            "**Instruction** `RAL`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: CY\n\n\
             Rotates Accumulator A left through Carry flag.\n\n\
             **Example:**\n\
             ```e8085\n\
             ral\n\
             ```",
        ),
        "RAR" => Some(
            "**Instruction** `RAR`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: CY\n\n\
             Rotates Accumulator A right through Carry flag.\n\n\
             **Example:**\n\
             ```e8085\n\
             rar\n\
             ```",
        ),
        "CMA" => Some(
            "**Instruction** `CMA`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: None\n\n\
             Complements each bit of Accumulator A (1's complement).\n\n\
             **Example:**\n\
             ```e8085\n\
             cma\n\
             ```",
        ),
        "CMC" => Some(
            "**Instruction** `CMC`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: CY\n\n\
             Complements the Carry flag.\n\n\
             **Example:**\n\
             ```e8085\n\
             cmc\n\
             ```",
        ),
        "STC" => Some(
            "**Instruction** `STC`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: CY=1\n\n\
             Sets the Carry flag to 1.\n\n\
             **Example:**\n\
             ```e8085\n\
             stc\n\
             ```",
        ),
        "JMP" => Some(
            "**Instruction** `JMP addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states\n\
             - **Flags affected**: None\n\n\
             Unconditionally jumps program execution to the specified 16-bit address.\n\n\
             **Example:**\n\
             ```e8085\n\
             jmp main\n\
             ```",
        ),
        "JZ" => Some(
            "**Instruction** `JZ addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags affected**: None\n\n\
             Jumps to the specified address if Zero flag is set (Z == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             jz .exit\n\
             ```",
        ),
        "JNZ" => Some(
            "**Instruction** `JNZ addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags affected**: None\n\n\
             Jumps to the specified address if Zero flag is clear (Z == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             jnz .loop\n\
             ```",
        ),
        "JC" => Some(
            "**Instruction** `JC addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags affected**: None\n\n\
             Jumps to the specified address if Carry flag is set (CY == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             jc overflow\n\
             ```",
        ),
        "JNC" => Some(
            "**Instruction** `JNC addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags affected**: None\n\n\
             Jumps to the specified address if Carry flag is clear (CY == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             jnc no_carry\n\
             ```",
        ),
        "JP" => Some(
            "**Instruction** `JP addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags affected**: None\n\n\
             Jumps to the specified address if Sign flag is positive (S == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             jp positive\n\
             ```",
        ),
        "JM" => Some(
            "**Instruction** `JM addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags affected**: None\n\n\
             Jumps to the specified address if Sign flag is minus (S == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             jm negative\n\
             ```",
        ),
        "JPE" => Some(
            "**Instruction** `JPE addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags affected**: None\n\n\
             Jumps to the specified address if Parity is even (P == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             jpe even_parity\n\
             ```",
        ),
        "JPO" => Some(
            "**Instruction** `JPO addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags affected**: None\n\n\
             Jumps to the specified address if Parity is odd (P == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             jpo odd_parity\n\
             ```",
        ),
        "CALL" => Some(
            "**Instruction** `CALL addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states\n\
             - **Flags affected**: None\n\n\
             Pushes return address onto stack and calls subroutine at specified address.\n\n\
             **Example:**\n\
             ```e8085\n\
             call print\n\
             ```",
        ),
        "CZ" => Some(
            "**Instruction** `CZ addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states (9 T if false)\n\
             - **Flags affected**: None\n\n\
             Calls subroutine if Zero flag is set (Z == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             cz handle_zero\n\
             ```",
        ),
        "CNZ" => Some(
            "**Instruction** `CNZ addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states (9 T if false)\n\
             - **Flags affected**: None\n\n\
             Calls subroutine if Zero flag is clear (Z == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             cnz handle_nonzero\n\
             ```",
        ),
        "CC" => Some(
            "**Instruction** `CC addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states (9 T if false)\n\
             - **Flags affected**: None\n\n\
             Calls subroutine if Carry flag is set (CY == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             cc handle_carry\n\
             ```",
        ),
        "CNC" => Some(
            "**Instruction** `CNC addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states (9 T if false)\n\
             - **Flags affected**: None\n\n\
             Calls subroutine if Carry flag is clear (CY == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             cnc handle_nocarry\n\
             ```",
        ),
        "CP" => Some(
            "**Instruction** `CP addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states (9 T if false)\n\
             - **Flags affected**: None\n\n\
             Calls subroutine if Sign flag is positive (S == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             cp handle_pos\n\
             ```",
        ),
        "CM" => Some(
            "**Instruction** `CM addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states (9 T if false)\n\
             - **Flags affected**: None\n\n\
             Calls subroutine if Sign flag is minus (S == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             cm handle_neg\n\
             ```",
        ),
        "CPE" => Some(
            "**Instruction** `CPE addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states (9 T if false)\n\
             - **Flags affected**: None\n\n\
             Calls subroutine if Parity is even (P == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             cpe handle_even\n\
             ```",
        ),
        "CPO" => Some(
            "**Instruction** `CPO addr16`\n\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states (9 T if false)\n\
             - **Flags affected**: None\n\n\
             Calls subroutine if Parity is odd (P == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             cpo handle_odd\n\
             ```",
        ),
        "RET" => Some(
            "**Instruction** `RET`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 10 T-states\n\
             - **Flags affected**: None\n\n\
             Pops return address from stack and returns execution to caller.\n\n\
             **Example:**\n\
             ```e8085\n\
             ret\n\
             ```",
        ),
        "RZ" => Some(
            "**Instruction** `RZ`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states (6 T if false)\n\
             - **Flags affected**: None\n\n\
             Returns from subroutine if Zero flag is set (Z == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             rz\n\
             ```",
        ),
        "RNZ" => Some(
            "**Instruction** `RNZ`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states (6 T if false)\n\
             - **Flags affected**: None\n\n\
             Returns from subroutine if Zero flag is clear (Z == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             rnz\n\
             ```",
        ),
        "RC" => Some(
            "**Instruction** `RC`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states (6 T if false)\n\
             - **Flags affected**: None\n\n\
             Returns from subroutine if Carry flag is set (CY == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             rc\n\
             ```",
        ),
        "RNC" => Some(
            "**Instruction** `RNC`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states (6 T if false)\n\
             - **Flags affected**: None\n\n\
             Returns from subroutine if Carry flag is clear (CY == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             rnc\n\
             ```",
        ),
        "RP" => Some(
            "**Instruction** `RP`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states (6 T if false)\n\
             - **Flags affected**: None\n\n\
             Returns from subroutine if Sign flag is positive (S == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             rp\n\
             ```",
        ),
        "RM" => Some(
            "**Instruction** `RM`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states (6 T if false)\n\
             - **Flags affected**: None\n\n\
             Returns from subroutine if Sign flag is minus (S == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             rm\n\
             ```",
        ),
        "RPE" => Some(
            "**Instruction** `RPE`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states (6 T if false)\n\
             - **Flags affected**: None\n\n\
             Returns from subroutine if Parity is even (P == 1).\n\n\
             **Example:**\n\
             ```e8085\n\
             rpe\n\
             ```",
        ),
        "RPO" => Some(
            "**Instruction** `RPO`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states (6 T if false)\n\
             - **Flags affected**: None\n\n\
             Returns from subroutine if Parity is odd (P == 0).\n\n\
             **Example:**\n\
             ```e8085\n\
             rpo\n\
             ```",
        ),
        "RST" => Some(
            "**Instruction** `RST n`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states\n\
             - **Flags affected**: None\n\n\
             Pushes PC onto stack and vectors to software interrupt address `8 * n`.\n\n\
             **Example:**\n\
             ```e8085\n\
             rst 1\n\
             ```",
        ),
        "PUSH" => Some(
            "**Instruction** `PUSH pair`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states\n\
             - **Flags affected**: None\n\n\
             Pushes 16-bit register pair (BC, DE, HL) or PSW onto the stack.\n\n\
             **Example:**\n\
             ```e8085\n\
             push HL\n\
             ```",
        ),
        "POP" => Some(
            "**Instruction** `POP pair`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 10 T-states\n\
             - **Flags affected**: Z, S, P, CY, AC (if PSW), None (otherwise)\n\n\
             Pops 16-bit register pair (BC, DE, HL) or PSW from the stack.\n\n\
             **Example:**\n\
             ```e8085\n\
             pop HL\n\
             ```",
        ),
        "IN" => Some(
            "**Instruction** `IN port8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 10 T-states\n\
             - **Flags affected**: None\n\n\
             Reads an 8-bit byte from the specified I/O device port into Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             in 0x01\n\
             ```",
        ),
        "OUT" => Some(
            "**Instruction** `OUT port8`\n\n\
             - **Bytes**: 2\n\
             - **Cycles**: 10 T-states\n\
             - **Flags affected**: None\n\n\
             Sends the byte in Accumulator A to the specified 8-bit I/O device port.\n\n\
             **Example:**\n\
             ```e8085\n\
             out 0x02\n\
             ```",
        ),
        "NOP" => Some(
            "**Instruction** `NOP`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: None\n\n\
             Performs no operation and advances PC by 1.\n\n\
             **Example:**\n\
             ```e8085\n\
             nop\n\
             ```",
        ),
        "HLT" => Some(
            "**Instruction** `HLT`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 5 T-states\n\
             - **Flags affected**: None\n\n\
             Stops CPU instruction execution until an interrupt (TRAP/RST) or RESET occurs.\n\n\
             **Example:**\n\
             ```e8085\n\
             hlt\n\
             ```",
        ),
        "EI" => Some(
            "**Instruction** `EI`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: None\n\n\
             Enables the maskable interrupt system.\n\n\
             **Example:**\n\
             ```e8085\n\
             ei\n\
             ```",
        ),
        "DI" => Some(
            "**Instruction** `DI`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: None\n\n\
             Disables the maskable interrupt system.\n\n\
             **Example:**\n\
             ```e8085\n\
             di\n\
             ```",
        ),
        "RIM" => Some(
            "**Instruction** `RIM`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: None\n\n\
             Reads interrupt masks, pending interrupts, and SID pin into Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             rim\n\
             ```",
        ),
        "SIM" => Some(
            "**Instruction** `SIM`\n\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags affected**: None\n\n\
             Programs RST 7.5/6.5/5.5 interrupt masks and outputs to SOD pin from Accumulator A.\n\n\
             **Example:**\n\
             ```e8085\n\
             sim\n\
             ```",
        ),
        _ => None,
    }
}

fn get_register_hover(reg: &str) -> Option<String> {
    match reg {
        "A" => Some(
            "**Register** `A` (8-bit)\n\n\
             Primary 8-bit Accumulator register. Destination for all ALU arithmetic and logical operations."
                .to_string(),
        ),
        "B" | "C" | "D" | "E" | "H" | "L" => Some(format!(
            "**Register** `{}` (8-bit)\n\n\
             General-purpose 8-bit CPU register. Can be used individually or combined as a 16-bit register pair.",
            reg
        )),
        "M" => Some(
            "**Memory** `M` (8-bit pointer [HL])\n\n\
             Memory reference pointing to the 8-bit RAM location addressed by the HL register pair."
                .to_string(),
        ),
        "BC" => Some(
            "**Register** `BC` (16-bit pair)\n\n\
             16-bit register pair composed of high byte B and low byte C."
                .to_string(),
        ),
        "DE" => Some(
            "**Register** `DE` (16-bit pair)\n\n\
             16-bit register pair composed of high byte D and low byte E."
                .to_string(),
        ),
        "HL" => Some(
            "**Register** `HL` (16-bit pair)\n\n\
             Primary 16-bit memory pointer register pair composed of high byte H and low byte L."
                .to_string(),
        ),
        "SP" => Some(
            "**Register** `SP` (16-bit)\n\n\
             Stack Pointer register pointing to the current top of the descending stack in RAM."
                .to_string(),
        ),
        "PSW" => Some(
            "**Register** `PSW` (16-bit)\n\n\
             Program Status Word pair composed of Accumulator A (high byte) and CPU Flags (low byte: `[S Z 0 AC 0 P 1 CY]`)."
                .to_string(),
        ),
        _ => None,
    }
}

fn get_directive_hover(directive: &str) -> Option<&'static str> {
    match directive.to_lowercase().as_str() {
        "%define" => Some(
            "**Directive** `%define NAME VALUE`\n\n\
             Defines a compile-time numeric, character, or string constant.\n\n\
             **Example:**\n\
             ```e8085\n\
             %define BUFFER_SIZE 64\n\
             ```",
        ),
        "%include" => Some(
            "**Directive** `%include \"path/to/file.e8085\"`\n\n\
             Textually embeds and compiles the referenced assembly source file.\n\n\
             **Example:**\n\
             ```e8085\n\
             %include \"devices/terminal.e8085\"\n\
             ```",
        ),
        "%repeat" => Some(
            "**Directive** `%repeat COUNT VALUE`\n\n\
             Repeats a byte, word, or character value multiple times in memory.\n\n\
             **Example:**\n\
             ```e8085\n\
             buffer BYTE %repeat 16 0x00\n\
             ```",
        ),
        "%len" => Some(
            "**Directive** `%len(VAR_NAME)`\n\n\
             Evaluates to the byte length of the referenced data variable or string.\n\n\
             **Example:**\n\
             ```e8085\n\
             mvi B, %len(prompt)\n\
             ```",
        ),
        "global" => Some(
            "**Keyword** `global`\n\n\
             Exports a label or subroutine symbol to the `.8085.bin` symbol table for external linking.\n\n\
             **Example 1:** Inline declaration\n\
             ```e8085\n\
             global print:\n\
             ```\n\n\
             **Example 2:** Standalone declaration\n\
             ```e8085\n\
             global print\n\
             ```",
        ),
        "extern" => Some(
            "**Keyword** `extern`\n\n\
             Declares an external symbol to be resolved at link time via `-l library.8085.bin`.\n\n\
             **Example:**\n\
             ```e8085\n\
             extern print\n\
             ```",
        ),
        "segment" => Some(
            "**Keyword** `segment`\n\n\
             Specifies the active memory segment for subsequent declarations and code.\n\n\
             **Example 1:** Text Segment\n\
             ```e8085\n\
             segment .text\n\
             ```\n\n\
             **Example 2:** Data Segment\n\
             ```e8085\n\
             segment .data\n\
             ```\n\n\
             **Example 3:** BSS Segment\n\
             ```e8085\n\
             segment .bss\n\
             ```",
        ),
        ".text" | "text" => Some(
            "**Segment** `.text`\n\n\
             Executable machine code segment. Stores program instructions and subroutine entry points.\n\n\
             **Example:**\n\
             ```e8085\n\
             segment .text\n\
             main:\n\
                 mvi A, 0x05\n\
                 hlt\n\
             ```",
        ),
        ".data" | "data" => Some(
            "**Segment** `.data`\n\n\
             Initialized data segment. Stores strings, byte arrays, and initialized numeric variables.\n\n\
             **Example:**\n\
             ```e8085\n\
             segment .data\n\
             greeting \"Hello, World!\\n\"\n\
             scores BYTE 10, 20, 30\n\
             ```",
        ),
        ".bss" | "bss" => Some(
            "**Segment** `.bss`\n\n\
             Uninitialized zero-allocated memory buffer segment. Stores reserved RAM variables.\n\n\
             **Example:**\n\
             ```e8085\n\
             segment .bss\n\
             input_buffer BYTE 64\n\
             ```",
        ),
        "byte" => Some(
            "**Keyword** `BYTE count`\n\n\
             Declares 8-bit byte memory storage in data or uninitialized buffer in BSS.\n\n\
             **Example:**\n\
             ```e8085\n\
             buffer BYTE 32\n\
             ```",
        ),
        "word" => Some(
            "**Keyword** `WORD count`\n\n\
             Declares 16-bit little-endian word storage in data or buffer in BSS.\n\n\
             **Example:**\n\
             ```e8085\n\
             pointers WORD 10\n\
             ```",
        ),
        _ => None,
    }
}

/// Scans the document source for user label / variable definitions and doc-comments.
fn get_user_symbol_hover(doc: &Document, symbol: &str, position: &Position) -> Option<String> {
    let lines: Vec<&str> = doc.text.lines().collect();

    // 1. First check the current file
    if let Some(hover) = get_user_symbol_hover_in_single_doc(&lines, symbol, Some(position)) {
        return Some(hover);
    }

    // 2. If not defined locally, search in %include imported files
    let mut visited = std::collections::HashSet::new();
    find_user_symbol_hover_in_included_files(doc, symbol, &mut visited)
}

fn get_user_symbol_hover_in_single_doc(lines: &[&str], symbol: &str, position: Option<&Position>) -> Option<String> {
    // 1. Check extern declarations (e.g. `extern print`)
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("extern ") {
            let name = rest.trim();
            if name == symbol {
                let doc_comment = collect_preceding_comments(lines, i);
                // let mut out = format!("**Extern** `{}`", symbol);
                let mut out = format!("```\nextern {}\n```", symbol);
                if !doc_comment.is_empty() {
                    out.push_str("\n\n");
                    out.push_str(&doc_comment);
                }
                return Some(out);
            }
        }
    }

    // 2. Check local labels (starts with '.')
    if symbol.starts_with('.') {
        let current_line = position.map(|p| p.line as usize).unwrap_or(lines.len());
        let mut parent_start = 0;
        let mut parent_name = "unknown";
        for i in (0..=current_line.min(lines.len().saturating_sub(1))).rev() {
            let trimmed = lines[i].trim();
            if let Some(rest) = trimmed.strip_suffix(':') {
                let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
                if !clean.starts_with('.') && !clean.is_empty() {
                    parent_start = i;
                    parent_name = clean;
                    break;
                }
            }
        }

        // Search within parent scope
        for (idx, line) in lines[parent_start..].iter().enumerate() {
            let line_num = parent_start + idx;
            let trimmed = line.trim();
            if idx > 0 {
                if let Some(rest) = trimmed.strip_suffix(':') {
                    let clean = rest.strip_prefix("global ").unwrap_or(rest).trim();
                    if !clean.starts_with('.') && !clean.is_empty() {
                        break;
                    }
                }
            }

            if let Some(rest) = trimmed.strip_suffix(':') {
                let clean = rest.trim();
                if clean == symbol {
                    let doc_comment = collect_preceding_comments(lines, line_num);
                    let mut out = format!("```\n{}{}:\n```", parent_name, symbol);
                    if !doc_comment.is_empty() {
                        out.push_str("\n\n");
                        out.push_str(&doc_comment);
                    }
                    return Some(out);
                }
            }
        }
    }

    // 3. Check regular global labels (e.g. `main:`, `global print:`)
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_suffix(':') {
            let label_ident = rest
                .strip_prefix("global ")
                .unwrap_or(rest)
                .trim();

            if label_ident == symbol {
                let doc_comment = collect_preceding_comments(lines, i);
                let mut out = format!("```\n{}:\n```", symbol);
                if !doc_comment.is_empty() {
                    out.push_str("\n\n");
                    out.push_str(&doc_comment);
                }
                return Some(out);
            }
        }
    }

    // 4. Check %define constants
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("%define") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == symbol {
                let val = &trimmed[trimmed.find(parts[2]).unwrap_or(0)..];
                let doc_comment = collect_preceding_comments(lines, i);
                let mut out = format!("```\n%define {} {}\n```", symbol, val);
                if !doc_comment.is_empty() {
                    out.push_str("\n\n");
                    out.push_str(&doc_comment);
                }
                return Some(out);
            }
        }
    }

    // 5. Check Variables in .data / .bss
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("segment ") || trimmed.starts_with('%') {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if !parts.is_empty() && parts[0] == symbol {
            let doc_comment = collect_preceding_comments(lines, i);
            let decl_rest = trimmed[parts[0].len()..].trim();

            let mut out = if decl_rest.starts_with('"') || decl_rest.starts_with('\'') {
                format!("```\n{} byte {}\n```", symbol, decl_rest)
            } else {
                format!("```\n{} {}\n```", symbol, decl_rest)
            };

            if !doc_comment.is_empty() {
                out.push_str("\n\n");
                out.push_str(&doc_comment);
            }
            return Some(out);
        }
    }

    None
}

fn find_user_symbol_hover_in_included_files(
    doc: &Document,
    symbol: &str,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Option<String> {
    for line in doc.text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("%include") {
            let quote_char = if line.contains('"') {
                Some('"')
            } else if line.contains('\'') {
                Some('\'')
            } else {
                None
            };

            if let Some(q) = quote_char {
                if let Some(start_quote) = line.find(q) {
                    if let Some(end_quote_offset) = line[start_quote + 1..].find(q) {
                        let rel_path = &line[start_quote + 1..start_quote + 1 + end_quote_offset];
                        if let Some(target_path) = resolve_relative_path(&doc.uri, rel_path) {
                            if visited.insert(target_path.clone()) {
                                if let Ok(content) = std::fs::read_to_string(&target_path) {
                                    let inc_lines: Vec<&str> = content.lines().collect();
                                    if let Some(hover) = get_user_symbol_hover_in_single_doc(&inc_lines, symbol, None) {
                                        return Some(hover);
                                    }
                                    if let Ok(inc_uri) = Url::from_file_path(&target_path) {
                                        let inc_doc = Document::new(inc_uri, 1, content);
                                        if let Some(hover) = find_user_symbol_hover_in_included_files(&inc_doc, symbol, visited) {
                                            return Some(hover);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn get_include_hover(doc: &Document, position: &Position) -> Option<Hover> {
    let line = doc.text.lines().nth(position.line as usize)?;
    let trimmed = line.trim();

    if trimmed.starts_with("%include") {
        let quote_char = if line.contains('"') {
            Some('"')
        } else if line.contains('\'') {
            Some('\'')
        } else {
            None
        }?;

        let start_quote = line.find(quote_char)?;
        let end_quote_offset = line[start_quote + 1..].find(quote_char)?;
        let end_quote = start_quote + 1 + end_quote_offset;

        let char_col = position.character as usize;
        // Only trigger include module doc when cursor is on/within the quoted string path
        if char_col < start_quote || char_col > end_quote {
            return None;
        }

        let rel_path = &line[start_quote + 1..end_quote];
        let target_path = super::document::resolve_relative_path(&doc.uri, rel_path)?;
        let module_doc = extract_file_module_doc(&target_path);

        let val = module_doc.unwrap_or_default();

        let range = Range {
            start: Position {
                line: position.line,
                character: start_quote as u32,
            },
            end: Position {
                line: position.line,
                character: (end_quote + 1) as u32,
            },
        };

        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: val,
            }),
            range: Some(range),
        });
    }

    None
}

fn extract_file_module_doc(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut comments = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(comment) = trimmed.strip_prefix(';') {
            let clean = comment.trim();
            if !clean.starts_with("====") && !clean.starts_with("----") {
                comments.push(clean);
            }
        } else if trimmed.is_empty() {
            if !comments.is_empty() {
                // Keep collecting comments
            }
        } else {
            // First non-comment, non-empty statement (e.g. %define, segment, instructions)
            break;
        }
    }

    if comments.is_empty() {
        None
    } else {
        Some(comments.join("\n"))
    }
}

fn collect_preceding_comments(lines: &[&str], target_line_idx: usize) -> String {
    let mut comments = Vec::new();
    let mut curr = target_line_idx;

    while curr > 0 {
        curr -= 1;
        let line = lines[curr].trim();
        if let Some(comment) = line.strip_prefix(';') {
            let clean = comment.trim();
            if !clean.starts_with("====") && !clean.starts_with("----") {
                comments.push(clean);
            }
        } else {
            // Non-comment line or blank line terminates doc comment collection
            break;
        }
    }

    comments.reverse();
    comments.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_hover_include_module_doc() {
        let temp_dir = std::env::temp_dir();
        let target_file = temp_dir.join("test_terminal_doc.e8085");
        let target_content = "; Module providing TerminalDevice I/O subroutines.\n; Supports print, input, and putch.\n\nsegment .text\nprint:\n    ret\n";
        std::fs::write(&target_file, target_content).unwrap();

        let source_file = temp_dir.join("main_hover.e8085");
        let text = "%include \"test_terminal_doc.e8085\"\nmain:\n    hlt\n".to_string();
        let uri = Url::from_file_path(&source_file).unwrap();
        let doc = Document::new(uri, 1, text);

        let h_inc = get_hover(&doc, &Position { line: 0, character: 12 }).unwrap();
        if let HoverContents::Markup(m) = h_inc.contents {
            assert!(m.value.contains("Module providing TerminalDevice I/O"));
            assert!(m.value.contains("Supports print, input, and putch."));
        } else {
            panic!("expected markup");
        }

        let range = h_inc.range.unwrap();
        assert_eq!(range.start.character, 9);
        assert_eq!(range.end.character, 34);

        let _ = std::fs::remove_file(target_file);
    }

    #[test]
    fn test_hover_instruction_and_register() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    mvi A, 0x05\n    lxi HL, 0x1000\n    call print\n".to_string();
        let doc = Document::new(uri, 1, text);

        // Hover over MVI
        let h_mvi = get_hover(&doc, &Position { line: 1, character: 5 }).unwrap();
        if let HoverContents::Markup(m) = h_mvi.contents {
            assert!(m.value.contains("**Instruction** `MVI dest, data8`"));
            assert!(m.value.contains("7 T-states"));
        } else {
            panic!("expected markup");
        }

        // Hover over A
        let h_a = get_hover(&doc, &Position { line: 1, character: 8 }).unwrap();
        if let HoverContents::Markup(m) = h_a.contents {
            assert!(m.value.contains("**Register** `A` (8-bit)"));
        } else {
            panic!("expected markup");
        }

        // Hover over HL
        let h_hl = get_hover(&doc, &Position { line: 2, character: 9 }).unwrap();
        if let HoverContents::Markup(m) = h_hl.contents {
            assert!(m.value.contains("**Register** `HL` (16-bit pair)"));
        } else {
            panic!("expected markup");
        }
    }

    #[test]
    fn test_hover_number_formats() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    mvi A, 0x41\n    mvi B, 10\n".to_string();
        let doc = Document::new(uri, 1, text);

        // Hover over 0x41 ('A' / 65)
        let h_num = get_hover(&doc, &Position { line: 1, character: 12 }).unwrap();
        if let HoverContents::Markup(m) = h_num.contents {
            assert!(m.value.contains("decimal:"));
            assert!(m.value.contains("65"));
            assert!(m.value.contains("0x41"));
            assert!(m.value.contains("'A'"));
        } else {
            panic!("expected markup");
        }

        // Hover over 10 ('\n')
        let h_num10 = get_hover(&doc, &Position { line: 2, character: 12 }).unwrap();
        if let HoverContents::Markup(m) = h_num10.contents {
            assert!(m.value.contains("decimal:"));
            assert!(m.value.contains("10"));
            assert!(m.value.contains("0xA"));
            assert!(m.value.contains("'\\n'"));
        } else {
            panic!("expected markup");
        }
    }

    #[test]
    fn test_hover_local_label_and_extern() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
extern print_str

func_a:
; Loop counter doc
.loop:
    dcr A
    jnz .loop
    call print_str
    ret
"#.to_string();
        let doc = Document::new(uri, 1, text);

        // Hover over .loop
        let h_local = get_hover(&doc, &Position { line: 7, character: 9 }).unwrap();
        if let HoverContents::Markup(m) = h_local.contents {
            assert!(m.value.contains("func_a.loop:"));
            assert!(m.value.contains("Loop counter doc"));
        } else {
            panic!("expected markup");
        }

        // Hover over print_str
        let h_ext = get_hover(&doc, &Position { line: 8, character: 11 }).unwrap();
        if let HoverContents::Markup(m) = h_ext.contents {
            assert!(m.value.contains("extern print_str"));
        } else {
            panic!("expected markup");
        }
    }

    #[test]
    fn test_hover_user_symbol_with_doc_comment() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
; Multiplies two numbers in B and C
; Result returned in A
multiply:
    mov A, B
    ret

main:
    call multiply
    hlt
"#.to_string();
        let doc = Document::new(uri, 1, text);

        let h_sym = get_hover(&doc, &Position { line: 8, character: 11 }).unwrap();
        if let HoverContents::Markup(m) = h_sym.contents {
            assert!(m.value.contains("multiply:"));
            assert!(m.value.contains("Multiplies two numbers in B and C"));
            assert!(m.value.contains("Result returned in A"));
        } else {
            panic!("expected markup");
        }
    }

    #[test]
    fn test_hover_directives_and_segments() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "segment .text\n%define MAX_LEN 32\nsegment .data\nsegment .bss\n".to_string();
        let doc = Document::new(uri, 1, text);

        // Hover over `segment`
        let h_seg = get_hover(&doc, &Position { line: 0, character: 2 }).unwrap();
        if let HoverContents::Markup(m) = h_seg.contents {
            assert!(m.value.contains("**Keyword** `segment`"));
            assert!(m.value.contains("Text Segment"));
        } else {
            panic!("expected markup");
        }

        // Hover over `.text`
        let h_text = get_hover(&doc, &Position { line: 0, character: 10 }).unwrap();
        if let HoverContents::Markup(m) = h_text.contents {
            assert!(m.value.contains("**Segment** `.text`"));
        } else {
            panic!("expected markup");
        }

        // Hover over `%define`
        let h_def = get_hover(&doc, &Position { line: 1, character: 2 }).unwrap();
        if let HoverContents::Markup(m) = h_def.contents {
            assert!(m.value.contains("**Directive** `%define NAME VALUE`"));
        } else {
            panic!("expected markup");
        }
    }

    #[test]
    fn test_hover_variables_and_memory_m() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
%define BUFFER_CAP 64

segment .data
; User greeting string
greeting "Hello, World!\n"
scores BYTE 10, 20, 30

segment .bss
; Storage for input line
input_buf BYTE 64

segment .text
main:
    mov A, M
    mvi B, BUFFER_CAP
    hlt
"#.to_string();
        let doc = Document::new(uri, 1, text);

        // Hover over memory M
        let h_m = get_hover(&doc, &Position { line: 14, character: 11 }).unwrap();
        if let HoverContents::Markup(m) = h_m.contents {
            assert!(m.value.contains("**Memory** `M` (8-bit pointer [HL])"));
        } else {
            panic!("expected markup");
        }

        // Hover over greeting variable
        let h_greet = get_hover(&doc, &Position { line: 5, character: 2 }).unwrap();
        if let HoverContents::Markup(m) = h_greet.contents {
            assert!(m.value.contains("greeting byte \"Hello, World!\\n\""));
            assert!(m.value.contains("User greeting string"));
        } else {
            panic!("expected markup");
        }

        // Hover over input_buf BSS variable
        let h_buf = get_hover(&doc, &Position { line: 10, character: 2 }).unwrap();
        if let HoverContents::Markup(m) = h_buf.contents {
            assert!(m.value.contains("input_buf BYTE 64"));
            assert!(m.value.contains("Storage for input line"));
        } else {
            panic!("expected markup");
        }

        // Hover over BUFFER_CAP constant
        let h_const = get_hover(&doc, &Position { line: 15, character: 14 }).unwrap();
        if let HoverContents::Markup(m) = h_const.contents {
            assert!(m.value.contains("%define BUFFER_CAP 64"));
        } else {
            panic!("expected markup");
        }
    }

    #[test]
    fn test_hover_imported_symbol_from_include() {
        let temp_dir = std::env::temp_dir();
        let helper_file = temp_dir.join("lib_math_helper.e8085");
        let helper_content = r#"
; Multiplies two 8-bit numbers in B and C
; Returns product in HL
multiply_fast:
    ret
"#;
        std::fs::write(&helper_file, helper_content).unwrap();

        let main_file = temp_dir.join("main_calc.e8085");
        let text = "%include \"lib_math_helper.e8085\"\n\nmain:\n    call multiply_fast\n    hlt\n".to_string();
        let uri = Url::from_file_path(&main_file).unwrap();
        let doc = Document::new(uri, 1, text);

        // Hover over `multiply_fast` in main
        let h_sym = get_hover(&doc, &Position { line: 3, character: 12 }).unwrap();
        if let HoverContents::Markup(m) = h_sym.contents {
            assert!(m.value.contains("multiply_fast:"));
            assert!(m.value.contains("Multiplies two 8-bit numbers in B and C"));
            assert!(m.value.contains("Returns product in HL"));
        } else {
            panic!("expected markup with imported docstring");
        }

        let _ = std::fs::remove_file(helper_file);
    }

    #[test]
    fn test_hover_non_contiguous_comment_not_docstring() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = r#"
; This is an unrelated comment separated by an empty line

%define LIMIT 42

segment .text
main:
    mvi A, LIMIT
    hlt
"#.to_string();
        let doc = Document::new(uri, 1, text);

        // Hover over LIMIT constant
        let h_const = get_hover(&doc, &Position { line: 3, character: 10 }).unwrap();
        if let HoverContents::Markup(m) = h_const.contents {
            assert_eq!(m.value, "```\n%define LIMIT 42\n```");
            assert!(!m.value.contains("unrelated comment"));
        } else {
            panic!("expected markup");
        }
    }
}
