//! Instruction representation: the [`opcode::Opcode`] enum plus an assembled
//! [`Instruction`] (opcode, up to two operands, and an optional defining label).
//!
//! Registers are encoded *in* the opcode (`MOV_A_B`, `MVI_A`, …), exactly as in the
//! reference, so operands only ever carry immediate data, a direct address, or a label
//! reference — see [`Operand`].

pub mod opcode;

pub use opcode::Opcode;

/// An instruction operand: an 8-bit immediate, a 16-bit immediate/address, or a symbolic
/// label that the [`Program`](crate::program::Program) compiler resolves to an address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    /// A one-byte immediate (`MVI`, `ADI`, `IN`/`OUT` port, …).
    Byte(u8),
    /// A two-byte immediate or address, emitted little-endian (`LXI`, `JMP`, `LDA`, …).
    Word(u16),
    /// A reference to a label defined elsewhere; resolves to that label's address.
    Label(String),
}

impl Operand {
    /// An 8-bit immediate operand.
    pub fn byte(v: u8) -> Self {
        Operand::Byte(v)
    }

    /// A 16-bit immediate/address operand.
    pub fn word(v: u16) -> Self {
        Operand::Word(v)
    }

    /// A label reference operand.
    pub fn label(name: impl Into<String>) -> Self {
        Operand::Label(name.into())
    }

    /// How many bytes this operand contributes to the encoded instruction.
    pub(crate) fn size(&self) -> u16 {
        match self {
            Operand::Byte(_) => 1,
            Operand::Word(_) | Operand::Label(_) => 2,
        }
    }
}

/// One assembled instruction: an opcode, up to two operands, and an optional label that
/// marks this instruction's address for others to reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// The operation.
    pub opcode: Opcode,
    /// First operand, if any.
    pub arg1: Option<Operand>,
    /// Second operand, if any.
    pub arg2: Option<Operand>,
    /// Label defined at this instruction, if any.
    pub label: Option<String>,
}

impl Instruction {
    /// An instruction with no operands (`NOP`, `HLT`, `MOV_A_B`, …).
    pub fn new(opcode: Opcode) -> Self {
        Instruction { opcode, arg1: None, arg2: None, label: None }
    }

    /// An instruction with one operand (`MVI_A`, `JMP`, `LXI`, …).
    pub fn with(opcode: Opcode, arg: Operand) -> Self {
        Instruction { opcode, arg1: Some(arg), arg2: None, label: None }
    }

    /// An instruction with two operands.
    pub fn with2(opcode: Opcode, a1: Operand, a2: Operand) -> Self {
        Instruction { opcode, arg1: Some(a1), arg2: Some(a2), label: None }
    }

    /// Attach a defining label to this instruction (builder style).
    pub fn labeled(mut self, name: impl Into<String>) -> Self {
        self.label = Some(name.into());
        self
    }

    /// Encoded size in bytes: one opcode byte plus each operand's width.
    pub fn size(&self) -> u16 {
        1 + self.arg1.as_ref().map_or(0, Operand::size)
            + self.arg2.as_ref().map_or(0, Operand::size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_sizes() {
        assert_eq!(Instruction::new(Opcode::NOP).size(), 1);
        assert_eq!(Instruction::with(Opcode::MVI_A, Operand::byte(5)).size(), 2);
        assert_eq!(Instruction::with(Opcode::JMP, Operand::word(0x1234)).size(), 3);
        assert_eq!(Instruction::with(Opcode::JMP, Operand::label("L")).size(), 3);
    }
}
