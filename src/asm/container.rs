//! Binary container format for .8085.bin files.
//!
//! Provides a structured, self-describing container for 8085 machine code images,
//! including magic identifier, entry point, section boundaries (.text, .data, .bss),
//! vector table metadata, and exported symbol tables for library linking.

pub const CONTAINER_MAGIC: [u8; 4] = *b"8085";
pub const CONTAINER_VERSION: u8 = 1;
pub const HEADER_SIZE: usize = 32;

/// Flag bit indicating vector table is present in container payload.
pub const FLAG_HAS_VEC_TABLE: u8 = 0x01;
/// Flag bit indicating export symbol table is present in container payload.
pub const FLAG_HAS_EXPORT_SYMS: u8 = 0x02;

/// 32-byte header for .8085.bin files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerHeader {
    pub magic: [u8; 4],
    pub version: u8,
    pub flags: u8,
    pub entry_pc: u16,
    pub sp_init: u16,
    pub text_addr: u16,
    pub text_size: u16,
    pub data_addr: u16,
    pub data_size: u16,
    pub bss_addr: u16,
    pub bss_size: u16,
    pub vec_size: u16,
    pub sym_size: u16,
    pub reserved: [u8; 6],
}

impl ContainerHeader {
    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4] = self.version;
        buf[5] = self.flags;
        buf[6..8].copy_from_slice(&self.entry_pc.to_le_bytes());
        buf[8..10].copy_from_slice(&self.sp_init.to_le_bytes());
        buf[10..12].copy_from_slice(&self.text_addr.to_le_bytes());
        buf[12..14].copy_from_slice(&self.text_size.to_le_bytes());
        buf[14..16].copy_from_slice(&self.data_addr.to_le_bytes());
        buf[16..18].copy_from_slice(&self.data_size.to_le_bytes());
        buf[18..20].copy_from_slice(&self.bss_addr.to_le_bytes());
        buf[20..22].copy_from_slice(&self.bss_size.to_le_bytes());
        buf[22..24].copy_from_slice(&self.vec_size.to_le_bytes());
        buf[24..26].copy_from_slice(&self.sym_size.to_le_bytes());
        buf[26..32].copy_from_slice(&self.reserved);
        buf
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < HEADER_SIZE {
            return Err("file is smaller than 32-byte header".into());
        }
        if &bytes[0..4] != &CONTAINER_MAGIC {
            return Err("invalid magic identifier (expected '8085')".into());
        }
        let version = bytes[4];
        if version != CONTAINER_VERSION {
            return Err(format!("unsupported container version: {version}"));
        }
        let flags = bytes[5];
        let entry_pc = u16::from_le_bytes([bytes[6], bytes[7]]);
        let sp_init = u16::from_le_bytes([bytes[8], bytes[9]]);
        let text_addr = u16::from_le_bytes([bytes[10], bytes[11]]);
        let text_size = u16::from_le_bytes([bytes[12], bytes[13]]);
        let data_addr = u16::from_le_bytes([bytes[14], bytes[15]]);
        let data_size = u16::from_le_bytes([bytes[16], bytes[17]]);
        let bss_addr = u16::from_le_bytes([bytes[18], bytes[19]]);
        let bss_size = u16::from_le_bytes([bytes[20], bytes[21]]);
        let vec_size = u16::from_le_bytes([bytes[22], bytes[23]]);
        let sym_size = u16::from_le_bytes([bytes[24], bytes[25]]);
        let mut reserved = [0u8; 6];
        reserved.copy_from_slice(&bytes[26..32]);

        Ok(ContainerHeader {
            magic: CONTAINER_MAGIC,
            version,
            flags,
            entry_pc,
            sp_init,
            text_addr,
            text_size,
            data_addr,
            data_size,
            bss_addr,
            bss_size,
            vec_size,
            sym_size,
            reserved,
        })
    }
}

