use dashmap::DashMap;
use std::sync::Arc;
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

        // Check if cursor is on a character literal 'x' or '\n'
        if let Some((ch_str, ch_range)) = self.get_char_literal_at_offset(offset) {
            return Some((ch_str, ch_range));
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

    fn get_char_literal_at_offset(&self, offset: usize) -> Option<(String, Range)> {
        let bytes = self.text.as_bytes();
        if bytes.is_empty() {
            return None;
        }
        let pos = offset.min(bytes.len().saturating_sub(1));
        // Search left for '\'' on the same line
        let mut start = pos;
        while start > 0 && bytes[start] != b'\'' && bytes[start] != b'\n' && pos - start < 10 {
            start -= 1;
        }
        if bytes[start] != b'\'' {
            return None;
        }
        // Search right for closing '\'' on the same line
        let mut end = start + 1;
        while end < bytes.len() && bytes[end] != b'\'' && bytes[end] != b'\n' && end - start < 10 {
            if bytes[end] == b'\\' && end + 1 < bytes.len() {
                end += 2;
            } else {
                end += 1;
            }
        }
        if end < bytes.len() && bytes[end] == b'\'' {
            let slice = &self.text[start..=end];
            if slice.starts_with('\'') && slice.ends_with('\'') && slice.len() >= 3 {
                let range = Range {
                    start: self.offset_to_position(start),
                    end: self.offset_to_position(end + 1),
                };
                if offset >= start && offset <= end + 1 {
                    return Some((slice.to_string(), range));
                }
            }
        }
        None
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
        self.docs
            .insert(uri.clone(), Document::new(uri, version, text));
    }

    pub fn update(&self, uri: &Url, version: i32, text: String) {
        if let Some(mut doc) = self.docs.get_mut(uri) {
            doc.update(version, text);
        } else {
            self.docs
                .insert(uri.clone(), Document::new(uri.clone(), version, text));
        }
    }

    pub fn remove(&self, uri: &Url) {
        self.docs.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<Document> {
        self.docs.get(uri).map(|d| d.clone())
    }
}

/// Resolves a relative file path against a document URI or current working directory within project root.
pub fn resolve_relative_path(doc_uri: &Url, rel_path: &str) -> Option<std::path::PathBuf> {
    let clean_rel = rel_path.trim_start_matches('/');
    if let Ok(doc_path) = doc_uri.to_file_path() {
        let project_root = crate::asm::include::find_project_root(&doc_path);
        // 1. Try project root first
        let root_candidate = project_root.join(clean_rel);
        if !crate::asm::include::is_outside_root(&root_candidate, &project_root) && root_candidate.exists() {
            return Some(root_candidate);
        }
        // 2. Try file parent directory as secondary fallback
        if let Some(parent) = doc_path.parent() {
            let candidate = parent.join(clean_rel);
            if !crate::asm::include::is_outside_root(&candidate, &project_root) && candidate.exists() {
                return Some(candidate);
            }
        }
    }
    let candidate = std::path::PathBuf::from(clean_rel);
    if candidate.exists() {
        let project_root = crate::asm::include::find_project_root(&candidate);
        if !crate::asm::include::is_outside_root(&candidate, &project_root) {
            return Some(candidate);
        }
    }
    None
}

/// Resolves an include target (either standard library `<...>` or quoted `"..."`) against document URI.
pub fn resolve_include_target(doc_uri: &Url, raw_path: &str, is_system: bool) -> Option<std::path::PathBuf> {
    if is_system {
        let mod_name = if raw_path.ends_with(".e8085") {
            raw_path.to_string()
        } else {
            format!("{raw_path}.e8085")
        };
        let doc_path = doc_uri.to_file_path().ok();
        let stdlib_dir = crate::asm::include::find_stdlib_dir(doc_path.as_deref());
        if let Some(sdir) = stdlib_dir {
            let target = sdir.join(&mod_name);
            if target.exists() {
                return target.canonicalize().ok().or(Some(target));
            }
        }
        // Check embedded stdlib fallback
        if let Some((embed_path, _)) = crate::asm::include::EMBEDDED_STDLIB.iter().find(|(p, _)| *p == mod_name || *p == raw_path) {
            let fallback_dir = doc_path
                .as_ref()
                .map(|p| crate::asm::include::find_project_root(p).join("stdlib"))
                .unwrap_or_else(|| std::path::PathBuf::from("stdlib"));
            return Some(fallback_dir.join(embed_path));
        }
        None
    } else {
        resolve_relative_path(doc_uri, raw_path)
    }
}

/// Extracts the path and whether it is a system `<...>` include from a line of code.
pub fn extract_include_info(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("%include") {
        return None;
    }
    if let Some(start_angle) = line.find('<') {
        if let Some(end_angle_offset) = line[start_angle + 1..].find('>') {
            let path = &line[start_angle + 1..start_angle + 1 + end_angle_offset];
            return Some((path.trim().to_string(), true));
        }
    }
    for q in ['"', '\''] {
        if let Some(start_q) = line.find(q) {
            if let Some(end_q_offset) = line[start_q + 1..].find(q) {
                let path = &line[start_q + 1..start_q + 1 + end_q_offset];
                return Some((path.trim().to_string(), false));
            }
        }
    }
    // Check raw unquoted include
    let rest = trimmed.strip_prefix("%include").unwrap().trim();
    if !rest.is_empty() && !rest.starts_with(';') {
        let raw = rest
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        if !raw.is_empty() {
            return Some((raw.to_string(), false));
        }
    }
    None
}

/// Segment kind in 8085 assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentKind {
    Text,
    Data,
    Bss,
}

