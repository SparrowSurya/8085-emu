use std::sync::Arc;
use dashmap::DashMap;
use tower_lsp::lsp_types::{Position, Range, Url};

/// Represents an in-memory tracked document in the LSP workspace.
#[derive(Debug, Clone)]
pub struct Document {
    pub uri: Url,
    pub version: i32,
    pub text: String,
    line_offsets: Vec<usize>,
}

impl Document {
    pub fn new(uri: Url, version: i32, text: String) -> Self {
        let line_offsets = compute_line_offsets(&text);
        Self {
            uri,
            version,
            text,
            line_offsets,
        }
    }

    pub fn update(&mut self, version: i32, text: String) {
        self.version = version;
        self.line_offsets = compute_line_offsets(&text);
        self.text = text;
    }

    /// Converts an LSP 0-indexed Position (line, character) to a UTF-8 byte offset.
    pub fn position_to_offset(&self, position: &Position) -> Option<usize> {
        let line = position.line as usize;
        let char_offset = position.character as usize;

        if line >= self.line_offsets.len() {
            return None;
        }

        let line_start = self.line_offsets[line];
        let line_end = if line + 1 < self.line_offsets.len() {
            self.line_offsets[line + 1]
        } else {
            self.text.len()
        };

        let line_str = &self.text[line_start..line_end];
        let mut utf16_count = 0;
        let mut byte_idx = 0;

        for ch in line_str.chars() {
            if utf16_count >= char_offset {
                break;
            }
            utf16_count += ch.len_utf16();
            byte_idx += ch.len_utf8();
        }

        Some((line_start + byte_idx).min(self.text.len()))
    }

    /// Converts a UTF-8 byte offset to an LSP 0-indexed Position (line, character).
    pub fn offset_to_position(&self, offset: usize) -> Position {
        let clamped = offset.min(self.text.len());
        let line = match self.line_offsets.binary_search(&clamped) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        let line_start = self.line_offsets[line];
        let line_str = &self.text[line_start..clamped];
        let character = line_str.chars().map(|c| c.len_utf16()).sum::<usize>() as u32;

        Position {
            line: line as u32,
            character,
        }
    }

    /// Retrieves the word/token under the given LSP position along with its range.
    pub fn get_word_at_position(&self, position: &Position) -> Option<(String, Range)> {
        let offset = self.position_to_offset(position)?;
        let bytes = self.text.as_bytes();

        if offset > bytes.len() {
            return None;
        }

        // Find boundary of identifier (letters, digits, underscore, dot, percent)
        fn is_ident_char(b: u8) -> bool {
            b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'%'
        }

        let mut start = offset;
        while start > 0 && is_ident_char(bytes[start - 1]) {
            start -= 1;
        }

        let mut end = offset;
        while end < bytes.len() && is_ident_char(bytes[end]) {
            end += 1;
        }

        if start >= end {
            return None;
        }

        let word = self.text[start..end].to_string();
        let range = Range {
            start: self.offset_to_position(start),
            end: self.offset_to_position(end),
        };

        Some((word, range))
    }
}

fn compute_line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Thread-safe in-memory Virtual File System store for open documents.
#[derive(Debug, Default)]
pub struct DocumentStore {
    docs: DashMap<Url, Document>,
}

impl DocumentStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            docs: DashMap::new(),
        })
    }

    pub fn insert(&self, uri: Url, version: i32, text: String) {
        self.docs.insert(uri.clone(), Document::new(uri, version, text));
    }

    pub fn update(&self, uri: &Url, version: i32, text: String) {
        if let Some(mut doc) = self.docs.get_mut(uri) {
            doc.update(version, text);
        } else {
            self.docs.insert(uri.clone(), Document::new(uri.clone(), version, text));
        }
    }

    pub fn remove(&self, uri: &Url) {
        self.docs.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<Document> {
        self.docs.get(uri).map(|d| d.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_and_offset_roundtrip() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    mvi A, 0x05\n    hlt\n".to_string();
        let doc = Document::new(uri, 1, text);

        let pos = Position {
            line: 1,
            character: 4,
        };
        let offset = doc.position_to_offset(&pos).unwrap();
        assert_eq!(offset, 10); // 'm' in "mvi"

        let roundtrip_pos = doc.offset_to_position(offset);
        assert_eq!(roundtrip_pos, pos);
    }

    #[test]
    fn test_get_word_at_position() {
        let uri = Url::parse("file:///test.e8085").unwrap();
        let text = "main:\n    lxi HL, buffer\n    call .my_sub\n".to_string();
        let doc = Document::new(uri, 1, text);

        // Word 'lxi'
        let (word1, range1) = doc
            .get_word_at_position(&Position {
                line: 1,
                character: 5,
            })
            .unwrap();
        assert_eq!(word1, "lxi");
        assert_eq!(range1.start.character, 4);
        assert_eq!(range1.end.character, 7);

        // Word 'buffer'
        let (word2, _) = doc
            .get_word_at_position(&Position {
                line: 1,
                character: 13,
            })
            .unwrap();
        assert_eq!(word2, "buffer");

        // Local label '.my_sub'
        let (word3, _) = doc
            .get_word_at_position(&Position {
                line: 2,
                character: 11,
            })
            .unwrap();
        assert_eq!(word3, ".my_sub");
    }

    #[test]
    fn test_document_store_crud() {
        let store = DocumentStore::new();
        let uri = Url::parse("file:///workspace/demo.e8085").unwrap();

        store.insert(uri.clone(), 1, "nop".to_string());
        assert_eq!(store.get(&uri).unwrap().text, "nop");

        store.update(&uri, 2, "hlt".to_string());
        assert_eq!(store.get(&uri).unwrap().text, "hlt");
        assert_eq!(store.get(&uri).unwrap().version, 2);

        store.remove(&uri);
        assert!(store.get(&uri).is_none());
    }
}

