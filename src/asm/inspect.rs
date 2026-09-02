//! Binary inspection and diagnostic tools for .8085.bin containers.
//!
//! Provides detailed structural analysis, segment boundary maps, symbol tables,
//! header metadata, and ASCII string extraction.

use crate::asm::container::{
    BinaryContainer, ContainerHeader, FLAG_HAS_EXPORT_SYMS, FLAG_HAS_VEC_TABLE, HEADER_SIZE,
};

/// Options configuring which diagnostic sections to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectOptions {
    pub show_header: bool,
    pub show_segments: bool,
    pub show_symbols: bool,
    pub show_strings: bool,
    pub min_string_len: usize,
}

impl Default for InspectOptions {
    fn default() -> Self {
        Self {
            show_header: true,
            show_segments: true,
            show_symbols: true,
            show_strings: true,
            min_string_len: 3,
        }
    }
}

/// A segment boundary record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentRecord {
    pub name: &'static str,
    pub file_offset_start: usize,
    pub file_offset_end: usize,
    pub ram_addr: u16,
    pub size_bytes: u16,
    pub description: &'static str,
}

/// An extracted ASCII string from a container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedString {
    pub ram_addr: u16,
    pub file_offset: usize,
    pub segment: &'static str,
    pub content: String,
}

/// Compute segment records for a binary container.
pub fn get_segments(container: &BinaryContainer) -> Vec<SegmentRecord> {
    let mut segments = Vec::new();
    let mut cur_offset = 0;

    // 1. Header (32 bytes)
    segments.push(SegmentRecord {
        name: ".header",
        file_offset_start: cur_offset,
        file_offset_end: cur_offset + HEADER_SIZE,
        ram_addr: 0x0000,
        size_bytes: HEADER_SIZE as u16,
        description: "32-byte self-describing container header",
    });
    cur_offset += HEADER_SIZE;

    // 2. Vector Table (.vec)
    if container.header.vec_size > 0 {
        let size = container.header.vec_size as usize;
        segments.push(SegmentRecord {
            name: ".vec",
            file_offset_start: cur_offset,
            file_offset_end: cur_offset + size,
            ram_addr: 0x0000,
            size_bytes: container.header.vec_size,
            description: "64-byte Reset & Interrupt Vector Table",
        });
        cur_offset += size;
    }

    // 3. Data (.data)
    if container.header.data_size > 0 {
        let size = container.header.data_size as usize;
        segments.push(SegmentRecord {
            name: ".data",
            file_offset_start: cur_offset,
            file_offset_end: cur_offset + size,
            ram_addr: container.header.data_addr,
            size_bytes: container.header.data_size,
            description: "Initialized variables and string constants",
        });
        cur_offset += size;
    }

    // 4. Code (.text)
    if container.header.text_size > 0 {
        let size = container.header.text_size as usize;
        segments.push(SegmentRecord {
            name: ".text",
            file_offset_start: cur_offset,
            file_offset_end: cur_offset + size,
            ram_addr: container.header.text_addr,
            size_bytes: container.header.text_size,
            description: "Executable 8085 CPU machine code",
        });
        cur_offset += size;
    }

    // 5. BSS (.bss) - allocated in RAM, 0 bytes in file
    if container.header.bss_size > 0 {
        segments.push(SegmentRecord {
            name: ".bss",
            file_offset_start: cur_offset,
            file_offset_end: cur_offset,
            ram_addr: container.header.bss_addr,
            size_bytes: container.header.bss_size,
            description: "Zero-initialized RAM buffer reservations (unstored)",
        });
    }

    // 6. Export Symbol Table (.symtab)
    if container.header.sym_size > 0 {
        let size = container.header.sym_size as usize;
        segments.push(SegmentRecord {
            name: ".symtab",
            file_offset_start: cur_offset,
            file_offset_end: cur_offset + size,
            ram_addr: 0x0000,
            size_bytes: container.header.sym_size,
            description: "Exported global symbol table payload",
        });
    }

    segments
}

/// Extract printable ASCII strings from the container's data and text payloads.
pub fn extract_strings(container: &BinaryContainer, min_len: usize) -> Vec<ExtractedString> {
    let mut results = Vec::new();
    let min_len = min_len.max(1);

    // Scan .data segment
    let data_offset = HEADER_SIZE + container.header.vec_size as usize;
    scan_slice_strings(
        &container.data_bytes,
        container.header.data_addr,
        data_offset,
        ".data",
        min_len,
        &mut results,
    );

    // Scan .text segment
    let text_offset = data_offset + container.header.data_size as usize;
    scan_slice_strings(
        &container.text_bytes,
        container.header.text_addr,
        text_offset,
        ".text",
        min_len,
        &mut results,
    );

    results
}

fn scan_slice_strings(
    slice: &[u8],
    base_ram: u16,
    base_file_offset: usize,
    segment: &'static str,
    min_len: usize,
    out: &mut Vec<ExtractedString>,
) {
    let mut i = 0;
    while i < slice.len() {
        if is_printable(slice[i]) {
            let start = i;
            while i < slice.len() && is_printable(slice[i]) {
                i += 1;
            }
            let len = i - start;
            if len >= min_len {
                let text = String::from_utf8_lossy(&slice[start..i]).to_string();
                out.push(ExtractedString {
                    ram_addr: base_ram.wrapping_add(start as u16),
                    file_offset: base_file_offset + start,
                    segment,
                    content: text,
                });
            }
        } else {
            i += 1;
        }
    }
}

fn is_printable(b: u8) -> bool {
    (0x20..=0x7E).contains(&b)
}

