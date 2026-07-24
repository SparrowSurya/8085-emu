"""
This module provides the programmable interface objects for hardware.
"""

from collections.abc import Sequence
from dataclasses import dataclass

from emu_8085.core import Data, MachineCode, Mem

from .instruction import Instruction
from .opcode import Opcode

__all__ = (
    "Program",
    "Instruction",
    "Opcode",
)

@dataclass(frozen=True)
class Program:
    """Represents a cpu program."""

    instructions: Sequence[Instruction]
    """Program instructions."""

    def compile(self, start_mem: Mem = Mem(0)) -> MachineCode:
        """Compiles the program into sequence of machine code resolving labels."""
        symbol_table: dict[str, int] = {}
        current_addr = int(start_mem)

        # Pass 1: Build the symbol table mapping label names to absolute memory addresses.
        for inst in self.instructions:
            if inst.label is not None:
                symbol_table[inst.label] = current_addr
            current_addr += inst.get_size()

        # Pass 2: Generate the machine code.
        machine_code = []
        for inst in self.instructions:
            machine_code.append(Data(inst.opcode))
            for arg in (inst.arg1, inst.arg2):
                if arg is None:
                    continue

                if isinstance(arg, str):
                    if arg not in symbol_table:
                        raise ValueError(f"Undefined label reference: '{arg}'")
                    resolved_addr = symbol_table[arg]
                    # Convert to little-endian representation (low-byte first, then high-byte)
                    # serialized as big-endian bytes in the emulator's data representation.
                    resolved_arg = Data.words(resolved_addr & 0xFF, (resolved_addr >> 8) & 0xFF)
                    machine_code.append(resolved_arg)
                else:
                    machine_code.append(arg)

        return machine_code
