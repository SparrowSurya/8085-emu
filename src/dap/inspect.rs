use crate::cpu::registers::{Reg8, Reg16};
use crate::dap::error::DapError;
use crate::dap::eval::eval_term;
use crate::dap::protocol::{Scope, Variable};
use crate::dap::sourcemap::SourceMap;
use crate::machine::Machine;
use crate::value::Addr;

pub const SCOPE_REGISTERS: i64 = 1000;
pub const SCOPE_PAIRS: i64 = 2000;
pub const SCOPE_FLAGS: i64 = 3000;
pub const SCOPE_DATA: i64 = 4000;
pub const SCOPE_BSS: i64 = 5000;
pub const SCOPE_STACK: i64 = 6000;
pub const SCOPE_PERIPHERALS: i64 = 7000;
pub const SCOPE_MACRO_ARGS: i64 = 8000;

pub fn get_scopes(frame_id: i64, active_macro: Option<&crate::dap::sourcemap::MacroInvocation>) -> Vec<Scope> {
    let mut scopes = Vec::new();

    if let Some(m) = active_macro {
        if !m.arguments.is_empty() {
            scopes.push(Scope {
                name: format!("Macro Arguments ({})", m.name),
                presentation_hint: Some("arguments".to_string()),
                variables_reference: SCOPE_MACRO_ARGS + frame_id,
                named_variables: Some(m.arguments.len()),
                indexed_variables: None,
                expensive: false,
            });
        }
    }

    scopes.push(Scope {
        name: "CPU Registers".to_string(),
        presentation_hint: Some("registers".to_string()),
        variables_reference: SCOPE_REGISTERS + frame_id,
        named_variables: Some(10),
        indexed_variables: None,
        expensive: false,
    });
    scopes.push(Scope {
        name: "Register Pairs".to_string(),
        presentation_hint: Some("registers".to_string()),
        variables_reference: SCOPE_PAIRS + frame_id,
        named_variables: Some(4),
        indexed_variables: None,
        expensive: false,
    });
    scopes.push(Scope {
        name: "Flags (PSW)".to_string(),
        presentation_hint: Some("status".to_string()),
        variables_reference: SCOPE_FLAGS + frame_id,
        named_variables: Some(6),
        indexed_variables: None,
        expensive: false,
    });
    scopes.push(Scope {
        name: "Data Segment (.data)".to_string(),
        presentation_hint: Some("locals".to_string()),
        variables_reference: SCOPE_DATA + frame_id,
        named_variables: None,
        indexed_variables: None,
        expensive: false,
    });
    scopes.push(Scope {
        name: "BSS Segment (.bss)".to_string(),
        presentation_hint: Some("locals".to_string()),
        variables_reference: SCOPE_BSS + frame_id,
        named_variables: None,
        indexed_variables: None,
        expensive: false,
    });
    scopes.push(Scope {
        name: "Stack".to_string(),
        presentation_hint: Some("locals".to_string()),
        variables_reference: SCOPE_STACK + frame_id,
        named_variables: None,
        indexed_variables: None,
        expensive: false,
    });
    scopes.push(Scope {
        name: "Peripherals & I/O".to_string(),
        presentation_hint: Some("globals".to_string()),
        variables_reference: SCOPE_PERIPHERALS + frame_id,
        named_variables: Some(3),
        indexed_variables: None,
        expensive: false,
    });

    scopes
}

