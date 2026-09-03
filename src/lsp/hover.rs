use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

use super::document::Document;

/// Provides rich hover documentation for 8085 instructions, registers, directives, and symbols.
pub fn get_hover(doc: &Document, position: &Position) -> Option<Hover> {
    let (word, range) = doc.get_word_at_position(position)?;
    let upper_word = word.to_uppercase();

    // 1. Check 8085 Instruction Mnemonics
    if let Some(doc_str) = get_instruction_hover(&upper_word) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_str.to_string(),
            }),
            range: Some(range),
        });
    }

    // 2. Check 8085 Registers
    if let Some(doc_str) = get_register_hover(&upper_word) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_str.to_string(),
            }),
            range: Some(range),
        });
    }

    // 3. Check Directives & Keywords
    if let Some(doc_str) = get_directive_hover(&word) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_str.to_string(),
            }),
            range: Some(range),
        });
    }

    // 4. Check User-defined Symbol (label or variable in source)
    if let Some(doc_str) = get_user_symbol_hover(doc, &word) {
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

fn get_instruction_hover(mnemonic: &str) -> Option<&'static str> {
    match mnemonic {
        "MOV" => Some(
            "### `MOV destination, source` (Move Register)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T if memory `M`)\n\
             - **Flags Affected**: None\n\
             - **Operation**: `destination <- source`\n\n\
             Copies data from the source register/memory to the destination register/memory.",
        ),
        "MVI" => Some(
            "### `MVI register, data8` (Move Immediate 8-bit)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states (10 T if memory `M`)\n\
             - **Flags Affected**: None\n\
             - **Operation**: `register <- byte`\n\n\
             Loads the 8-bit immediate byte into the specified register or memory location.",
        ),
        "LXI" => Some(
            "### `LXI reg_pair, data16` (Load Register Pair Immediate)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `reg_pair <- data16` (High byte in 1st reg, Low byte in 2nd reg)\n\n\
             Loads a 16-bit constant address or value into `BC`, `DE`, `HL`, or `SP`.",
        ),
        "LDA" => Some(
            "### `LDA addr16` (Load Accumulator Direct)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 13 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `A <- [addr16]`\n\n\
             Copies the byte at the specified 16-bit memory address into Accumulator `A`.",
        ),
        "STA" => Some(
            "### `STA addr16` (Store Accumulator Direct)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 13 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `[addr16] <- A`\n\n\
             Stores the contents of Accumulator `A` into the specified 16-bit memory address.",
        ),
        "LHLD" => Some(
            "### `LHLD addr16` (Load H and L Direct)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 16 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `L <- [addr16]`, `H <- [addr16 + 1]`\n\n\
             Loads register pair `HL` from the specified memory location.",
        ),
        "SHLD" => Some(
            "### `SHLD addr16` (Store H and L Direct)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 16 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `[addr16] <- L`, `[addr16 + 1] <- H`\n\n\
             Stores register pair `HL` into the specified memory location.",
        ),
        "LDAX" => Some(
            "### `LDAX reg_pair` (Load Accumulator Indirect)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `A <- [reg_pair]` (`BC` or `DE`)\n\n\
             Copies the byte at the address pointed to by `BC` or `DE` into `A`.",
        ),
        "STAX" => Some(
            "### `STAX reg_pair` (Store Accumulator Indirect)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `[reg_pair] <- A` (`BC` or `DE`)\n\n\
             Stores the byte in `A` into the address pointed to by `BC` or `DE`.",
        ),
        "XCHG" => Some(
            "### `XCHG` (Exchange H & L with D & E)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `H <-> D`, `L <-> E`\n\n\
             Swaps the contents of register pair `HL` with register pair `DE`.",
        ),
        "ADD" => Some(
            "### `ADD register` (Add Register to Accumulator)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: `A <- A + register`",
        ),
        "ADI" => Some(
            "### `ADI data8` (Add Immediate to Accumulator)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: `A <- A + data8`",
        ),
        "ADC" => Some(
            "### `ADC register` (Add Register to Accumulator with Carry)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: `A <- A + register + CY`",
        ),
        "ACI" => Some(
            "### `ACI data8` (Add Immediate to Accumulator with Carry)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: `A <- A + data8 + CY`",
        ),
        "SUB" => Some(
            "### `SUB register` (Subtract Register from Accumulator)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: `A <- A - register`",
        ),
        "SUI" => Some(
            "### `SUI data8` (Subtract Immediate from Accumulator)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: `A <- A - data8`",
        ),
        "SBB" => Some(
            "### `SBB register` (Subtract Register with Borrow)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: `A <- A - register - CY`",
        ),
        "SBI" => Some(
            "### `SBI data8` (Subtract Immediate with Borrow)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: `A <- A - data8 - CY`",
        ),
        "INR" => Some(
            "### `INR register` (Increment Register)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (10 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, AC` (Carry `CY` is untouched)\n\
             - **Operation**: `register <- register + 1`",
        ),
        "DCR" => Some(
            "### `DCR register` (Decrement Register)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (10 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, AC` (Carry `CY` is untouched)\n\
             - **Operation**: `register <- register - 1`",
        ),
        "INX" => Some(
            "### `INX reg_pair` (Increment Register Pair 16-bit)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 6 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `reg_pair <- reg_pair + 1` (`BC`, `DE`, `HL`, `SP`)",
        ),
        "DCX" => Some(
            "### `DCX reg_pair` (Decrement Register Pair 16-bit)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 6 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `reg_pair <- reg_pair - 1` (`BC`, `DE`, `HL`, `SP`)",
        ),
        "DAD" => Some(
            "### `DAD reg_pair` (16-bit Add to HL)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 10 T-states\n\
             - **Flags Affected**: `CY`\n\
             - **Operation**: `HL <- HL + reg_pair` (`BC`, `DE`, `HL`, `SP`)",
        ),
        "DAA" => Some(
            "### `DAA` (Decimal Adjust Accumulator)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: Converts binary sum in `A` into two 4-bit packed BCD digits.",
        ),
        "ANA" => Some(
            "### `ANA register` (Logical AND with Accumulator)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, CY=0, AC=1`\n\
             - **Operation**: `A <- A & register`",
        ),
        "ANI" => Some(
            "### `ANI data8` (Logical AND Immediate)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: `Z, S, P, CY=0, AC=1`\n\
             - **Operation**: `A <- A & data8`",
        ),
        "XRA" => Some(
            "### `XRA register` (Logical XOR with Accumulator)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, CY=0, AC=0`\n\
             - **Operation**: `A <- A ^ register`\n\n\
             *Tip*: `xra A` clears Accumulator to `0` and resets Carry in 1 byte / 4 T-states.",
        ),
        "XRI" => Some(
            "### `XRI data8` (Logical XOR Immediate)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: `Z, S, P, CY=0, AC=0`\n\
             - **Operation**: `A <- A ^ data8`",
        ),
        "ORA" => Some(
            "### `ORA register` (Logical OR with Accumulator)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, CY=0, AC=0`\n\
             - **Operation**: `A <- A | register`",
        ),
        "ORI" => Some(
            "### `ORI data8` (Logical OR Immediate)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: `Z, S, P, CY=0, AC=0`\n\
             - **Operation**: `A <- A | data8`",
        ),
        "CMP" => Some(
            "### `CMP register` (Compare with Accumulator)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states (7 T for `M`)\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: Evaluates `A - register` without altering `A`:\n\
               - If `A == reg`: `Z = 1, CY = 0`\n\
               - If `A < reg`: `Z = 0, CY = 1`\n\
               - If `A > reg`: `Z = 0, CY = 0`",
        ),
        "CPI" => Some(
            "### `CPI data8` (Compare Immediate with Accumulator)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 7 T-states\n\
             - **Flags Affected**: `Z, S, P, CY, AC`\n\
             - **Operation**: Evaluates `A - data8` without altering `A`:\n\
               - If `A == data`: `Z = 1, CY = 0`\n\
               - If `A < data`: `Z = 0, CY = 1`\n\
               - If `A > data`: `Z = 0, CY = 0`",
        ),
        "RLC" => Some(
            "### `RLC` (Rotate Accumulator Left)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: `CY`\n\
             - **Operation**: Bit 7 moves to `CY` and Bit 0.",
        ),
        "RRC" => Some(
            "### `RRC` (Rotate Accumulator Right)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: `CY`\n\
             - **Operation**: Bit 0 moves to `CY` and Bit 7.",
        ),
        "RAL" => Some(
            "### `RAL` (Rotate Accumulator Left Through Carry)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: `CY`\n\
             - **Operation**: 9-bit rotation left: `CY <- A7`, `A0 <- old CY`.",
        ),
        "RAR" => Some(
            "### `RAR` (Rotate Accumulator Right Through Carry)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: `CY`\n\
             - **Operation**: 9-bit rotation right: `CY <- A0`, `A7 <- old CY`.",
        ),
        "CMA" => Some(
            "### `CMA` (Complement Accumulator / 1's Complement)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `A <- ~A` (Flips all bits of `A`).",
        ),
        "CMC" => Some(
            "### `CMC` (Complement Carry Flag)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: `CY`\n\
             - **Operation**: `CY <- !CY`.",
        ),
        "STC" => Some(
            "### `STC` (Set Carry Flag)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: `CY = 1`\n\
             - **Operation**: `CY <- 1`.",
        ),
        "JMP" => Some(
            "### `JMP addr16` (Jump Unconditional)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `PC <- addr16`",
        ),
        "JZ" => Some(
            "### `JZ addr16` (Jump if Zero)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags Affected**: None\n\
             - **Operation**: If `Z == 1`, `PC <- addr16`",
        ),
        "JNZ" => Some(
            "### `JNZ addr16` (Jump if Not Zero)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags Affected**: None\n\
             - **Operation**: If `Z == 0`, `PC <- addr16`",
        ),
        "JC" => Some(
            "### `JC addr16` (Jump if Carry)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags Affected**: None\n\
             - **Operation**: If `CY == 1`, `PC <- addr16`",
        ),
        "JNC" => Some(
            "### `JNC addr16` (Jump if No Carry)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags Affected**: None\n\
             - **Operation**: If `CY == 0`, `PC <- addr16`",
        ),
        "JP" => Some(
            "### `JP addr16` (Jump if Positive / Sign 0)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags Affected**: None\n\
             - **Operation**: If `S == 0`, `PC <- addr16`",
        ),
        "JM" => Some(
            "### `JM addr16` (Jump if Minus / Sign 1)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags Affected**: None\n\
             - **Operation**: If `S == 1`, `PC <- addr16`",
        ),
        "JPE" => Some(
            "### `JPE addr16` (Jump if Parity Even)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags Affected**: None\n\
             - **Operation**: If `P == 1`, `PC <- addr16`",
        ),
        "JPO" => Some(
            "### `JPO addr16` (Jump if Parity Odd)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 10 T-states (7 T if no jump)\n\
             - **Flags Affected**: None\n\
             - **Operation**: If `P == 0`, `PC <- addr16`",
        ),
        "CALL" => Some(
            "### `CALL addr16` (Call Subroutine Unconditionally)\n\
             - **Bytes**: 3\n\
             - **Cycles**: 18 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `(SP-1) <- PCH`, `(SP-2) <- PCL`, `SP <- SP - 2`, `PC <- addr16`",
        ),
        "CZ" => Some("### `CZ addr16` (Call if Zero)\n- **Bytes**: 3\n- **Cycles**: 18 T-states (9 T if false)\n- **Flags**: None\n- **Operation**: If `Z == 1`, `CALL addr16`"),
        "CNZ" => Some("### `CNZ addr16` (Call if Not Zero)\n- **Bytes**: 3\n- **Cycles**: 18 T-states (9 T if false)\n- **Flags**: None\n- **Operation**: If `Z == 0`, `CALL addr16`"),
        "CC" => Some("### `CC addr16` (Call if Carry)\n- **Bytes**: 3\n- **Cycles**: 18 T-states (9 T if false)\n- **Flags**: None\n- **Operation**: If `CY == 1`, `CALL addr16`"),
        "CNC" => Some("### `CNC addr16` (Call if No Carry)\n- **Bytes**: 3\n- **Cycles**: 18 T-states (9 T if false)\n- **Flags**: None\n- **Operation**: If `CY == 0`, `CALL addr16`"),
        "CP" => Some("### `CP addr16` (Call if Positive)\n- **Bytes**: 3\n- **Cycles**: 18 T-states (9 T if false)\n- **Flags**: None\n- **Operation**: If `S == 0`, `CALL addr16`"),
        "CM" => Some("### `CM addr16` (Call if Minus)\n- **Bytes**: 3\n- **Cycles**: 18 T-states (9 T if false)\n- **Flags**: None\n- **Operation**: If `S == 1`, `CALL addr16`"),
        "CPE" => Some("### `CPE addr16` (Call if Parity Even)\n- **Bytes**: 3\n- **Cycles**: 18 T-states (9 T if false)\n- **Flags**: None\n- **Operation**: If `P == 1`, `CALL addr16`"),
        "CPO" => Some("### `CPO addr16` (Call if Parity Odd)\n- **Bytes**: 3\n- **Cycles**: 18 T-states (9 T if false)\n- **Flags**: None\n- **Operation**: If `P == 0`, `CALL addr16`"),
        "RET" => Some(
            "### `RET` (Return from Subroutine Unconditionally)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 10 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `PCL <- [SP]`, `PCH <- [SP+1]`, `SP <- SP + 2`",
        ),
        "RZ" => Some("### `RZ` (Return if Zero)\n- **Bytes**: 1\n- **Cycles**: 12 T-states (6 T if false)\n- **Flags**: None\n- **Operation**: If `Z == 1`, `RET`"),
        "RNZ" => Some("### `RNZ` (Return if Not Zero)\n- **Bytes**: 1\n- **Cycles**: 12 T-states (6 T if false)\n- **Flags**: None\n- **Operation**: If `Z == 0`, `RET`"),
        "RC" => Some("### `RC` (Return if Carry)\n- **Bytes**: 1\n- **Cycles**: 12 T-states (6 T if false)\n- **Flags**: None\n- **Operation**: If `CY == 1`, `RET`"),
        "RNC" => Some("### `RNC` (Return if No Carry)\n- **Bytes**: 1\n- **Cycles**: 12 T-states (6 T if false)\n- **Flags**: None\n- **Operation**: If `CY == 0`, `RET`"),
        "RP" => Some("### `RP` (Return if Positive)\n- **Bytes**: 1\n- **Cycles**: 12 T-states (6 T if false)\n- **Flags**: None\n- **Operation**: If `S == 0`, `RET`"),
        "RM" => Some("### `RM` (Return if Minus)\n- **Bytes**: 1\n- **Cycles**: 12 T-states (6 T if false)\n- **Flags**: None\n- **Operation**: If `S == 1`, `RET`"),
        "RPE" => Some("### `RPE` (Return if Parity Even)\n- **Bytes**: 1\n- **Cycles**: 12 T-states (6 T if false)\n- **Flags**: None\n- **Operation**: If `P == 1`, `RET`"),
        "RPO" => Some("### `RPO` (Return if Parity Odd)\n- **Bytes**: 1\n- **Cycles**: 12 T-states (6 T if false)\n- **Flags**: None\n- **Operation**: If `P == 0`, `RET`"),
        "RST" => Some(
            "### `RST n` (Restart / Software Interrupt)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: Pushes `PC` to stack and jumps to vector address `n * 8` (`0x0000` to `0x0038`).",
        ),
        "PUSH" => Some(
            "### `PUSH reg_pair` (Push Register Pair onto Stack)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 12 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `[SP-1] <- High`, `[SP-2] <- Low`, `SP <- SP - 2` (`BC`, `DE`, `HL`, `PSW`)",
        ),
        "POP" => Some(
            "### `POP reg_pair` (Pop Register Pair from Stack)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 10 T-states\n\
             - **Flags Affected**: If `PSW`: `Z, S, P, CY, AC` restored; otherwise None\n\
             - **Operation**: `Low <- [SP]`, `High <- [SP+1]`, `SP <- SP + 2`",
        ),
        "IN" => Some(
            "### `IN port8` (Input Byte from I/O Port)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 10 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `A <- Port[port8]`\n\n\
             Reads an 8-bit byte from the specified 8-bit I/O device port into Accumulator `A`.",
        ),
        "OUT" => Some(
            "### `OUT port8` (Output Byte to I/O Port)\n\
             - **Bytes**: 2\n\
             - **Cycles**: 10 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: `Port[port8] <- A`\n\n\
             Sends the byte in Accumulator `A` to the specified 8-bit I/O device port.",
        ),
        "NOP" => Some(
            "### `NOP` (No Operation)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 4 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: Advances `PC` by 1 with no register side effects.",
        ),
        "HLT" => Some(
            "### `HLT` (Halt Microprocessor)\n\
             - **Bytes**: 1\n\
             - **Cycles**: 5 T-states\n\
             - **Flags Affected**: None\n\
             - **Operation**: Stops instruction execution until an interrupt (e.g. TRAP/RST) or RESET occurs.",
        ),
        "EI" => Some("### `EI` (Enable Interrupts)\n- **Bytes**: 1\n- **Cycles**: 4 T-states\n- **Flags**: None\n- **Operation**: Enables the maskable interrupt system."),
        "DI" => Some("### `DI` (Disable Interrupts)\n- **Bytes**: 1\n- **Cycles**: 4 T-states\n- **Flags**: None\n- **Operation**: Disables the maskable interrupt system (TRAP cannot be disabled)."),
        "PCHL" => Some("### `PCHL` (Jump to HL Indirect)\n- **Bytes**: 1\n- **Cycles**: 6 T-states\n- **Flags**: None\n- **Operation**: `PC <- HL`"),
        "SPHL" => Some("### `SPHL` (Copy HL to Stack Pointer)\n- **Bytes**: 1\n- **Cycles**: 6 T-states\n- **Flags**: None\n- **Operation**: `SP <- HL`"),
        "XTHL" => Some("### `XTHL` (Exchange Top of Stack with HL)\n- **Bytes**: 1\n- **Cycles**: 16 T-states\n- **Flags**: None\n- **Operation**: `L <-> [SP]`, `H <-> [SP+1]`"),
        "RIM" => Some("### `RIM` (Read Interrupt Mask & SID)\n- **Bytes**: 1\n- **Cycles**: 4 T-states\n- **Flags**: None\n- **Operation**: Reads interrupt masks, pending interrupts, and SID pin into `A`."),
        "SIM" => Some("### `SIM` (Set Interrupt Mask & SOD)\n- **Bytes**: 1\n- **Cycles**: 4 T-states\n- **Flags**: None\n- **Operation**: Programs RST 7.5/6.5/5.5 interrupt masks and outputs to SOD pin from `A`."),
        _ => None,
    }
}

