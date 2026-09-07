//! Strongly-typed errors for the Debug Adapter Protocol (DAP) subsystem.

use std::fmt;
use std::path::PathBuf;

/// Strongly-typed error representing failure conditions during DAP session management,
/// expression evaluation, variable mutation, or time-travel navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DapError {
    /// Target program file was not found on the local filesystem.
    FileNotFound(PathBuf),
    /// File I/O failure when reading source or binary images.
    Io(String),
    /// Assembly or compilation error during program preparation.
    Assembly(String),
    /// Binary container decoding error when reading `.bin` or `.8085` files.
    ContainerDecode(String),
    /// Error loading image into virtual machine memory or registers.
    MachineLoad(String),
    /// No earlier execution history snapshot available for time-travel reverse step/continue.
    NoHistory,
    /// Invalid or malformed expression string in debugger evaluate request.
    InvalidExpression(String),
    /// Unknown symbol, register, flag, or variable identifier.
    UnknownIdentifier(String),
    /// Variable modification failed (read-only target or invalid value syntax).
    InvalidVariable(String),
    /// Execution exception or hardware TRAP encountered.
    ExecutionFault(String),
    /// General session state or command error.
    Session(String),
}

impl fmt::Display for DapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound(p) => write!(f, "program file not found: {}", p.display()),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Assembly(msg) => write!(f, "assembly error: {msg}"),
            Self::ContainerDecode(msg) => write!(f, "binary container decode error: {msg}"),
            Self::MachineLoad(msg) => write!(f, "machine load error: {msg}"),
            Self::NoHistory => write!(f, "no earlier execution history available"),
            Self::InvalidExpression(msg) => write!(f, "invalid expression: {msg}"),
            Self::UnknownIdentifier(id) => write!(f, "unknown symbol, register, or identifier '{id}'"),
            Self::InvalidVariable(msg) => write!(f, "variable error: {msg}"),
            Self::ExecutionFault(msg) => write!(f, "execution fault: {msg}"),
            Self::Session(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DapError {}

impl From<std::io::Error> for DapError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

impl From<crate::error::EmuError> for DapError {
    fn from(err: crate::error::EmuError) -> Self {
        Self::MachineLoad(err.to_string())
    }
}

impl From<crate::asm::error::AsmError> for DapError {
    fn from(err: crate::asm::error::AsmError) -> Self {
        Self::Assembly(format!("assembly failed at line {}:{}: {}", err.span.line, err.span.col, err.kind))
    }
}