pub fn get_variables(
    variables_reference: i64,
    machine: &Machine,
    source_map: Option<&SourceMap>,
    elapsed_t_states: u64,
    instruction_count: u64,
    active_macro: Option<&crate::dap::sourcemap::MacroInvocation>,
) -> Vec<Variable> {
    // Normalize reference by stripping frame_id modulo
    let base_ref = (variables_reference / 1000) * 1000;

    match base_ref {
        SCOPE_MACRO_ARGS => {
            let mut list = Vec::new();
            if let Some(m) = active_macro {
                for arg in &m.arguments {
                    list.push(resolve_macro_arg(&arg.name, &arg.expr, machine, source_map));
                }
            }
            if list.is_empty() {
                list.push(Variable {
                    name: "(No macro arguments)".to_string(),
                    value: "".to_string(),
                    r#type: None,
                    variables_reference: 0,
                    evaluate_name: None,
                    memory_reference: None,
                });
            }
            list
        }
        SCOPE_REGISTERS => {
            let regs = &machine.cpu.regs;
            let hl_addr = regs.get16(Reg16::HL).0;
            let m_val = machine.ram.read(Addr(hl_addr));

            vec![
                var_8bit("A", regs.get8(Reg8::A)),
                var_8bit("B", regs.get8(Reg8::B)),
                var_8bit("C", regs.get8(Reg8::C)),
                var_8bit("D", regs.get8(Reg8::D)),
                var_8bit("E", regs.get8(Reg8::E)),
                var_8bit("H", regs.get8(Reg8::H)),
                var_8bit("L", regs.get8(Reg8::L)),
                var_8bit(&format!("M [HL: 0x{hl_addr:04X}]"), m_val),
                var_16bit("SP", regs.sp.0, source_map),
                var_16bit("PC", regs.pc.0, source_map),
                Variable {
                    name: "T-States".to_string(),
                    value: format!("{elapsed_t_states} cycles"),
                    r#type: Some("timing".to_string()),
                    variables_reference: 0,
                    evaluate_name: Some("T-States".to_string()),
                    memory_reference: None,
                },
                Variable {
                    name: "Instructions Executed".to_string(),
                    value: format!("{instruction_count}"),
                    r#type: Some("counter".to_string()),
                    variables_reference: 0,
                    evaluate_name: Some("Instructions".to_string()),
                    memory_reference: None,
                },
            ]
        }
        SCOPE_PAIRS => {
            let regs = &machine.cpu.regs;
            let psw = machine.cpu.flags.to_psw();
            let af = ((regs.get8(Reg8::A) as u16) << 8) | (psw as u16);

            vec![
                var_16bit("BC", regs.get16(Reg16::BC).0, source_map),
                var_16bit("DE", regs.get16(Reg16::DE).0, source_map),
                var_16bit("HL", regs.get16(Reg16::HL).0, source_map),
                Variable {
                    name: "PSW (AF)".to_string(),
                    value: format!("0x{af:04X} (A: 0x{:02X}, Flags: 0x{psw:02X})", regs.get8(Reg8::A)),
                    r#type: Some("u16".to_string()),
                    variables_reference: 0,
                    evaluate_name: Some("PSW".to_string()),
                    memory_reference: None,
                },
            ]
        }
        SCOPE_FLAGS => {
            let f = &machine.cpu.flags;
            let psw_byte = f.to_psw();
            vec![
                Variable {
                    name: "Flags Byte (PSW)".to_string(),
                    value: format!("0x{psw_byte:02X} (0b{psw_byte:08b})"),
                    r#type: Some("u8".to_string()),
                    variables_reference: 0,
                    evaluate_name: Some("PSW".to_string()),
                    memory_reference: None,
                },
                var_flag("Zero (Z)", f.zero, "Result was zero"),
                var_flag("Sign (S)", f.sign, "Result MSB was 1 (negative)"),
                var_flag("Parity (P)", f.parity, "Even number of 1-bits"),
                var_flag("Carry (CY)", f.carry, "Arithmetic carry/borrow"),
                var_flag("Aux Carry (AC)", f.aux_carry, "BCD half-carry from bit 3 to 4"),
            ]
        }
        SCOPE_DATA => {
            let mut list = Vec::new();
            if let Some(sm) = source_map {
                for var in &sm.variables {
                    if var.segment_kind == crate::dap::sourcemap::SegmentKind::Data {
                        let mut bytes = Vec::new();
                        for i in 0..var.size_bytes {
                            bytes.push(machine.ram.read(Addr(var.address.wrapping_add(i as u16))));
                        }
                        let is_word = var.type_name.starts_with("WORD");
                        let type_char = if is_word { 'W' } else { 'B' };
                        let count = if is_word { var.size_bytes / 2 } else { var.size_bytes };
                        let name_label = format!("{} 0x{:04X} ({count}{type_char})", var.name, var.address);
                        let val_display = format_escaped_string(&bytes);

                        list.push(Variable {
                            name: name_label,
                            value: val_display,
                            r#type: Some(var.type_name.clone()),
                            variables_reference: 0,
                            evaluate_name: Some(var.name.clone()),
                            memory_reference: Some(format!("0x{:04X}", var.address)),
                        });
                    }
                }
            }
            if list.is_empty() {
                list.push(Variable {
                    name: "(No data segment variables)".to_string(),
                    value: "".to_string(),
                    r#type: None,
                    variables_reference: 0,
                    evaluate_name: None,
                    memory_reference: None,
                });
            }
            list
        }
        SCOPE_BSS => {
            let mut list = Vec::new();
            if let Some(sm) = source_map {
                for var in &sm.variables {
                    if var.segment_kind == crate::dap::sourcemap::SegmentKind::Bss {
                        let mut bytes = Vec::new();
                        for i in 0..var.size_bytes {
                            bytes.push(machine.ram.read(Addr(var.address.wrapping_add(i as u16))));
                        }
                        let is_word = var.type_name.starts_with("WORD");
                        let type_char = if is_word { 'W' } else { 'B' };
                        let count = if is_word { var.size_bytes / 2 } else { var.size_bytes };
                        let name_label = format!("{} 0x{:04X} ({count}{type_char})", var.name, var.address);
                        let val_display = format_escaped_string(&bytes);

                        list.push(Variable {
                            name: name_label,
                            value: val_display,
                            r#type: Some(var.type_name.clone()),
                            variables_reference: 0,
                            evaluate_name: Some(var.name.clone()),
                            memory_reference: Some(format!("0x{:04X}", var.address)),
                        });
                    }
                }
            }
            if list.is_empty() {
                list.push(Variable {
                    name: "(No BSS variables)".to_string(),
                    value: "".to_string(),
                    r#type: None,
                    variables_reference: 0,
                    evaluate_name: None,
                    memory_reference: None,
                });
            }
            list
        }
        SCOPE_STACK => {
            let mut list = Vec::new();
            let sp = machine.cpu.regs.sp.0;

            // Stack grows downward. Valid stack entries are from SP up to 0xFFFE.
            // Iterate in 2-byte word increments.
            if sp != 0 && sp <= 0xFFFE {
                let mut addr = sp;
                let mut count = 0;
                while addr <= 0xFFFE && count < 64 {
                    let low = machine.ram.read(Addr(addr));
                    let high = machine.ram.read(Addr(addr.wrapping_add(1)));
                    let word = ((high as u16) << 8) | (low as u16);
                    let str_repr = format_escaped_string(&[high, low]);

                    // Check for symbol / label associated with word or addr
                    let sym_label = if let Some(sm) = source_map {
                        sm.reverse_symbols.get(&word).or_else(|| sm.reverse_symbols.get(&addr))
                    } else {
                        None
                    };

                    let marker = match (addr == sp, sym_label) {
                        (true, Some(sym)) => format!("(SP, {sym})"),
                        (true, None) => "(SP)".to_string(),
                        (false, Some(sym)) => format!("({sym})"),
                        (false, None) => "".to_string(),
                    };

                    let val_formatted = if marker.is_empty() {
                        format!("0x{word:04X}  {str_repr}")
                    } else {
                        format!("0x{word:04X}  {str_repr:<8}  {marker}")
                    };

                    list.push(Variable {
                        name: format!("0x{addr:04X}"),
                        value: val_formatted,
                        r#type: Some("word".to_string()),
                        variables_reference: 0,
                        evaluate_name: None,
                        memory_reference: Some(format!("0x{addr:04X}")),
                    });

                    if addr == 0xFFFE {
                        break;
                    }
                    addr = addr.wrapping_add(2);
                    count += 1;
                }
            }

            if list.is_empty() {
                list.push(Variable {
                    name: "(Stack is empty)".to_string(),
                    value: format!("SP = 0x{sp:04X}"),
                    r#type: None,
                    variables_reference: 0,
                    evaluate_name: None,
                    memory_reference: None,
                });
            }
            list
        }
        SCOPE_PERIPHERALS => {
            let mut list = Vec::new();
            // TextIO
            if let Some(term) = machine.devices.find_device::<crate::device::TextIODevice>() {
                let out_text = term.output_string();
                list.push(Variable {
                    name: "TextIO Output".to_string(),
                    value: if out_text.is_empty() { "\"(empty)\"".to_string() } else { format!("\"{out_text}\"") },
                    r#type: Some("TextIODevice".to_string()),
                    variables_reference: 0,
                    evaluate_name: None,
                    memory_reference: None,
                });
            }
            // Keyboard
            if let Some(kbd) = machine.devices.find_device::<crate::device::KeyboardDevice>() {
                list.push(Variable {
                    name: "Keyboard Buffer".to_string(),
                    value: format!("{} bytes queued", kbd.buffer_len()),
                    r#type: Some("KeyboardDevice".to_string()),
                    variables_reference: 0,
                    evaluate_name: None,
                    memory_reference: None,
                });
            }
            // Printer
            if let Some(prn) = machine.devices.find_device::<crate::device::PrinterDevice>() {
                let hist = &prn.history;
                list.push(Variable {
                    name: "Printer Output".to_string(),
                    value: if hist.is_empty() { "\"(empty)\"".to_string() } else { format!("\"{hist}\"") },
                    r#type: Some("PrinterDevice".to_string()),
                    variables_reference: 0,
                    evaluate_name: None,
                    memory_reference: None,
                });
            }
            list
        }
        _ => Vec::new(),
    }
}

