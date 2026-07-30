"""
This module provides the bas datatypes for this library.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from enum import IntEnum
from typing import Any, NewType, Self, TypeAlias

__all__ = (
    "bits",
    "Mem",
    "VEC_RST_0",
    "VEC_RST_1",
    "VEC_RST_2",
    "VEC_RST_3",
    "VEC_RST_4",
    "VEC_RST_5",
    "VEC_RST_6",
    "VEC_RST_7",
    "VEC_TRAP",
    "VEC_RST_5_5",
    "VEC_RST_6_5",
    "VEC_RST_7_5",
    "DataSize",
    "Data",
    "MachineCode",
    "Mask",
    "MaskedData",
)


def bits(count: int) -> int:
    """Provides the bits of given count."""
    return (1 << count) - 1


Mem = NewType("Mem", int)

# Software Interrupt Vector Addresses
VEC_RST_0: Mem = Mem(0x0000)
VEC_RST_1: Mem = Mem(0x0008)
VEC_RST_2: Mem = Mem(0x0010)
VEC_RST_3: Mem = Mem(0x0018)
VEC_RST_4: Mem = Mem(0x0020)
VEC_RST_5: Mem = Mem(0x0028)
VEC_RST_6: Mem = Mem(0x0030)
VEC_RST_7: Mem = Mem(0x0038)

# Hardware Interrupt Vector Addresses
VEC_TRAP: Mem = Mem(0x0024)
VEC_RST_5_5: Mem = Mem(0x002C)
VEC_RST_6_5: Mem = Mem(0x0034)
VEC_RST_7_5: Mem = Mem(0x003C)

class DataSize(IntEnum):
    """Represents the size of data."""

    UNKNOWN = 0
    BIT     = 1
    NIBBLE  = 4
    BYTE    = 8
    WORD    = 16
    DWORD   = 32
    QWORD   = 64

    def __str__(self) -> str:
        return self.name

    def __repr__(self) -> str:
        return f"DataSize({self.name})"


@dataclass(slots=True, repr=False)
class Data:
    """Arbitrary data of some size."""

    _value: int = 0
    """Underlying data value."""

    size: DataSize = DataSize.UNKNOWN
    """Size of the data."""

    @property
    def value(self) -> int:
        """Appropriate value of this data."""
        if self.size == DataSize.UNKNOWN:
            return self._value
        return self._value & bits(self.size)

    @classmethod
    def on(cls) -> Self:
        """Data representing on state."""
        return cls(1, DataSize.BIT)

    @classmethod
    def off(cls) -> Self:
        """Data representing off state."""
        return cls(0, DataSize.BIT)

    @classmethod
    def ch(cls, c: str | bytes) -> Self:
        """Byte representing character."""
        val = c[0] if isinstance(c, bytes) else ord(c)
        return cls(val, DataSize.BYTE)

    @classmethod
    def byte(cls, b1: int = 0) -> Self:
        """Byte data as single byte."""
        val = b1 & 0xFF
        return cls(val, DataSize.BYTE)

    @classmethod
    def words(cls, b1: int = 0, b2: int = 0) -> Self:
        """Word data as combinations of 2 bytes."""
        b1, b2 = b1 & 0xFF, b2 & 0xFF
        val = (b1 << 8) | b2
        return cls(val, DataSize.WORD)

    @classmethod
    def word(cls, b: int = 0) -> Self:
        """Word data as combinations of 2 bytes."""
        return cls(b & 0xFF_FF, DataSize.WORD)

    @classmethod
    def dwords(cls, b1: int = 0, b2: int = 0, b3: int = 0) -> Self:
        """Dword data as combinations of 4 bytes."""
        b1, b2, b3 = b1 & 0xFF, b2 & 0xFF, b3 & 0xFF
        val = b1 << 16 | b2 << 8 | b3
        return cls(val, DataSize.DWORD)

    @classmethod
    def dword(cls, b: int = 0) -> Self:
        """Dword data as combinations of 4 bytes."""
        return cls(b & 0xFFFF_FFFF, DataSize.DWORD)

    @classmethod
    def qwords(cls, b1: int = 0, b2: int = 0, b3: int = 0, b4: int = 0) -> Self:
        """Qword data as combinations of 8 bytes."""
        b1, b2, b3, b4 = b1 & 0xFF, b2 & 0xFF, b3 & 0xFF, b4 & 0xFF
        val = b1 << 32 | b2 << 16 | b3 << 8 | b4
        return cls(val, DataSize.QWORD)

    @classmethod
    def qword(cls, b: int = 0) -> Self:
        """Qword data as combinations of 8 bytes."""
        return cls(b & 0xFFFF_FFFF_FFFF_FFFF, DataSize.QWORD)

    @classmethod
    def mem(cls, mem: Mem) -> Self:
        """Creates data as LSB memory value from mem."""
        return cls.words(mem & 0xFF, (mem >> 8) & 0xFF)

    def byte_at(self, b: int = 0) -> int:
        """Provides bytes value from position.."""
        return (self.value >> (b * 8)) & 0xFF

    def reverse(self) -> Data:
        """Provides data with reversed byte order."""
        if self.size < 8:
            return Data(self.value, self.size)
        val = int.from_bytes(
            self.value.to_bytes(self.size // 8, "little"),
            "big"
        )
        return Data(val, self.size)

    def to_size(self, size: DataSize) -> Data:
        """Provide new data of given size."""
        return Data(self._value, size)

    def __getitem__(self, b: int) -> int:
        return self.byte_at(b)

    def __add__(self, other: Data | int) -> Data:
        size = max(self.size, other.size) if isinstance(other, Data) else self.size
        value = other.value if isinstance(other, Data) else other
        return Data(self.value + value, size)


    def __sub__(self, other: Data | int) -> Data:
        size = max(self.size, other.size) if isinstance(other, Data) else self.size
        value = other.value if isinstance(other, Data) else other
        return Data(self.value - value, size)

    def __mul__(self, other: Data | int) -> Data:
        size = max(self.size, other.size) if isinstance(other, Data) else self.size
        value = other.value if isinstance(other, Data) else other
        return Data(self.value * value, size)

    def __and__(self, other: Data | int) -> Data:
        size = max(self.size, other.size) if isinstance(other, Data) else self.size
        value = other.value if isinstance(other, Data) else other
        return Data(self.value & value, size)

    def __or__(self, other: Data | int) -> Data:
        size = max(self.size, other.size) if isinstance(other, Data) else self.size
        value = other.value if isinstance(other, Data) else other
        return Data(self.value | value, size)

    def __xor__(self, other: Data | int) -> Data:
        size = max(self.size, other.size) if isinstance(other, Data) else self.size
        value = other.value if isinstance(other, Data) else other
        return Data(self.value ^ value, size)

    def __lshift__(self, value: int) -> Data:
        return Data(self.value << value, self.size)

    def __rshift__(self, value: int) -> Data:
        return Data(self.value >> value, self.size)

    def __invert__(self) -> Data:
        return Data(~self.value, self.size)

    def __radd__(self, other: int) -> Data:
        return Data(other + self.value, self.size)

    def __rsub__(self, other: int) -> Data:
        return Data(other - self.value, self.size)

    def __rmul__(self, other: int) -> Data:
        return Data(other * self.value, self.size)

    def __rand__(self, other: int) -> Data:
        return Data(other & self.value, self.size)

    def __ror__(self, other: int) -> Data:
        return Data(other | self.value, self.size)

    def __rxor__(self, other: int) -> Data:
        return Data(other ^ self.value, self.size)

    def __int__(self) -> int:
        return self.value

    def __eq__(self, other: Any) -> bool:
        if isinstance(other, Data):
            return self.value == other.value
        if isinstance(other, int):
            return self.value == other
        return False

    def __bytes__(self) -> bytes:
        if self.size == DataSize.UNKNOWN:
            length = max(1, (self.value.bit_length() + 7) // 8)
        else:
            length = max(1, self.size // 8)
        return self.value.to_bytes(length, 'big')

    def __repr__(self) -> str:
        return f"Data(value={self.value}, bits={self.value:_b}, size={self.size!s})"


MachineCode: TypeAlias = Sequence[Data]


@dataclass(frozen=True)
class Mask:
    """Bit mask representation."""

    value: int
    """Mask data."""

    offset: int = 0
    """Stored right-shift offset position."""

    @classmethod
    def bits(cls, count: int, offset: int = 0) -> Self:
        """Creates mask of given bit count and offset."""
        return cls(value=bits(count) << offset, offset=offset)

    def apply(self,
        value: Data | int,
        shift: bool = True,
        *,
        size: DataSize = DataSize.UNKNOWN,
    ) -> Data:
        """
        Applies the mask to the value. Additionally applies right shift to masked value.
        """
        extracted = self.value & value
        size = value.size if isinstance(value, Data) else size
        return Data(int(extracted >> self.offset if shift else extracted), size)

    def __len__(self) -> int:
        return self.value.bit_length()

    def bit_count(self) -> int:
        """Bits count of the mask."""
        return self.value.bit_count()

    def __repr__(self) -> str:
        return f"Mask(value={self.value:_b}, offset={self.offset})"

    def __str__(self) -> str:
        return f"{self.value:_b}"


@dataclass(slots=True, repr=False)
class MaskedData:
    """Represents arbitrary masked data."""

    mask: Mask
    """Defines the readable region bounds. The mask should have 0 offset."""

    value: int = 0
    """Defines the actual raw positionally aligned integer data."""

    def __post_init__(self):
        assert self.mask.offset == 0
        self.value = int(self.mask.apply(self.value))

    @classmethod
    def bit_count(cls, count: int, *args: Any, **kwargs: Any) -> Self:
        """Creates data container of custom bit count width."""
        return cls(Mask.bits(count), *args, **kwargs)

    @classmethod
    def bit(cls, *args: Any, **kwargs: Any) -> Self:
        """Creates data container structure of 1 bit width."""
        return cls(Mask.bits(1), *args, **kwargs)

    @classmethod
    def nibble(cls, *args: Any, **kwargs: Any) -> Self:
        """Creates data container structure of 4 bits width."""
        return cls(Mask.bits(4), *args, **kwargs)

    @classmethod
    def byte(cls, *args: Any, **kwargs: Any) -> Self:
        """Creates data container structure of 8 bits or 1 byte width."""
        return cls(Mask.bits(8), *args, **kwargs)

    @classmethod
    def word(cls, *args: Any, **kwargs: Any) -> Self:
        """Creates data container structure of 16 bits or 2 bytes width."""
        return cls(Mask.bits(16), *args, **kwargs)

    @classmethod
    def dword(cls, *args: Any, **kwargs: Any) -> Self:
        """Creates data container structure of 32 bits or 4 bytes width."""
        return cls(Mask.bits(32), *args, **kwargs)

    @classmethod
    def qword(cls, *args: Any, **kwargs: Any) -> Self:
        """Creates data container structure of 64 bits or 8 bytes width."""
        return cls(Mask.bits(64), *args, **kwargs)

    def read(self) -> Data:
        """Read the raw bounded value payload."""
        return Data(self.value)

    def write(self, value: Data | int, mask: Mask | None = None):
        """Writes raw value payload safely with optional sub-mask alignments."""
        val = value.value if isinstance(value, Data) else value
        if mask is None:
            self.value = int(self.mask.apply(val))
        else:
            cleaned = self.value & ~mask.value
            max_val = (1 << mask.bit_count()) - 1
            aligned = ((val & max_val) << mask.offset) & mask.value
            self.value = cleaned | aligned

    @property
    def bits(self) -> int:
        """Returns total bits count configured inside structural mask."""
        return self.mask.bit_count()

    def __len__(self) -> int:
        return len(self.mask)

    def __repr__(self) -> str:
        return f"Data(value={self.value}, bits={self.value:_b})"

    def __str__(self) -> str:
        return f"{self.value:_b}"
