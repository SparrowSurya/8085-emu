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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSymbol {
    pub name: String,
    pub address: u16,
    pub size_bytes: usize,
    pub type_name: String, // "BYTE" | "WORD" | "BUFFER"
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    pub main_file: PathBuf,
    pub addr_to_loc: BTreeMap<u16, SourceLocation>,
    pub loc_to_addr: BTreeMap<(PathBuf, usize), u16>,
    pub symbols: BTreeMap<String, u16>,
    pub reverse_symbols: BTreeMap<u16, String>,
    pub variables: Vec<VariableSymbol>,
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
        let sym_map = symbols.clone();
        let mut reverse_symbols = BTreeMap::new();
        let mut variables = Vec::new();

        for (name, &addr) in &sym_map {
            reverse_symbols.insert(addr, name.clone());
        }

        // Populate from listing rows
        for entry in listing {
            if entry.line > 0 {
                let file = entry.file_path.clone().unwrap_or_else(|| main_file.clone());
                let loc = SourceLocation {
                    file_path: file.clone(),
                    line: entry.line,
                    col: entry.col,
                    line_text: entry.source.clone(),
                };
                addr_to_loc.insert(entry.addr, loc);
                loc_to_addr.insert((file, entry.line), entry.addr);
            }
        }

        // Extract variables from program AST if available
        if let Some(prog) = program {
            for seg in &prog.segments {
                match seg {
                    Segment::Data(defs) => {
                        for def in defs {
                            if let Some(&addr) = sym_map.get(&def.name) {
                                let (size_bytes, type_name) = match def.size {
                                    Size::Byte => {
                                        let count = def.values.len().max(1);
                                        (count, if count == 1 { "BYTE".to_string() } else { format!("BYTE[{count}]") })
                                    }
                                    Size::Word => {
                                        let count = def.values.len().max(1);
                                        (count * 2, if count == 1 { "WORD".to_string() } else { format!("WORD[{count}]") })
                                    }
                                };
                                variables.push(VariableSymbol {
                                    name: def.name.clone(),
                                    address: addr,
                                    size_bytes,
                                    type_name,
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
                                let (size_bytes, type_name) = match res.size {
                                    Size::Byte => (count_num, format!("BYTE[{count_num}]")),
                                    Size::Word => (count_num * 2, format!("WORD[{count_num}]")),
                                };
                                variables.push(VariableSymbol {
                                    name: res.name.clone(),
                                    address: addr,
                                    size_bytes,
                                    type_name,
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
            symbols: sym_map,
            reverse_symbols,
            variables,
            entry_pc: image.entry,
        }
    }

    pub fn address_to_location(&self, addr: u16) -> Option<&SourceLocation> {
        self.addr_to_loc.get(&addr)
    }

    pub fn location_to_address(&self, path: &Path, line: usize) -> Option<u16> {
        let key = (path.to_path_buf(), line);
        if let Some(&addr) = self.loc_to_addr.get(&key) {
            return Some(addr);
        }
        // Fallback matching by file name if path difference
        if let Some(file_name) = path.file_name() {
            for ((p, l), &addr) in &self.loc_to_addr {
                if *l == line && p.file_name() == Some(file_name) {
                    return Some(addr);
                }
            }
        }
        None
    }

    pub fn find_nearest_executable_line(&self, path: &Path, requested_line: usize) -> Option<(usize, u16)> {
        // Direct match
        if let Some(addr) = self.location_to_address(path, requested_line) {
            return Some((requested_line, addr));
        }

        // Search forward up to 50 lines
        let file_name = path.file_name();
        let mut candidates: Vec<(usize, u16)> = self
            .loc_to_addr
            .iter()
            .filter(|((p, l), _)| {
                let matches_file = p == path || (file_name.is_some() && p.file_name() == file_name);
                matches_file && *l >= requested_line
            })
            .map(|((_, l), &addr)| (*l, addr))
            .collect();

        candidates.sort_by_key(|(l, _)| *l);
        candidates.first().copied()
    }

    pub fn address_to_symbol(&self, addr: u16) -> Option<&str> {
        self.reverse_symbols.get(&addr).map(|s| s.as_str())
    }

    pub fn symbol_to_address(&self, symbol: &str) -> Option<u16> {
        self.symbols.get(symbol).copied()
    }
}