pub fn set_variable(
    _variables_reference: i64,
    name: &str,
    value: &str,
    machine: &mut Machine,
    source_map: Option<&SourceMap>,
) -> Result<(String, String), DapError> {
    let raw_val = eval_term(value, machine, source_map)?;
    let clean_name = name.split_whitespace().next().unwrap_or(name);
    let upper = clean_name.to_ascii_uppercase();

    match upper.as_str() {
        "A" => {
            machine.cpu.regs.set8(Reg8::A, raw_val as u8);
            Ok((format!("0x{:02X}", raw_val as u8), "u8".to_string()))
        }
        "B" => {
            machine.cpu.regs.set8(Reg8::B, raw_val as u8);
            Ok((format!("0x{:02X}", raw_val as u8), "u8".to_string()))
        }
        "C" => {
            machine.cpu.regs.set8(Reg8::C, raw_val as u8);
            Ok((format!("0x{:02X}", raw_val as u8), "u8".to_string()))
        }
        "D" => {
            machine.cpu.regs.set8(Reg8::D, raw_val as u8);
            Ok((format!("0x{:02X}", raw_val as u8), "u8".to_string()))
        }
        "E" => {
            machine.cpu.regs.set8(Reg8::E, raw_val as u8);
            Ok((format!("0x{:02X}", raw_val as u8), "u8".to_string()))
        }
        "H" => {
            machine.cpu.regs.set8(Reg8::H, raw_val as u8);
            Ok((format!("0x{:02X}", raw_val as u8), "u8".to_string()))
        }
        "L" => {
            machine.cpu.regs.set8(Reg8::L, raw_val as u8);
            Ok((format!("0x{:02X}", raw_val as u8), "u8".to_string()))
        }
        "SP" => {
            machine.cpu.regs.sp = Addr(raw_val);
            Ok((format!("0x{raw_val:04X}"), "u16".to_string()))
        }
        "PC" => {
            machine.cpu.regs.pc = Addr(raw_val);
            Ok((format!("0x{raw_val:04X}"), "u16".to_string()))
        }
        "BC" => {
            machine.cpu.regs.set16(Reg16::BC, Addr(raw_val));
            Ok((format!("0x{raw_val:04X}"), "u16".to_string()))
        }
        "DE" => {
            machine.cpu.regs.set16(Reg16::DE, Addr(raw_val));
            Ok((format!("0x{raw_val:04X}"), "u16".to_string()))
        }
        "HL" => {
            machine.cpu.regs.set16(Reg16::HL, Addr(raw_val));
            Ok((format!("0x{raw_val:04X}"), "u16".to_string()))
        }
        _ => {
            // Check variable in source_map
            if let Some(sm) = source_map {
                let base_name = clean_name.split('(').next().unwrap_or(clean_name).trim();
                if let Some(&addr) = sm.symbols.get(base_name) {
                    machine.ram.write(Addr(addr), raw_val as u8);
                    return Ok((format!("0x{:02X}", raw_val as u8), "u8".to_string()));
                }
            }
            Err(DapError::InvalidVariable(format!("cannot modify variable/register '{clean_name}'")))
        }
    }
}