/// A decoded 8085 binary container image with optional symbol table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryContainer {
    pub header: ContainerHeader,
    pub vec_bytes: Vec<u8>,
    pub data_bytes: Vec<u8>,
    pub text_bytes: Vec<u8>,
    pub export_symbols: Vec<(String, u16)>,
}

impl BinaryContainer {
    /// Look up the address of an exported symbol.
    pub fn lookup_symbol(&self, name: &str) -> Option<u16> {
        self.export_symbols
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, addr)| *addr)
    }

    /// Encode export symbols into raw bytes.
    fn encode_symbols(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, addr) in &self.export_symbols {
            let bytes = name.as_bytes();
            if bytes.len() <= 255 {
                out.push(bytes.len() as u8);
                out.extend_from_slice(bytes);
                out.extend_from_slice(&addr.to_le_bytes());
            }
        }
        out
    }

    /// Decode export symbols from raw bytes.
    fn decode_symbols(mut bytes: &[u8]) -> Result<Vec<(String, u16)>, String> {
        let mut syms = Vec::new();
        while !bytes.is_empty() {
            if bytes.len() < 3 {
                return Err("malformed symbol table payload".into());
            }
            let name_len = bytes[0] as usize;
            bytes = &bytes[1..];
            if bytes.len() < name_len + 2 {
                return Err("malformed symbol table payload".into());
            }
            let name_bytes = &bytes[..name_len];
            let name = String::from_utf8(name_bytes.to_vec())
                .map_err(|_| "symbol name is not valid UTF-8")?;
            bytes = &bytes[name_len..];
            let addr = u16::from_le_bytes([bytes[0], bytes[1]]);
            bytes = &bytes[2..];
            syms.push((name, addr));
        }
        Ok(syms)
    }

    pub fn encode(&self) -> Vec<u8> {
        let sym_bytes = self.encode_symbols();
        let total_size = HEADER_SIZE
            + self.vec_bytes.len()
            + self.data_bytes.len()
            + self.text_bytes.len()
            + sym_bytes.len();
        let mut out = Vec::with_capacity(total_size);

        let mut header = self.header;
        header.vec_size = self.vec_bytes.len() as u16;
        header.data_size = self.data_bytes.len() as u16;
        header.text_size = self.text_bytes.len() as u16;
        header.sym_size = sym_bytes.len() as u16;

        if !self.vec_bytes.is_empty() {
            header.flags |= FLAG_HAS_VEC_TABLE;
        }
        if !self.export_symbols.is_empty() {
            header.flags |= FLAG_HAS_EXPORT_SYMS;
        }

        out.extend_from_slice(&header.encode());
        out.extend_from_slice(&self.vec_bytes);
        out.extend_from_slice(&self.data_bytes);
        out.extend_from_slice(&self.text_bytes);
        out.extend_from_slice(&sym_bytes);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let header = ContainerHeader::decode(bytes)?;

        let expected_payload = header.vec_size as usize
            + header.data_size as usize
            + header.text_size as usize
            + header.sym_size as usize;
        let total_expected = HEADER_SIZE + expected_payload;

        if bytes.len() < total_expected {
            return Err(format!(
                "truncated binary container: expected {total_expected} bytes, got {}",
                bytes.len()
            ));
        }

        let vec_start = HEADER_SIZE;
        let vec_end = vec_start + header.vec_size as usize;
        let data_start = vec_end;
        let data_end = data_start + header.data_size as usize;
        let text_start = data_end;
        let text_end = text_start + header.text_size as usize;
        let sym_start = text_end;
        let sym_end = sym_start + header.sym_size as usize;

        let vec_bytes = bytes[vec_start..vec_end].to_vec();
        let data_bytes = bytes[data_start..data_end].to_vec();
        let text_bytes = bytes[text_start..text_end].to_vec();
        let export_symbols = if header.sym_size > 0 {
            Self::decode_symbols(&bytes[sym_start..sym_end])?
        } else {
            Vec::new()
        };

        Ok(BinaryContainer {
            header,
            vec_bytes,
            data_bytes,
            text_bytes,
            export_symbols,
        })
    }
}
