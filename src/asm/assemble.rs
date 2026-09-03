//! Final assembly: resolve directives, lay out memory, build the symbol table, and emit
//! the load image.
//!
//! Memory layout: a 3-byte `JMP entry` bootstrap at `0x0000`, then `.data`, then `.bss`
//! (zero-filled), then `.text`; the remainder is stack. Execution begins at `0x0000`, so
//! every program starts at a fixed address; `entry` is the `main` label if present,
//! otherwise the first instruction of `.text`.

use std::collections::{BTreeMap, HashMap};

use super::ast::*;
use super::container::{
    BinaryContainer, CONTAINER_MAGIC, CONTAINER_VERSION, ContainerHeader, FLAG_HAS_EXPORT_SYMS,
    FLAG_HAS_VEC_TABLE,
};
use super::encode::{AReg8, AReg16, Operand, encode};
use super::error::{AsmError, AsmErrorKind, Span};
use super::{lex, parse};

/// Load an assembled image into a machine: write the bytes at `0x0000`, point the PC at
/// the bootstrap or entry point, and set the stack pointer. Execution begins with the bootstrap
/// `JMP` to the program entry or directly at `entry`.
pub fn load(
    machine: &mut crate::machine::Machine,
    image: &LoadImage,
) -> Result<(), crate::error::EmuError> {
    // 1. Write bootstrap jump at 0x0000 if entry != 0
    if image.entry != 0 && image.bytes.len() >= 3 {
        machine.ram.write(crate::value::Addr(0), image.bytes[0]);
        machine.ram.write(crate::value::Addr(1), image.bytes[1]);
        machine.ram.write(crate::value::Addr(2), image.bytes[2]);
    }

    // 2. Write ISR vector table hooks (0x0008..0x0040)
    let vec_len = (image.text_addr as usize)
        .min(image.data_addr as usize)
        .min(image.bytes.len())
        .min(VECTOR_TABLE_LEN as usize);
    if vec_len > 3 {
        for addr in 3..vec_len {
            let b = image.bytes[addr];
            if b != 0 {
                machine.ram.write(crate::value::Addr(addr as u16), b);
            }
        }
    }

    // 3. Write .data
    let data_start = image.data_addr as usize;
    let data_end = data_start + image.data_size as usize;
    if image.bytes.len() >= data_end {
        for (i, &b) in image.bytes[data_start..data_end].iter().enumerate() {
            machine.ram.write(
                crate::value::Addr(image.data_addr.wrapping_add(i as u16)),
                b,
            );
        }
    }

    // 4. Zero .bss
    for i in 0..image.bss_size {
        machine
            .ram
            .write(crate::value::Addr(image.bss_addr.wrapping_add(i)), 0);
    }

    // 5. Write .text
    let text_start = image.text_addr as usize;
    let text_end = text_start + image.text_size as usize;
    if image.bytes.len() >= text_end {
        for (i, &b) in image.bytes[text_start..text_end].iter().enumerate() {
            machine.ram.write(
                crate::value::Addr(image.text_addr.wrapping_add(i as u16)),
                b,
            );
        }
    }

    machine.cpu.regs.pc = crate::value::Addr(image.entry);
    machine.cpu.regs.sp = crate::value::Addr(image.sp_init);
    Ok(())
}

/// The 64-byte vector table occupies the bottom of memory (0x0000 - 0x003F).
const VECTOR_TABLE_LEN: u16 = 0x0040;
/// Data/Code begins immediately after the vector table.
const DATA_BASE: u16 = VECTOR_TABLE_LEN;

/// Standard interrupt vector table targets mapped to well-known ISR label names.
const VECTOR_HOOKS: &[(&[&str], u16, &str)] = &[
    (
        &["isr_rst1", "rst1_isr", "isr_rst_1"],
        0x0008,
        "; RST 1 vector",
    ),
    (
        &["isr_rst2", "rst2_isr", "isr_rst_2"],
        0x0010,
        "; RST 2 vector",
    ),
    (
        &["isr_rst3", "rst3_isr", "isr_rst_3"],
        0x0018,
        "; RST 3 vector",
    ),
    (
        &["isr_rst4", "rst4_isr", "isr_rst_4"],
        0x0020,
        "; RST 4 vector",
    ),
    (
        &["isr_trap", "trap_isr", "isr_trap_handler"],
        0x0024,
        "; TRAP vector",
    ),
    (
        &["isr_rst5", "rst5_isr", "isr_rst_5"],
        0x0028,
        "; RST 5 vector",
    ),
    (
        &["isr_rst55", "rst55_isr", "isr_rst_5_5", "isr_rst5_5"],
        0x002C,
        "; RST 5.5 vector",
    ),
    (
        &["isr_rst6", "rst6_isr", "isr_rst_6"],
        0x0030,
        "; RST 6 vector",
    ),
    (
        &["isr_rst65", "rst65_isr", "isr_rst_6_5", "isr_rst6_5"],
        0x0034,
        "; RST 6.5 vector",
    ),
    (
        &["isr_rst7", "rst7_isr", "isr_rst_7"],
        0x0038,
        "; RST 7 vector",
    ),
    (
        &["isr_rst75", "rst75_isr", "isr_rst_7_5", "isr_rst7_5"],
        0x003C,
        "; RST 7.5 vector",
    ),
];

/// A ready-to-load program image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadImage {
    /// The contiguous bytes, starting at address `0x0000`.
    pub bytes: Vec<u8>,
    /// The program entry point (target of the bootstrap jump), or 0 if no entry point.
    pub entry: u16,
    /// The stack pointer's initial value.
    pub sp_init: u16,
    /// RAM address where .text begins.
    pub text_addr: u16,
    /// Byte length of .text section.
    pub text_size: u16,
    /// RAM address where .data begins.
    pub data_addr: u16,
    /// Byte length of .data section.
    pub data_size: u16,
    /// RAM address where .bss begins.
    pub bss_addr: u16,
    /// Byte length of .bss section.
    pub bss_size: u16,
    /// Exported global symbols: `(name, address)`.
    pub export_symbols: Vec<(String, u16)>,
}

