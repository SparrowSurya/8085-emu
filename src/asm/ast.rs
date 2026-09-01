//! The abstract syntax tree produced by the parser.
//!
//! Directive expressions (`%repeat`, `%len`) and symbol references are kept as unresolved
//! nodes here; a later preprocessing/resolution stage turns them into concrete bytes and
//! addresses.

use super::encode::{AReg16, AReg8};
use super::error::Span;

/// A whole program: leading `%include`s, `%define`s, `extern`/`global` declarations, and segments.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Top-level `%include "path"` directives.
    pub includes: Vec<Include>,
    /// Top-level constant definitions.
    pub defines: Vec<Define>,
    /// External symbols declared via `extern <name>`.
    pub externs: Vec<String>,
    /// Exported global symbols declared via `global <name>`.
    pub globals: Vec<String>,
    /// Segments in source order.
    pub segments: Vec<Segment>,
}

/// A `%include "path"` directive.
#[derive(Debug, Clone, PartialEq)]
pub struct Include {
    /// The file path to include.
    pub path: String,
    /// Source position.
    pub span: Span,
}

/// A `%define NAME VALUE` constant.
#[derive(Debug, Clone, PartialEq)]
pub struct Define {
    /// The identifier being defined.
    pub name: String,
    /// Its replacement value.
    pub value: Value,
    /// Source position of the directive.
    pub span: Span,
}

/// Unit size for data and reservations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// 1-byte units.
    Byte,
    /// 2-byte, little-endian units.
    Word,
}

/// A data/directive value. `Ident`, `Len`, and `Repeat` are unresolved until preprocessing.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A numeric literal.
    Number(u32),
    /// A string literal (a byte per character).
    Str(String),
    /// A single character literal (one byte).
    Char(u8),
    /// A reference to a `%define`d identifier.
    Ident(String),
    /// `%len IDENT` — the byte length of a variable or string define.
    Len(String),
    /// `%repeat COUNT VALUE` — `VALUE` repeated `COUNT` times.
    Repeat {
        /// How many repetitions (a value that must evaluate to a number).
        count: Box<Value>,
        /// The value to repeat.
        value: Box<Value>,
    },
}

/// A segment and its contents.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    /// `.data` — initialised variables.
    Data(Vec<DataDef>),
    /// `.bss` — zero-filled reservations.
    Bss(Vec<BssDecl>),
    /// `.text` — labels, declarations, and instructions.
    Text(Vec<TextItem>),
}

/// A `.data` variable: `NAME SIZE VALUES…`.
#[derive(Debug, Clone, PartialEq)]
pub struct DataDef {
    /// Variable name (a symbol pointing at its address).
    pub name: String,
    /// Unit size of each value.
    pub size: Size,
    /// One or more values.
    pub values: Vec<Value>,
    /// Source position.
    pub span: Span,
}

/// A `.bss` reservation: `NAME SIZE COUNT`.
#[derive(Debug, Clone, PartialEq)]
pub struct BssDecl {
    /// Variable name.
    pub name: String,
    /// Unit size.
    pub size: Size,
    /// Number of units to reserve (a value that must evaluate to a number).
    pub count: Value,
    /// Source position.
    pub span: Span,
}

/// An item inside `.text`: labels, symbol visibility declarations, or instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum TextItem {
    /// A label definition on its own line: `name:`
    Label(String, Span),
    /// An inline global/exported label: `global name:` or `export name:`
    GlobalLabel(String, Span),
    /// A local label scoped to parent label: `.name:`
    LocalLabel(String, Span),
    /// Standalone `global name` or `export name` declaration.
    GlobalDecl(String, Span),
    /// Standalone `extern name` declaration.
    ExternDecl(String, Span),
    /// An instruction.
    Instr(Instr),
}

/// A parsed instruction: mnemonic plus zero to two operands.
#[derive(Debug, Clone, PartialEq)]
pub struct Instr {
    /// The mnemonic, as written (case preserved; matched case-insensitively later).
    pub mnemonic: String,
    /// Its operands, left to right.
    pub operands: Vec<POperand>,
    /// Source position of the mnemonic.
    pub span: Span,
}

/// A parsed operand. `Sym`, `LocalSym`, and `Len` are unresolved numeric sources.
#[derive(Debug, Clone, PartialEq)]
pub enum POperand {
    /// An 8-bit register (or `M`).
    Reg8(AReg8),
    /// A 16-bit register pair.
    Reg16(AReg16),
    /// A numeric literal.
    Num(u32),
    /// A character literal used as an 8-bit immediate.
    Char(u8),
    /// A standard symbol reference: a label, variable, or `%define`.
    Sym(String),
    /// A local symbol reference: e.g. `.loop` (resolved against the enclosing parent label).
    LocalSym(String),
    /// `%len IDENT` in operand position.
    Len(String),
}
