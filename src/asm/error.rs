//! Errors and source positions produced by the assembler pipeline.

use std::fmt;
use std::path::{Path, PathBuf};

/// A 1-based position in the source text, used to point at the offending token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number of the token's first character.
    pub col: u32,
    /// 0-based file index in file registry (0 = main file, 1.. = included files).
    pub file_id: u32,
}

impl Span {
    /// Construct a span at `line`:`col`.
    pub fn new(line: u32, col: u32) -> Self {
        Span { line, col, file_id: 0 }
    }

    /// Construct a span with a specific `file_id`.
    pub fn with_file(line: u32, col: u32, file_id: u32) -> Self {
        Span { line, col, file_id }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// A single frame in a macro expansion traceback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroTraceFrame {
    /// Name of the macro being expanded.
    pub macro_name: String,
    /// Arguments passed to the macro at the call site.
    pub args: Vec<String>,
    /// Span of the macro invocation / call site.
    pub call_site: Span,
    /// Path of the file where the macro was called.
    pub call_file: Option<PathBuf>,
    /// Source text of the call site line.
    pub call_source: Option<String>,
    /// Span of the definition / instruction inside the macro body.
    pub def_site: Option<Span>,
    /// Path of the file where the macro is defined.
    pub def_file: Option<PathBuf>,
    /// Source text of the definition line.
    pub def_source: Option<String>,
}

/// Everything that can go wrong while lexing or encoding, tagged with a source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsmError {
    /// Where in the source the error was detected.
    pub span: Span,
    /// The category of error.
    pub kind: AsmErrorKind,
    /// Source file path where `span` is located.
    pub file_path: Option<PathBuf>,
    /// Macro expansion traceback stack, from outermost caller to innermost macro.
    pub traceback: Vec<MacroTraceFrame>,
}

impl AsmError {
    /// Build an error at `span`.
    pub fn new(span: Span, kind: AsmErrorKind) -> Self {
        AsmError {
            span,
            kind,
            file_path: None,
            traceback: Vec::new(),
        }
    }

