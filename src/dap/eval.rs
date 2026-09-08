//! Debugger expression evaluation engine, REPL command handler, and symbol resolver.

use crate::cpu::registers::{Reg8, Reg16};
use crate::dap::error::DapError;
use crate::dap::sourcemap::SourceMap;
use crate::machine::Machine;
use crate::value::Addr;

/// Result of evaluating an expression or debugger command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalResult {
    /// Formatted display string shown in debug console and variable hovers.
    pub display: String,
    /// Data type tag (e.g. `"u8"`, `"u16"`, `"bool"`, `"registers"`, `"memory_dump"`).
    pub r#type: String,
    /// Raw evaluated numeric value when applicable.
    pub raw_value: Option<u16>,
}

/// Evaluates a debugger expression, comparison, arithmetic expression, or REPL command.
pub fn evaluate_expression(
    expr: &str,
    machine: &Machine,
    source_map: Option<&SourceMap>,
) -> Result<EvalResult, DapError> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Err(DapError::InvalidExpression("empty expression".to_string()));
    }

    // 1. Check REPL commands (starting with ':')
    if let Some(cmd) = trimmed.strip_prefix(':') {
        return handle_repl_command(cmd.trim(), machine, source_map);
    }

    // 2. Check conditional comparisons (e.g. `A == 0x0A`, `C > 5`, `Z == 1`)
    for op in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((lhs, rhs)) = split_binary_op(trimmed, op) {
            let left_val = eval_term(lhs.trim(), machine, source_map)?;
            let right_val = eval_term(rhs.trim(), machine, source_map)?;
            let res = match op {
                "==" => left_val == right_val,
                "!=" => left_val != right_val,
                "<" => left_val < right_val,
                "<=" => left_val <= right_val,
                ">" => left_val > right_val,
                ">=" => left_val >= right_val,
                _ => false,
            };
            return Ok(EvalResult {
                display: if res { "true (1)".to_string() } else { "false (0)".to_string() },
                r#type: "bool".to_string(),
                raw_value: Some(if res { 1 } else { 0 }),
            });
        }
    }

    // 3. Evaluate binary arithmetic (+ / -)
    if let Some((lhs, rhs)) = split_binary_op(trimmed, "+") {
        let left_val = eval_term(lhs.trim(), machine, source_map)?;
        let right_val = eval_term(rhs.trim(), machine, source_map)?;
        let sum = left_val.wrapping_add(right_val);
        return format_eval_value(sum, true);
    }
    if let Some((lhs, rhs)) = split_binary_op(trimmed, "-") {
        let left_val = eval_term(lhs.trim(), machine, source_map)?;
        let right_val = eval_term(rhs.trim(), machine, source_map)?;
        let diff = left_val.wrapping_sub(right_val);
        return format_eval_value(diff, true);
    }

    // 4. Single term evaluation
    let val = eval_term(trimmed, machine, source_map)?;
    format_eval_value(val, false)
}

fn split_binary_op<'a>(s: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    // If op is "-" or "+", skip leading unary sign at index 0 so unary prefix is not treated as a binary split
    let search_start = if (op == "-" || op == "+") && (s.starts_with('-') || s.starts_with('+')) {
        1
    } else {
        0
    };
    if search_start >= s.len() {
        return None;
    }
    if let Some(idx) = s[search_start..].find(op) {
        let actual_idx = search_start + idx;
        let lhs = &s[..actual_idx];
        let rhs = &s[actual_idx + op.len()..];
        if !lhs.trim().is_empty() && !rhs.trim().is_empty() {
            return Some((lhs, rhs));
        }
    }
    None
}

