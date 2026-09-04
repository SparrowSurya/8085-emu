//! Source code formatter for 8085 assembly (`.e8085`).
//!
//! Enforces:
//! 1. Single space between tokens and operands.
//! 2. Grouping of continuous instructions / declarations.
//! 3. Trailing comment alignment per group at `group_width + 3 spaces`.
//! 4. Colon attached directly to label names (`main:`, `.loop:`).
//! 5. Canonical segment formatting: `segment .segment_name`.
//! 6. Local labels formatted as `.name:` at column 0.
//! 7. Standardized operand format (`hlt`, `add A`, `lxi HL, prompt`).
//! 8. `global` and `extern` inside `segment .text` indented 4 spaces with grouping maintained.
//! 9. Canonical separations:
//!    - Top-level doc comments followed by 2 empty lines.
//!    - Tabular view for `%define` directives within each group.
//!    - Segments surrounded by 1 empty line above and below.
//!    - Labels preceded by 2 empty lines (above label or its doc comment).
//!    - Groups in text segment separated by 1 empty line above docstring / group.

/// Formats `.e8085` assembly source code according to canonical styling rules.
pub fn format_source(src: &str) -> String {
    let raw_lines: Vec<&str> = src.lines().collect();
    if raw_lines.is_empty() {
        return String::new();
    }

    // Step 1: Split raw lines into code and comment parts
    let parsed_lines: Vec<RawLine> = raw_lines
        .into_iter()
        .map(|l| {
            let (code, comment) = split_code_and_comment(l);
            RawLine {
                code: code.trim().to_string(),
                comment: comment.map(|c| c.trim().to_string()),
            }
        })
        .collect();

    // Step 2: Extract top-level document comments
    let (top_doc_comments, remaining_lines) = extract_top_doc_comments(&parsed_lines);

    // Step 3: Parse remaining lines into a sequential document stream
    let items = parse_document_stream(&remaining_lines);

    // Step 4: Render formatted document
    render_document(&top_doc_comments, &items)
}

