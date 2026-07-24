"""
This module provides various hardware registers.
"""

from collections.abc import Sequence
from dataclasses import dataclass, field
from typing import Any, Literal, Self, override

from emu_8085.core import Data, DataSize, Mask, MaskedData
from emu_8085.program.opcode import Opcode

__all__ = (
    "Register",
    "InstructionRegister",
    "RegisterRef",
    "FlagRegister",
)

@dataclass(slots=True)
class Register(MaskedData):
    """CPU register implementation extending baseline masked data logic."""

    name: str = ""
    """Name of register."""

    @classmethod
    @override
    def bit_count(cls, count: int, name: str, **kwargs: Any) -> Self:
        """Creates a named register tracking custom width bit counts."""
        return cls(mask=Mask.bits(count), name=name, **kwargs)

    @classmethod
    @override
    def bit(cls, name: str, value: int = 0, *args: Any, **kwargs: Any) -> Self:
        """Creates a named register tracking 1 bit configurations."""
        return cls(Mask.bits(1), value, name=name, *args, **kwargs)

    @classmethod
    @override
    def nibble(cls, name: str, value: int = 0, *args: Any, **kwargs: Any) -> Self:
        """Creates a named register tracking 4 bits configurations."""
        return cls(Mask.bits(4), value, name=name, *args, **kwargs)

    @classmethod
    @override
    def byte(cls, name: str, value: int = 0, *args: Any, **kwargs: Any) -> Self:
        """Creates a named register tracking 8 bits configurations."""
        return cls(Mask.bits(8), value, name=name, *args, **kwargs)

    @classmethod
    @override
    def word(cls, name: str, value: int = 0, *args: Any, **kwargs: Any) -> Self:
        """Creates a named register tracking 16 bits configurations."""
        return cls(Mask.bits(16), value, name=name, *args, **kwargs)

    @classmethod
    @override
    def dword(cls, name: str, value: int = 0, *args: Any, **kwargs: Any) -> Self:
        """Creates a named register tracking 32 bits configurations."""
        return cls(Mask.bits(32), value, name=name, *args, **kwargs)

    @classmethod
    @override
    def qword(cls, name: str, value: int = 0, *args: Any, **kwargs: Any) -> Self:
        """Creates a named register tracking 64 bits configurations."""
        return cls(Mask.bits(64), value, name=name, *args, **kwargs)

    def __repr__(self) -> str:
        return f"Register(name={self.name}, value={self.value}, bits={self.value:_b})"

    def increment(self):
        """Increments its value by one."""
        self.write(self.read()+1)

    def decrement(self):
        """Decrements its value by one."""
        self.write(self.read()-1)


class InstructionRegister(Register):
    """Stores instruction."""

    def __eq__(self, other: Any) -> bool:
        if isinstance(other, Opcode):
            return self.read() == other.value
        if isinstance(other, (Data, int)):
            return self.read() == other
        return False