impl LoadImage {
    /// Converts this LoadImage into a BinaryContainer for .8085.bin files.
    pub fn to_container(&self) -> BinaryContainer {
        let vec_size = (self.text_addr as usize)
            .min(self.data_addr as usize)
            .min(VECTOR_TABLE_LEN as usize);
        let vec_bytes = if self.bytes.len() >= vec_size {
            self.bytes[0..vec_size].to_vec()
        } else {
            Vec::new()
        };

        let data_start = self.data_addr as usize;
        let data_end = data_start + self.data_size as usize;
        let data_bytes = if self.bytes.len() >= data_end {
            self.bytes[data_start..data_end].to_vec()
        } else {
            Vec::new()
        };

        let text_start = self.text_addr as usize;
        let text_end = text_start + self.text_size as usize;
        let text_bytes = if self.bytes.len() >= text_end {
            self.bytes[text_start..text_end].to_vec()
        } else {
            Vec::new()
        };

        let header = ContainerHeader {
            magic: CONTAINER_MAGIC,
            version: CONTAINER_VERSION,
            flags: if !vec_bytes.is_empty() {
                FLAG_HAS_VEC_TABLE
            } else {
                0
            } | if !self.export_symbols.is_empty() {
                FLAG_HAS_EXPORT_SYMS
            } else {
                0
            },
            entry_pc: self.entry,
            sp_init: self.sp_init,
            text_addr: self.text_addr,
            text_size: self.text_size,
            data_addr: self.data_addr,
            data_size: self.data_size,
            bss_addr: self.bss_addr,
            bss_size: self.bss_size,
            vec_size: vec_bytes.len() as u16,
            sym_size: 0,
            reserved: [0u8; 6],
        };

        BinaryContainer {
            header,
            vec_bytes,
            data_bytes,
            text_bytes,
            export_symbols: self.export_symbols.clone(),
        }
    }
}

/// One row of an assembly listing: the address, the bytes emitted there, and the source
/// text that produced them. A label row has no bytes; the bootstrap row has no source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingRow {
    /// Address of the first byte (or of the labelled instruction).
    pub addr: u16,
    /// Bytes emitted for this line (empty for a label).
    pub bytes: Vec<u8>,
    /// The trimmed source line.
    pub source: String,
}

/// Assemble source text into a [`LoadImage`].
pub fn assemble(src: &str) -> Result<LoadImage, AsmError> {
    assemble_and_link(src, None, &[])
}

/// Assemble source text with an optional base directory for `%include` and external symbols for linking.
pub fn assemble_with_options(
    src: &str,
    base_dir: Option<&std::path::Path>,
    external_symbols: &HashMap<String, u16>,
) -> Result<LoadImage, AsmError> {
    let raw_program = parse(lex(src)?)?;
    let program = if let Some(dir) = base_dir {
        super::include::resolve_includes(dir, &raw_program)?
    } else {
        let cur_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        super::include::resolve_includes(&cur_dir, &raw_program)?
    };

    let (image, _symbols, _listing) =
        Assembler::new(&program, src, external_symbols.clone(), Vec::new()).run()?;
    Ok(image)
}

/// Assemble source text and statically link precompiled `.8085.bin` libraries into a unified standalone [`LoadImage`].
pub fn assemble_and_link(
    src: &str,
    base_dir: Option<&std::path::Path>,
    linked_containers: &[BinaryContainer],
) -> Result<LoadImage, AsmError> {
    let raw_program = parse(lex(src)?)?;
    let program = if let Some(dir) = base_dir {
        super::include::resolve_includes(dir, &raw_program)?
    } else {
        let cur_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        super::include::resolve_includes(&cur_dir, &raw_program)?
    };

    let (image, _symbols, _listing) =
        Assembler::new(&program, src, HashMap::new(), linked_containers.to_vec()).run()?;
    Ok(image)
}

/// Assemble, also returning the resolved symbol table (name → absolute address), sorted
/// by name. Useful for listings, debugging, and `--dump`.
pub fn assemble_with_symbols(src: &str) -> Result<(LoadImage, BTreeMap<String, u16>), AsmError> {
    let raw_program = parse(lex(src)?)?;
    let cur_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let program = super::include::resolve_includes(&cur_dir, &raw_program)?;

    let (image, symbols, _listing) =
        Assembler::new(&program, src, HashMap::new(), Vec::new()).run()?;
    Ok((image, symbols.into_iter().collect()))
}

/// Assemble, returning the image plus a per-line listing (address, bytes, source).
pub fn assemble_listing(src: &str) -> Result<(LoadImage, Vec<ListingRow>), AsmError> {
    let raw_program = parse(lex(src)?)?;
    let cur_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let program = super::include::resolve_includes(&cur_dir, &raw_program)?;

    let (image, _symbols, listing) =
        Assembler::new(&program, src, HashMap::new(), Vec::new()).run()?;
    Ok((image, listing))
}

/// A resolved `%define` value.
#[derive(Debug, Clone)]
enum DVal {
    Num(u32),
    Str(String),
    Ch(u8),
}

/// One emitted data element: a literal byte, or the little-endian address of a symbol.
#[derive(Debug, Clone)]
enum Emit {
    Byte(u8),
    Sym16(String, Span),
}

impl Emit {
    fn width(&self) -> u16 {
        match self {
            Emit::Byte(_) => 1,
            Emit::Sym16(..) => 2,
        }
    }
}

/// A laid-out variable (its plan is `None` for `.bss`).
struct Var {
    name: String,
    span: Span,
    size: u16,
    plan: Option<Vec<Emit>>,
}