/// Formats the container header into a human-readable table.
pub fn format_header(header: &ContainerHeader, file_size: usize) -> String {
    let mut out = String::new();
    out.push_str(
        "============================ [ CONTAINER HEADER ] ============================\n",
    );
    let magic_str = String::from_utf8_lossy(&header.magic);
    out.push_str(&format!(
        "  Magic Identifier     : {magic_str} (0x{:02X}{:02X}{:02X}{:02X})\n",
        header.magic[0], header.magic[1], header.magic[2], header.magic[3]
    ));
    out.push_str(&format!("  Format Version       : {}\n", header.version));

    let mut flag_names = Vec::new();
    if (header.flags & FLAG_HAS_VEC_TABLE) != 0 {
        flag_names.push("HAS_VEC_TABLE");
    }
    if (header.flags & FLAG_HAS_EXPORT_SYMS) != 0 {
        flag_names.push("HAS_EXPORT_SYMS");
    }
    let flags_desc = if flag_names.is_empty() {
        "0x00 (None)".to_string()
    } else {
        format!("0x{:02X} ({})", header.flags, flag_names.join(" | "))
    };
    out.push_str(&format!("  Flags                : {flags_desc}\n"));

    if header.entry_pc == 0 {
        out.push_str("  Entry Point (PC)     : 0x0000 (None - Pure Subroutine Library)\n");
    } else {
        out.push_str(&format!(
            "  Entry Point (PC)     : 0x{:04X} (<main>)\n",
            header.entry_pc
        ));
    }

    out.push_str(&format!(
        "  Initial Stack (SP)   : 0x{:04X}\n",
        header.sp_init
    ));
    out.push_str(&format!(
        "  Total File Size      : {} bytes (0x{:04X})\n",
        file_size, file_size
    ));
    out
}

/// Formats the segment table.
pub fn format_segments(segments: &[SegmentRecord]) -> String {
    let mut out = String::new();
    out.push_str(
        "============================ [ SEGMENT TABLE ] ===============================\n",
    );
    out.push_str("  Segment   File Offset         RAM Load Address   Size (Bytes)   Description\n");
    out.push_str("  --------  ------------------  -----------------  -------------  ------------------------------\n");
    for s in segments {
        let file_range = if s.file_offset_start == s.file_offset_end {
            format!("0x{:04X} (Unstored)", s.file_offset_start)
        } else {
            format!("0x{:04X}..0x{:04X}", s.file_offset_start, s.file_offset_end)
        };
        out.push_str(&format!(
            "  {:<8}  {:<18}  0x{:04X}             {:<5} ({:>3} B)    {}\n",
            s.name,
            file_range,
            s.ram_addr,
            format!("0x{:04X}", s.size_bytes),
            s.size_bytes,
            s.description
        ));
    }
    out
}

/// Formats the symbol table and entry point.
pub fn format_symbols(container: &BinaryContainer) -> String {
    let mut out = String::new();
    out.push_str(
        "============================ [ SYMBOL TABLE & ENTRY ] ========================\n",
    );
    if container.header.entry_pc == 0 {
        out.push_str("  Entry Point          : None (Pure Subroutine Library - no main label)\n");
    } else {
        out.push_str(&format!(
            "  Entry Point          : 0x{:04X} (<main>)\n",
            container.header.entry_pc
        ));
    }

    if container.export_symbols.is_empty() {
        out.push_str("  Exported Symbols     : (None - all symbols private)\n");
    } else {
        out.push_str(&format!(
            "  Exported Symbols ({}) :\n",
            container.export_symbols.len()
        ));
        out.push_str("    Symbol Name                   Address   Segment / Scope\n");
        out.push_str("    ----------------------------  --------  ----------------\n");
        for (sym, addr) in &container.export_symbols {
            let seg = if *addr >= container.header.text_addr
                && *addr < container.header.text_addr + container.header.text_size
            {
                ".text (code)"
            } else if *addr >= container.header.data_addr
                && *addr < container.header.data_addr + container.header.data_size
            {
                ".data (variable)"
            } else if *addr >= container.header.bss_addr
                && *addr < container.header.bss_addr + container.header.bss_size
            {
                ".bss (buffer)"
            } else {
                "external/other"
            };
            out.push_str(&format!("    {:<28}  0x{:04X}    {}\n", sym, addr, seg));
        }
    }
    out
}

/// Formats extracted strings.
pub fn format_strings(strings: &[ExtractedString]) -> String {
    let mut out = String::new();
    out.push_str(
        "============================ [ EMBEDDED STRINGS ] ============================\n",
    );
    if strings.is_empty() {
        out.push_str("  (No printable strings found matching minimum length)\n");
    } else {
        out.push_str("  RAM Addr  File Offset  Segment  String Content\n");
        out.push_str("  --------  -----------  -------  ------------------------------------\n");
        for s in strings {
            out.push_str(&format!(
                "  0x{:04X}    0x{:04X}       {:<7}  \"{}\"\n",
                s.ram_addr, s.file_offset, s.segment, s.content
            ));
        }
    }
    out
}

/// Perform inspection of a binary container with the given options.
pub fn inspect_container(
    container: &BinaryContainer,
    file_size: usize,
    options: &InspectOptions,
) -> String {
    let mut out = String::new();

    if options.show_header {
        out.push_str(&format_header(&container.header, file_size));
        out.push('\n');
    }

    if options.show_segments {
        let segments = get_segments(container);
        out.push_str(&format_segments(&segments));
        out.push('\n');
    }

    if options.show_symbols {
        out.push_str(&format_symbols(container));
        out.push('\n');
    }

    if options.show_strings {
        let strings = extract_strings(container, options.min_string_len);
        out.push_str(&format_strings(&strings));
        out.push('\n');
    }

    out
}