@dataclass(repr=False)
class RegisterRef:
    """
    Provides a uniform multi-width interface over a sequence of 8-bit registers to manage
    them as arbitrary Byte, Word, Dword, or Qword lengths.
    """

    seq: Sequence[Register] = field(default_factory=list)
    """Sequence of registers."""

    def set(self, seq: Sequence[Register]):
        """Binds a sequence of hardware registers to this reference vector."""
        self.seq = seq

    def read_byte(self, order: Literal[0, 1, 2, 3] = 0) -> Data:
        """Reads a single byte from the register at the specified order index."""
        return self.seq[-(order + 1)].read()

    def write_byte(self, value: Data | int, order: Literal[0, 1, 2, 3] = 0):
        """Writes a single byte to the register at the specified order index."""
        val = value.value if isinstance(value, Data) else value
        self.seq[-(order + 1)].write(val)

    def read_word(self, order: Literal[0, 1, 2] = 0) -> Data:
        """Combines two adjacent registers to read a 16-bit word."""
        high = int(self.seq[-(order + 2)].read())
        low = int(self.seq[-(order + 1)].read())
        return Data((high << 8) | low)

    def write_word(self, value: Data | int, order: Literal[0, 1, 2] = 0):
        """Splits and writes a 16-bit word across two adjacent registers."""
        val = value.value if isinstance(value, Data) else value
        self.seq[-(order + 2)].write(Data((val >> 8) & 0xFF))
        self.seq[-(order + 1)].write(Data(val & 0xFF))

    def read_dword(self) -> Data:
        """Combines four registers to read a unified 32-bit double-word."""
        return Data(
            (int(self.seq[0].read()) << 24) |
            (int(self.seq[1].read()) << 16) |
            (int(self.seq[2].read()) << 8)  |
            int(self.seq[3].read())
        )

    def write_dword(self, value: Data | int):
        """Splits and writes a 32-bit double-word across four registers."""
        val = value.value if isinstance(value, Data) else value
        self.seq[0].write(Data((val >> 24) & 0xFF))
        self.seq[1].write(Data((val >> 16) & 0xFF))
        self.seq[2].write(Data((val >> 8) & 0xFF))
        self.seq[3].write(Data(val & 0xFF))

    def read_qword(self) -> Data:
        """Dynamically loops through the register sequence to construct a 64-bit quad-word."""
        result = 0
        for i, reg in enumerate(self.seq):
            shift = (len(self.seq) - 1 - i) * 8
            result |= reg.read().value << shift
        return Data(result)

    def write_qword(self, value: Data | int):
        """Splits and writes a 64-bit quad-word value across the entire register sequence."""
        val = value.value if isinstance(value, Data) else value
        for i, reg in enumerate(self.seq):
            shift = (len(self.seq) - 1 - i) * 8
            reg.write(Data((val >> shift) & 0xFF))


@dataclass(repr=False)
class FlagRegister:
    """CPU flag register."""

    _value: MaskedData = field(init=False, default_factory=lambda: MaskedData.byte())
    """Internal raw value."""

    _mask_carry: Mask = field(init=False, default=Mask.bits(1, 0))
    """Carry flag mask."""

    _mask_parity: Mask = field(init=False, default=Mask.bits(1, 2))
    """Carry flag mask."""

    _mask_aux: Mask = field(init=False, default=Mask.bits(1, 4))
    """Carry flag mask."""

    _mask_zero: Mask = field(init=False, default=Mask.bits(1, 6))
    """Carry flag mask."""

    _mask_sign: Mask = field(init=False, default=Mask.bits(1, 7))
    """Carry flag mask."""

    @property
    def value(self) -> int:
        """Actual raw value."""
        return self._value.value

    @value.setter
    def value(self, val: Data | int):
        """Sets raw value."""
        self._value.write(val.value if isinstance(val, Data) else val)

    def update(self, mask: Mask, value: int):
        """Updates the flag with given value on maksed bits."""
        mask_value = mask.value
        new_value = (~mask_value & self.value) | (mask_value & value)
        self._value.write(new_value)

    @property
    def carry(self) -> int:
        """Carry flag."""
        return self._mask_carry.apply(self.value, size=DataSize.BIT).value

    @carry.setter
    def carry(self, value: int):
        """Set carry flag."""
        value = (value & 0b1) << self._mask_carry.offset
        self.update(self._mask_carry, value)

    @property
    def parity(self) -> int:
        """Parity flag."""
        return self._mask_parity.apply(self.value, size=DataSize.BIT).value

    @parity.setter
    def parity(self, value: int):
        """Set parity flag."""
        value = (value & 0b1) << self._mask_parity.offset
        self.update(self._mask_parity, value)

    @property
    def aux(self) -> int:
        """Auxiliary carry flag."""
        return self._mask_aux.apply(self.value, size=DataSize.BIT).value

    @aux.setter
    def aux(self, value: int):
        """Set auxiliary carry flag."""
        value = (value & 0b1) << self._mask_aux.offset
        self.update(self._mask_aux, value)

    @property
    def zero(self) -> int:
        """Zero flag."""
        return self._mask_zero.apply(self.value, size=DataSize.BIT).value

    @zero.setter
    def zero(self, value: int):
        """Set zero flag."""
        value = (value & 0b1) << self._mask_zero.offset
        self.update(self._mask_zero, value)

    @property
    def sign(self) -> int:
        """Sign flag."""
        return self._mask_sign.apply(self.value, size=DataSize.BIT).value

    @sign.setter
    def sign(self, value: int):
        """Set sign flag."""
        value = (value & 0b1) << self._mask_sign.offset
        self.update(self._mask_sign, value)