fn var_8bit(name: &str, val: u8) -> Variable {
    let ascii = if (0x20..=0x7E).contains(&val) {
        format!("'{}'", val as char)
    } else {
        "'.'".to_string()
    };
    Variable {
        name: name.to_string(),
        value: format!("0x{val:02X} ({val}, {ascii})"),
        r#type: Some("u8".to_string()),
        variables_reference: 0,
        evaluate_name: Some(name.split_whitespace().next().unwrap_or(name).to_string()),
        memory_reference: None,
    }
}

fn var_16bit(name: &str, val: u16, source_map: Option<&SourceMap>) -> Variable {
    let sym_label = if let Some(sm) = source_map {
        sm.reverse_symbols.get(&val).cloned()
    } else {
        None
    };

    let bracket_content = match sym_label {
        Some(sym) => sym,
        None => format!("{val}"),
    };

    Variable {
        name: name.to_string(),
        value: format!("0x{val:04X} ({bracket_content})"),
        r#type: Some("u16".to_string()),
        variables_reference: 0,
        evaluate_name: Some(name.to_string()),
        memory_reference: Some(format!("0x{val:04X}")),
    }
}

fn var_flag(name: &str, bit: bool, doc: &str) -> Variable {
    Variable {
        name: name.to_string(),
        value: if bit { format!("1 (True - {doc})") } else { format!("0 (False - {doc})") },
        r#type: Some("flag".to_string()),
        variables_reference: 0,
        evaluate_name: Some(name.split_whitespace().next().unwrap_or(name).to_string()),
        memory_reference: None,
    }
}