/// Evaluates a single term: register, memory dereference, flag, symbol, or numeric literal (including unary +/-).
pub fn eval_term(
    term: &str,
    machine: &Machine,
    source_map: Option<&SourceMap>,
) -> Result<u16, DapError> {
    let s = term.trim();
    if s.is_empty() {
        return Err(DapError::InvalidExpression("empty operand".to_string()));
    }

    // Unary negative / negation (e.g. `-1`, `-0x10`, `-A`)
    if let Some(rest) = s.strip_prefix('-') {
        let rest_trimmed = rest.trim();
        if rest_trimmed.is_empty() {
            return Err(DapError::InvalidExpression(format!("invalid negative operand '{s}'")));
        }
        let inner_val = eval_term(rest_trimmed, machine, source_map)?;
        return Ok(0u16.wrapping_sub(inner_val));
    }

    // Unary positive (e.g. `+5`, `+0x20`)
    if let Some(rest) = s.strip_prefix('+') {
        let rest_trimmed = rest.trim();
        if rest_trimmed.is_empty() {
            return Err(DapError::InvalidExpression(format!("invalid positive operand '{s}'")));
        }
        return eval_term(rest_trimmed, machine, source_map);
    }

    // Macro parameter reference: `%1`, `%2`, etc.
    if let Some(idx_str) = s.strip_prefix('%') {
        if let Ok(idx) = idx_str.parse::<usize>() {
            if let Some(sm) = source_map {
                let current_pc = machine.cpu.regs.pc.0;
                let active_macro = sm.find_macro_containing(current_pc).or_else(|| sm.find_macro_at_start(current_pc));
                if let Some(m) = active_macro {
                    if idx > 0 && idx <= m.arguments.len() {
                        let arg_expr = &m.arguments[idx - 1].expr;
                        return eval_term(arg_expr, machine, source_map);
                    }
                }
            }
        }
    }

    // Directive `%len <ident>`
    if let Some(ident) = s.strip_prefix("%len").or_else(|| s.strip_prefix("%LEN")) {
        let var_name = ident.trim();
        if let Some(sm) = source_map {
            if let Some(var) = sm.variables.iter().find(|v| v.name.eq_ignore_ascii_case(var_name)) {
                return Ok(var.size_bytes as u16);
            }
            if sm.symbols.contains_key(var_name) {
                // If symbol is found without explicit variable record, fallback to 1 or 0
                return Ok(1);
            }
        }
    }

    // Dereference pointer `*0x2000`, `*(HL)`, `*var`
    if let Some(inner) = s.strip_prefix('*') {
        let addr = eval_term(inner.trim().trim_matches(|c| c == '(' || c == ')'), machine, source_map)?;
        let byte_val = machine.ram.read(Addr(addr));
        return Ok(byte_val as u16);
    }

    // Single-quoted character `'A'`, `'\n'`
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
        let inner = &s[1..s.len() - 1];
        let ch = match inner {
            "\\n" => b'\n',
            "\\t" => b'\t',
            "\\r" => b'\r',
            "\\0" => b'\0',
            "\\\\" => b'\\',
            "\\'" => b'\'',
            "\\\"" => b'\"',
            _ if inner.len() == 1 => inner.as_bytes()[0],
            _ => return Err(DapError::InvalidExpression(format!("invalid character literal: {s}"))),
        };
        return Ok(ch as u16);
    }

    // Hex number 0x...
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u16::from_str_radix(hex, 16).map_err(|e| DapError::InvalidExpression(format!("invalid hex number '{s}': {e}")));
    }

    // Binary number 0b...
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return u16::from_str_radix(bin, 2).map_err(|e| DapError::InvalidExpression(format!("invalid binary number '{s}': {e}")));
    }

    // Octal number 0o...
    if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return u16::from_str_radix(oct, 8).map_err(|e| DapError::InvalidExpression(format!("invalid octal number '{s}': {e}")));
    }

    // Decimal number
    if s.chars().all(|c| c.is_ascii_digit()) {
        return s.parse::<u16>().map_err(|e| DapError::InvalidExpression(format!("invalid decimal number '{s}': {e}")));
    }

    // Registers (8-bit)
    let upper = s.to_ascii_uppercase();
    match upper.as_str() {
        "A" => return Ok(machine.cpu.regs.get8(Reg8::A) as u16),
        "B" => return Ok(machine.cpu.regs.get8(Reg8::B) as u16),
        "C" => return Ok(machine.cpu.regs.get8(Reg8::C) as u16),
        "D" => return Ok(machine.cpu.regs.get8(Reg8::D) as u16),
        "E" => return Ok(machine.cpu.regs.get8(Reg8::E) as u16),
        "H" => return Ok(machine.cpu.regs.get8(Reg8::H) as u16),
        "L" => return Ok(machine.cpu.regs.get8(Reg8::L) as u16),
        "M" => {
            let hl = machine.cpu.regs.get16(Reg16::HL);
            return Ok(machine.ram.read(hl) as u16);
        }
        "SP" => return Ok(machine.cpu.regs.sp.0),
        "PC" => return Ok(machine.cpu.regs.pc.0),
        "BC" => return Ok(machine.cpu.regs.get16(Reg16::BC).0),
        "DE" => return Ok(machine.cpu.regs.get16(Reg16::DE).0),
        "HL" => return Ok(machine.cpu.regs.get16(Reg16::HL).0),
        "PSW" | "FLAGS" => return Ok(machine.cpu.flags.to_psw() as u16),
        "Z" => return Ok(if machine.cpu.flags.zero { 1 } else { 0 }),
        "S" => return Ok(if machine.cpu.flags.sign { 1 } else { 0 }),
        "P" => return Ok(if machine.cpu.flags.parity { 1 } else { 0 }),
        "CY" => return Ok(if machine.cpu.flags.carry { 1 } else { 0 }),
        "AC" => return Ok(if machine.cpu.flags.aux_carry { 1 } else { 0 }),
        _ => {}
    }

    // Look in symbol table
    if let Some(sm) = source_map {
        if let Some(&addr) = sm.symbols.get(s).or_else(|| sm.symbols.get(&upper)) {
            let byte_val = machine.ram.read(Addr(addr));
            return Ok(byte_val as u16);
        }
    }

    Err(DapError::UnknownIdentifier(s.to_string()))
}

