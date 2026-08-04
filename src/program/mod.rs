//! A `Program` is a sequence of [`Instruction`]s that compiles to a flat byte image,
//! resolving labels in two passes: the first assigns each instruction an address and
//! records label definitions; the second emits bytes, turning label references into the
//! concrete little-endian addresses collected in pass one. This is the direct analogue
//! of the Python `Program.compile`.

use crate::error::EmuError;
use crate::instruction::{Instruction, Operand};
use crate::value::Addr;
use std::collections::HashMap;

/// An assembled program: an ordered list of instructions, some of which may define or
/// reference labels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Program {
    /// The instructions in program order.
    pub instructions: Vec<Instruction>,
}

impl Program {
    /// Build a program from a list of instructions.
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Program { instructions }
    }

    /// Compile to machine code, assuming the program is loaded at `start`.
    ///
    /// Errors on a duplicate label definition or a reference to a label that no
    /// instruction defines.
    pub fn compile(&self, start: Addr) -> Result<Vec<u8>, EmuError> {
        let symbols = self.build_symbol_table(start)?;

        let mut code = Vec::new();
        for inst in &self.instructions {
            code.push(inst.opcode.to_byte());
            for arg in [inst.arg1.as_ref(), inst.arg2.as_ref()].into_iter().flatten() {
                self.emit_operand(arg, &symbols, &mut code)?;
            }
        }
        Ok(code)
    }

    /// Pass one: map each defined label to its absolute address.
    fn build_symbol_table(&self, start: Addr) -> Result<HashMap<String, u16>, EmuError> {
        let mut symbols = HashMap::new();
        let mut addr = start.0;
        for inst in &self.instructions {
            if let Some(label) = &inst.label {
                if symbols.insert(label.clone(), addr).is_some() {
                    return Err(EmuError::DuplicateLabel(label.clone()));
                }
            }
            addr = addr.wrapping_add(inst.size());
        }
        Ok(symbols)
    }

    /// Pass two: append one operand's bytes, resolving a label reference to its address.
    fn emit_operand(
        &self,
        arg: &Operand,
        symbols: &HashMap<String, u16>,
        code: &mut Vec<u8>,
    ) -> Result<(), EmuError> {
        match arg {
            Operand::Byte(b) => code.push(*b),
            Operand::Word(w) => {
                code.push((w & 0xFF) as u8);
                code.push((w >> 8) as u8);
            }
            Operand::Label(name) => {
                let addr = *symbols
                    .get(name)
                    .ok_or_else(|| EmuError::UnresolvedLabel(name.clone()))?;
                code.push((addr & 0xFF) as u8);
                code.push((addr >> 8) as u8);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::Opcode;

    #[test]
    fn immediates_emit_little_endian() {
        // LXI H, 0x1234 -> 21 34 12 ; HLT -> 76
        let prog = Program::new(vec![
            Instruction::with(Opcode::MVI_HL, Operand::word(0x1234)),
            Instruction::new(Opcode::HLT),
        ]);
        assert_eq!(prog.compile(Addr(0)).unwrap(), vec![0x21, 0x34, 0x12, 0x76]);
    }

    #[test]
    fn backward_reference_resolves() {
        // LOOP: NOP ; JMP LOOP  -> at 0x0000: 00 ; C3 00 00
        let prog = Program::new(vec![
            Instruction::new(Opcode::NOP).labeled("LOOP"),
            Instruction::with(Opcode::JMP, Operand::label("LOOP")),
        ]);
        assert_eq!(prog.compile(Addr(0)).unwrap(), vec![0x00, 0xC3, 0x00, 0x00]);
    }

    #[test]
    fn forward_reference_resolves() {
        // JMP END ; NOP ; END: HLT   loaded at 0x0100
        // JMP(3) at 0x0100, NOP(1) at 0x0103, END(HLT) at 0x0104
        let prog = Program::new(vec![
            Instruction::with(Opcode::JMP, Operand::label("END")),
            Instruction::new(Opcode::NOP),
            Instruction::new(Opcode::HLT).labeled("END"),
        ]);
        let code = prog.compile(Addr(0x0100)).unwrap();
        assert_eq!(code, vec![0xC3, 0x04, 0x01, 0x00, 0x76]);
    }

    #[test]
    fn unresolved_label_errors() {
        let prog = Program::new(vec![Instruction::with(Opcode::JMP, Operand::label("NOPE"))]);
        assert_eq!(
            prog.compile(Addr(0)),
            Err(EmuError::UnresolvedLabel("NOPE".into()))
        );
    }

    #[test]
    fn duplicate_label_errors() {
        let prog = Program::new(vec![
            Instruction::new(Opcode::NOP).labeled("X"),
            Instruction::new(Opcode::NOP).labeled("X"),
        ]);
        assert_eq!(prog.compile(Addr(0)), Err(EmuError::DuplicateLabel("X".into())));
    }
}