pub fn format_escaped_string(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2 + 2);
    s.push('"');
    for &b in bytes {
        match b {
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7E => s.push(b as char),
            _ => {
                use std::fmt::Write;
                let _ = write!(s, "\\x{b:02X}");
            }
        }
    }
    s.push('"');
    s
}

pub fn resolve_macro_arg(
    arg_name: &str,
    expr: &str,
    machine: &Machine,
    source_map: Option<&SourceMap>,
) -> Variable {
    let trimmed = expr.trim();
    let (display_val, type_str, mem_ref) = if let Some(ident) = trimmed.strip_prefix("%len").or_else(|| trimmed.strip_prefix("%LEN")) {
        let var_name = ident.trim();
        let len_opt = if let Some(sm) = source_map {
            sm.variables.iter().find(|v| v.name.eq_ignore_ascii_case(var_name)).map(|v| v.size_bytes)
        } else {
            None
        };
        if let Some(len) = len_opt {
            (format!("%len {var_name} ({len} = 0x{len:02X})"), "length".to_string(), None)
        } else {
            (trimmed.to_string(), "length".to_string(), None)
        }
    } else if let Some(sm) = source_map {
        if let Some(&addr) = sm.symbols.get(trimmed) {
            if let Some(var) = sm.variables.iter().find(|v| v.address == addr) {
                let mut bytes = Vec::new();
                for i in 0..var.size_bytes {
                    bytes.push(machine.ram.read(Addr(addr.wrapping_add(i as u16))));
                }
                let is_str = var.type_name.starts_with("BYTE") && var.size_bytes > 1;
                let val_disp = if is_str {
                    format_escaped_string(&bytes)
                } else if var.size_bytes == 1 {
                    format!("0x{:02X} ({})", bytes[0], bytes[0])
                } else if var.size_bytes == 2 {
                    let w = ((bytes[1] as u16) << 8) | (bytes[0] as u16);
                    format!("0x{w:04X} ({w})")
                } else {
                    format_escaped_string(&bytes)
                };
                (format!("{trimmed} (0x{addr:04X}: {val_disp})"), var.type_name.clone(), Some(format!("0x{addr:04X}")))
            } else {
                (format!("{trimmed} (0x{addr:04X})"), "symbol".to_string(), Some(format!("0x{addr:04X}")))
            }
        } else if let Ok(val) = eval_term(trimmed, machine, source_map) {
            (format!("{trimmed} (0x{val:04X})"), "immediate".to_string(), None)
        } else {
            (trimmed.to_string(), "operand".to_string(), None)
        }
    } else if let Ok(val) = eval_term(trimmed, machine, source_map) {
        (format!("{trimmed} (0x{val:04X})"), "immediate".to_string(), None)
    } else {
        (trimmed.to_string(), "operand".to_string(), None)
    };

    Variable {
        name: arg_name.to_string(),
        value: display_val,
        r#type: Some(type_str),
        variables_reference: 0,
        evaluate_name: Some(arg_name.to_string()),
        memory_reference: mem_ref,
    }
}

