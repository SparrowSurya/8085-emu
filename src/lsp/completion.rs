use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionResponse, InsertTextFormat,
    Position,
};

use super::document::Document;

/// Computes intelligent auto-completions based on cursor context in the document.
pub fn get_completions(doc: &Document, position: &Position) -> Option<CompletionResponse> {
    let line = doc.text.lines().nth(position.line as usize).unwrap_or("");
    let char_idx = (position.character as usize).min(line.len());
    let line_prefix = &line[..char_idx].trim_start();

    let mut items = Vec::new();

    // 1. Contextual Register completions following an instruction mnemonic
    if let Some(reg_items) = get_contextual_registers(line_prefix) {
        items.extend(reg_items);
    }

    // 2. Directives (e.g. `%define`, `%include`, `segment`)
    if line_prefix.starts_with('%') || line_prefix.is_empty() {
        items.extend(get_directive_completions());
    }

    // 3. Instruction Mnemonics with snippet expansions
    items.extend(get_instruction_completions());

    // 4. In-Scope User Symbols (Labels, Variables, Constants)
    items.extend(get_in_scope_symbols(doc));

    Some(CompletionResponse::List(CompletionList {
        is_incomplete: false,
        items,
    }))
}

fn get_contextual_registers(line_prefix: &str) -> Option<Vec<CompletionItem>> {
    let tokens: Vec<&str> = line_prefix.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let first = tokens[0].trim_end_matches(',').to_uppercase();

    match first.as_str() {
        "LXI" | "INX" | "DCX" | "DAD" => Some(vec![
            make_reg_item("BC", "Register Pair BC (16-bit)"),
            make_reg_item("DE", "Register Pair DE (16-bit)"),
            make_reg_item("HL", "Register Pair HL (16-bit)"),
            make_reg_item("SP", "Stack Pointer SP (16-bit)"),
        ]),
        "PUSH" | "POP" => Some(vec![
            make_reg_item("BC", "Register Pair BC (16-bit)"),
            make_reg_item("DE", "Register Pair DE (16-bit)"),
            make_reg_item("HL", "Register Pair HL (16-bit)"),
            make_reg_item("PSW", "Program Status Word (A + Flags)"),
        ]),
        "LDAX" | "STAX" => Some(vec![
            make_reg_item("BC", "Register Pair BC pointer"),
            make_reg_item("DE", "Register Pair DE pointer"),
        ]),
        "MOV" | "MVI" | "ADD" | "ADC" | "SUB" | "SBB" | "INR" | "DCR" | "ANA" | "XRA"
        | "ORA" | "CMP" => Some(vec![
            make_reg_item("A", "Accumulator A (8-bit)"),
            make_reg_item("B", "Register B (8-bit)"),
            make_reg_item("C", "Register C (8-bit)"),
            make_reg_item("D", "Register D (8-bit)"),
            make_reg_item("E", "Register E (8-bit)"),
            make_reg_item("H", "Register H (8-bit)"),
            make_reg_item("L", "Register L (8-bit)"),
            make_reg_item("M", "Memory Reference [HL]"),
        ]),
        _ => None,
    }
}

fn make_reg_item(name: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::VARIABLE),
        detail: Some(detail.to_string()),
        ..Default::default()
    }
}

fn get_directive_completions() -> Vec<CompletionItem> {
    vec![
        make_snippet_item(
            "%define",
            "%define ${1:NAME} ${2:value}",
            "Define a macro or constant value",
        ),
        make_snippet_item(
            "%include",
            "%include \"${1:devices/terminal.e8085}\"",
            "Include an external source file",
        ),
        make_snippet_item(
            "%repeat",
            "%repeat ${1:count} ${2:value}",
            "Repeat a data element multiple times",
        ),
        make_snippet_item(
            "%len",
            "%len ${1:symbol}",
            "Compute byte length of a symbol",
        ),
        make_snippet_item(
            "segment .text",
            "segment .text\n$0",
            "Executable code segment",
        ),
        make_snippet_item(
            "segment .data",
            "segment .data\n$0",
            "Initialized data segment",
        ),
        make_snippet_item(
            "segment .bss",
            "segment .bss\n$0",
            "Uninitialized memory buffer segment",
        ),
        make_snippet_item(
            "global",
            "global ${1:subroutine_name}:\n    $0\n    ret",
            "Export global subroutine",
        ),
        make_snippet_item(
            "extern",
            "extern ${1:function_name}",
            "Declare external linked symbol",
        ),
    ]
}