fn get_register_hover(reg: &str) -> Option<&'static str> {
    match reg {
        "A" => Some(
            "### Accumulator `A` (8-bit)\n\
             Primary 8-bit operational register for arithmetic, logical, and I/O instructions.",
        ),
        "B" | "C" | "D" | "E" | "H" | "L" => Some(
            "### General Purpose Register (8-bit)\n\
             Can be used individually as an 8-bit data register or combined as a 16-bit register pair (`BC`, `DE`, `HL`).",
        ),
        "M" => Some(
            "### Memory Reference `M` (8-bit `[HL]`)\n\
             References the 8-bit memory location addressed by register pair `HL` (`RAM[HL]`).",
        ),
        "BC" => Some("### Register Pair `BC` (16-bit)\nComposed of `B` (High) and `C` (Low). Used with `LXI`, `INX`, `DCX`, `DAD`, `PUSH`, `POP`, `LDAX`, `STAX`."),
        "DE" => Some("### Register Pair `DE` (16-bit)\nComposed of `D` (High) and `E` (Low). Used with `LXI`, `INX`, `DCX`, `DAD`, `PUSH`, `POP`, `LDAX`, `STAX`, `XCHG`."),
        "HL" => Some("### Register Pair `HL` (16-bit)\nPrimary 16-bit memory pointer. Composed of `H` (High) and `L` (Low). Used with `M` operands, `LXI`, `DAD`, `XCHG`, `XTHL`, `SPHL`, `PCHL`."),
        "SP" => Some("### Stack Pointer `SP` (16-bit)\nPoints to the top of the descending call stack in RAM."),
        "PSW" => Some(
            "### Program Status Word `PSW` (16-bit)\n\
             Pair composed of Accumulator `A` (High) and Flag Register `F` (Low):\n\
             `[S Z 0 AC 0 P 1 CY]`\n\
             Used with `PUSH PSW` and `POP PSW`.",
        ),
        _ => None,
    }
}

