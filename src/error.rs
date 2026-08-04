//! The crate's single error type. Everything fallible returns `Result<T, EmuError>`
//! instead of panicking, mirroring the places the Python code raised exceptions
//! (invalid opcode, out-of-range access, unresolved label).

use thiserror::Error;

/// Anything that can go wrong while assembling or running a program.
///
/// These are *recoverable, caller-facing* conditions. Genuine invariant violations
/// (a register enum with an impossible value, say) still use `panic!`/`unwrap`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EmuError {
    /// A byte was fetched or decoded that is not a defined 8085 opcode.
    #[error("invalid or undefined opcode: {0:#04X}")]
    InvalidOpcode(u8),

    /// A memory access fell outside the installed RAM.
    #[error("address {addr:#06X} out of bounds (size {size:#X})")]
    AddressOutOfBounds {
        /// The offending address.
        addr: u16,
        /// The total number of addressable bytes.
        size: usize,
    },

    /// A program referenced a label that no instruction defines.
    #[error("unresolved label reference: {0:?}")]
    UnresolvedLabel(String),

    /// The same label was defined by more than one instruction.
    #[error("duplicate label definition: {0:?}")]
    DuplicateLabel(String),
}