fn get_instruction_completions() -> Vec<CompletionItem> {
    vec![
        make_snippet_item("mov", "mov ${1:dest}, ${2:src}", "Copy register/memory"),
        make_snippet_item("mvi", "mvi ${1:dest}, ${2:byte}", "Move immediate byte"),
        make_snippet_item("lxi", "lxi ${1:pair}, ${2:addr16}", "Load 16-bit register pair"),
        make_snippet_item("lda", "lda ${1:addr16}", "Load Accumulator direct"),
        make_snippet_item("sta", "sta ${1:addr16}", "Store Accumulator direct"),
        make_snippet_item("lhld", "lhld ${1:addr16}", "Load HL direct"),
        make_snippet_item("shld", "shld ${1:addr16}", "Store HL direct"),
        make_snippet_item("ldax", "ldax ${1|BC,DE|}", "Load Accumulator indirect"),
        make_snippet_item("stax", "stax ${1|BC,DE|}", "Store Accumulator indirect"),
        make_snippet_item("xchg", "xchg", "Exchange HL with DE"),
        make_snippet_item("add", "add ${1:reg}", "Add to Accumulator"),
        make_snippet_item("adi", "adi ${1:byte}", "Add immediate to Accumulator"),
        make_snippet_item("adc", "adc ${1:reg}", "Add with Carry to Accumulator"),
        make_snippet_item("aci", "aci ${1:byte}", "Add immediate with Carry"),
        make_snippet_item("sub", "sub ${1:reg}", "Subtract from Accumulator"),
        make_snippet_item("sui", "sui ${1:byte}", "Subtract immediate from Accumulator"),
        make_snippet_item("sbb", "sbb ${1:reg}", "Subtract with Borrow"),
        make_snippet_item("sbi", "sbi ${1:byte}", "Subtract immediate with Borrow"),
        make_snippet_item("inr", "inr ${1:reg}", "Increment register"),
        make_snippet_item("dcr", "dcr ${1:reg}", "Decrement register"),
        make_snippet_item("inx", "inx ${1|BC,DE,HL,SP|}", "Increment register pair (16-bit)"),
        make_snippet_item("dcx", "dcx ${1|BC,DE,HL,SP|}", "Decrement register pair (16-bit)"),
        make_snippet_item("dad", "dad ${1|BC,DE,HL,SP|}", "16-bit add to HL"),
        make_snippet_item("daa", "daa", "Decimal adjust Accumulator"),
        make_snippet_item("ana", "ana ${1:reg}", "Logical AND with Accumulator"),
        make_snippet_item("ani", "ani ${1:byte}", "Logical AND immediate"),
        make_snippet_item("xra", "xra ${1:reg}", "Logical XOR with Accumulator"),
        make_snippet_item("xri", "xri ${1:byte}", "Logical XOR immediate"),
        make_snippet_item("ora", "ora ${1:reg}", "Logical OR with Accumulator"),
        make_snippet_item("ori", "ori ${1:byte}", "Logical OR immediate"),
        make_snippet_item("cmp", "cmp ${1:reg}", "Compare with Accumulator"),
        make_snippet_item("cpi", "cpi ${1:byte}", "Compare immediate with Accumulator"),
        make_snippet_item("rlc", "rlc", "Rotate Accumulator left"),
        make_snippet_item("rrc", "rrc", "Rotate Accumulator right"),
        make_snippet_item("ral", "ral", "Rotate Accumulator left through carry"),
        make_snippet_item("rar", "rar", "Rotate Accumulator right through carry"),
        make_snippet_item("cma", "cma", "Complement Accumulator"),
        make_snippet_item("cmc", "cmc", "Complement Carry flag"),
        make_snippet_item("stc", "stc", "Set Carry flag"),
        make_snippet_item("jmp", "jmp ${1:label}", "Unconditional jump"),
        make_snippet_item("jz", "jz ${1:label}", "Jump if Zero"),
        make_snippet_item("jnz", "jnz ${1:label}", "Jump if Not Zero"),
        make_snippet_item("jc", "jc ${1:label}", "Jump if Carry"),
        make_snippet_item("jnc", "jnc ${1:label}", "Jump if No Carry"),
        make_snippet_item("jp", "jp ${1:label}", "Jump if Positive"),
        make_snippet_item("jm", "jm ${1:label}", "Jump if Minus"),
        make_snippet_item("jpe", "jpe ${1:label}", "Jump if Parity Even"),
        make_snippet_item("jpo", "jpo ${1:label}", "Jump if Parity Odd"),
        make_snippet_item("call", "call ${1:subroutine}", "Call subroutine"),
        make_snippet_item("cz", "cz ${1:subroutine}", "Call if Zero"),
        make_snippet_item("cnz", "cnz ${1:subroutine}", "Call if Not Zero"),
        make_snippet_item("cc", "cc ${1:subroutine}", "Call if Carry"),
        make_snippet_item("cnc", "cnc ${1:subroutine}", "Call if No Carry"),
        make_snippet_item("cp", "cp ${1:subroutine}", "Call if Positive"),
        make_snippet_item("cm", "cm ${1:subroutine}", "Call if Minus"),
        make_snippet_item("cpe", "cpe ${1:subroutine}", "Call if Parity Even"),
        make_snippet_item("cpo", "cpo ${1:subroutine}", "Call if Parity Odd"),
        make_snippet_item("ret", "ret", "Return from subroutine"),
        make_snippet_item("rz", "rz", "Return if Zero"),
        make_snippet_item("rnz", "rnz", "Return if Not Zero"),
        make_snippet_item("rc", "rc", "Return if Carry"),
        make_snippet_item("rnc", "rnc", "Return if No Carry"),
        make_snippet_item("rp", "rp", "Return if Positive"),
        make_snippet_item("rm", "rm", "Return if Minus"),
        make_snippet_item("rpe", "rpe", "Return if Parity Even"),
        make_snippet_item("rpo", "rpo", "Return if Parity Odd"),
        make_snippet_item("rst", "rst ${1|0,1,2,3,4,5,6,7|}", "Restart software interrupt"),
        make_snippet_item("push", "push ${1|BC,DE,HL,PSW|}", "Push pair onto stack"),
        make_snippet_item("pop", "pop ${1|BC,DE,HL,PSW|}", "Pop pair from stack"),
        make_snippet_item("in", "in ${1:port}", "Input from I/O port"),
        make_snippet_item("out", "out ${1:port}", "Output to I/O port"),
        make_snippet_item("nop", "nop", "No operation"),
        make_snippet_item("hlt", "hlt", "Halt CPU"),
        make_snippet_item("ei", "ei", "Enable maskable interrupts"),
        make_snippet_item("di", "di", "Disable maskable interrupts"),
        make_snippet_item("rim", "rim", "Read interrupt mask & SID"),
        make_snippet_item("sim", "sim", "Set interrupt mask & SOD"),
        make_snippet_item("pchl", "pchl", "Jump to HL indirect"),
        make_snippet_item("sphl", "sphl", "Copy HL to Stack Pointer"),
        make_snippet_item("xthl", "xthl", "Exchange top of stack with HL"),
    ]
}