struct Assembler<'a> {
    program: &'a Program,
    src_lines: Vec<String>,
    defines: HashMap<String, DVal>,
    /// Byte size of every variable, filled in source order so `%len` can look backward.
    sizes: HashMap<String, u16>,
    data_vars: Vec<Var>,
    bss_vars: Vec<Var>,
    external_symbols: HashMap<String, u16>,
    linked_containers: Vec<BinaryContainer>,
}

impl<'a> Assembler<'a> {
    fn new(
        program: &'a Program,
        src: &str,
        mut external_symbols: HashMap<String, u16>,
        linked_containers: Vec<BinaryContainer>,
    ) -> Self {
        for lib in &linked_containers {
            for (sym, addr) in &lib.export_symbols {
                external_symbols.insert(sym.clone(), *addr);
            }
        }
        Assembler {
            program,
            src_lines: src.lines().map(|l| l.to_string()).collect(),
            defines: HashMap::new(),
            sizes: HashMap::new(),
            data_vars: Vec::new(),
            bss_vars: Vec::new(),
            external_symbols,
            linked_containers,
        }
    }

    /// The trimmed source text of a 1-based line (empty if out of range).
    fn src_line(&self, line: u32) -> String {
        self.src_lines
            .get(line.saturating_sub(1) as usize)
            .map(|l| l.trim().to_string())
            .unwrap_or_default()
    }

