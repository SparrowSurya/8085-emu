//! Include directive preprocessor.
//!
//! Recursively resolves and merges `%include "path"` files with checks against:
//! 1. Self-inclusion (`%include` current file)
//! 2. Circular inclusion / recursion
//! 3. Duplicate inclusion (including the same file multiple times)
//! 4. Files residing outside the project root directory

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use super::ast::{Program, Segment, TextItem};
use super::error::{AsmError, AsmErrorKind};
use super::lexer::lex;


#[derive(Debug, Clone)]
pub struct ResolvedIncludes {
    pub program: Program,
    pub file_table: Vec<(PathBuf, String)>,
}

struct IncludeContext<'a> {
    project_root: PathBuf,
    /// Stack of canonical paths in the active include chain (for self-inclusion and recursion)
    active_stack: Vec<PathBuf>,
    /// Set of all canonical paths included in this program unit (for duplicate include prevention)
    included_files: HashSet<PathBuf>,
    /// Source file table
    file_table: &'a mut Vec<(PathBuf, String)>,
}

pub const EMBEDDED_STDLIB: &[(&str, &str)] = &[
    ("device/canvas.e8085", include_str!("../../stdlib/device/canvas.e8085")),
    ("device/text_io.e8085", include_str!("../../stdlib/device/text_io.e8085")),
    ("cmp.e8085", include_str!("../../stdlib/cmp.e8085")),
    ("conv.e8085", include_str!("../../stdlib/conv.e8085")),
    ("math.e8085", include_str!("../../stdlib/math.e8085")),
];

/// Finds the standard library root directory using a multi-tier search:
/// 1. Environment variable override (`EMU8085_STDLIB_DIR` or `EMU8085_STDLIB`)
/// 2. Project root discovery (ancestor walk from `start` looking for `stdlib/`)
/// 3. Executable-relative discovery (ancestor walk from `std::env::current_exe()` looking for `stdlib/`)
/// 4. System standard paths (`/usr/local/share/emu8085/stdlib`, etc.)
pub fn find_stdlib_dir(start: Option<&Path>) -> Option<PathBuf> {
    // 1. Environment variables
    if let Ok(val) = std::env::var("EMU8085_STDLIB_DIR").or_else(|_| std::env::var("EMU8085_STDLIB")) {
        let p = PathBuf::from(val);
        if p.is_dir() {
            return Some(canonicalize_or_norm(&p));
        }
    }

    // 2. Project root / ancestor walk from `start` or current_dir
    let cur_dir = std::env::current_dir().ok();
    let start_dir = start
        .map(|s| {
            if s.is_file() || s.extension().is_some() {
                s.parent().unwrap_or(s)
            } else {
                s
            }
        })
        .or(cur_dir.as_deref());

    if let Some(dir) = start_dir {
        let canon = canonicalize_or_norm(dir);
        for ancestor in canon.ancestors() {
            let candidate = ancestor.join("stdlib");
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }

    // 3. Executable-relative lookup (covers bin/../stdlib, target/debug/../../stdlib, etc.)
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_canon = canonicalize_or_norm(&exe_path);
        for ancestor in exe_canon.ancestors().skip(1) {
            let candidate = ancestor.join("stdlib");
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }

    // 4. System standard directories
    let sys_candidates = [
        "/usr/local/share/emu8085/stdlib",
        "/usr/share/emu8085/stdlib",
    ];
    for sys in &sys_candidates {
        let p = Path::new(sys);
        if p.is_dir() {
            return Some(p.to_path_buf());
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let user_candidate = PathBuf::from(home).join(".local/share/emu8085/stdlib");
        if user_candidate.is_dir() {
            return Some(user_candidate);
        }
    }

    None
}

/// Finds the project root by walking upward from `start` looking for `.git`, `Cargo.toml`, or `e8085.toml`.
pub fn find_project_root(start: &Path) -> PathBuf {
    let start_dir = if start.is_file() || start.extension().is_some() {
        start.parent().unwrap_or(start)
    } else {
        start
    };
    let start_canon = canonicalize_or_norm(start_dir);

    for ancestor in start_canon.ancestors() {
        if ancestor.join(".git").exists()
            || ancestor.join("Cargo.toml").exists()
            || ancestor.join("e8085.toml").exists()
        {
            return ancestor.to_path_buf();
        }
    }

    start_canon
}

/// Normalizes path components (`.` and `..`) without requiring filesystem access.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(Component::Normal(_)) = components.last() {
                    components.pop();
                } else {
                    components.push(comp);
                }
            }
            _ => components.push(comp),
        }
    }
    components.into_iter().collect()
}

