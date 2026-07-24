"""
This module provides instruction dataclass.
"""

from dataclasses import dataclass

from emu_8085.core import Data, DataSize, Mem

from .opcode import Opcode

__all__ = (
    "Instruction",
)


@dataclass(frozen=True)
class Instruction:
    """CPU instruction (intel type)."""

    opcode: Opcode
    """Instruction opcode."""

    arg1: Mem | Data | str | None = None
    """Represents first argument (can be a direct value or a label name)."""

    arg2: Mem | Data | str | None = None
    """Represents second argument."""

    label: str | None = None
    """Optional label definition pointing to this instruction."""

    def __post_init__(self):
        if self.arg1 is None:
            assert(self.arg2 is None)
        if self.arg2 is not None:
            assert(self.arg1 is not None)

    def get_size(self) -> int:
        """Provides the instruction size in bytes."""
        size = 1  # Opcode byte
        for arg in (self.arg1, self.arg2):
            if arg is not None:
                if isinstance(arg, str) or isinstance(arg, int) or getattr(arg, 'size', None) == DataSize.WORD:
                    size += 2
                else:
                    size += 1
        return size

    def __repr__(self) -> str:
        values = ", ".join([
            str(self.opcode),
            *([str(self.arg1)] if self.arg1 else []),
            *([str(self.arg2)] if self.arg2 else []),
        ])
        return f"Inst({values})"