fn get_directive_hover(directive: &str) -> Option<&'static str> {
    match directive.to_lowercase().as_str() {
        "%define" => Some(
            "### Directive `%define NAME value`\n\
             Defines a textual macro or numerical constant.\n\n\
             **Example**:\n\
             ```assembly\n\
             %define MAX_LEN 32\n\
             mvi B, MAX_LEN\n\
             ```",
        ),
        "%include" => Some(
            "### Directive `%include \"path/file.e8085\"`\n\
             Embeds the contents of the referenced `.e8085` source file at this location.\n\n\
             **Example**:\n\
             ```assembly\n\
             %include \"devices/terminal.e8085\"\n\
             ```",
        ),
        "%repeat" => Some(
            "### Directive `%repeat count value`\n\
             Repeats a byte or character value `count` times in `.data` or `.bss`.\n\n\
             **Example**:\n\
             ```assembly\n\
             buffer BYTE %repeat 32 0\n\
             ```",
        ),
        "%len" => Some(
            "### Directive `%len symbol`\n\
             Computes the byte length of a `.data` or `.bss` variable at assembly time.\n\n\
             **Example**:\n\
             ```assembly\n\
             mvi B, %len prompt\n\
             ```",
        ),
        "segment" => Some(
            "### Directive `segment .name`\n\
             Switches memory allocation segment:\n\
             - `segment .text`: Executable machine code instructions\n\
             - `segment .data`: Initialized strings, arrays, and constants\n\
             - `segment .bss`: Uninitialized zero-allocated memory buffers",
        ),
        "global" | "export" => Some(
            "### Keyword `global name:` / `export name:`\n\
             Exports a label or subroutine symbol to the `.symtab` symbol table so external programs can link it.",
        ),
        "extern" => Some(
            "### Keyword `extern name`\n\
             Declares an unresolved external symbol that will be resolved at link time via `-l library.8085.bin`.",
        ),
        "byte" => Some("### Type `BYTE count`\nDeclares 8-bit byte memory storage."),
        "word" => Some("### Type `WORD count`\nDeclares 16-bit little-endian word storage."),
        _ => None,
    }
}