/// Returns the canonical path if available, or canonicalizes the longest existing ancestor.
pub fn canonicalize_or_norm(path: &Path) -> PathBuf {
    if let Ok(canon) = path.canonicalize() {
        return canon;
    }
    let norm = normalize_path(path);
    if let Some(parent) = norm.parent() {
        if let Ok(parent_canon) = parent.canonicalize() {
            if let Some(file_name) = norm.file_name() {
                return parent_canon.join(file_name);
            }
        }
    }
    norm
}

/// Checks whether `path` resolves outside `project_root`.
pub fn is_outside_root(path: &Path, project_root: &Path) -> bool {
    let root_canon = canonicalize_or_norm(project_root);
    let path_canon = canonicalize_or_norm(path);
    !path_canon.starts_with(&root_canon)
}

/// Recursively resolves all `%include` directives in `program`, starting relative to `base_dir`.
pub fn resolve_includes(base_dir: &Path, program: &Program) -> Result<Program, AsmError> {
    resolve_includes_full(None, None, base_dir, None, program).map(|r| r.program)
}

/// Recursively resolves all `%include` directives with optional main file and project root.
pub fn resolve_includes_with_main_and_root(
    main_file: Option<&Path>,
    base_dir: &Path,
    project_root: Option<&Path>,
    program: &Program,
) -> Result<Program, AsmError> {
    resolve_includes_full(main_file, None, base_dir, project_root, program).map(|r| r.program)
}

/// Recursively resolves `%include` directives tracking full source file paths and sources.
pub fn resolve_includes_with_sources(
    main_file: &Path,
    main_src: &str,
    base_dir: &Path,
    program: &Program,
) -> Result<ResolvedIncludes, AsmError> {
    resolve_includes_full(Some(main_file), Some(main_src), base_dir, None, program)
}

/// Recursively resolves `%include` directives with full context: main file, main source, base dir, and project root.
pub fn resolve_includes_full(
    main_file: Option<&Path>,
    main_src: Option<&str>,
    base_dir: &Path,
    project_root: Option<&Path>,
    program: &Program,
) -> Result<ResolvedIncludes, AsmError> {
    let resolved_root = project_root
        .map(|r| r.to_path_buf())
        .unwrap_or_else(|| {
            if let Some(mf) = main_file {
                find_project_root(mf)
            } else {
                find_project_root(base_dir)
            }
        });

    let mut visited_included = HashSet::new();
    let mut active_stack = Vec::new();
    let mut file_table = Vec::new();

    if let Some(mf) = main_file {
        let canon_main = canonicalize_or_norm(mf);
        active_stack.push(canon_main.clone());
        visited_included.insert(canon_main.clone());
        file_table.push((canon_main, main_src.unwrap_or("").to_string()));
    } else {
        file_table.push((base_dir.to_path_buf(), main_src.unwrap_or("").to_string()));
    }

    let mut ctx = IncludeContext {
        project_root: resolved_root,
        active_stack,
        included_files: visited_included,
        file_table: &mut file_table,
    };

    let resolved_prog = resolve_program_includes(base_dir, program, &mut ctx)?;
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
    for m in &mut program.macros {
        m.span.file_id = file_id;
        for line in &mut m.body_lines {
            line.span.file_id = file_id;
            for tok in &mut line.tokens {
                tok.span.file_id = file_id;
            }
        }
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
                        | TextItem::ExternDecl(_, span)
                        | TextItem::MacroCall(_, _, span) => {
                            span.file_id = file_id;
                        }
                        TextItem::MacroDef(m) => {
                            m.span.file_id = file_id;
                            for line in &mut m.body_lines {
                                line.span.file_id = file_id;
                                for tok in &mut line.tokens {
                                    tok.span.file_id = file_id;
                                }
                            }
                        }
                        TextItem::Instr(ins) => {
                            ins.span.file_id = file_id;
                        }
                        TextItem::MacroEnd => {}
                    }
                }
            }
        }
    }
}

