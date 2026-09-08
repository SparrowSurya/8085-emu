//! Bidirectional source map, symbol table, and listing index for 8085 debugging.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::asm::{ListingRow, LoadImage, Program, Segment, Size, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file_path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub line_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Data,
    Bss,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSymbol {
    pub name: String,
    pub address: u16,
    pub size_bytes: usize,
    pub type_name: String, // "BYTE" | "WORD" | "BUFFER"
    pub segment_kind: SegmentKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroArgument {
    pub name: String,
    pub expr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroInvocation {
    pub name: String,
    pub call_location: SourceLocation,
    pub start_addr: u16,
    pub end_addr: u16,
    pub arguments: Vec<MacroArgument>,
    pub body_locations: BTreeMap<u16, SourceLocation>,
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    pub main_file: PathBuf,
    pub addr_to_loc: BTreeMap<u16, SourceLocation>,
    pub loc_to_addr: BTreeMap<(PathBuf, usize), u16>,
    pub loc_to_addrs: BTreeMap<(PathBuf, usize), Vec<u16>>,
    pub symbols: BTreeMap<String, u16>,
    pub reverse_symbols: BTreeMap<u16, String>,
    pub variables: Vec<VariableSymbol>,
    pub macros: Vec<MacroInvocation>,
    pub entry_pc: u16,
}

impl SourceMap {
    pub fn from_assembly(
        main_file: PathBuf,
        image: &LoadImage,
        symbols: &BTreeMap<String, u16>,
        listing: &[ListingRow],
        program: Option<&Program>,
    ) -> Self {
        let mut addr_to_loc = BTreeMap::new();
        let mut loc_to_addr = BTreeMap::new();
        let mut loc_to_addrs: BTreeMap<(PathBuf, usize), Vec<u16>> = BTreeMap::new();
        let sym_map = symbols.clone();
        let mut reverse_symbols = BTreeMap::new();
        let mut variables = Vec::new();
        let mut macros = Vec::new();
        let mut macro_stack: Vec<(String, SourceLocation, u16, Vec<MacroArgument>, BTreeMap<u16, SourceLocation>)> = Vec::new();

        for (name, &addr) in &sym_map {
            reverse_symbols.insert(addr, name.clone());
        }

        // Populate from listing rows
        for entry in listing {
            if entry.line == 0 && entry.source == "%macro_end" {
                if let Some((name, call_loc, start_addr, arguments, body_locs)) = macro_stack.pop() {
                    macros.push(MacroInvocation {
                        name,
                        call_location: call_loc,
                        start_addr,
                        end_addr: entry.addr,
                        arguments,
                        body_locations: body_locs,
                    });
                }
                continue;
            }

            if entry.line > 0 {
                let file = entry.file_path.clone().unwrap_or_else(|| main_file.clone());

                if entry.source.starts_with("%macro_call:") {
                    let rest = entry.source.strip_prefix("%macro_call:").unwrap_or("");
                    let parts: Vec<&str> = rest.splitn(3, '|').collect();
                    let macro_name = parts.get(0).copied().unwrap_or("").to_string();
                    let line_text = parts.get(1).copied().unwrap_or("").to_string();
                    let raw_args: Vec<String> = parts
                        .get(2)
                        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                        .unwrap_or_default();

                    let arguments = raw_args
                        .into_iter()
                        .enumerate()
                        .map(|(idx, expr)| MacroArgument {
                            name: format!("%{}", idx + 1),
                            expr,
                        })
                        .collect();

                    let loc = SourceLocation {
                        file_path: file.clone(),
                        line: entry.line,
                        col: entry.col,
                        line_text,
                    };
                    macro_stack.push((macro_name, loc.clone(), entry.addr, arguments, BTreeMap::new()));
                    loc_to_addr.insert((file.clone(), entry.line), entry.addr);
                    let addrs = loc_to_addrs.entry((file, entry.line)).or_default();
                    if !addrs.contains(&entry.addr) {
                        addrs.push(entry.addr);
                    }
                } else {
                    let loc = SourceLocation {
                        file_path: file.clone(),
                        line: entry.line,
                        col: entry.col,
                        line_text: entry.source.clone(),
                    };
                    if !entry.bytes.is_empty() {
                        for (_, _, _, _, body_locs) in &mut macro_stack {
                            body_locs.insert(entry.addr, loc.clone());
                        }
                    }
                    if !entry.bytes.is_empty() || !addr_to_loc.contains_key(&entry.addr) {
                        addr_to_loc.insert(entry.addr, loc);
                    }
                    loc_to_addr.insert((file.clone(), entry.line), entry.addr);
                    let addrs = loc_to_addrs.entry((file, entry.line)).or_default();
                    if !addrs.contains(&entry.addr) {
                        addrs.push(entry.addr);
                    }
                }
            }
        }

        // Extract variables from program AST if available
        if let Some(prog) = program {
            for seg in &prog.segments {
                match seg {
                    Segment::Data(defs) => {
                        for def in defs {
                            if let Some(&addr) = sym_map.get(&def.name) {
                                let mut total_units = 0;
                                for val in &def.values {
                                    match val {
                                        Value::Number(_) | Value::Char(_) => total_units += 1,
                                        Value::Str(s) => total_units += s.len(),
                                        Value::Repeat { count, .. } => {
                                            let n = match count.as_ref() {
                                                Value::Number(num) => *num as usize,
                                                _ => 1,
                                             };
                                            total_units += n;
                                        }
                                        _ => total_units += 1,
                                    }
                                }
                                let size_bytes = match def.size {
                                    Size::Byte => total_units.max(1),
                                    Size::Word => total_units.max(1) * 2,
                                };
                                let type_name = match def.size {
                                    Size::Byte => if size_bytes == 1 { "BYTE".to_string() } else { format!("BYTE[{size_bytes}]") },
                                    Size::Word => if size_bytes == 2 { "WORD".to_string() } else { format!("WORD[{}]", size_bytes / 2) },
                                };
                                variables.push(VariableSymbol {
                                    name: def.name.clone(),
                                    address: addr,
                                    size_bytes,
                                    type_name,
                                    segment_kind: SegmentKind::Data,
                                });
                            }
                        }
                    }
                    Segment::Bss(reservations) => {
                        for res in reservations {
                            if let Some(&addr) = sym_map.get(&res.name) {
                                let count_num = match &res.count {
                                    Value::Number(n) => *n as usize,
                                    _ => 1,
                                };
                                let size_bytes = match res.size {
                                    Size::Byte => count_num,
                                    Size::Word => count_num * 2,
                                };
                                let type_name = match res.size {
                                    Size::Byte => if size_bytes == 1 { "BYTE".to_string() } else { format!("BYTE[{size_bytes}]") },
                                    Size::Word => if size_bytes == 2 { "WORD".to_string() } else { format!("WORD[{}]", size_bytes / 2) },
                                };
                                variables.push(VariableSymbol {
                                    name: res.name.clone(),
                                    address: addr,
                                    size_bytes,
                                    type_name,
                                    segment_kind: SegmentKind::Bss,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Self {
            main_file,
            addr_to_loc,
            loc_to_addr,
            loc_to_addrs,
            symbols: sym_map,
            reverse_symbols,
            variables,
            macros,
            entry_pc: image.entry,
        }
    }

    pub fn find_macro_at_start(&self, addr: u16) -> Option<&MacroInvocation> {
        self.macros.iter().find(|m| m.start_addr == addr)
    }

    pub fn find_macro_containing(&self, addr: u16) -> Option<&MacroInvocation> {
        self.macros.iter().find(|m| m.start_addr <= addr && addr < m.end_addr)
    }

    pub fn address_to_location(&self, addr: u16) -> Option<&SourceLocation> {
        self.addr_to_loc.get(&addr)
    }

    pub fn location_to_address(&self, path: &Path, line: usize) -> Option<u16> {
        let key = (path.to_path_buf(), line);
        if let Some(&addr) = self.loc_to_addr.get(&key) {
            return Some(addr);
        }
        for ((p, l), &addr) in &self.loc_to_addr {
            if *l == line && paths_match(p, path) {
                return Some(addr);
            }
        }
        None
    }

    pub fn line_to_addresses(&self, path: &Path, line: usize) -> Vec<u16> {
        let key = (path.to_path_buf(), line);
        if let Some(addrs) = self.loc_to_addrs.get(&key) {
            if !addrs.is_empty() {
                return addrs.clone();
            }
        }
        let mut result = Vec::new();
        for ((p, l), addrs) in &self.loc_to_addrs {
            if *l == line && paths_match(p, path) {
                for &a in addrs {
                    if !result.contains(&a) {
                        result.push(a);
                    }
                }
            }
        }
        if !result.is_empty() {
            return result;
        }
        if let Some(addr) = self.location_to_address(path, line) {
            vec![addr]
        } else {
            Vec::new()
        }
    }

    pub fn find_nearest_executable_line(&self, path: &Path, requested_line: usize) -> Option<(usize, u16)> {
        let addrs = self.line_to_addresses(path, requested_line);
        if let Some(&first) = addrs.first() {
            return Some((requested_line, first));
        }

        // Search forward up to 50 lines
        let mut candidates: Vec<(usize, u16)> = self
            .loc_to_addrs
            .iter()
            .filter(|((p, l), _)| {
                paths_match(p, path) && *l >= requested_line
            })
            .filter_map(|((_, l), addrs)| addrs.first().map(|&a| (*l, a)))
            .collect();

        candidates.sort_by_key(|(l, _)| *l);
        candidates.first().copied()
    }

    pub fn find_nearest_executable_line_all(&self, path: &Path, requested_line: usize) -> Option<(usize, Vec<u16>)> {
        let addrs = self.line_to_addresses(path, requested_line);
        if !addrs.is_empty() {
            return Some((requested_line, addrs));
        }

        // Search forward up to 50 lines
        let mut candidates: Vec<(usize, Vec<u16>)> = self
            .loc_to_addrs
            .iter()
            .filter(|((p, l), _)| {
                paths_match(p, path) && *l >= requested_line
            })
            .map(|((_, l), addrs)| (*l, addrs.clone()))
            .collect();

        candidates.sort_by_key(|(l, _)| *l);
        candidates.first().cloned()
    }

    pub fn address_to_symbol(&self, addr: u16) -> Option<&str> {
        self.reverse_symbols.get(&addr).map(|s| s.as_str())
    }

    pub fn symbol_to_address(&self, symbol: &str) -> Option<u16> {
        self.symbols.get(symbol).copied()
    }
}

pub fn paths_match(p1: &Path, p2: &Path) -> bool {
    if p1 == p2 {
        return true;
    }
    if let (Ok(c1), Ok(c2)) = (std::fs::canonicalize(p1), std::fs::canonicalize(p2)) {
        if c1 == c2 {
            return true;
        }
    }
    if p1.ends_with(p2) || p2.ends_with(p1) {
        return true;
    }
    match (p1.file_name(), p2.file_name()) {
        (Some(f1), Some(f2)) => f1 == f2,
        _ => false,
    }
}
