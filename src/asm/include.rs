//! Include directive preprocessor.
//!
//! Recursively resolves and merges `%include "path"` files with circular include protection.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::ast::{Program, Segment};
use super::error::{AsmError, AsmErrorKind};
use super::lexer::lex;
use super::parser::parse;

/// Recursively resolves all `%include` directives in `program`, starting relative to `base_dir`.
pub fn resolve_includes(base_dir: &Path, program: &Program) -> Result<Program, AsmError> {
    let mut visited = HashSet::new();
    resolve_program_includes(base_dir, program, &mut visited)
}

fn resolve_program_includes(
    base_dir: &Path,
    program: &Program,
    visited: &mut HashSet<PathBuf>,
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

        let inc_tokens = lex(&src)?;
        let inc_program = parse(inc_tokens)?;

        let inc_dir = inc_path.parent().unwrap_or(base_dir);
        let resolved_inc = resolve_program_includes(inc_dir, &inc_program, visited)?;

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