    fn run(mut self) -> Result<(LoadImage, HashMap<String, u16>, Vec<ListingRow>), AsmError> {
        self.resolve_defines()?;
        self.size_variables()?;

        let (
            linked_text_bytes,
            linked_text_start,
            linked_text_end,
            linked_data_bytes,
            linked_data_size,
        ) = if !self.linked_containers.is_empty() {
            let mut text_bytes = Vec::new();
            let mut data_bytes = Vec::new();
            let mut start_text = DATA_BASE;
            for lib in &self.linked_containers {
                start_text = start_text.min(lib.header.text_addr);
                text_bytes.extend_from_slice(&lib.text_bytes);
                data_bytes.extend_from_slice(&lib.data_bytes);
            }
            let end_text = start_text + text_bytes.len() as u16;
            let data_len = data_bytes.len() as u16;
            (text_bytes, start_text, end_text, data_bytes, data_len)
        } else {
            (Vec::new(), DATA_BASE, DATA_BASE, Vec::new(), 0)
        };

        let main_data_size: u32 = self.data_vars.iter().map(|v| v.size as u32).sum();
        let main_bss_size: u32 = self.bss_vars.iter().map(|v| v.size as u32).sum();

        let mut symtab: HashMap<String, u16> = self.external_symbols.clone();
        let mut sym_spans: HashMap<String, Span> = HashMap::new();
        let mut export_names = Vec::new();

        let (
            main_text_base,
            main_text_size,
            total_text_start,
            total_text_size,
            total_data_start,
            total_data_size,
            total_bss_start,
            total_bss_size,
            entry,
        ) = if !self.linked_containers.is_empty() {
            let main_text_base = linked_text_end;
            let raw_entry = self.layout_text(main_text_base, &mut symtab, &mut sym_spans, &mut export_names)?;

            let mut taddr = main_text_base;
            let mut current_parent_label: Option<String> = None;
            for seg in &self.program.segments {
                if let Segment::Text(items) = seg {
                    for item in items {
                        match item {
                            TextItem::Label(name, _) | TextItem::GlobalLabel(name, _) => {
                                current_parent_label = Some(name.clone());
                            }
                            TextItem::Instr(ins) => {
                                let zero_ops =
                                    self.length_operands(ins, current_parent_label.as_deref())?;
                                let len = encode(&ins.mnemonic, ins.span, &zero_ops)?.len() as u16;
                                taddr = taddr.wrapping_add(len);
                            }
                            _ => {}
                        }
                    }
                }
            }
            let main_text_size = taddr.saturating_sub(main_text_base);
            let total_text_size = linked_text_bytes.len() as u16 + main_text_size;

            let data_base = main_text_base.wrapping_add(main_text_size);
            let mut addr = data_base.wrapping_add(linked_data_size);
            for v in &self.data_vars {
                insert_symbol(&mut symtab, &mut sym_spans, &v.name, addr, v.span)?;
                addr += v.size;
            }
            let total_data_size = linked_data_size + main_data_size as u16;

            let bss_base = addr;
            for v in &self.bss_vars {
                insert_symbol(&mut symtab, &mut sym_spans, &v.name, addr, v.span)?;
                addr += v.size;
            }
            let total_bss_size = main_bss_size as u16;

            let has_main = symtab.keys().any(|k| k.eq_ignore_ascii_case("main"));
            let entry = if has_main { raw_entry } else { 0 };

            (
                main_text_base,
                main_text_size,
                linked_text_start,
                total_text_size,
                data_base,
                total_data_size,
                bss_base,
                total_bss_size,
                entry,
            )
        } else {
            let mut data_base = DATA_BASE as u32;
            for &sym_addr in self.external_symbols.values() {
                if (sym_addr as u32) >= data_base {
                    data_base = (sym_addr as u32) + 0x40;
                }
            }
            let bss_base = data_base + main_data_size;
            let text_base = bss_base + main_bss_size;
            if text_base > 0xFFFF {
                return Err(AsmError::new(Span::default(), AsmErrorKind::ImageOverflow));
            }

            let mut addr = data_base as u16;
            for v in &self.data_vars {
                insert_symbol(&mut symtab, &mut sym_spans, &v.name, addr, v.span)?;
                addr += v.size;
            }
            let mut addr = bss_base as u16;
            for v in &self.bss_vars {
                insert_symbol(&mut symtab, &mut sym_spans, &v.name, addr, v.span)?;
                addr += v.size;
            }

            let raw_entry = self.layout_text(text_base as u16, &mut symtab, &mut sym_spans, &mut export_names)?;

            let mut taddr = text_base as u16;
            let mut current_parent_label: Option<String> = None;
            for seg in &self.program.segments {
                if let Segment::Text(items) = seg {
                    for item in items {
                        match item {
                            TextItem::Label(name, _) | TextItem::GlobalLabel(name, _) => {
                                current_parent_label = Some(name.clone());
                            }
                            TextItem::Instr(ins) => {
                                let zero_ops =
                                    self.length_operands(ins, current_parent_label.as_deref())?;
                                let len = encode(&ins.mnemonic, ins.span, &zero_ops)?.len() as u16;
                                taddr = taddr.wrapping_add(len);
                            }
                            _ => {}
                        }
                    }
                }
            }
            let text_size = taddr.saturating_sub(text_base as u16);

            let has_main = symtab.keys().any(|k| k.eq_ignore_ascii_case("main"));
            let entry = if has_main { raw_entry } else { 0 };

            (
                text_base as u16,
                text_size,
                text_base as u16,
                text_size,
                data_base as u16,
                main_data_size as u16,
                bss_base as u16,
                main_bss_size as u16,
                entry,
            )
        };

        let max_extent = (total_text_start as usize + total_text_size as usize)
            .max(total_data_start as usize + total_data_size as usize)
            .max(total_bss_start as usize + total_bss_size as usize)
            .max(DATA_BASE as usize);

        if max_extent > 0x10000 {
            return Err(AsmError::new(Span::default(), AsmErrorKind::ImageOverflow));
        }

        let mut bytes = vec![0u8; max_extent];
        let mut listing = Vec::new();

        if entry != 0 {
            let [lo, hi] = [(entry & 0xFF) as u8, (entry >> 8) as u8];
            bytes[0] = 0xC3;
            bytes[1] = lo;
            bytes[2] = hi;
            listing.push(ListingRow {
                addr: 0,
                bytes: vec![0xC3, lo, hi],
                source: "; JMP entry (bootstrap)".into(),
            });
        }

        // Populate vector table hooks for recognized ISR labels.
        for (names, vec_addr, desc) in VECTOR_HOOKS {
            for name in *names {
                let matched_sym = symtab
                    .keys()
                    .find(|k| k.eq_ignore_ascii_case(name))
                    .cloned();
                if let Some(sym) = matched_sym {
                    let target = symtab[&sym];
                    let idx = *vec_addr as usize;
                    let [tlo, thi] = [(target & 0xFF) as u8, (target >> 8) as u8];
                    bytes[idx] = 0xC3;
                    bytes[idx + 1] = tlo;
                    bytes[idx + 2] = thi;
                    listing.push(ListingRow {
                        addr: *vec_addr,
                        bytes: vec![0xC3, tlo, thi],
                        source: format!("{desc} -> {sym}"),
                    });
                    break;
                }
            }
        }

        // Copy linked text & data bytes if present
        if !linked_text_bytes.is_empty() {
            let t_start = linked_text_start as usize;
            let t_end = t_start + linked_text_bytes.len();
            bytes[t_start..t_end].copy_from_slice(&linked_text_bytes);
        }

        if !linked_data_bytes.is_empty() {
            let d_start = total_data_start as usize;
            let d_end = d_start + linked_data_bytes.len();
            bytes[d_start..d_end].copy_from_slice(&linked_data_bytes);
        }

        let mut cur_d_addr = (total_data_start + linked_data_size) as usize;
        for v in &self.data_vars {
            let mut vbytes = Vec::new();
            for e in v.plan.as_ref().unwrap() {
                match e {
                    Emit::Byte(b) => vbytes.push(*b),
                    Emit::Sym16(name, span) => {
                        let a = *symtab.get(name).ok_or_else(|| {
                            AsmError::new(*span, AsmErrorKind::UndefinedName(name.clone()))
                        })?;
                        vbytes.push((a & 0xFF) as u8);
                        vbytes.push((a >> 8) as u8);
                    }
                }
            }
            listing.push(ListingRow {
                addr: symtab[&v.name],
                bytes: vbytes.clone(),
                source: self.src_line(v.span.line),
            });
            bytes[cur_d_addr..cur_d_addr + vbytes.len()].copy_from_slice(&vbytes);
            cur_d_addr += vbytes.len();
        }

        for v in &self.bss_vars {
            listing.push(ListingRow {
                addr: symtab[&v.name],
                bytes: Vec::new(),
                source: self.src_line(v.span.line),
            });
        }

        let mut taddr = main_text_base;
        let mut current_parent_label: Option<String> = None;
        for seg in &self.program.segments {
            if let Segment::Text(items) = seg {
                for item in items {
                    match item {
                        TextItem::Label(name, span) | TextItem::GlobalLabel(name, span) => {
                            current_parent_label = Some(name.clone());
                            listing.push(ListingRow {
                                addr: taddr,
                                bytes: Vec::new(),
                                source: self.src_line(span.line),
                            });
                        }
                        TextItem::LocalLabel(name, span) => {
                            listing.push(ListingRow {
                                addr: taddr,
                                bytes: Vec::new(),
                                source: self.src_line(span.line),
                            });
                            let _ = name;
                        }
                        TextItem::GlobalDecl(_name, span) | TextItem::ExternDecl(_name, span) => {
                            listing.push(ListingRow {
                                addr: taddr,
                                bytes: Vec::new(),
                                source: self.src_line(span.line),
                            });
                        }
                        TextItem::Instr(ins) => {
                            let ops =
                                self.final_operands(ins, &symtab, current_parent_label.as_deref())?;
                            let b = encode(&ins.mnemonic, ins.span, &ops)?;
                            listing.push(ListingRow {
                                addr: taddr,
                                bytes: b.clone(),
                                source: self.src_line(ins.span.line),
                            });
                            let start = taddr as usize;
                            let end = start + b.len();
                            bytes[start..end].copy_from_slice(&b);
                            taddr = taddr.wrapping_add(b.len() as u16);
                        }
                    }
                }
            }
        }

        let _ = main_text_size;

        // Build exported symbol table (excluding 'main')
        let mut export_symbols = Vec::new();
        for name in export_names {
            if !name.eq_ignore_ascii_case("main") {
                if let Some(&sym_addr) = symtab.get(&name) {
                    if !export_symbols
                        .iter()
                        .any(|(n, _): &(String, u16)| n == &name)
                    {
                        export_symbols.push((name, sym_addr));
                    }
                }
            }
        }
        for lib in &self.linked_containers {
            for (sym, addr) in &lib.export_symbols {
                if !export_symbols.iter().any(|(n, _)| n == sym) {
                    export_symbols.push((sym.clone(), *addr));
                }
            }
        }

        Ok((
            LoadImage {
                bytes,
                entry,
                sp_init: 0xFFFF,
                text_addr: total_text_start,
                text_size: total_text_size,
                data_addr: total_data_start,
                data_size: total_data_size,
                bss_addr: total_bss_start,
                bss_size: total_bss_size,
                export_symbols,
            },
            symtab,
            listing,
        ))
    }

