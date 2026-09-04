//! Include directive preprocessor.
//!
//! Recursively resolves and merges `%include "path"` files with circular include protection.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::ast::{Program, Segment, TextItem};
use super::error::{AsmError, AsmErrorKind};
use super::lexer::lex;
use super::parser::parse;

#[derive(Debug, Clone)]
pub struct ResolvedIncludes {
    pub program: Program,
    pub file_table: Vec<(PathBuf, String)>,
}

/// Recursively resolves all `%include` directives in `program`, starting relative to `base_dir`.
pub fn resolve_includes(base_dir: &Path, program: &Program) -> Result<Program, AsmError> {
    let mut visited = HashSet::new();
    let mut file_table = vec![(base_dir.to_path_buf(), String::new())];
    resolve_program_includes(base_dir, program, &mut visited, &mut file_table)
}

/// Recursively resolves `%include` directives tracking full source file paths and sources.
pub fn resolve_includes_with_sources(
    main_file: &Path,
    main_src: &str,
    base_dir: &Path,
    program: &Program,
) -> Result<ResolvedIncludes, AsmError> {
    let mut visited = HashSet::new();
    let canon_main = main_file.canonicalize().unwrap_or_else(|_| main_file.to_path_buf());
    visited.insert(canon_main.clone());
    let mut file_table = vec![(canon_main, main_src.to_string())];
    let resolved_prog = resolve_program_includes(base_dir, program, &mut visited, &mut file_table)?;
    Ok(ResolvedIncludes {
        program: resolved_prog,
        file_table,
    })
}

fn tag_program_spans(program: &mut Program, file_id: u32) {
    for inc in &mut program.includes {
        inc.span.file_id = file_id;
    }
    for def in &mut program.defines {
        def.span.file_id = file_id;
    }
    for seg in &mut program.segments {
        match seg {
            Segment::Data(defs) => {
                for d in defs {
                    d.span.file_id = file_id;
                }
            }
            Segment::Bss(decls) => {
                for b in decls {
                    b.span.file_id = file_id;
                }
            }
            Segment::Text(items) => {
                for item in items {
                    match item {
                        TextItem::Label(_, span)
                        | TextItem::GlobalLabel(_, span)
                        | TextItem::LocalLabel(_, span)
                        | TextItem::GlobalDecl(_, span)
                        | TextItem::ExternDecl(_, span) => {
                            span.file_id = file_id;
                        }
                        TextItem::Instr(ins) => {
                            ins.span.file_id = file_id;
                        }
                    }
                }
            }
        }
    }
}

fn resolve_program_includes(
    base_dir: &Path,
    program: &Program,
    visited: &mut HashSet<PathBuf>,
    file_table: &mut Vec<(PathBuf, String)>,
) -> Result<Program, AsmError> {
    let mut merged_defines = Vec::new();
    let mut merged_externs = program.externs.clone();
    let mut merged_globals = program.globals.clone();
    let mut merged_data = Vec::new();
    let mut merged_bss = Vec::new();
    let mut merged_text = Vec::new();

    // 1. Process all %include directives first
    for inc in &program.includes {
        let inc_path = base_dir.join(&inc.path);
        let canonical_path = match inc_path.canonicalize() {
            Ok(p) => p,
            Err(_) => inc_path.clone(),
        };

        if visited.contains(&canonical_path) {
            // Already included (idempotent / circular include protection)
            continue;
        }
        visited.insert(canonical_path.clone());

        let src = std::fs::read_to_string(&inc_path).map_err(|e| {
            AsmError::new(
                inc.span,
                AsmErrorKind::IncludeError(format!("cannot read '{}': {e}", inc.path)),
            )
        })?;

        let file_id = file_table.len() as u32;
        file_table.push((canonical_path.clone(), src.clone()));

        let inc_tokens = lex(&src)?;
        let mut inc_program = parse(inc_tokens)?;
        tag_program_spans(&mut inc_program, file_id);

        let inc_dir = inc_path.parent().unwrap_or(base_dir);
        let resolved_inc = resolve_program_includes(inc_dir, &inc_program, visited, file_table)?;

        // Merge defines
        for d in resolved_inc.defines {
            if !merged_defines
                .iter()
                .any(|existing: &super::ast::Define| existing.name == d.name)
            {
                merged_defines.push(d);
            }
        }

        // Merge externs and globals
        merged_externs.extend(resolved_inc.externs);
        merged_globals.extend(resolved_inc.globals);

        // Merge segments
        for seg in resolved_inc.segments {
            match seg {
                Segment::Data(defs) => merged_data.extend(defs),
                Segment::Bss(decls) => merged_bss.extend(decls),
                Segment::Text(items) => merged_text.extend(items),
            }
        }
    }

    let mut had_data = false;
    let mut had_bss = false;
    let mut had_text = false;

    // 2. Add current program's defines
    for d in &program.defines {
        if !merged_defines
            .iter()
            .any(|existing| existing.name == d.name)
        {
            merged_defines.push(d.clone());
        }
    }

    // 3. Add current program's segments
    for seg in &program.segments {
        match seg {
            Segment::Data(defs) => {
                had_data = true;
                merged_data.extend(defs.clone());
            }
            Segment::Bss(decls) => {
                had_bss = true;
                merged_bss.extend(decls.clone());
            }
            Segment::Text(items) => {
                had_text = true;
                merged_text.extend(items.clone());
            }
        }
    }

    let mut segments = Vec::new();
    if had_data || !merged_data.is_empty() {
        segments.push(Segment::Data(merged_data));
    }
    if had_bss || !merged_bss.is_empty() {
        segments.push(Segment::Bss(merged_bss));
    }
    if had_text || !merged_text.is_empty() {
        segments.push(Segment::Text(merged_text));
    }

    Ok(Program {
        includes: Vec::new(), // All includes are resolved
        defines: merged_defines,
        externs: merged_externs,
        globals: merged_globals,
        segments,
    })
}