/// Scans the document source for user label / variable definitions and doc-comments.
fn get_user_symbol_hover(doc: &Document, symbol: &str) -> Option<String> {
    let lines: Vec<&str> = doc.text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Check label definition (e.g. `main:`, `global print:`, `to_uint8:`, `.loop:`)
        if let Some(rest) = trimmed.strip_suffix(':') {
            let label_ident = rest
                .strip_prefix("global ")
                .or_else(|| rest.strip_prefix("export "))
                .unwrap_or(rest)
                .trim();

            if label_ident == symbol {
                let doc_comment = collect_preceding_comments(&lines, i);
                let is_global = trimmed.starts_with("global") || trimmed.starts_with("export");
                let scope = if is_global { "Global Subroutine" } else { "Local / Code Label" };

                let mut out = format!("### `{}` ({})\n- **Defined at**: Line {}\n", symbol, scope, i + 1);
                if !doc_comment.is_empty() {
                    out.push_str("\n---\n**Documentation**:\n");
                    out.push_str(&doc_comment);
                }
                return Some(out);
            }
        }

        // Check data/bss variable declaration (e.g. `prompt "What is your name? "`, `buffer BYTE 32`)
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if !parts.is_empty() && parts[0] == symbol {
            let doc_comment = collect_preceding_comments(&lines, i);
            let mut out = format!("### Variable `{}`\n- **Declared at**: Line {}\n- **Declaration**: `{}`\n", symbol, i + 1, trimmed);
            if !doc_comment.is_empty() {
                out.push_str("\n---\n**Documentation**:\n");
                out.push_str(&doc_comment);
            }
            return Some(out);
        }
    }

    None
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
        } else if line.is_empty() {
            continue;
        } else {
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
    fn test_hover_instruction_and_register() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    mvi A, 0x05\n    lxi HL, 0x1000\n    call print\n".to_string();
        let doc = Document::new(uri, 1, text);

        // Hover over MVI
        let h_mvi = get_hover(&doc, &Position { line: 1, character: 5 }).unwrap();
        if let HoverContents::Markup(m) = h_mvi.contents {
            assert!(m.value.contains("Move Immediate 8-bit"));
            assert!(m.value.contains("7 T-states"));
        } else {
            panic!("expected markup");
        }

        // Hover over A
        let h_a = get_hover(&doc, &Position { line: 1, character: 8 }).unwrap();
        if let HoverContents::Markup(m) = h_a.contents {
            assert!(m.value.contains("Accumulator `A`"));
        } else {
            panic!("expected markup");
        }

        // Hover over HL
        let h_hl = get_hover(&doc, &Position { line: 2, character: 9 }).unwrap();
        if let HoverContents::Markup(m) = h_hl.contents {
            assert!(m.value.contains("Register Pair `HL`"));
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
            assert!(m.value.contains("multiply"));
            assert!(m.value.contains("Multiplies two numbers"));
            assert!(m.value.contains("Result returned in A"));
        } else {
            panic!("expected markup");
        }
    }
}