#[derive(Debug, Clone)]
struct RawLine {
    code: String,
    comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LineItem {
    pub indent: usize,
    pub code: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Group {
    pub is_define_group: bool,
    pub lines: Vec<LineItem>,
}

impl Group {
    pub fn new() -> Self {
        Self {
            is_define_group: false,
            lines: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn format(&self) -> Vec<String> {
        if self.lines.is_empty() {
            return Vec::new();
        }

        if self.is_define_group {
            return format_define_group(&self.lines);
        }

        // Calculate maximum code width across all code-bearing lines in this group
        let mut max_code_width = 0;
        let mut has_trailing_comment = false;

        for line in &self.lines {
            if !line.code.is_empty() {
                let code_len = line.indent + line.code.len();
                if code_len > max_code_width {
                    max_code_width = code_len;
                }
                if line.comment.is_some() {
                    has_trailing_comment = true;
                }
            }
        }

        let target_comment_col = if has_trailing_comment {
            max_code_width + 3
        } else {
            0
        };

        let mut output = Vec::new();
        for line in &self.lines {
            let indent_str = " ".repeat(line.indent);
            if line.code.is_empty() {
                // Standalone comment
                if let Some(ref comment) = line.comment {
                    output.push(format!("{indent_str}{comment}"));
                }
            } else if let Some(ref comment) = line.comment {
                let full_code = format!("{indent_str}{}", line.code);
                let spaces = " ".repeat(target_comment_col.saturating_sub(full_code.len()));
                output.push(format!("{full_code}{spaces}{comment}"));
            } else {
                output.push(format!("{indent_str}{}", line.code));
            }
        }

        output
    }
}

#[derive(Debug, Clone)]
enum DocItem {
    SegmentHeader {
        name: String,
        comment: Option<String>,
    },
    Label {
        is_global_keyword: bool,
        is_local: bool,
        doc_comments: Vec<String>,
        name: String,
        comment: Option<String>,
    },
    Group(Group),
}

fn extract_top_doc_comments(lines: &[RawLine]) -> (Vec<String>, Vec<RawLine>) {
    let mut top_doc = Vec::new();
    let mut idx = 0;

    // Scan leading comment lines
    while idx < lines.len() {
        let line = &lines[idx];
        if line.code.is_empty() && line.comment.is_some() {
            top_doc.push(line.comment.clone().unwrap());
            idx += 1;
        } else if line.code.is_empty() && line.comment.is_none() {
            // If we have collected top docs, check if more comments follow
            let has_more_comments = lines[idx + 1..]
                .iter()
                .take_while(|l| l.code.is_empty())
                .any(|l| l.comment.is_some());
            if !top_doc.is_empty() && has_more_comments {
                top_doc.push(String::new());
                idx += 1;
            } else if top_doc.is_empty() {
                idx += 1; // Skip initial empty lines
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Trim trailing empty strings
    while top_doc.last().map(|s| s.is_empty()).unwrap_or(false) {
        top_doc.pop();
    }

    // Skip any blank lines between top doc comments and first content
    while idx < lines.len() && lines[idx].code.is_empty() && lines[idx].comment.is_none() {
        idx += 1;
    }

    (top_doc, lines[idx..].to_vec())
}

fn parse_document_stream(lines: &[RawLine]) -> Vec<DocItem> {
    let mut items = Vec::new();
    let mut current_segment: Option<String> = None;
    let mut pending_comments: Vec<String> = Vec::new();
    let mut current_group = Group::new();

    let flush_group = |items: &mut Vec<DocItem>, grp: &mut Group| {
        if !grp.is_empty() {
            items.push(DocItem::Group(grp.clone()));
            *grp = Group::new();
        }
    };

    let flush_pending_into_group = |grp: &mut Group, pending: &mut Vec<String>, indent: usize| {
        for doc in pending.drain(..) {
            grp.lines.push(LineItem {
                indent,
                code: String::new(),
                comment: Some(doc),
            });
        }
    };

    for raw_line in lines {
        let trimmed_code = raw_line.code.trim();

        // 1. Blank line (empty code and no comment) -> marks end of current group
        if trimmed_code.is_empty() && raw_line.comment.is_none() {
            flush_group(&mut items, &mut current_group);
            continue;
        }

        // 2. Standalone comment line
        if trimmed_code.is_empty() {
            if let Some(ref cmt) = raw_line.comment {
                pending_comments.push(cmt.clone());
            }
            continue;
        }

        // 3. Segment header: segment .data, segment.text, segment .bss, etc.
        if let Some(seg_name) = parse_segment_name(trimmed_code) {
            flush_group(&mut items, &mut current_group);

            current_segment = Some(seg_name.clone());

            // If there were pending comments above the segment, flush them as a top group
            if !pending_comments.is_empty() {
                let mut cmt_grp = Group::new();
                flush_pending_into_group(&mut cmt_grp, &mut pending_comments, 0);
                items.push(DocItem::Group(cmt_grp));
            }

            items.push(DocItem::SegmentHeader {
                name: seg_name,
                comment: raw_line.comment.clone(),
            });
            continue;
        }

        // 4. Label declaration: main:, draw:, .loop1:, global print:, etc.
        if let Some((is_global, label_name, rest_code)) = parse_label_line(trimmed_code) {
            flush_group(&mut items, &mut current_group);

            let doc_comments = pending_comments.clone();
            pending_comments.clear();

            let is_local = label_name.starts_with('.');
            items.push(DocItem::Label {
                is_global_keyword: is_global,
                is_local,
                doc_comments,
                name: label_name,
                comment: if rest_code.is_none() {
                    raw_line.comment.clone()
                } else {
                    None
                },
            });

            if let Some(rest) = rest_code {
                let fmt_instr = format_instruction_code(&rest);
                current_group.lines.push(LineItem {
                    indent: 4,
                    code: fmt_instr,
                    comment: raw_line.comment.clone(),
                });
            }
            continue;
        }

        // 5. Code line (directive, declaration, data definition, bss declaration, instruction)
        let is_define = trimmed_code.starts_with("%define");

        // If transitioning into or out of %define, finish current group
        if current_group.is_define_group != is_define && !current_group.is_empty() {
            flush_group(&mut items, &mut current_group);
        }

        current_group.is_define_group = is_define;

        let indent = match current_segment.as_deref() {
            Some(".data") | Some(".bss") | Some(".text") => 4,
            None => {
                // If top level directive (%include, %define, %origin), indent 0
                if trimmed_code.starts_with('%') {
                    0
                } else {
                    4
                }
            }
            _ => 4,
        };

        // Flush any pending comments into the current group with appropriate indentation
        flush_pending_into_group(&mut current_group, &mut pending_comments, indent);

        let fmt_code = match current_segment.as_deref() {
            Some(".data") => format_data_def_code(trimmed_code),
            Some(".bss") => format_bss_decl_code(trimmed_code),
            Some(".text") => {
                if trimmed_code.starts_with("global") || trimmed_code.starts_with("extern") {
                    format_directive_code(trimmed_code)
                } else {
                    format_instruction_code(trimmed_code)
                }
            }
            None => {
                if trimmed_code.starts_with('%')
                    || trimmed_code.starts_with("global")
                    || trimmed_code.starts_with("extern")
                {
                    format_directive_code(trimmed_code)
                } else {
                    format_instruction_code(trimmed_code)
                }
            }
            _ => format_instruction_code(trimmed_code),
        };

        current_group.lines.push(LineItem {
            indent,
            code: fmt_code,
            comment: raw_line.comment.clone(),
        });
    }

    // Flush any trailing comments / active group
    if !pending_comments.is_empty() {
        let indent = if current_segment.is_some() { 4 } else { 0 };
        flush_pending_into_group(&mut current_group, &mut pending_comments, indent);
    }
    flush_group(&mut items, &mut current_group);

    items
}

fn render_document(top_doc: &[String], items: &[DocItem]) -> String {
    let mut out: Vec<String> = Vec::new();

    // 1. Top doc comments
    if !top_doc.is_empty() {
        for line in top_doc {
            out.push(line.clone());
        }
        ensure_blank_lines(&mut out, 2);
    }

    let mut last_was_segment_header = false;
    let mut last_was_label = false;
    let mut is_first_item_in_file = top_doc.is_empty();

    for item in items {
        match item {
            DocItem::SegmentHeader { name, comment } => {
                // 1 empty line above segment declaration (unless at start of file without top doc)
                if !is_first_item_in_file {
                    ensure_blank_lines(&mut out, 1);
                }

                let seg_hdr = if let Some(cmt) = comment {
                    format!("segment {name}   {cmt}")
                } else {
                    format!("segment {name}")
                };
                out.push(seg_hdr);

                // 1 empty line below segment declaration
                ensure_blank_lines(&mut out, 1);
                last_was_segment_header = true;
                last_was_label = false;
                is_first_item_in_file = false;
            }

            DocItem::Label {
                is_global_keyword,
                is_local,
                doc_comments,
                name,
                comment,
            } => {
                if last_was_segment_header {
                    // Already has 1 empty line below segment header
                } else if is_first_item_in_file {
                    // Top of file
                } else if *is_local {
                    // Local label: 1 empty line above
                    ensure_blank_lines(&mut out, 1);
                } else {
                    // Global label: 2 empty lines above label (or above doc comments)
                    ensure_blank_lines(&mut out, 2);
                }

                // Emit doc comments at column 0
                for doc in doc_comments {
                    out.push(doc.clone());
                }

                // Label header line
                let label_text = if *is_global_keyword {
                    format!("global {name}:")
                } else {
                    format!("{name}:")
                };

                let line_str = if let Some(cmt) = comment {
                    format!("{label_text}   {cmt}")
                } else {
                    label_text
                };
                out.push(line_str);

                last_was_segment_header = false;
                last_was_label = true;
                is_first_item_in_file = false;
            }

            DocItem::Group(group) => {
                if !last_was_segment_header && !last_was_label && !is_first_item_in_file {
                    ensure_blank_lines(&mut out, 1);
                }

                out.extend(group.format());
                last_was_segment_header = false;
                last_was_label = false;
                is_first_item_in_file = false;
            }
        }
    }

    // Clean up trailing empty lines and ensure single \n at end
    while out.last().map(|s| s.is_empty()).unwrap_or(false) {
        out.pop();
    }

    if out.is_empty() {
        String::new()
    } else {
        let mut result = out.join("\n");
        result.push('\n');
        result
    }
}

fn parse_segment_name(code: &str) -> Option<String> {
    let lower = code.trim().to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("segment") {
        if rest.is_empty() {
            return None;
        }
        let first_char = rest.chars().next().unwrap();
        if !first_char.is_whitespace() && first_char != '.' {
            return None;
        }
        let trimmed_rest = rest.trim();
        let name_part = trimmed_rest.split_whitespace().next().unwrap_or(trimmed_rest);
        if name_part.is_empty() {
            None
        } else if name_part.starts_with('.') {
            Some(name_part.to_string())
        } else {
            Some(format!(".{name_part}"))
        }
    } else {
        None
    }
}

fn parse_label_line(code: &str) -> Option<(bool, String, Option<String>)> {
    let trimmed = code.trim();
    let (is_global, rest) = if let Some(stripped) = trimmed.strip_prefix("global") {
        (true, stripped.trim())
    } else {
        (false, trimmed)
    };

    if let Some(colon_idx) = find_colon_outside_quotes(rest) {
        let label_name = rest[..colon_idx].trim();
        // Label name cannot contain whitespace
        if !label_name.is_empty() && !label_name.contains(char::is_whitespace) {
            let after_colon = rest[colon_idx + 1..].trim();
            let remainder = if after_colon.is_empty() {
                None
            } else {
                Some(after_colon.to_string())
            };
            return Some((is_global, label_name.to_string(), remainder));
        }
    }
    None
}

fn ensure_blank_lines(out: &mut Vec<String>, count: usize) {
    if out.is_empty() {
        return;
    }
    let mut trailing = 0;
    while trailing < out.len() && out[out.len() - 1 - trailing].is_empty() {
        trailing += 1;
    }
    while trailing < count {
        out.push(String::new());
        trailing += 1;
    }
}

/// Splits a line into `(code, Option<comment>)`, correctly ignoring `;` and `#` inside quotes.
pub fn split_code_and_comment(line: &str) -> (&str, Option<&str>) {
    let mut in_quote: Option<char> = None;
    let mut prev_char = '\0';
    for (i, c) in line.char_indices() {
        if let Some(q) = in_quote {
            if c == q && prev_char != '\\' {
                in_quote = None;
            }
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
        } else if c == ';' || c == '#' {
            let code = &line[..i];
            let comment = &line[i..];
            return (code, Some(comment));
        }
        prev_char = c;
    }
    (line, None)
}

fn find_colon_outside_quotes(s: &str) -> Option<usize> {
    let mut in_quote: Option<char> = None;
    let mut prev_char = '\0';
    for (i, c) in s.char_indices() {
        if let Some(q) = in_quote {
            if c == q && prev_char != '\\' {
                in_quote = None;
            }
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
        } else if c == ':' {
            return Some(i);
        }
        prev_char = c;
    }
    None
}

/// Formats a `%define` contiguous block with tabular alignment and aligned comments.
pub fn format_define_group(lines: &[LineItem]) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut parsed: Vec<(Option<(String, String)>, Option<String>)> = Vec::new();
    let mut max_name_len = 0;

    for line in lines {
        if line.code.is_empty() {
            parsed.push((None, line.comment.clone()));
        } else {
            let code = line.code.trim();
            if let Some(rest) = code.strip_prefix("%define") {
                let tokens = split_tokens_outside_quotes(rest);
                if tokens.len() >= 2 {
                    let name = tokens[0].clone();
                    let val = tokens[1..].join(" ");
                    if name.len() > max_name_len {
                        max_name_len = name.len();
                    }
                    parsed.push((Some((name, val)), line.comment.clone()));
                } else if tokens.len() == 1 {
                    let name = tokens[0].clone();
                    if name.len() > max_name_len {
                        max_name_len = name.len();
                    }
                    parsed.push((Some((name, String::new())), line.comment.clone()));
                } else {
                    parsed.push((None, line.comment.clone()));
                }
            } else {
                parsed.push((None, line.comment.clone()));
            }
        }
    }

    let mut formatted_group = Group::new();
    for (def_opt, comment) in parsed {
        if let Some((name, val)) = def_opt {
            let padded_name = format!("{:<width$}", name, width = max_name_len);
            let code = if val.is_empty() {
                format!("%define {padded_name}")
            } else {
                format!("%define {padded_name} {val}")
            };
            formatted_group.lines.push(LineItem {
                indent: 0,
                code,
                comment,
            });
        } else if let Some(comment) = comment {
            formatted_group.lines.push(LineItem {
                indent: 0,
                code: String::new(),
                comment: Some(comment),
            });
        }
    }

    formatted_group.format()
}

/// Formats preprocessor directives `%include`, `%origin`, `global`, `extern` with single space.
pub fn format_directive_code(code: &str) -> String {
    let trimmed = code.trim();
    if let Some(rest) = trimmed.strip_prefix("%include") {
        let tokens = split_tokens_outside_quotes(rest);
        format!("%include {}", tokens.join(" "))
    } else if let Some(rest) = trimmed.strip_prefix("%origin") {
        let tokens = split_tokens_outside_quotes(rest);
        format!("%origin {}", tokens.join(" "))
    } else if let Some(rest) = trimmed.strip_prefix("global") {
        let tokens = split_tokens_outside_quotes(rest);
        format!("global {}", tokens.join(" "))
    } else if let Some(rest) = trimmed.strip_prefix("extern") {
        let tokens = split_tokens_outside_quotes(rest);
        format!("extern {}", tokens.join(" "))
    } else {
        trimmed.to_string()
    }
}

/// Formats data segment definitions with single space, uppercase BYTE/WORD, %repeat/%len.
pub fn format_data_def_code(code: &str) -> String {
    let tokens = split_tokens_outside_quotes(code);
    if tokens.is_empty() {
        return code.to_string();
    }

    let var_name = &tokens[0];
    if tokens.len() == 1 {
        return var_name.to_string();
    }

    let second = tokens[1].to_ascii_uppercase();
    if second == "BYTE" || second == "WORD" {
        let size_str = second;
        let values_part = format_values_tokens(&tokens[2..]);
        if values_part.is_empty() {
            format!("{var_name} {size_str}")
        } else {
            format!("{var_name} {size_str} {values_part}")
        }
    } else {
        let values_part = format_values_tokens(&tokens[1..]);
        format!("{var_name} {values_part}")
    }
}

/// Formats BSS segment declarations with single space, uppercase BYTE/WORD.
pub fn format_bss_decl_code(code: &str) -> String {
    let tokens = split_tokens_outside_quotes(code);
    if tokens.is_empty() {
        return code.to_string();
    }

    let var_name = &tokens[0];
    if tokens.len() >= 3 {
        let size_str = tokens[1].to_ascii_uppercase();
        let count = tokens[2..].join(" ");
        format!("{var_name} {size_str} {count}")
    } else if tokens.len() == 2 {
        format!("{var_name} BYTE {}", tokens[1])
    } else {
        var_name.to_string()
    }
}

/// Formats an 8085 instruction: mnemonic, single space, uppercase registers, comma-space operands.
pub fn format_instruction_code(code: &str) -> String {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let mnemonic = parts.next().unwrap_or(trimmed).to_ascii_lowercase();
    let rest = parts.next().map(|s| s.trim()).unwrap_or("");

    if rest.is_empty() {
        return mnemonic;
    }

    let raw_ops = split_by_comma_outside_quotes(rest);
    let mut formatted_ops = Vec::new();

    for op in raw_ops {
        let trimmed_op = op.trim();
        let upper_op = trimmed_op.to_ascii_uppercase();

        if matches!(
            upper_op.as_str(),
            "A" | "B" | "C" | "D" | "E" | "H" | "L" | "M" | "BC" | "DE" | "HL" | "SP" | "PSW"
        ) {
            formatted_ops.push(upper_op);
        } else if let Some(rest_len) = trimmed_op
            .strip_prefix("%len")
            .or_else(|| trimmed_op.strip_prefix("%Len"))
            .or_else(|| trimmed_op.strip_prefix("%LEN"))
        {
            let var = rest_len.trim();
            formatted_ops.push(format!("%len {var}"));
        } else if let Some(rest_rep) = trimmed_op
            .strip_prefix("%repeat")
            .or_else(|| trimmed_op.strip_prefix("%Repeat"))
            .or_else(|| trimmed_op.strip_prefix("%REPEAT"))
        {
            let rep_tokens = split_tokens_outside_quotes(rest_rep.trim());
            if rep_tokens.len() >= 2 {
                formatted_ops.push(format!(
                    "%repeat {} {}",
                    rep_tokens[0],
                    rep_tokens[1..].join(" ")
                ));
            } else {
                formatted_ops.push(format!("%repeat {}", rest_rep.trim()));
            }
        } else {
            let tokens = split_tokens_outside_quotes(trimmed_op);
            formatted_ops.push(tokens.join(" "));
        }
    }

    let ops_str = formatted_ops.join(", ");
    format!("{mnemonic} {ops_str}")
}

fn format_values_tokens(tokens: &[String]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        let lower = tok.to_ascii_lowercase();
        if lower == "%repeat" && i + 2 < tokens.len() {
            out.push(format!("%repeat {} {}", tokens[i + 1], tokens[i + 2]));
            i += 3;
        } else if lower == "%len" && i + 1 < tokens.len() {
            out.push(format!("%len {}", tokens[i + 1]));
            i += 2;
        } else {
            out.push(tok.clone());
            i += 1;
        }
    }
    out.join(" ")
}

fn split_tokens_outside_quotes(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;
    let mut prev_char = '\0';

    for c in s.chars() {
        if let Some(q) = in_quote {
            current.push(c);
            if c == q && prev_char != '\\' {
                in_quote = None;
            }
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
            current.push(c);
        } else if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
        prev_char = c;
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn split_by_comma_outside_quotes(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut in_quote: Option<char> = None;
    let mut prev_char = '\0';
    let mut last_idx = 0;

    for (i, c) in s.char_indices() {
        if let Some(q) = in_quote {
            if c == q && prev_char != '\\' {
                in_quote = None;
            }
        } else if c == '"' || c == '\'' {
            in_quote = Some(c);
        } else if c == ',' {
            parts.push(&s[last_idx..i]);
            last_idx = i + 1;
        }
        prev_char = c;
    }

    if last_idx <= s.len() {
        parts.push(&s[last_idx..]);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_group_comment_alignment() {
        let input = r#"
main:
    ; Below is group 1
    mvi A, 2 ; Comment
    lxi HL, greeting ; comment
    call someroutine

    ; Below is group 2
    mov A, M ; comment
    add 3 ; comment

    ; Below is group 3
    hlt
"#;

        let formatted = format_source(input);
        let expected = r#"main:
    ; Below is group 1
    mvi A, 2           ; Comment
    lxi HL, greeting   ; comment
    call someroutine

    ; Below is group 2
    mov A, M   ; comment
    add 3      ; comment

    ; Below is group 3
    hlt
"#;

        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_tabular_define_directives() {
        let input = r#"
%define TERM_CMD_WRITE 0x00 ; Write mode
%define TERM_CMD_DISPLAY 0x01
%define TERM_CMD_READ 0x02 ; Read mode
%define TERM_DATA_PORT 0x01
%define TERM_CMD_PORT 0x02 ; Command port
"#;

        let formatted = format_source(input);
        let expected = r#"%define TERM_CMD_WRITE   0x00   ; Write mode
%define TERM_CMD_DISPLAY 0x01
%define TERM_CMD_READ    0x02   ; Read mode
%define TERM_DATA_PORT   0x01
%define TERM_CMD_PORT    0x02   ; Command port
"#;

        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_top_doc_comments_and_separations() {
        let input = r#"
; ==============================================================================
; Program: Demo
; ==============================================================================
%include "lib/math.e8085"

segment .data
msg BYTE "Hello"

segment .text
main:
    hlt
"#;

        let formatted = format_source(input);
        let expected = r#"; ==============================================================================
; Program: Demo
; ==============================================================================


%include "lib/math.e8085"

segment .data

    msg BYTE "Hello"

segment .text

main:
    hlt
"#;

        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_text_declarations_and_two_empty_lines_before_labels() {
        let input = r#"
segment .text
global main
extern print

main:
    call print
    hlt

; Draw function
draw:
    ret
"#;

        let formatted = format_source(input);
        let expected = r#"segment .text

    global main
    extern print


main:
    call print
    hlt


; Draw function
draw:
    ret
"#;

        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_labels_colon_attached_and_local_labels() {
        let input = r#"
segment .text
main :
    cpi 0
    jz .exit
.loop1 :
    mov B, A
.exit :
    hlt
"#;

        let formatted = format_source(input);
        let expected = r#"segment .text

main:
    cpi 0
    jz .exit

.loop1:
    mov B, A

.exit:
    hlt
"#;

        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_operands_formatting_and_single_spaces() {
        let input = r#"
segment .text
main:
    nop
    add   a
    inr   m
    push   bc
    pop   psw
    mvi   a  ,   2
    lxi   hl  ,   msg
    mvi   b  ,   %len   msg
    call   print
    cpi   0
    hlt
"#;

        let formatted = format_source(input);
        let expected = r#"segment .text

main:
    nop
    add A
    inr M
    push BC
    pop PSW
    mvi A, 2
    lxi HL, msg
    mvi B, %len msg
    call print
    cpi 0
    hlt
"#;

        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_data_and_bss_segment_formatting() {
        let input = r#"
segment .data
msg   BYTE   "Hello, World!"   0x0A ; Greeting text
pattern   byte   %repeat   4   0xAA ; 4 repeated bytes

segment .bss
buffer   BYTE   32 ; Data buffer
length   1
"#;

        let formatted = format_source(input);
        let expected = r#"segment .data

    msg BYTE "Hello, World!" 0x0A   ; Greeting text
    pattern BYTE %repeat 4 0xAA     ; 4 repeated bytes

segment .bss

    buffer BYTE 32   ; Data buffer
    length BYTE 1
"#;

        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_multiple_define_groups_preserved() {
        let input = r#"
%define CMD_A 1 ; Command A
%define LONG_CMD_B 2 ; Command B

%define REG_X 10
%define REG_YYY 20 ; Reg YYY
"#;

        let formatted = format_source(input);
        let expected = r#"%define CMD_A      1   ; Command A
%define LONG_CMD_B 2   ; Command B

%define REG_X   10
%define REG_YYY 20   ; Reg YYY
"#;

        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_segment_syntax_variations() {
        let input = r#"
segment.text
main:
    hlt

segment.data
msg BYTE "Hello"

segment.bss
buf BYTE 10
"#;

        let formatted = format_source(input);
        let expected = r#"segment .text

main:
    hlt

segment .data

    msg BYTE "Hello"

segment .bss

    buf BYTE 10
"#;

        assert_eq!(formatted, expected);
    }
}