    // ── %define ────────────────────────────────────────────────────────────

    fn resolve_defines(&mut self) -> Result<(), AsmError> {
        for d in &self.program.defines {
            let v = self.define_value(&d.value, d.span)?;
            self.defines.insert(d.name.clone(), v);
        }
        Ok(())
    }

    /// Resolve a define's value to a literal (chaining through earlier defines).
    fn define_value(&self, v: &Value, span: Span) -> Result<DVal, AsmError> {
        match v {
            Value::Number(n) => Ok(DVal::Num(*n)),
            Value::Str(s) => Ok(DVal::Str(s.clone())),
            Value::Char(b) => Ok(DVal::Ch(*b)),
            Value::Ident(name) => self
                .defines
                .get(name)
                .cloned()
                .ok_or_else(|| AsmError::new(span, AsmErrorKind::UndefinedName(name.clone()))),
            Value::Len(_) | Value::Repeat { .. } => Err(AsmError::new(
                span,
                AsmErrorKind::NotANumber("a %define value".into()),
            )),
        }
    }

    // ── sizing ───────────────────────────────────────────────────────────

    fn size_variables(&mut self) -> Result<(), AsmError> {
        for seg in &self.program.segments {
            match seg {
                Segment::Data(defs) => {
                    for d in defs {
                        let mut plan = Vec::new();
                        for val in &d.values {
                            self.emit_value(val, d.size, d.span, &mut plan)?;
                        }
                        let size: u16 = plan.iter().map(|e| e.width()).sum();
                        self.sizes.insert(d.name.clone(), size);
                        self.data_vars.push(Var {
                            name: d.name.clone(),
                            span: d.span,
                            size,
                            plan: Some(plan),
                        });
                    }
                }
                Segment::Bss(decls) => {
                    for b in decls {
                        let count = self.eval_number(&b.count, b.span)?;
                        let unit = match b.size {
                            Size::Byte => 1,
                            Size::Word => 2,
                        };
                        let size = (count as u16)
                            .checked_mul(unit)
                            .ok_or_else(|| AsmError::new(b.span, AsmErrorKind::ImageOverflow))?;
                        self.sizes.insert(b.name.clone(), size);
                        self.bss_vars.push(Var {
                            name: b.name.clone(),
                            span: b.span,
                            size,
                            plan: None,
                        });
                    }
                }
                Segment::Text(_) => {}
            }
        }
        Ok(())
    }

    /// Append a data value's bytes (or symbol placeholder) to `out`.
    fn emit_value(
        &self,
        val: &Value,
        size: Size,
        span: Span,
        out: &mut Vec<Emit>,
    ) -> Result<(), AsmError> {
        match val {
            Value::Number(n) => push_scalar(*n, size, span, out),
            Value::Char(b) => push_scalar(*b as u32, size, span, out),
            Value::Str(s) => {
                for &b in s.as_bytes() {
                    out.push(Emit::Byte(b));
                }
                Ok(())
            }
            Value::Ident(name) => match self.defines.get(name) {
                Some(DVal::Num(n)) => push_scalar(*n, size, span, out),
                Some(DVal::Ch(b)) => push_scalar(*b as u32, size, span, out),
                Some(DVal::Str(s)) => {
                    for &b in s.as_bytes() {
                        out.push(Emit::Byte(b));
                    }
                    Ok(())
                }
                // Not a define -> a symbol's address; only meaningful as a WORD.
                None => match size {
                    Size::Word => {
                        out.push(Emit::Sym16(name.clone(), span));
                        Ok(())
                    }
                    Size::Byte => Err(AsmError::new(
                        span,
                        AsmErrorKind::NotANumber(format!("address of {name:?} in a BYTE")),
                    )),
                },
            },
            Value::Len(name) => {
                let n = self.len_of(name, span)?;
                push_scalar(n, size, span, out)
            }
            Value::Repeat { count, value } => {
                let n = self.eval_number(count, span)?;
                for _ in 0..n {
                    self.emit_value(value, size, span, out)?;
                }
                Ok(())
            }
        }
    }

    /// Evaluate a value that must be a plain number (repeat counts, `%len` args, etc.).
    fn eval_number(&self, val: &Value, span: Span) -> Result<u32, AsmError> {
        match val {
            Value::Number(n) => Ok(*n),
            Value::Char(b) => Ok(*b as u32),
            Value::Len(name) => self.len_of(name, span),
            Value::Ident(name) => match self.defines.get(name) {
                Some(DVal::Num(n)) => Ok(*n),
                Some(DVal::Ch(b)) => Ok(*b as u32),
                _ => Err(AsmError::new(
                    span,
                    AsmErrorKind::NotANumber(format!("{name:?}")),
                )),
            },
            Value::Str(_) | Value::Repeat { .. } => Err(AsmError::new(
                span,
                AsmErrorKind::NotANumber("this value".into()),
            )),
        }
    }