fn format_eval_value(val: u16, is_word: bool) -> Result<EvalResult, DapError> {
    let ascii_char = if val <= 0x7E && val >= 0x20 {
        format!("'{}'", (val as u8) as char)
    } else {
        "'.'".to_string()
    };

    let display = if is_word || val > 0xFF {
        format!("{val} (0x{val:04X}, 0b{val:016b})")
    } else {
        let b = val as u8;
        format!("{b} (0x{b:02X}, 0b{b:08b}, {ascii_char})")
    };

    Ok(EvalResult {
        display,
        r#type: if is_word || val > 0xFF { "u16".to_string() } else { "u8".to_string() },
        raw_value: Some(val),
    })
}

fn handle_repl_command(
    cmd: &str,
    machine: &Machine,
    source_map: Option<&SourceMap>,
) -> Result<EvalResult, DapError> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err(DapError::InvalidExpression("empty command".to_string()));
    }

    match parts[0] {
        "mem" | "dump" => {
            if parts.len() < 2 {
                return Err(DapError::InvalidExpression("usage: :mem <addr> [count]".to_string()));
            }
            let addr = eval_term(parts[1], machine, source_map)?;
            let count = if parts.len() >= 3 {
                eval_term(parts[2], machine, source_map)? as usize
            } else {
                16
            };
            let mut lines = Vec::new();
            for i in (0..count).step_by(16) {
                let row_addr = addr.wrapping_add(i as u16);
                let row_count = (count - i).min(16);
                let mut hex_parts = Vec::new();
                let mut ascii_parts = String::new();
                for j in 0..row_count {
                    let b = machine.ram.read(Addr(row_addr.wrapping_add(j as u16)));
                    hex_parts.push(format!("{b:02X}"));
                    ascii_parts.push(if b >= 0x20 && b <= 0x7E { b as char } else { '.' });
                }
                lines.push(format!("0x{row_addr:04X}: {:<48} |{ascii_parts}|", hex_parts.join(" ")));
            }
            Ok(EvalResult {
                display: lines.join("\n"),
                r#type: "memory_dump".to_string(),
                raw_value: None,
            })
        }
        "regs" => {
            let psw = machine.cpu.flags.to_psw();
            let out = format!(
                "A: 0x{:02X}  B: 0x{:02X}  C: 0x{:02X}  D: 0x{:02X}  E: 0x{:02X}  H: 0x{:02X}  L: 0x{:02X}\nSP: 0x{:04X}  PC: 0x{:04X}  PSW: 0x{:02X} [Z:{} S:{} P:{} CY:{} AC:{}]",
                machine.cpu.regs.get8(Reg8::A),
                machine.cpu.regs.get8(Reg8::B),
                machine.cpu.regs.get8(Reg8::C),
                machine.cpu.regs.get8(Reg8::D),
                machine.cpu.regs.get8(Reg8::E),
                machine.cpu.regs.get8(Reg8::H),
                machine.cpu.regs.get8(Reg8::L),
                machine.cpu.regs.sp.0,
                machine.cpu.regs.pc.0,
                psw,
                machine.cpu.flags.zero as u8,
                machine.cpu.flags.sign as u8,
                machine.cpu.flags.parity as u8,
                machine.cpu.flags.carry as u8,
                machine.cpu.flags.aux_carry as u8,
            );
            Ok(EvalResult {
                display: out,
                r#type: "registers".to_string(),
                raw_value: None,
            })
        }
        "help" => Ok(EvalResult {
            display: "8085 Debugger Commands:\n  <expr>             - Evaluate register, variable, or memory (*0x2000)\n  :mem <addr> [len]  - Dump memory slice\n  :regs              - Display all registers and flags\n  :in <text>         - Feed input string to TextIO device\n  :key <char>        - Press key on Keyboard device\n  :help              - Show this help summary".to_string(),
            r#type: "help".to_string(),
            raw_value: None,
        }),
        _ => Err(DapError::InvalidExpression(format!("unknown debug command ':{cmd}'"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_literal_and_unary_sign_evaluation() {
        let mut machine = Machine::default();
        machine.cpu.regs.set8(Reg8::A, 0x05);

        // Negative literal -1 evaluates to 0xFFFF
        let res_neg1 = evaluate_expression("-1", &machine, None).unwrap();
        assert_eq!(res_neg1.raw_value, Some(0xFFFF));

        // Negative hex -0x10 evaluates to 0xFFF0
        let res_neg_hex = evaluate_expression("-0x10", &machine, None).unwrap();
        assert_eq!(res_neg_hex.raw_value, Some(0xFFF0));

        // Positive literal +42
        let res_pos = evaluate_expression("+42", &machine, None).unwrap();
        assert_eq!(res_pos.raw_value, Some(42));

        // Subtraction with register: A - 1 = 4
        let res_sub = evaluate_expression("A - 1", &machine, None).unwrap();
        assert_eq!(res_sub.raw_value, Some(4));

        // Negative arithmetic: -5 - 2 = -7 (0xFFF9)
        let res_neg_sub = evaluate_expression("-5 - 2", &machine, None).unwrap();
        assert_eq!(res_neg_sub.raw_value, Some(0xFFF9));

        // Comparison with negative literal
        let res_cmp = evaluate_expression("A != -1", &machine, None).unwrap();
        assert_eq!(res_cmp.raw_value, Some(1));
    }

    #[test]
    fn register_flag_and_memory_dereference() {
        let mut machine = Machine::default();
        machine.cpu.regs.set8(Reg8::B, 0x42);
        machine.cpu.flags.zero = true;
        machine.ram.write(Addr(0x2000), 0x77);

        assert_eq!(evaluate_expression("B", &machine, None).unwrap().raw_value, Some(0x42));
        assert_eq!(evaluate_expression("Z", &machine, None).unwrap().raw_value, Some(1));
        assert_eq!(evaluate_expression("*0x2000", &machine, None).unwrap().raw_value, Some(0x77));
        assert_eq!(evaluate_expression("B == 0x42", &machine, None).unwrap().raw_value, Some(1));
    }
}