    /// Attach a source file path to the error.
    pub fn with_file_path(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Attach a macro expansion traceback.
    pub fn with_traceback(mut self, traceback: Vec<MacroTraceFrame>) -> Self {
        self.traceback = traceback;
        self
    }

    /// Primary span to report to the user: outermost call site if in a traceback, else `self.span`.
    pub fn primary_span(&self) -> Span {
        if let Some(first) = self.traceback.first() {
            first.call_site
        } else {
            self.span
        }
    }

    /// Primary file path to report to the user: outermost call file if in a traceback, else `self.file_path`.
    pub fn primary_file(&self) -> Option<&Path> {
        if let Some(first) = self.traceback.first() {
            if let Some(ref cf) = first.call_file {
                return Some(cf.as_path());
            }
        }
        self.file_path.as_deref()
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
    /// A segment of the same kind was declared more than once in a single file.
    DuplicateSegment(String),
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
    /// Two labels or variables share a name, with source position of the first definition.
    DuplicateDefinition {
        /// The duplicate name.
        name: String,
        /// Where the name was first defined.
        first_defined: Span,
    },
    /// A string-valued `%define` was used in a `.text` operand.
    StringInText(String),
    /// A value that must be a number (a `%repeat` count, `%len` argument, or a byte/word
    /// literal) was not one.
    NotANumber(String),
    /// A `.text` segment with no instructions (nothing for the entry point to run).
    EmptyText,
    /// Missing `.text` segment entirely (e.g. empty file or only data/bss).
    MissingTextSegment,
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
    /// The `main` label cannot be declared `global`.
    GlobalMainForbidden,
    /// A symbol was declared `global` more than once.
    DuplicateGlobal(String),
    /// A symbol was declared `extern` more than once.
    DuplicateExtern(String),
    /// A symbol was declared as both `global` and `extern`.
    ConflictingDeclaration(String),
    /// Attempted to access a private symbol from another module.
    PrivateSymbolAccess {
        /// The referenced symbol.
        symbol: String,
        /// The module defining it.
        module: String,
    },

    // ---- macros ----
    /// A macro was defined more than once.
    DuplicateMacro(String),
    /// Macro argument count was not a number literal.
    InvalidMacroArgCount(String),
    /// Invalid macro argument index (e.g. %0 or index > arg_count).
    InvalidMacroArgRef { index: usize, max: usize },
    /// Macro invocation was given the wrong number of arguments.
    MacroArgCountMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    /// A macro was started with `%macro` but never ended with `%end`.
    UnterminatedMacro(String),
    /// Macro name collides with another symbol or instruction.
    MacroCollision {
        name: String,
        collision_with: String,
    },
    /// Invalid argument kind passed to macro (only variables, constants, and literals allowed).
    InvalidMacroArg(String),
    /// Macro recursion limit exceeded.
    MacroRecursionLimitExceeded(String),

    // ---- conditional compilation ----
    /// An `%if` block was started but never closed before end of file.
    UnterminatedIf,
    /// An `%elif` was encountered without a matching `%if`.
    ElifWithoutIf,
    /// An `%else` was encountered without a matching `%if`.
    ElseWithoutIf,
    /// An `%else` was encountered after an `%else` in the same `%if` block.
    DuplicateElse,
    /// An `%elif` was encountered after an `%else` in the same `%if` block.
    ElifAfterElse,
    /// An unexpected `%end` or `%endif` without a matching opening block.
    UnexpectedEnd,
    /// Malformed or invalid condition expression.
    InvalidCondition(String),

    // ---- static expressions ----
    /// Division or modulo by zero in a static expression.
    DivisionByZero,
    /// Malformed or invalid static expression in a directive.
    InvalidExpression(String),

    // ---- functional defines ----
    /// A functional macro was invoked with the wrong number of arguments.
    FuncMacroArgCountMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    /// A functional macro definition had an invalid parameter list.
    InvalidFuncMacroDef(String),
    /// Functional macro recursion limit exceeded.
    FuncMacroRecursionLimitExceeded(String),
}

impl fmt::Display for AsmErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AsmErrorKind::*;
        match self {
            DivisionByZero => write!(f, "division by zero in expression"),
            InvalidExpression(s) => write!(f, "invalid expression: {s}"),
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
            DuplicateSegment(s) => write!(f, "duplicate segment .{s}"),
            UnknownDirective(s) => write!(f, "unknown directive %{s}"),
            ReservedName(s) => write!(f, "{s:?} is a reserved keyword and cannot be a name"),
            TooManyOperands => write!(f, "too many operands (instructions take at most two)"),
            UndefinedName(s) => write!(f, "undefined name {s:?}"),
            DuplicateName(s) => write!(f, "duplicate definition of {s:?}"),
            DuplicateDefinition {
                name,
                first_defined,
            } => {
                write!(
                    f,
                    "duplicate definition of '{name}' (first defined at {first_defined})"
                )
            }
            StringInText(s) => write!(f, "string constant {s:?} cannot be used in .text"),
            NotANumber(s) => write!(f, "{s} must evaluate to a number"),
            EmptyText => write!(f, ".text has no instructions"),
            MissingTextSegment => write!(f, "missing text segment"),
            ImageOverflow => write!(f, "program does not fit in 64 KiB"),
            LocalLabelWithoutParent(s) => {
                write!(f, "local label .{s} has no preceding parent label")
            }
            UnresolvedSymbol(s) => write!(f, "unresolved external symbol {s:?}"),
            CircularInclude(s) => write!(f, "circular include detected for {s}"),
            IncludeError(s) => write!(f, "include error: {s}"),
            GlobalMainForbidden => write!(f, "main label cannot be declared global"),
            DuplicateGlobal(s) => write!(f, "duplicate global declaration of '{s}'"),
            DuplicateExtern(s) => write!(f, "duplicate extern declaration of '{s}'"),
            ConflictingDeclaration(s) => write!(f, "symbol '{s}' cannot be both global and extern"),
            PrivateSymbolAccess { symbol, module } => {
                write!(f, "symbol {symbol:?} is private to module {module:?}")
            }
            DuplicateMacro(s) => write!(f, "duplicate macro definition '{s}'"),
            InvalidMacroArgCount(s) => write!(f, "macro argument count must be a numeric literal, found '{s}'"),
            InvalidMacroArgRef { index, max } => write!(f, "invalid macro argument reference '%{index}' (macro has {max} parameter(s))"),
            MacroArgCountMismatch { name, expected, found } => write!(f, "macro '{name}' expects {expected} argument(s) but {found} were given"),
            UnterminatedMacro(s) => write!(f, "unterminated macro '{s}' (missing %end)"),
            MacroCollision { name, collision_with } => write!(f, "macro name '{name}' collides with existing {collision_with}"),
            InvalidMacroArg(s) => write!(f, "invalid macro argument '{s}': only variables, constants, and literals are allowed"),
            MacroRecursionLimitExceeded(s) => write!(f, "macro expansion depth limit exceeded in macro '{s}'"),
            UnterminatedIf => write!(f, "unterminated %if block (missing %end or %endif)"),
            ElifWithoutIf => write!(f, "%elif without matching %if"),
            ElseWithoutIf => write!(f, "%else without matching %if"),
            DuplicateElse => write!(f, "duplicate %else in %if block"),
            ElifAfterElse => write!(f, "%elif cannot follow %else in %if block"),
            UnexpectedEnd => write!(f, "unexpected %end or %endif without matching opening directive"),
            InvalidCondition(s) => write!(f, "invalid condition expression: {s}"),
            FuncMacroArgCountMismatch { name, expected, found } => write!(f, "functional macro '{name}' expects {expected} argument(s) but {found} were given"),
            InvalidFuncMacroDef(s) => write!(f, "invalid functional macro definition: {s}"),
            FuncMacroRecursionLimitExceeded(s) => write!(f, "functional macro recursion limit exceeded in '{s}'"),
        }
    }
}