fn make_snippet_item(label: &str, snippet: &str, doc: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(doc.to_string()),
        insert_text: Some(snippet.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

fn get_in_scope_symbols(doc: &Document) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for line in doc.text.lines() {
        let trimmed = line.trim();

        // Labels
        if let Some(rest) = trimmed.strip_suffix(':') {
            let label = rest
                .strip_prefix("global ")
                .or_else(|| rest.strip_prefix("export "))
                .unwrap_or(rest)
                .trim();
            if !label.is_empty() {
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some("Label / Subroutine".to_string()),
                    ..Default::default()
                });
            }
        }

        // Variables
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if !parts.is_empty() && !is_reserved_keyword(parts[0]) && (trimmed.contains('"') || trimmed.contains("BYTE") || trimmed.contains("WORD")) {
            items.push(CompletionItem {
                label: parts[0].to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some("Data Variable".to_string()),
                ..Default::default()
            });
        }

        // Constants (%define)
        if trimmed.starts_with("%define") && parts.len() >= 2 {
            items.push(CompletionItem {
                label: parts[1].to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Defined Constant".to_string()),
                ..Default::default()
            });
        }
    }

    items
}

fn is_reserved_keyword(word: &str) -> bool {
    matches!(
        word.to_uppercase().as_str(),
        "MOV" | "MVI" | "LXI" | "LDA" | "STA" | "LHLD" | "SHLD" | "LDAX" | "STAX" |
        "XCHG" | "XTHL" | "SPHL" | "PCHL" | "ADD" | "ADI" | "ADC" | "ACI" | "SUB" |
        "SUI" | "SBB" | "SBI" | "INR" | "DCR" | "INX" | "DCX" | "DAD" | "DAA" |
        "ANA" | "ANI" | "XRA" | "XRI" | "ORA" | "ORI" | "CMP" | "CPI" | "CMA" |
        "CMC" | "STC" | "RLC" | "RRC" | "RAL" | "RAR" | "PUSH" | "POP" | "IN" |
        "OUT" | "NOP" | "HLT" | "EI" | "DI" | "RIM" | "SIM" | "JMP" | "JZ" | "JNZ" |
        "JC" | "JNC" | "JP" | "JM" | "JPE" | "JPO" | "CALL" | "CZ" | "CNZ" | "CC" |
        "CNC" | "CP" | "CM" | "CPE" | "CPO" | "RET" | "RZ" | "RNZ" | "RC" | "RNC" |
        "RP" | "RM" | "RPE" | "RPO" | "RST" | "BYTE" | "WORD" | "SEGMENT" | "GLOBAL" |
        "EXPORT" | "EXTERN"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Url;

    #[test]
    fn test_instruction_completions() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    ".to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(&doc, &Position { line: 1, character: 4 }).unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "lxi"));
            assert!(list.items.iter().any(|item| item.label == "mov"));
            assert!(list.items.iter().any(|item| item.label == "%include"));
        } else {
            panic!("expected list of completions");
        }
    }

    #[test]
    fn test_contextual_register_completions() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    lxi ".to_string();
        let doc = Document::new(uri, 1, text);

        let res = get_completions(&doc, &Position { line: 1, character: 8 }).unwrap();
        if let CompletionResponse::List(list) = res {
            assert!(list.items.iter().any(|item| item.label == "HL"));
            assert!(list.items.iter().any(|item| item.label == "BC"));
            assert!(list.items.iter().any(|item| item.label == "SP"));
        } else {
            panic!("expected list of completions");
        }
    }
}