/// Represents an included document resolved from an `%include` directive.
#[derive(Debug, Clone)]
pub struct IncludedDocument {
    pub uri: Url,
    pub path: std::path::PathBuf,
    pub content: String,
    pub is_system: bool,
    pub spec: String,
}

/// Recursively traverses and collects all transitively included documents starting from `doc`.
/// Employs a visited set to avoid infinite cycles.
pub fn collect_all_included_documents(doc: &Document) -> Vec<IncludedDocument> {
    let mut results = Vec::new();
    let mut visited = std::collections::HashSet::new();
    if let Ok(doc_path) = doc.uri.to_file_path() {
        let key = doc_path.canonicalize().unwrap_or(doc_path);
        visited.insert(key);
    }
    collect_from_doc(doc, &mut visited, &mut results);
    results
}

fn collect_from_doc(
    doc: &Document,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
    results: &mut Vec<IncludedDocument>,
) {
    for line in doc.text.lines() {
        let code_line = line.split(';').next().unwrap_or("").trim();
        if let Some((rel_path, is_system)) = extract_include_info(code_line) {
            if let Some(target_path) = resolve_include_target(&doc.uri, &rel_path, is_system) {
                let key = target_path.canonicalize().unwrap_or_else(|_| target_path.clone());
                if visited.insert(key) {
                    let content_opt = std::fs::read_to_string(&target_path).ok().or_else(|| {
                        if is_system {
                            let mod_name = if rel_path.ends_with(".e8085") {
                                rel_path.clone()
                            } else {
                                format!("{rel_path}.e8085")
                            };
                            crate::asm::include::EMBEDDED_STDLIB
                                .iter()
                                .find(|(p, _)| *p == mod_name || *p == rel_path)
                                .map(|(_, s)| s.to_string())
                        } else {
                            None
                        }
                    });

                    if let Some(content) = content_opt {
                        let abs_path = if target_path.is_absolute() {
                            target_path.clone()
                        } else {
                            std::env::current_dir()
                                .unwrap_or_default()
                                .join(&target_path)
                        };
                        let inc_uri = Url::from_file_path(&abs_path)
                            .unwrap_or_else(|_| doc.uri.clone());
                        let inc_doc = Document::new(inc_uri.clone(), 1, content.clone());
                        results.push(IncludedDocument {
                            uri: inc_uri,
                            path: target_path,
                            content,
                            is_system,
                            spec: rel_path,
                        });
                        collect_from_doc(&inc_doc, visited, results);
                    }
                }
            }
        }
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

    #[test]
    fn test_extract_include_info_and_collect_all() {
        assert_eq!(
            extract_include_info("%include <device/canvas>"),
            Some(("device/canvas".to_string(), true))
        );
        assert_eq!(
            extract_include_info("%include \"lib/math.e8085\""),
            Some(("lib/math.e8085".to_string(), false))
        );
        assert_eq!(
            extract_include_info("%include 'lib/sys.e8085'"),
            Some(("lib/sys.e8085".to_string(), false))
        );
        assert_eq!(
            extract_include_info("%include stdlib/device/canvas.e8085"),
            Some(("stdlib/device/canvas.e8085".to_string(), false))
        );

        let root_dir = std::env::temp_dir().join("emu8085_test_include_collect");
        let _ = std::fs::remove_dir_all(&root_dir);
        let _ = std::fs::create_dir_all(&root_dir);

        let a_path = root_dir.join("a.e8085");
        let b_path = root_dir.join("b.e8085");
        let c_path = root_dir.join("c.e8085");

        std::fs::write(&a_path, "%include \"b.e8085\"\nmain:\n    hlt\n").unwrap();
        std::fs::write(&b_path, "%include \"c.e8085\"\nb_fn:\n    ret\n").unwrap();
        // c.e8085 includes a.e8085 to test cyclic resolution
        std::fs::write(&c_path, "%include \"a.e8085\"\n%define C_VAL 99\n").unwrap();

        let a_uri = Url::from_file_path(&a_path).unwrap();
        let a_doc = Document::new(a_uri, 1, std::fs::read_to_string(&a_path).unwrap());

        let docs = collect_all_included_documents(&a_doc);
        assert_eq!(docs.len(), 2, "Should collect b and c without infinite cycle");
        assert!(docs.iter().any(|d| d.content.contains("b_fn:")));
        assert!(docs.iter().any(|d| d.content.contains("%define C_VAL 99")));

        let _ = std::fs::remove_dir_all(&root_dir);
    }
}