    /// `%len name`: the byte length of a variable (sized so far) or a string define.
    fn len_of(&self, name: &str, span: Span) -> Result<u32, AsmError> {
        if let Some(sz) = self.sizes.get(name) {
            return Ok(*sz as u32);
        }
        if let Some(DVal::Str(s)) = self.defines.get(name) {
            return Ok(s.len() as u32);
        }
        Err(AsmError::new(
            span,
            AsmErrorKind::UndefinedName(name.to_string()),
        ))
    }

    fn is_defined_in_program(&self, name: &str) -> bool {
        if self.data_vars.iter().any(|v| v.name == name)
            || self.bss_vars.iter().any(|v| v.name == name)
        {
            return true;
        }
        for seg in &self.program.segments {
            if let Segment::Text(items) = seg {
                for item in items {
                    match item {
                        TextItem::Label(l, _) | TextItem::GlobalLabel(l, _) => {
                            if l == name {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        false
    }

    // ── .text layout & operand resolution ───────────────────────────────────

    fn layout_text(
        &self,
        text_base: u16,
        symtab: &mut HashMap<String, u16>,
        sym_spans: &mut HashMap<String, Span>,
        export_syms: &mut Vec<String>,
    ) -> Result<u16, AsmError> {
        let mut count = 0usize;
        let mut addr = text_base;
        let mut current_parent_label: Option<String> = None;
        let mut has_text_segment = false;

        for seg in &self.program.segments {
            if let Segment::Text(items) = seg {
                has_text_segment = true;
                for item in items {
                    match item {
                        TextItem::Label(name, span) => {
                            current_parent_label = Some(name.clone());
                            insert_symbol(symtab, sym_spans, name, addr, *span)?;
                        }
                        TextItem::GlobalLabel(name, span) => {
                            current_parent_label = Some(name.clone());
                            insert_symbol(symtab, sym_spans, name, addr, *span)?;
                            if !export_syms.contains(name) {
                                export_syms.push(name.clone());
                            }
                        }
                        TextItem::LocalLabel(name, span) => {
                            let parent = current_parent_label.as_ref().ok_or_else(|| {
                                AsmError::new(
                                    *span,
                                    AsmErrorKind::LocalLabelWithoutParent(name.clone()),
                                )
                            })?;
                            let scoped_name = format!("{parent}.{name}");
                            insert_symbol(symtab, sym_spans, &scoped_name, addr, *span)?;
                        }
                        TextItem::GlobalDecl(name, _span) => {
                            if !export_syms.contains(name) {
                                export_syms.push(name.clone());
                            }
                        }
                        TextItem::ExternDecl(name, span) => {
                            if self.is_defined_in_program(name) {
                                return Err(AsmError::new(
                                    *span,
                                    AsmErrorKind::DuplicateName(format!(
                                        "symbol '{name}' cannot be both declared extern and defined"
                                    )),
                                ));
                            }
                        }
                        TextItem::Instr(ins) => {
                            // Length is value-independent, so encode with zero placeholders.
                            let zero_ops =
                                self.length_operands(ins, current_parent_label.as_deref())?;
                            let len = encode(&ins.mnemonic, ins.span, &zero_ops)?.len() as u16;
                            addr += len;
                            count += 1;
                        }
                    }
                }
            }
        }

        for ext in &self.program.externs {
            if self.is_defined_in_program(ext) {
                return Err(AsmError::new(
                    Span::default(),
                    AsmErrorKind::DuplicateName(format!(
                        "symbol '{ext}' cannot be both declared extern and defined"
                    )),
                ));
            }
        }

        for g in &self.program.globals {
            if !export_syms.contains(g) {
                export_syms.push(g.clone());
            }
        }

        if !has_text_segment {
            return Err(AsmError::new(Span::default(), AsmErrorKind::MissingTextSegment));
        }
        if count == 0 {
            return Err(AsmError::new(Span::default(), AsmErrorKind::EmptyText));
        }
        let entry = symtab.get("main").copied().unwrap_or(text_base);
        Ok(entry)
    }

    /// Operands for the length pass: registers as-is, everything numeric as `Imm(0)`.
    fn length_operands(
        &self,
        ins: &Instr,
        parent_label: Option<&str>,
    ) -> Result<Vec<Operand>, AsmError> {
        ins.operands
            .iter()
            .map(|p| self.operand(p, ins.span, None, parent_label, Some(&ins.mnemonic)))
            .collect()
    }

    /// Operands for final codegen: symbols and `%len` resolved to their values.
    fn final_operands(
        &self,
        ins: &Instr,
        symtab: &HashMap<String, u16>,
        parent_label: Option<&str>,
    ) -> Result<Vec<Operand>, AsmError> {
        ins.operands
            .iter()
            .map(|p| self.operand(p, ins.span, Some(symtab), parent_label, Some(&ins.mnemonic)))
            .collect()
    }

    /// Convert one parsed operand to an encoder operand. With `symtab == None` (length
    /// pass) unresolved numerics become `Imm(0)`; with `Some` they resolve fully.
    fn operand(
        &self,
        p: &POperand,
        span: Span,
        symtab: Option<&HashMap<String, u16>>,
        parent_label: Option<&str>,
        mnemonic: Option<&str>,
    ) -> Result<Operand, AsmError> {
        if let Some(m) = mnemonic {
            if is_branch_mnemonic(m) {
                match p {
                    POperand::Sym(name) => {
                        if self.data_vars.iter().any(|v| v.name == *name)
                            || self.bss_vars.iter().any(|v| v.name == *name)
                        {
                            return Err(AsmError::new(
                                span,
                                AsmErrorKind::BadOperand {
                                    mnemonic: m.to_string(),
                                    detail: format!(
                                        "expected code label or subroutine, found data variable '{name}'"
                                    ),
                                },
                            ));
                        }
                        if self.defines.contains_key(name) {
                            return Err(AsmError::new(
                                span,
                                AsmErrorKind::BadOperand {
                                    mnemonic: m.to_string(),
                                    detail: format!(
                                        "expected code label or subroutine, found defined constant '{name}'"
                                    ),
                                },
                            ));
                        }
                    }
                    POperand::Len(_) => {
                        return Err(AsmError::new(
                            span,
                            AsmErrorKind::BadOperand {
                                mnemonic: m.to_string(),
                                detail: "%len cannot be used as target of call or jump".into(),
                            },
                        ));
                    }
                    _ => {}
                }
            }
        }

        Ok(match p {
            POperand::Reg8(r) => Operand::Reg8(*r),
            POperand::Reg16(r) => Operand::Reg16(*r),
            POperand::Num(n) => Operand::Imm(*n),
            POperand::Char(b) => Operand::Imm(*b as u32),
            POperand::Len(name) => match symtab {
                None => Operand::Imm(0),
                Some(_) => Operand::Imm(self.len_of(name, span)?),
            },
            POperand::LocalSym(name) => match symtab {
                None => Operand::Imm(0),
                Some(t) => {
                    let parent = parent_label.ok_or_else(|| {
                        AsmError::new(span, AsmErrorKind::LocalLabelWithoutParent(name.clone()))
                    })?;
                    let scoped_name = format!("{parent}.{name}");
                    let addr = *t.get(&scoped_name).ok_or_else(|| {
                        AsmError::new(span, AsmErrorKind::UndefinedName(format!(".{name}")))
                    })?;
                    Operand::Imm(addr as u32)
                }
            },
            POperand::Sym(name) => {
                // A define reference resolves to its value; a real symbol or external symbol to its address.
                match self.defines.get(name) {
                    Some(DVal::Num(n)) => Operand::Imm(*n),
                    Some(DVal::Ch(b)) => Operand::Imm(*b as u32),
                    Some(DVal::Str(s)) => {
                        return Err(AsmError::new(span, AsmErrorKind::StringInText(s.clone())));
                    }
                    None => match symtab {
                        None => Operand::Imm(0),
                        Some(t) => {
                            if let Some(&addr) = t.get(name) {
                                Operand::Imm(addr as u32)
                            } else if let Some(&addr) = self.external_symbols.get(name) {
                                Operand::Imm(addr as u32)
                            } else if let Some(parent) = parent_label {
                                let scoped = format!("{parent}.{name}");
                                if let Some(&addr) = t.get(&scoped) {
                                    Operand::Imm(addr as u32)
                                } else {
                                    return Err(AsmError::new(
                                        span,
                                        AsmErrorKind::UndefinedName(name.clone()),
                                    ));
                                }
                            } else {
                                return Err(AsmError::new(
                                    span,
                                    AsmErrorKind::UndefinedName(name.clone()),
                                ));
                            }
                        }
                    },
                }
            }
        })
    }
}

fn is_branch_mnemonic(m: &str) -> bool {
    matches!(
        m.to_uppercase().as_str(),
        "CALL"
            | "CZ"
            | "CNZ"
            | "CC"
            | "CNC"
            | "CP"
            | "CM"
            | "CPE"
            | "CPO"
            | "JMP"
            | "JZ"
            | "JNZ"
            | "JC"
            | "JNC"
            | "JP"
            | "JM"
            | "JPE"
            | "JPO"
    )
}

fn insert_symbol(
    symtab: &mut HashMap<String, u16>,
    sym_spans: &mut HashMap<String, Span>,
    name: &str,
    addr: u16,
    span: Span,
) -> Result<(), AsmError> {
    if let Some(&first_defined) = sym_spans.get(name) {
        return Err(AsmError::new(
            span,
            AsmErrorKind::DuplicateDefinition {
                name: name.to_string(),
                first_defined,
            },
        ));
    }
    symtab.insert(name.to_string(), addr);
    sym_spans.insert(name.to_string(), span);
    Ok(())
}

/// Push a scalar as one byte (`BYTE`) or two little-endian bytes (`WORD`), range-checked.
fn push_scalar(n: u32, size: Size, span: Span, out: &mut Vec<Emit>) -> Result<(), AsmError> {
    match size {
        Size::Byte => {
            if n > 0xFF {
                return Err(AsmError::new(
                    span,
                    AsmErrorKind::ImmediateOutOfRange {
                        value: n,
                        max: 0xFF,
                    },
                ));
            }
            out.push(Emit::Byte(n as u8));
        }
        Size::Word => {
            if n > 0xFFFF {
                return Err(AsmError::new(
                    span,
                    AsmErrorKind::ImmediateOutOfRange {
                        value: n,
                        max: 0xFFFF,
                    },
                ));
            }
            out.push(Emit::Byte((n & 0xFF) as u8));
            out.push(Emit::Byte((n >> 8) as u8));
        }
    }
    Ok(())
}

// Silence unused-import warnings if the register enums are only used via Operand.
#[allow(unused_imports)]
use AReg8 as _AReg8;
#[allow(unused_imports)]
use AReg16 as _AReg16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_jumps_to_entry_and_layout_is_ordered() {
        // data (2 bytes) + bss (3 bytes) then text; main is the entry.
        let img = assemble(
            "segment .data\n\
             msg BYTE 0x41 0x42\n\
             segment .bss\n\
             buf BYTE 3\n\
             segment .text\n\
             main:\n\
             hlt\n",
        )
        .unwrap();
        // Bootstrap: JMP entry.
        assert_eq!(img.bytes[0], 0xC3);
        let entry = img.bytes[1] as u16 | ((img.bytes[2] as u16) << 8);
        assert_eq!(entry, img.entry);
        // data at 0x0040..0x0042 = 'A','B'; bss 0x0042..0x0045 = zeros; text at 0x0045.
        assert_eq!(&img.bytes[0x40..0x42], &[0x41, 0x42]);
        assert_eq!(&img.bytes[0x42..0x45], &[0, 0, 0]);
        assert_eq!(img.entry, 0x0045);
        assert_eq!(img.bytes[0x45], 0x76); // HLT
        assert_eq!(img.sp_init, 0xFFFF);
    }

    #[test]
    fn labels_resolve_forward_and_backward() {
        // jmp to a forward label, then a backward jump.
        let img = assemble(
            "segment .text\n\
             main:\n\
             jmp skip\n\
             nop\n\
             skip:\n\
             jmp main\n\
             hlt\n",
        )
        .unwrap();
        // text_base = 0x0040 (no data/bss). main=0x0040.
        // JMP skip at 0x0040 (3 bytes) -> skip is after nop: 0x0040+3 +1(nop) = 0x0044.
        assert_eq!(&img.bytes[0x40..0x43], &[0xC3, 0x44, 0x00]); // jmp skip
        assert_eq!(img.bytes[0x43], 0x00); // nop
        assert_eq!(&img.bytes[0x44..0x47], &[0xC3, 0x40, 0x00]); // jmp main
        assert_eq!(img.bytes[0x47], 0x76); // hlt
        assert_eq!(img.entry, 0x0040);
    }

    #[test]
    fn define_len_and_repeat_resolve() {
        // %len over a data string, %repeat filling bytes, %define numeric substitution.
        let img = assemble(
            "%define FILL 0xFF\n\
             segment .data\n\
             prompt BYTE \"Hi\" 0x0A\n\
             pad BYTE %repeat %len prompt FILL\n\
             segment .text\n\
             mvi A, %len prompt\n\
             hlt\n",
        )
        .unwrap();
        // prompt = 'H','i',0x0A (3 bytes) at 0x0040; pad = 3 x 0xFF at 0x0043; text at 0x0046.
        assert_eq!(&img.bytes[0x40..0x43], &[b'H', b'i', 0x0A]);
        assert_eq!(&img.bytes[0x43..0x46], &[0xFF, 0xFF, 0xFF]);
        // mvi A, %len prompt -> 0x3E, 3
        assert_eq!(&img.bytes[0x46..0x48], &[0x3E, 0x03]);
        assert_eq!(img.bytes[0x48], 0x76);
    }

    #[test]
    fn word_variable_holding_a_symbol_address() {
        let img = assemble(
            "segment .data\n\
             target BYTE 0x99\n\
             ptr WORD target\n\
             segment .text\n\
             lhld ptr\n\
             hlt\n",
        )
        .unwrap();
        // target at 0x0040 (1 byte); ptr at 0x0041 (2 bytes) = LE address of target = 0x0040.
        assert_eq!(img.bytes[0x40], 0x99);
        assert_eq!(&img.bytes[0x41..0x43], &[0x40, 0x00]);
        // lhld ptr at 0x0043 -> 0x2A, addr(ptr)=0x0041
        assert_eq!(&img.bytes[0x43..0x46], &[0x2A, 0x41, 0x00]);
    }

    #[test]
    fn resolution_errors() {
        assert!(matches!(
            assemble("segment .text\njmp nowhere\nhlt\n")
                .unwrap_err()
                .kind,
            AsmErrorKind::UndefinedName(_)
        ));
        assert!(matches!(
            assemble("%define G \"hi\"\nsegment .text\nmvi A, G\nhlt\n")
                .unwrap_err()
                .kind,
            AsmErrorKind::StringInText(_)
        ));
        assert!(matches!(
            assemble("segment .data\nx BYTE 1\nx BYTE 2\nsegment .text\nhlt\n")
                .unwrap_err()
                .kind,
            AsmErrorKind::DuplicateDefinition { .. } | AsmErrorKind::DuplicateName(_)
        ));
        assert!(matches!(
            assemble("segment .text\n").unwrap_err().kind,
            AsmErrorKind::EmptyText
        ));
        assert!(matches!(
            assemble("segment .data\nbig BYTE 0x100\nsegment .text\nhlt\n")
                .unwrap_err()
                .kind,
            AsmErrorKind::ImmediateOutOfRange { .. }
        ));
        assert!(matches!(
            assemble("segment .data\nvar BYTE 10\nsegment .text\nmain:\ncall var\nhlt\n")
                .unwrap_err()
                .kind,
            AsmErrorKind::BadOperand { .. }
        ));
        assert!(matches!(
            assemble("segment .text\nglobal main:\nhlt\n")
                .unwrap_err()
                .kind,
            AsmErrorKind::GlobalMainForbidden
        ));
        assert!(matches!(
            assemble("extern func\nsegment .text\nfunc:\nhlt\n")
                .unwrap_err()
                .kind,
            AsmErrorKind::DuplicateName(_)
        ));
    }

    #[test]
    fn listing_interleaves_addresses_bytes_and_source() {
        let (_img, rows) = assemble_listing(
            "segment .data\n\
             msg BYTE 0x41 0x42\n\
             segment .text\n\
             main:\n\
             hlt\n",
        )
        .unwrap();
        // bootstrap, data var, label, instruction.
        assert_eq!(rows[0].addr, 0x0000);
        assert_eq!(rows[0].bytes, vec![0xC3, 0x42, 0x00]); // JMP main (main at 0x0042)
        assert_eq!(rows[1].addr, 0x0040);
        assert_eq!(rows[1].bytes, vec![0x41, 0x42]);
        assert_eq!(rows[1].source, "msg BYTE 0x41 0x42");
        assert_eq!(rows[2].source, "main:");
        assert!(rows[2].bytes.is_empty()); // label row
        assert_eq!(rows[2].addr, 0x0042);
        assert_eq!(rows[3].addr, 0x0042);
        assert_eq!(rows[3].bytes, vec![0x76]); // HLT
        assert_eq!(rows[3].source, "hlt");
    }
}