fn resolve_program_includes(
    base_dir: &Path,
    program: &Program,
    ctx: &mut IncludeContext,
) -> Result<Program, AsmError> {
    let mut merged_defines = Vec::new();
    let mut merged_macros = Vec::new();
    let mut merged_externs = program.externs.clone();
    let mut merged_globals = program.globals.clone();
    let mut merged_data = Vec::new();
    let mut merged_bss = Vec::new();
    let mut merged_text = Vec::new();

    // 1. Process all %include directives first
    for inc in &program.includes {
        let (canonical_path, src) = if inc.is_system {
            let mod_name = if inc.path.ends_with(".e8085") {
                inc.path.clone()
            } else {
                format!("{}.e8085", inc.path)
            };

            let stdlib_dir = find_stdlib_dir(Some(base_dir)).or_else(|| find_stdlib_dir(Some(&ctx.project_root)));

            let disk_found = if let Some(ref sdir) = stdlib_dir {
                let mut cur = sdir.clone();
                let mut escaped = false;
                for comp in Path::new(&mod_name).components() {
                    match comp {
                        std::path::Component::ParentDir => {
                            if cur == *sdir || !cur.starts_with(sdir) {
                                escaped = true;
                                break;
                            }
                            cur.pop();
                        }
                        std::path::Component::Normal(part) => {
                            cur.push(part);
                        }
                        _ => {}
                    }
                }
                if escaped || is_outside_root(&cur, sdir) {
                    return Err(AsmError::new(
                        inc.span,
                        AsmErrorKind::IncludeError(format!(
                            "cannot include '<{}>': path escapes standard library directory",
                            inc.path
                        )),
                    ));
                }
                if cur.exists() {
                    match cur.canonicalize() {
                        Ok(c) => {
                            if is_outside_root(&c, sdir) {
                                return Err(AsmError::new(
                                    inc.span,
                                    AsmErrorKind::IncludeError(format!(
                                        "cannot include '<{}>': path escapes standard library directory",
                                        inc.path
                                    )),
                                ));
                            }
                            std::fs::read_to_string(&c).ok().map(|s| (c, s))
                        }
                        Err(_) => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((canon, text)) = disk_found {
                (canon, text)
            } else {
                // Check embedded stdlib fallback
                let embedded_match = EMBEDDED_STDLIB.iter().find(|(path, _)| {
                    *path == mod_name || *path == inc.path
                });
                if let Some((embed_path, embed_src)) = embedded_match {
                    let canon = stdlib_dir
                        .unwrap_or_else(|| ctx.project_root.join("stdlib"))
                        .join(embed_path);
                    (canon, embed_src.to_string())
                } else {
                    return Err(AsmError::new(
                        inc.span,
                        AsmErrorKind::IncludeError(format!(
                            "cannot include '<{}>': standard library module not found",
                            inc.path
                        )),
                    ));
                }
            }
        } else {
            let clean_path = inc.path.trim_start_matches('/');

            // Verify resolution does not escape project root step-by-step
            let mut cur = ctx.project_root.clone();
            let mut escaped = false;
            for comp in Path::new(clean_path).components() {
                match comp {
                    std::path::Component::ParentDir => {
                        if cur == ctx.project_root || !cur.starts_with(&ctx.project_root) {
                            escaped = true;
                            break;
                        }
                        cur.pop();
                    }
                    std::path::Component::Normal(part) => {
                        cur.push(part);
                    }
                    _ => {}
                }
            }

            if escaped || is_outside_root(&cur, &ctx.project_root) {
                return Err(AsmError::new(
                    inc.span,
                    AsmErrorKind::IncludeError(format!(
                        "cannot include '{}': path is outside project root",
                        inc.path
                    )),
                ));
            }

            // Try project root path first: PWD_PATH / clean_path
            let inc_candidate = if cur.exists() {
                cur
            } else {
                // Secondary fallback for subfolder programs: base_dir / clean_path
                let sub_path = base_dir.join(clean_path);
                if !is_outside_root(&sub_path, &ctx.project_root) && sub_path.exists() {
                    sub_path
                } else {
                    cur
                }
            };

            // Check 2: Check if file exists and can be canonicalized
            let canonical_path = inc_candidate.canonicalize().map_err(|e| {
                AsmError::new(
                    inc.span,
                    AsmErrorKind::IncludeError(format!("cannot read '{}': {e}", inc.path)),
                )
            })?;

            // Re-verify canonical path against root
            if is_outside_root(&canonical_path, &ctx.project_root) {
                return Err(AsmError::new(
                    inc.span,
                    AsmErrorKind::IncludeError(format!(
                        "cannot include '{}': path is outside project root",
                        inc.path
                    )),
                ));
            }

            let text = std::fs::read_to_string(&canonical_path).map_err(|e| {
                AsmError::new(
                    inc.span,
                    AsmErrorKind::IncludeError(format!("cannot read '{}': {e}", inc.path)),
                )
            })?;

            (canonical_path, text)
        };

        let inc_display = if inc.is_system {
            format!("<{}>", inc.path)
        } else {
            format!("'{}'", inc.path)
        };

        // Check 3: Self-inclusion check (file includes itself)
        if let Some(current_file) = ctx.active_stack.last() {
            if &canonical_path == current_file {
                return Err(AsmError::new(
                    inc.span,
                    AsmErrorKind::IncludeError(format!(
                        "file cannot include itself: {inc_display}"
                    )),
                ));
            }
        }

        // Check 4: Circular include check (active in current call chain)
        if ctx.active_stack.contains(&canonical_path) {
            return Err(AsmError::new(
                inc.span,
                AsmErrorKind::IncludeError(format!(
                    "circular include detected: {inc_display}"
                )),
            ));
        }

        // Check 5: Duplicate include check (file included multiple times)
        if ctx.included_files.contains(&canonical_path) {
            return Err(AsmError::new(
                inc.span,
                AsmErrorKind::IncludeError(format!(
                    "duplicate include: file {inc_display} has already been included"
                )),
            ));
        }

        // Mark as included and push to active stack
        ctx.included_files.insert(canonical_path.clone());
        ctx.active_stack.push(canonical_path.clone());

        let file_id = ctx.file_table.len() as u32;
        ctx.file_table.push((canonical_path.clone(), src.clone()));

        let inc_dir = canonical_path.parent().unwrap_or(base_dir);
        let inc_tokens = lex(&src)?;
        let mut inc_program = super::parser::parse_with_base_dir(inc_tokens, Some(inc_dir))?;
        tag_program_spans(&mut inc_program, file_id);

        let resolved_inc = resolve_program_includes(inc_dir, &inc_program, ctx)?;

        // Pop from active stack
        ctx.active_stack.pop();

        // Merge defines
        for d in resolved_inc.defines {
            if !merged_defines
                .iter()
                .any(|existing: &super::ast::Define| existing.name == d.name)
            {
                merged_defines.push(d);
            }
        }

        // Merge macros
        merged_macros.extend(resolved_inc.macros);

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

    // 2b. Add current program's macros
    merged_macros.extend(program.macros.clone());

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
        macros: merged_macros,
        externs: merged_externs,
        globals: merged_globals,
        segments,
    })
}

#[cfg(test)]
mod tests {

use super::*;
use crate::asm::parse;

    #[test]
    fn test_self_include_rejected() {
        let temp_dir = std::env::temp_dir().join("emu8085_test_self_inc");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let main_file = temp_dir.join("main.e8085");
        std::fs::write(&main_file, "%include \"main.e8085\"\nsegment .text\nhlt\n").unwrap();

        let src = std::fs::read_to_string(&main_file).unwrap();
        let tokens = lex(&src).unwrap();
        let program = parse(tokens).unwrap();

        let res = resolve_includes_with_main_and_root(
            Some(&main_file),
            &temp_dir,
            Some(&temp_dir),
            &program,
        );
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(
            err.kind.to_string().contains("file cannot include itself"),
            "unexpected error: {}",
            err.kind
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_duplicate_include_rejected() {
        let temp_dir = std::env::temp_dir().join("emu8085_test_dup_inc");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let sub_file = temp_dir.join("sub.e8085");
        std::fs::write(&sub_file, "segment .text\nsub_fn:\n    ret\n").unwrap();

        let main_file = temp_dir.join("main.e8085");
        let src = "%include \"sub.e8085\"\n%include \"sub.e8085\"\nsegment .text\nmain:\n    call sub_fn\n    hlt\n";
        std::fs::write(&main_file, src).unwrap();

        let tokens = lex(src).unwrap();
        let program = parse(tokens).unwrap();

        let res = resolve_includes_with_main_and_root(
            Some(&main_file),
            &temp_dir,
            Some(&temp_dir),
            &program,
        );
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(
            err.kind.to_string().contains("duplicate include"),
            "unexpected error: {}",
            err.kind
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_outside_project_root_rejected() {
        let temp_root = std::env::temp_dir().join("emu8085_test_proj_root");
        let proj_dir = temp_root.join("proj");
        std::fs::create_dir_all(&proj_dir).unwrap();

        let main_file = proj_dir.join("main.e8085");
        let src = "%include \"../outside.e8085\"\nsegment .text\nmain:\n    hlt\n";
        std::fs::write(&main_file, src).unwrap();

        let tokens = lex(src).unwrap();
        let program = parse(tokens).unwrap();

        let res = resolve_includes_with_main_and_root(
            Some(&main_file),
            &proj_dir,
            Some(&proj_dir),
            &program,
        );
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(
            err.kind.to_string().contains("path is outside project root"),
            "unexpected error: {}",
            err.kind
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_circular_include_rejected() {
        let temp_dir = std::env::temp_dir().join("emu8085_test_circ_inc");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let a_file = temp_dir.join("a.e8085");
        let b_file = temp_dir.join("b.e8085");

        std::fs::write(&a_file, "%include \"b.e8085\"\nsegment .text\nfn_a:\n    ret\n").unwrap();
        std::fs::write(&b_file, "%include \"a.e8085\"\nsegment .text\nfn_b:\n    ret\n").unwrap();

        let src = std::fs::read_to_string(&a_file).unwrap();
        let tokens = lex(&src).unwrap();
        let program = parse(tokens).unwrap();

        let res = resolve_includes_with_main_and_root(
            Some(&a_file),
            &temp_dir,
            Some(&temp_dir),
            &program,
        );
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(
            err.kind.to_string().contains("circular include detected"),
            "unexpected error: {}",
            err.kind
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_include_relative_to_project_root_example1() {
        let temp_root = std::env::temp_dir().join("emu8085_test_example1");
        let lib_dir = temp_root.join("lib");
        std::fs::create_dir_all(&lib_dir).unwrap();

        let io_file = lib_dir.join("io.e8085");
        std::fs::write(&io_file, "segment .text\nglobal print_char:\n    ret\n").unwrap();

        let main_file = temp_root.join("main.e8085");

        // 1. Correct include: %include "lib/io.e8085"
        let good_src = "%include \"lib/io.e8085\"\nsegment .text\nmain:\n    call print_char\n    hlt\n";
        let tokens = lex(good_src).unwrap();
        let program = parse(tokens).unwrap();
        let res_good = resolve_includes_with_main_and_root(
            Some(&main_file),
            &temp_root,
            Some(&temp_root),
            &program,
        );
        assert!(res_good.is_ok(), "expected lib/io.e8085 to succeed: {:?}", res_good.err());

        // 2. Absolute-style include: %include "/lib/io.e8085"
        let abs_src = "%include \"/lib/io.e8085\"\nsegment .text\nmain:\n    call print_char\n    hlt\n";
        let tokens = lex(abs_src).unwrap();
        let program = parse(tokens).unwrap();
        let res_abs = resolve_includes_with_main_and_root(
            Some(&main_file),
            &temp_root,
            Some(&temp_root),
            &program,
        );
        assert!(res_abs.is_ok(), "expected /lib/io.e8085 to succeed: {:?}", res_abs.err());

        // 3. Incorrect include: %include "io.e8085" (not in root)
        let bad_src = "%include \"io.e8085\"\nsegment .text\nmain:\n    call print_char\n    hlt\n";
        let tokens = lex(bad_src).unwrap();
        let program = parse(tokens).unwrap();
        let res_bad = resolve_includes_with_main_and_root(
            Some(&main_file),
            &temp_root,
            Some(&temp_root),
            &program,
        );
        assert!(res_bad.is_err(), "expected io.e8085 to fail");

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_system_stdlib_resolution() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let src = "%include <math>\n%include <device/canvas>\nsegment .text\nmain:\n    call divide\n    call canvas_draw_rect\n    hlt\n";
        let tokens = lex(src).unwrap();
        let program = parse(tokens).unwrap();

        let resolved = resolve_includes(&manifest_dir, &program).expect("resolve system stdlib");
        assert!(resolved.externs.is_empty());
        assert!(!resolved.segments.is_empty());

        // Check file table contains stdlib/math.e8085 and stdlib/device/canvas.e8085
        let resolved_full = resolve_includes_full(
            Some(&manifest_dir.join("main.e8085")),
            Some(src),
            &manifest_dir,
            Some(&manifest_dir),
            &program,
        )
        .expect("resolve full system stdlib");

        assert!(resolved_full.file_table.iter().any(|(p, _)| p.ends_with("math.e8085")));
        assert!(resolved_full.file_table.iter().any(|(p, _)| p.ends_with("canvas.e8085")));
    }

    #[test]
    fn test_system_stdlib_embedded_fallback() {
        let temp_dir = std::env::temp_dir().join("emu8085_isolated_no_stdlib");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let src = "%include <math>\nsegment .text\nmain:\n    call divide\n    hlt\n";
        let tokens = lex(src).unwrap();
        let program = parse(tokens).unwrap();

        // Resolving in an isolated directory without a stdlib folder on disk uses embedded fallback
        let res = resolve_includes_with_main_and_root(
            Some(&temp_dir.join("main.e8085")),
            &temp_dir,
            Some(&temp_dir),
            &program,
        );
        assert!(res.is_ok(), "embedded fallback should resolve <math>: {:?}", res.err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_stdlib_dir_env_var() {
        let temp_dir = std::env::temp_dir().join("emu8085_custom_stdlib_env");
        std::fs::create_dir_all(&temp_dir).unwrap();

        unsafe {
            std::env::set_var("EMU8085_STDLIB_DIR", &temp_dir);
        }
        let found = find_stdlib_dir(None);
        unsafe {
            std::env::remove_var("EMU8085_STDLIB_DIR");
        }

        assert_eq!(found, Some(canonicalize_or_norm(&temp_dir)));
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
