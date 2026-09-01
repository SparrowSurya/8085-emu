//! Errors and source positions produced by the assembler pipeline.

use std::fmt;

/// A 1-based position in the source text, used to point at the offending token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number of the token's first character.
    pub col: u32,
}

impl Span {
    /// Construct a span at `line`:`col`.
    pub fn new(line: u32, col: u32) -> Self {
        Span { line, col }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Everything that can go wrong while lexing or encoding, tagged with a source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsmError {
    /// Where in the source the error was detected.
    pub span: Span,
    /// The category of error.
    pub kind: AsmErrorKind,
}

impl AsmError {
    /// Build an error at `span`.
    pub fn new(span: Span, kind: AsmErrorKind) -> Self {
        AsmError { span, kind }
    }
}

impl fmt::Display for AsmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.kind, self.span)
    }
}

impl std::error::Error for AsmError {}

/// The specific fault, independent of where it occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsmErrorKind {
    // ---- lexer ----
    /// A string literal was opened but never closed before end of line/input.
    UnterminatedString,
    /// A character literal had no character between the quotes.
    EmptyCharLiteral,
    /// A character literal contained more than one character.
    MultiCharLiteral,
    /// A numeric literal was malformed (bad digit or empty base).
    MalformedNumber(String),
    /// A character that cannot begin any token.
    UnexpectedChar(char),

    // ---- encoder ----
    /// The mnemonic is not a recognised instruction.
    UnknownMnemonic(String),
    /// The instruction was given the wrong number of operands.
    OperandCount {
        /// The mnemonic involved.
        mnemonic: String,
        /// How many operands the instruction takes.
        expected: usize,
        /// How many were supplied.
        found: usize,
    },
    /// An operand had the wrong shape for this instruction (e.g. a register where an
    /// immediate was required, or an unsupported register).
    BadOperand {
        /// The mnemonic involved.
        mnemonic: String,
        /// A short description of what was wrong.
        detail: String,
    },
    /// An immediate/address value did not fit the operand width.
    ImmediateOutOfRange {
        /// The offending value.
        value: u32,
        /// The maximum permitted value.
        max: u32,
    },

    // ---- parser ----
    /// A token appeared where a different one was expected.
    Unexpected {
        /// What the parser was looking for.
        expected: String,
        /// A short description of what it found instead.
        found: String,
    },
    /// A `%define` appeared after the first `segment` (they must precede all segments).
    DefineAfterSegment,
    /// A segment name other than `data`, `bss`, or `text`.
    UnknownSegment(String),
    /// A `%` directive other than `define`, `repeat`, or `len`.
    UnknownDirective(String),
    /// A reserved keyword was used where a label/variable/define name was required.
    ReservedName(String),
    /// More operands were given than any instruction accepts.
    TooManyOperands,

    // ---- resolution / layout / codegen ----
    /// A symbol, `%len` target, or `%define` reference that was never defined.
    UndefinedName(String),
    /// Two labels or variables share a name.
    DuplicateName(String),
    /// A string-valued `%define` was used in a `.text` operand.
    StringInText(String),
    /// A value that must be a number (a `%repeat` count, `%len` argument, or a byte/word
    /// literal) was not one.
    NotANumber(String),
    /// A `.text` segment with no instructions (nothing for the entry point to run).
    EmptyText,
    /// The assembled image would not fit in the 64 KiB address space.
    ImageOverflow,
    /// A local label (e.g. `.loop:`) appeared before any parent label.
    LocalLabelWithoutParent(String),
    /// An `extern` symbol was referenced but not provided by any included file or linked library.
    UnresolvedSymbol(String),
    /// Circular `%include` detected.
    CircularInclude(String),
    /// Failed to read or process an `%include` file.
    IncludeError(String),
    /// Attempted to access a private symbol from another module.
    PrivateSymbolAccess {
        /// The referenced symbol.
        symbol: String,
        /// The module defining it.
        module: String,
    },
}

impl fmt::Display for AsmErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AsmErrorKind::*;
        match self {
            UnterminatedString => write!(f, "unterminated string literal"),
            EmptyCharLiteral => write!(f, "empty character literal"),
            MultiCharLiteral => write!(f, "character literal must be exactly one character"),
            MalformedNumber(s) => write!(f, "malformed number literal: {s:?}"),
            UnexpectedChar(c) => write!(f, "unexpected character {c:?}"),
            UnknownMnemonic(m) => write!(f, "unknown mnemonic {m:?}"),
            OperandCount {
                mnemonic,
                expected,
                found,
            } => write!(
                f,
                "{mnemonic} takes {expected} operand(s) but {found} were given"
            ),
            BadOperand { mnemonic, detail } => {
                write!(f, "invalid operand for {mnemonic}: {detail}")
            }
            ImmediateOutOfRange { value, max } => {
                write!(f, "value {value:#X} out of range (max {max:#X})")
            }
            Unexpected { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            DefineAfterSegment => write!(f, "%define must appear before any segment"),
            UnknownSegment(s) => write!(f, "unknown segment {s:?} (expected data, bss, or text)"),
            UnknownDirective(s) => write!(f, "unknown directive %{s}"),
            ReservedName(s) => write!(f, "{s:?} is a reserved keyword and cannot be a name"),
            TooManyOperands => write!(f, "too many operands (instructions take at most two)"),
            UndefinedName(s) => write!(f, "undefined name {s:?}"),
            DuplicateName(s) => write!(f, "duplicate definition of {s:?}"),
            StringInText(s) => write!(f, "string constant {s:?} cannot be used in .text"),
            NotANumber(s) => write!(f, "{s} must evaluate to a number"),
            EmptyText => write!(f, ".text has no instructions"),
            ImageOverflow => write!(f, "program does not fit in 64 KiB"),
            LocalLabelWithoutParent(s) => write!(f, "local label .{s} has no preceding parent label"),
            UnresolvedSymbol(s) => write!(f, "unresolved external symbol {s:?}"),
            CircularInclude(s) => write!(f, "circular include detected for {s}"),
            IncludeError(s) => write!(f, "include error: {s}"),
            PrivateSymbolAccess { symbol, module } => {
                write!(f, "symbol {symbol:?} is private to module {module:?}")
            }
        }
    }
}
