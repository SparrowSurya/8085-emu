from __future__ import annotations

import math
from abc import ABC, abstractmethod
from collections.abc import Sequence
from dataclasses import dataclass, field
from enum import IntEnum, StrEnum, auto
from typing import Any, Callable, Literal, NewType, Self, TypeAlias, override


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


@dataclass(slots=True, repr=False)
class SystemBus:
    """Represents system bus component."""

    _data: MaskedData = field(default_factory=lambda: MaskedData.bit_count(24))
    """Represents internal data."""

    mask_mr: Mask = field(init=False, default=Mask.bits(1, 0))
    """Bit mask for memory read bit."""

    mask_mw: Mask = field(init=False, default=Mask.bits(1, 1))
    """Bit mask for memory write bit."""

    mask_ior: Mask = field(init=False, default=Mask.bits(1, 2))
    """Bit mask for i/o memory read bit."""

    mask_iow: Mask = field(init=False, default=Mask.bits(1, 3))
    """Bit mask for i/o memory write bit."""

    mask_inta: Mask = field(init=False, default=Mask.bits(1, 4))
    """Bit mask for interrupt acknowledge pin."""

    mask_hold: Mask = field(init=False, default=Mask.bits(1, 5))
    """Bit mask for DMA hold request pin."""

    mask_hlda: Mask = field(init=False, default=Mask.bits(1, 6))
    """Bit mask for DMA hold acknowledge pin."""

    mask_ready: Mask = field(init=False, default=Mask.bits(1, 7))
    """Bit mask for ready (wait state) pin."""

    mask_reset_in: Mask = field(init=False, default=Mask.bits(1, 8))
    """Bit mask for hardware reset input pin."""

    mask_reset_out: Mask = field(init=False, default=Mask.bits(1, 9))
    """Bit mask for hardware reset output pin."""

    mask_control: Mask = field(init=False, default=Mask.bits(12, 0))
    """Bit mask for control lines."""

    mask_address: Mask = field(default=Mask.bits(8, 12))
    """Bit mask for address lines."""

    mask_data: Mask = field(default=Mask.bits(4, 20))
    """Bit mask for data lines."""

    @classmethod
    def create(cls, address_lines: int, data_lines: int) -> Self:
        """Creates system bus from requirements."""
        control_lines = 10

        bus = cls(
            MaskedData.bit_count(control_lines + address_lines + data_lines),
            mask_address=Mask.bits(address_lines, control_lines),
            mask_data=Mask.bits(data_lines, control_lines + address_lines),
        )
        bus.ready = Data.on()
        return bus

    @property
    def value(self) -> int:
        """Provides the raw data value."""
        return int(self._data.read())

    @property
    def address(self) -> Mem:
        """Provides the address value in the bus."""
        return Mem(self.mask_address.apply(self.value).value)

    @address.setter
    def address(self, value: Mem):
        """Sets address value on the bus."""
        self._data.write(value, self.mask_address)

    @property
    def data(self) -> Data:
        """Provides the data value in the bus."""
        return self.mask_data.apply(self.value)

    @data.setter
    def data(self, value: Data):
        """Sets data value on the bus."""
        self._data.write(value, self.mask_data)

    @property
    def mr(self) -> Data:
        """Status for memory read."""
        return self.mask_mr.apply(self.value)

    @mr.setter
    def mr(self, value: Data):
        """Sets memory read value on the bus."""
        self._data.write(value, self.mask_mr)

    @property
    def mw(self) -> Data:
        """Status for memory write."""
        return self.mask_mw.apply(self.value)

    @mw.setter
    def mw(self, value: Data):
        """Sets memory write value on the bus."""
        self._data.write(value, self.mask_mw)

    @property
    def ior(self) -> Data:
        """Status for io memory read."""
        return self.mask_ior.apply(self.value)

    @ior.setter
    def ior(self, value: Data):
        """Sets io memory read value on the bus."""
        self._data.write(value, self.mask_ior)

    @property
    def iow(self) -> Data:
        """Status for io memory write."""
        return self.mask_iow.apply(self.value)

    @iow.setter
    def iow(self, value: Data):
        """Sets io memory write value on the bus."""
        self._data.write(value, self.mask_iow)

    @property
    def inta(self) -> Data:
        return self.mask_inta.apply(self.value)

    @inta.setter
    def inta(self, value: Data):
        self._data.write(value, self.mask_inta)

    @property
    def hold(self) -> Data:
        return self.mask_hold.apply(self.value)

    @hold.setter
    def hold(self, value: Data):
        self._data.write(value, self.mask_hold)

    @property
    def hlda(self) -> Data:
        return self.mask_hlda.apply(self.value)

    @hlda.setter
    def hlda(self, value: Data):
        self._data.write(value, self.mask_hlda)

    @property
    def ready(self) -> Data:
        return self.mask_ready.apply(self.value)

    @ready.setter
    def ready(self, value: Data):
        self._data.write(value, self.mask_ready)

    @property
    def reset_in(self) -> Data:
        return self.mask_reset_in.apply(self.value)

    @reset_in.setter
    def reset_in(self, value: Data):
        self._data.write(value, self.mask_reset_in)

    @property
    def reset_out(self) -> Data:
        return self.mask_reset_out.apply(self.value)

    @reset_out.setter
    def reset_out(self, value: Data):
        self._data.write(value, self.mask_reset_out)

    def reset(self):
        """Resets the signals."""
        self.mr = Data.off()
        self.mw = Data.off()
        self.ior = Data.off()
        self.iow = Data.off()
        self.inta = Data.off()


@dataclass(repr=False)
class Memory:
    """Represents memory component."""

    data: bytearray
    """Represents raw memeory data."""

    mask: Mask
    """Represents the mask for address lines."""

    @classmethod
    def from_lines(cls, lines: int) -> Self:
        """Creates memory of size respective to address lines."""
        return cls(bytearray(2**lines), Mask.bits(lines))

    def __len__(self) -> int:
        return len(self.data)

    @property
    def address_lines(self) -> int:
        """Provides the address lines of memory."""
        return math.ceil(math.log2(len(self)))

    def read(self, mem: Mem) -> Data:
        """Read memory location."""
        assert(mem in range(len(self)))
        ptr = self.mask.apply(mem).value
        val = self.data[ptr]
        return Data(int(val), DataSize.BYTE)

    def write(self, mem: Mem, val: Data | int):
        """Write memory location."""
        assert(mem in range(len(self)))
        value = val.value if isinstance(val, Data) else val
        ptr = self.mask.apply(mem).value
        self.data[ptr] = value & 0xFF

    def __repr__(self) -> str:
        return f"Memory(size={len(self)})"

    def __str__(self) -> str:
        return " ".join(f"{x:02X}" for x in self.data)

    def step(self, bus: SystemBus):
        """Reads system bus signal to performs action.."""
        if bus.mr == 1:
            bus.data = self.read(bus.address)
        elif bus.mw == 1:
            self.write(bus.address, bus.data)

    def write_code(self, machine_code: MachineCode, mem: Mem):
        """Write the code into memroy from location."""
        i = int(mem)
        for code in machine_code:
            values = bytes(code)
            for j in range(len(values)):
                self.write(Mem(i+j), values[j])
            i += len(values)


class Opcode(IntEnum):
    """Instruction opcode."""

    NOP = 0x00
    """No operation."""

    HLT = 0x76
    """Stops the processor execution loop."""

    # MVI (Move Immediate 8-bit)
    MVI_B = 0x06
    """Loads immediate byte value into register 'B'."""
    MVI_C = 0x0E
    """Loads immediate byte value into register 'C'."""
    MVI_D = 0x16
    """Loads immediate byte value into register 'D'."""
    MVI_E = 0x1E
    """Loads immediate byte value into register 'E'."""
    MVI_H = 0x26
    """Loads immediate byte value into register 'H'."""
    MVI_L = 0x2E
    """Loads immediate byte value into register 'L'."""
    MVI_M = 0x36
    """Writes immediate byte value to memory address pointed by 'HL'."""
    MVI_A = 0x3E
    """Loads immediate byte value into register 'A'."""

    # LXI / MVI 16-bit register pair
    MVI_BC = 0x01
    """Loads 16-bit immediate word into register pair 'BC'."""
    MVI_DE = 0x11
    """Loads 16-bit immediate word into register pair 'DE'."""
    MVI_HL = 0x21
    """Loads 16-bit immediate word into register pair 'HL'."""
    LXI = MVI_HL
    """Alternative alias: Loads 16-bit memory address into register pair 'HL'."""

    # MOV Memory (Destination = Memory M)
    MOV_M_B = 0x70
    """Writes register 'B' value to memory address pointed by 'HL'."""
    MOV_M_C = 0x71
    """Writes register 'C' value to memory address pointed by 'HL'."""
    MOV_M_D = 0x72
    """Writes register 'D' value to memory address pointed by 'HL'."""
    MOV_M_E = 0x73
    """Writes register 'E' value to memory address pointed by 'HL'."""
    MOV_M_H = 0x74
    """Writes register 'H' value to memory address pointed by 'HL'."""
    MOV_M_L = 0x75
    """Writes register 'L' value to memory address pointed by 'HL'."""
    MOV_M_A = 0x77
    """Writes register 'A' value to memory address pointed by 'HL'."""

    # MOV Memory (Source = Memory M)
    MOV_B_M = 0x46
    """Copies memory byte pointed by 'HL' into register 'B'."""
    MOV_C_M = 0x4E
    """Copies memory byte pointed by 'HL' into register 'C'."""
    MOV_D_M = 0x56
    """Copies memory byte pointed by 'HL' into register 'D'."""
    MOV_E_M = 0x5E
    """Copies memory byte pointed by 'HL' into register 'E'."""
    MOV_H_M = 0x66
    """Copies memory byte pointed by 'HL' into register 'H'."""
    MOV_L_M = 0x6E
    """Copies memory byte pointed by 'HL' into register 'L'."""
    MOV_A_M = 0x7E
    """Copies memory byte pointed by 'HL' into register 'A'."""

    # MOV Register-to-Register (Destination = A)
    MOV_A_B = 0x78
    """Copies value from register 'B' into register 'A'."""
    MOV_A_C = 0x79
    """Copies value from register 'C' into register 'A'."""
    MOV_A_D = 0x7A
    """Copies value from register 'D' into register 'A'."""
    MOV_A_E = 0x7B
    """Copies value from register 'E' into register 'A'."""
    MOV_A_H = 0x7C
    """Copies value from register 'H' into register 'A'."""
    MOV_A_L = 0x7D
    """Copies value from register 'L' into register 'A'."""

    # MOV Register-to-Register (Destination = B)
    MOV_B_B = 0x40
    """Copies value from register 'B' into register 'B'."""
    MOV_B_C = 0x41
    """Copies value from register 'C' into register 'B'."""
    MOV_B_D = 0x42
    """Copies value from register 'D' into register 'B'."""
    MOV_B_E = 0x43
    """Copies value from register 'E' into register 'B'."""
    MOV_B_H = 0x44
    """Copies value from register 'H' into register 'B'."""
    MOV_B_L = 0x45
    """Copies value from register 'L' into register 'B'."""
    MOV_B_A = 0x47
    """Copies value from register 'A' into register 'B'."""

    # MOV Register-to-Register (Destination = C)
    MOV_C_B = 0x48
    """Copies value from register 'B' into register 'C'."""
    MOV_C_C = 0x49
    """Copies value from register 'C' into register 'C'."""
    MOV_C_D = 0x4A
    """Copies value from register 'D' into register 'C'."""
    MOV_C_E = 0x4B
    """Copies value from register 'E' into register 'C'."""
    MOV_C_H = 0x4C
    """Copies value from register 'H' into register 'C'."""
    MOV_C_L = 0x4D
    """Copies value from register 'L' into register 'C'."""
    MOV_C_A = 0x4F
    """Copies value from register 'A' into register 'C'."""

    # MOV Register-to-Register (Destination = D)
    MOV_D_B = 0x50
    """Copies value from register 'B' into register 'D'."""
    MOV_D_C = 0x51
    """Copies value from register 'C' into register 'D'."""
    MOV_D_D = 0x52
    """Copies value from register 'D' into register 'D'."""
    MOV_D_E = 0x53
    """Copies value from register 'E' into register 'D'."""
    MOV_D_H = 0x54
    """Copies value from register 'H' into register 'D'."""
    MOV_D_L = 0x55
    """Copies value from register 'L' into register 'D'."""
    MOV_D_A = 0x57
    """Copies value from register 'A' into register 'D'."""

    # MOV Register-to-Register (Destination = E)
    MOV_E_B = 0x58
    """Copies value from register 'B' into register 'E'."""
    MOV_E_C = 0x59
    """Copies value from register 'C' into register 'E'."""
    MOV_E_D = 0x5A
    """Copies value from register 'D' into register 'E'."""
    MOV_E_E = 0x5B
    """Copies value from register 'E' into register 'E'."""
    MOV_E_H = 0x5C
    """Copies value from register 'H' into register 'E'."""
    MOV_E_L = 0x5D
    """Copies value from register 'L' into register 'E'."""
    MOV_E_A = 0x5F
    """Copies value from register 'A' into register 'E'."""

    # MOV Register-to-Register (Destination = H)
    MOV_H_B = 0x60
    """Copies value from register 'B' into register 'H'."""
    MOV_H_C = 0x61
    """Copies value from register 'C' into register 'H'."""
    MOV_H_D = 0x62
    """Copies value from register 'D' into register 'H'."""
    MOV_H_E = 0x63
    """Copies value from register 'E' into register 'H'."""
    MOV_H_H = 0x64
    """Copies value from register 'H' into register 'H'."""
    MOV_H_L = 0x65
    """Copies value from register 'L' into register 'H'."""
    MOV_H_A = 0x67
    """Copies value from register 'A' into register 'H'."""

    # MOV Register-to-Register (Destination = L)
    MOV_L_B = 0x68
    """Copies value from register 'B' into register 'L'."""
    MOV_L_C = 0x69
    """Copies value from register 'C' into register 'L'."""
    MOV_L_D = 0x6A
    """Copies value from register 'D' into register 'L'."""
    MOV_L_E = 0x6B
    """Copies value from register 'E' into register 'L'."""
    MOV_L_H = 0x6C
    """Copies value from register 'H' into register 'L'."""
    MOV_L_L = 0x6D
    """Copies value from register 'L' into register 'L'."""
    MOV_L_A = 0x6F
    """Copies value from register 'A' into register 'L'."""

    # Direct & Indirect Load/Store
    STA_BC = 0x02
    """Writes register 'A' value to memory address pointed by register pair 'BC'."""
    LDA_BC = 0x0A
    """Loads register 'A' with memory byte pointed by register pair 'BC'."""
    STA_DE = 0x12
    """Writes register 'A' value to memory address pointed by register pair 'DE'."""
    LDA_DE = 0x1A
    """Loads register 'A' with memory byte pointed by register pair 'DE'."""
    SHLD = 0x22
    """Writes register pair 'HL' values directly to 16-bit memory address."""
    LHLD = 0x2A
    """Loads register pair 'HL' directly with 16-bit data from memory address."""
    STA = 0x32
    """Writes register 'A' value directly to a 16-bit immediate memory address."""
    LDA = 0x3A
    """Loads register 'A' directly with data from a 16-bit immediate memory address."""
    XCHG = 0xEB
    """Exchanges 16-bit contents of register pairs 'DE' and 'HL'."""

    # ADD (Add Register / Memory to A)
    ADD_B = 0x80
    """Adds register 'B' value to register 'A'; updates all status flags."""
    ADD_C = 0x81
    """Adds register 'C' value to register 'A'; updates all status flags."""
    ADD_D = 0x82
    """Adds register 'D' value to register 'A'; updates all status flags."""
    ADD_E = 0x83
    """Adds register 'E' value to register 'A'; updates all status flags."""
    ADD_H = 0x84
    """Adds register 'H' value to register 'A'; updates all status flags."""
    ADD_L = 0x85
    """Adds register 'L' value to register 'A'; updates all status flags."""
    ADD_M = 0x86
    """Adds memory value pointed by 'HL' to register 'A'; updates all status flags."""
    ADD_A = 0x87
    """Adds register 'A' value to itself; updates all status flags."""

    # ADC (Add Register / Memory + Carry to A)
    ADC_B = 0x88
    """Adds register 'B' value and Carry flag to register 'A'; updates all status flags."""
    ADC_C = 0x89
    """Adds register 'C' value and Carry flag to register 'A'; updates all status flags."""
    ADC_D = 0x8A
    """Adds register 'D' value and Carry flag to register 'A'; updates all status flags."""
    ADC_E = 0x8B
    """Adds register 'E' value and Carry flag to register 'A'; updates all status flags."""
    ADC_H = 0x8C
    """Adds register 'H' value and Carry flag to register 'A'; updates all status flags."""
    ADC_L = 0x8D
    """Adds register 'L' value and Carry flag to register 'A'; updates all status flags."""
    ADC_M = 0x8E
    """Adds memory value pointed by 'HL' and Carry flag to register 'A'; updates all status flags."""
    ADC_A = 0x8F
    """Adds register 'A' value and the Carry flag to itself; updates all status flags."""

    # SUB (Subtract Register / Memory from A)
    SUB_B = 0x90
    """Subtracts register 'B' value from register 'A'; updates all status flags."""
    SUB_C = 0x91
    """Subtracts register 'C' value from register 'A'; updates all status flags."""
    SUB_D = 0x92
    """Subtracts register 'D' value from register 'A'; updates all status flags."""
    SUB_E = 0x93
    """Subtracts register 'E' value from register 'A'; updates all status flags."""
    SUB_H = 0x94
    """Subtracts register 'H' value from register 'A'; updates all status flags."""
    SUB_L = 0x95
    """Subtracts register 'L' value from register 'A'; updates all status flags."""
    SUB_M = 0x96
    """Subtracts memory value pointed by 'HL' from register 'A'; updates all status flags."""
    SUB_A = 0x97
    """Subtracts register 'A' value from itself; updates all status flags."""

    # SBB (Subtract Register / Memory + Carry from A)
    SBB_B = 0x98
    """Subtracts register 'B' value and Carry flag from register 'A'; updates all status flags."""
    SBB_C = 0x99
    """Subtracts register 'C' value and Carry flag from register 'A'; updates all status flags."""
    SBB_D = 0x9A
    """Subtracts register 'D' value and Carry flag from register 'A'; updates all status flags."""
    SBB_E = 0x9B
    """Subtracts register 'E' value and Carry flag from register 'A'; updates all status flags."""
    SBB_H = 0x9C
    """Subtracts register 'H' value and Carry flag from register 'A'; updates all status flags."""
    SBB_L = 0x9D
    """Subtracts register 'L' value and Carry flag from register 'A'; updates all status flags."""
    SBB_M = 0x9E
    """Subtracts memory value pointed by 'HL' and Carry flag from register 'A'; updates all status flags."""
    SBB_A = 0x9F
    """Subtracts register 'A' value and Carry flag from itself; updates all status flags."""

    # INR (Increment Register / Memory)
    INR_B = 0x04
    """Increments register 'B' value by 1; updates status flags."""
    INR_C = 0x0C
    """Increments register 'C' value by 1; updates status flags."""
    INR_D = 0x14
    """Increments register 'D' value by 1; updates status flags."""
    INR_E = 0x1C
    """Increments register 'E' value by 1; updates status flags."""
    INR_H = 0x24
    """Increments register 'H' value by 1; updates status flags."""
    INR_L = 0x2C
    """Increments register 'L' value by 1; updates status flags."""
    INR_M = 0x34
    """Increments memory value pointed by 'HL' by 1; updates status flags."""
    INR_A = 0x3C
    """Increments register 'A' value by 1; updates status flags."""

    # DCR (Decrement Register / Memory)
    DCR_B = 0x05
    """Decrements register 'B' value by 1; updates status flags."""
    DCR_C = 0x0D
    """Decrements register 'C' value by 1; updates status flags."""
    DCR_D = 0x15
    """Decrements register 'D' value by 1; updates status flags."""
    DCR_E = 0x1D
    """Decrements register 'E' value by 1; updates status flags."""
    DCR_H = 0x25
    """Decrements register 'H' value by 1; updates status flags."""
    DCR_L = 0x2D
    """Decrements register 'L' value by 1; updates status flags."""
    DCR_M = 0x35
    """Decrements memory value pointed by 'HL' by 1; updates status flags."""
    DCR_A = 0x3D
    """Decrements register 'A' value by 1; updates status flags."""

    # ANA (Logical AND Register / Memory with A)
    ANA_B = 0xA0
    """Performs logical AND of register 'B' value with register 'A'; updates status flags."""
    ANA_C = 0xA1
    """Performs logical AND of register 'C' value with register 'A'; updates status flags."""
    ANA_D = 0xA2
    """Performs logical AND of register 'D' value with register 'A'; updates status flags."""
    ANA_E = 0xA3
    """Performs logical AND of register 'E' value with register 'A'; updates status flags."""
    ANA_H = 0xA4
    """Performs logical AND of register 'H' value with register 'A'; updates status flags."""
    ANA_L = 0xA5
    """Performs logical AND of register 'L' value with register 'A'; updates status flags."""
    ANA_M = 0xA6
    """Performs logical AND of memory value pointed by 'HL' with register 'A'; updates status flags."""
    ANA_A = 0xA7
    """Performs logical AND of register 'A' value with itself; updates status flags."""
    ANI = 0xE6
    """Performs logical AND of immediate byte with register 'A'; updates status flags."""

    # XRA (Logical XOR Register / Memory with A)
    XRA_B = 0xA8
    """Performs logical XOR of register 'B' value with register 'A'; updates status flags."""
    XRA_C = 0xA9
    """Performs logical XOR of register 'C' value with register 'A'; updates status flags."""
    XRA_D = 0xAA
    """Performs logical XOR of register 'D' value with register 'A'; updates status flags."""
    XRA_E = 0xAB
    """Performs logical XOR of register 'E' value with register 'A'; updates status flags."""
    XRA_H = 0xAC
    """Performs logical XOR of register 'H' value with register 'A'; updates status flags."""
    XRA_L = 0xAD
    """Performs logical XOR of register 'L' value with register 'A'; updates status flags."""
    XRA_M = 0xAE
    """Performs logical XOR of memory value pointed by 'HL' with register 'A'; updates status flags."""
    XRA_A = 0xAF
    """Performs logical XOR of register 'A' value with itself; updates status flags."""
    XRI = 0xEE
    """Performs logical XOR of immediate byte with register 'A'; updates status flags."""

    # ORA (Logical OR Register / Memory with A)
    ORA_B = 0xB0
    """Performs logical OR of register 'B' value with register 'A'; updates status flags."""
    ORA_C = 0xB1
    """Performs logical OR of register 'C' value with register 'A'; updates status flags."""
    ORA_D = 0xB2
    """Performs logical OR of register 'D' value with register 'A'; updates status flags."""
    ORA_E = 0xB3
    """Performs logical OR of register 'E' value with register 'A'; updates status flags."""
    ORA_H = 0xB4
    """Performs logical OR of register 'H' value with register 'A'; updates status flags."""
    ORA_L = 0xB5
    """Performs logical OR of register 'L' value with register 'A'; updates status flags."""
    ORA_M = 0xB6
    """Performs logical OR of memory value pointed by 'HL' with register 'A'; updates status flags."""
    ORA_A = 0xB7
    """Performs logical OR of register 'A' value with itself; updates status flags."""
    ORI = 0xF6
    """Performs logical OR of immediate byte with register 'A'; updates status flags."""

    # CMP (Compare Register / Memory with A)
    CMP_B = 0xB8
    """Compares register 'B' value with register 'A'; updates status flags."""
    CMP_C = 0xB9
    """Compares register 'C' value with register 'A'; updates status flags."""
    CMP_D = 0xBA
    """Compares register 'D' value with register 'A'; updates status flags."""
    CMP_E = 0xBB
    """Compares register 'E' value with register 'A'; updates status flags."""
    CMP_H = 0xBC
    """Compares register 'H' value with register 'A'; updates status flags."""
    CMP_L = 0xBD
    """Compares register 'L' value with register 'A'; updates status flags."""
    CMP_M = 0xBE
    """Compares memory value pointed by 'HL' with register 'A'; updates status flags."""
    CMP_A = 0xBF
    """Compares register 'A' value with itself; updates status flags."""
    CPI = 0xFE
    """Compares immediate byte with register 'A'; updates status flags."""

    # Rotate & Special Accumulator Instructions
    RLC = 0x07
    """Rotates register 'A' value left circular; updates Carry flag."""
    RRC = 0x0F
    """Rotates register 'A' value right circular; updates Carry flag."""
    RAL = 0x17
    """Rotates register 'A' value left through Carry flag; updates Carry flag."""
    RAR = 0x1F
    """Rotates register 'A' value right through Carry flag; updates Carry flag."""
    DAA = 0x27
    """Decimal adjusts register 'A' value after addition; updates status flags."""
    CMA = 0x2F
    """Complements (inverts) register 'A' value; status flags are unchanged."""
    STC = 0x37
    """Sets Carry flag to 1; updates Carry flag."""
    CMC = 0x3F
    """Complements Carry flag; updates Carry flag."""

    # Unassigned / Custom Extensions
    DAS = 0x10
    """Decimal adjusts register 'A' value after subtraction; updates status flags."""
    AAA = 0x18
    """ASCII adjusts register 'A' value after addition; updates status flags."""
    AAS = 0x28
    """ASCII adjusts register 'A' value after subtraction; updates status flags."""

    # Immediate Arithmetic Operations
    ADI = 0xC6
    """Adds immediate byte to register 'A'; updates all status flags."""
    ACI = 0xCE
    """Adds immediate byte and Carry flag to register 'A'; updates all status flags."""
    SUI = 0xD6
    """Subtracts immediate byte from register 'A'; updates all status flags."""
    SBI = 0xDE
    """Subtracts immediate byte and Carry flag from register 'A'; updates all status flags."""

    # 16-Bit Register Pair Increment / Decrement / Add
    INX_BC = 0x03
    """Increments register pair 'BC' by 1; status flags are unchanged."""
    INX_DE = 0x13
    """Increments register pair 'DE' by 1; status flags are unchanged."""
    INX_HL = 0x23
    """Increments register pair 'HL' by 1; status flags are unchanged."""
    INX_SP = 0x33
    """Increments stack pointer 'SP' by 1; status flags are unchanged."""

    DCX_BC = 0x0B
    """Decrements register pair 'BC' by 1; status flags are unchanged."""
    DCX_DE = 0x1B
    """Decrements register pair 'DE' by 1; status flags are unchanged."""
    DCX_HL = 0x2B
    """Decrements register pair 'HL' by 1; status flags are unchanged."""
    DCX_SP = 0x3B
    """Decrements stack pointer 'SP' by 1; status flags are unchanged."""

    DAD_BC = 0x09
    """Adds register pair 'BC' to 'HL'; updates Carry flag only."""
    DAD_DE = 0x19
    """Adds register pair 'DE' to 'HL'; updates Carry flag only."""
    DAD_HL = 0x29
    """Adds register pair 'HL' to 'HL'; updates Carry flag only."""
    DAD_SP = 0x39
    """Adds stack pointer 'SP' to 'HL'; updates Carry flag only."""

    # Stack Push / Pop
    POP_BC = 0xC1
    """Pops top of stack into register pair 'BC'; status flags are unchanged."""
    PUSH_BC = 0xC5
    """Pushes register pair 'BC' onto the stack; status flags are unchanged."""
    POP_DE = 0xD1
    """Pops top of stack into register pair 'DE'; status flags are unchanged."""
    PUSH_DE = 0xD5
    """Pushes register pair 'DE' onto the stack; status flags are unchanged."""
    POP_HL = 0xE1
    """Pops top of stack into register pair 'HL'; status flags are unchanged."""
    PUSH_HL = 0xE5
    """Pushes register pair 'HL' onto the stack; status flags are unchanged."""
    POP_PSW = 0xF1
    """Pops top of stack into Program Status Word (Accumulator 'A' and Flags)."""
    PUSH_PSW = 0xF5
    """Pushes Program Status Word (Accumulator 'A' and Flags) onto the stack."""

    XTHL = 0xE3
    """Exchanges 16-bit contents of top of stack with register pair 'HL'."""
    SPHL = 0xF9
    """Loads stack pointer 'SP' with 16-bit contents of register pair 'HL'."""
    PCHL = 0xE9
    """Loads program counter 'PC' with 16-bit contents of register pair 'HL'."""

    # Unconditional Jump & Call & Return
    JMP = 0xC3
    """Unconditional jump; jumps to the memory address."""
    CALL = 0xCD
    """Unconditional call subroutine."""
    RET = 0xC9
    """Unconditional return from subroutine."""

    # Conditional Jumps
    JNZ = 0xC2
    """Jump if zero flag is 0."""
    JZ = 0xCA
    """Jump if zero flag is 1."""
    JNC = 0xD2
    """Jump if carry flag is 0."""
    JC = 0xDA
    """Jump if carry flag is 1."""
    JPO = 0xE2
    """Jump if parity flag is 0 (parity odd)."""
    JPE = 0xEA
    """Jump if parity flag is 1 (parity even)."""
    JP = 0xF2
    """Jump if sign flag is 0 (positive)."""
    JM = 0xFA
    """Jump if sign flag is 1 (minus)."""

    # Conditional Calls
    CNZ = 0xC4
    """Call subroutine if zero flag is 0."""
    CZ = 0xCC
    """Call subroutine if zero flag is 1."""
    CNC = 0xD4
    """Call subroutine if carry flag is 0."""
    CC = 0xDC
    """Call subroutine if carry flag is 1."""
    CPO = 0xE4
    """Call subroutine if parity flag is 0 (parity odd)."""
    CPE = 0xEC
    """Call subroutine if parity flag is 1 (parity even)."""
    CP = 0xF4
    """Call subroutine if sign flag is 0 (positive)."""
    CM = 0xFC
    """Call subroutine if sign flag is 1 (minus)."""

    # Conditional Returns
    RNZ = 0xC0
    """Return from subroutine if zero flag is 0."""
    RZ = 0xC8
    """Return from subroutine if zero flag is 1."""
    RNC = 0xD0
    """Return from subroutine if carry flag is 0."""
    RC = 0xD8
    """Return from subroutine if carry flag is 1."""
    RPO = 0xE0
    """Return from subroutine if parity flag is 0 (parity odd)."""
    RPE = 0xE8
    """Return from subroutine if parity flag is 1 (parity even)."""
    RP = 0xF0
    """Return from subroutine if sign flag is 0 (positive)."""
    RM = 0xF8
    """Return from subroutine if sign flag is 1 (minus)."""

    # Software Restarts / Interrupts
    RST_0 = 0xC7
    """Restart 0: Push PC and jump to 0x0000."""
    RST_1 = 0xCF
    """Restart 1: Push PC and jump to 0x0008."""
    RST_2 = 0xD7
    """Restart 2: Push PC and jump to 0x0010."""
    RST_3 = 0xDF
    """Restart 3: Push PC and jump to 0x0018."""
    RST_4 = 0xE7
    """Restart 4: Push PC and jump to 0x0020."""
    RST_5 = 0xEF
    """Restart 5: Push PC and jump to 0x0028."""
    RST_6 = 0xF7
    """Restart 6: Push PC and jump to 0x0030."""
    RST_7 = 0xFF
    """Restart 7: Push PC and jump to 0x0038."""

    # I/O & Interrupt Control
    IN = 0xDB
    """Reads 8-bit byte from I/O port into Accumulator A."""
    OUT = 0xD3
    """Writes 8-bit Accumulator A byte to I/O port."""
    EI = 0xFB
    """Enables maskable interrupts (sets INTE flag)."""
    DI = 0xF3
    """Disables maskable interrupts (clears INTE flag)."""
    RIM = 0x20
    """Read Interrupt Mask into Accumulator A."""
    SIM = 0x30
    """Set Interrupt Mask from Accumulator A."""

    def __repr__(self) -> str:
        return f"Opcode(name={self.name}, value={int(self)})"

    def __str__(self) -> str:
        return self.name


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


class MachineCycle(StrEnum):
    """Represents a one machine cycle."""

    FETCH = auto()
    EXECUTE = auto()
    HOLD = auto()
    WAIT = auto()


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


class Interrupt:
    int: str

@dataclass(repr=False)
class CPU:
    """Central processing unit."""

    reg_a: Register = field(init=False, default_factory=lambda: Register.byte('a'))
    """General register 'A'."""

    reg_b: Register = field(init=False, default_factory=lambda: Register.byte('b'))
    """General register 'B'."""

    reg_c: Register = field(init=False, default_factory=lambda: Register.byte('c'))
    """General register 'C'."""

    reg_d: Register = field(init=False, default_factory=lambda: Register.byte('d'))
    """General register 'D'."""

    reg_e: Register = field(init=False, default_factory=lambda: Register.byte('e'))
    """General register 'E'."""

    reg_h: Register = field(init=False, default_factory=lambda: Register.byte('h'))
    """General register 'H'."""

    reg_l: Register = field(init=False, default_factory=lambda: Register.byte('l'))
    """General register 'L'."""

    reg_w: Register = field(init=False, default_factory=lambda: Register.byte('w'))
    """Internal register 'W'."""

    reg_z: Register = field(init=False, default_factory=lambda: Register.byte('z'))
    """Internal register 'Z'."""

    reg_tmp: Register = field(init=False, default_factory=lambda: Register.byte('tmp'))
    """Internal temporary register."""

    reg_sp: Register = field(init=False, default_factory=lambda: Register.word('sp', value=0xFFFF))
    """Stack pointer."""

    flag_reg: FlagRegister = field(init=False, default_factory=lambda: FlagRegister())
    """Flag resgister."""

    reg_pc: Register = field(init=False, default_factory=lambda: InstructionRegister.word('pc'))
    """Program counter."""

    _cycle: MachineCycle = field(init=False, default=MachineCycle.FETCH)
    """Represents machine cycle."""

    t_state: int = field(init=False, default=0)
    """Represents cpu t state."""

    ireg: InstructionRegister = field(init=False, default_factory=lambda: InstructionRegister.word('ir'))
    """Represents instruction register."""

    is_halt: bool = field(init=False, default=True)
    """CPU halt state."""

    inte: bool = field(init=False, default=False)
    """Interrupt enable flip-flop."""

    mask_5_5: bool = field(init=False, default=False)
    """RST 5.5 mask bit."""

    mask_6_5: bool = field(init=False, default=False)
    """RST 6.5 mask bit."""

    mask_7_5: bool = field(init=False, default=False)
    """RST 7.5 mask bit."""

    pending_5_5: bool = field(init=False, default=False)
    """RST 5.5 pending flag."""

    pending_6_5: bool = field(init=False, default=False)
    """RST 6.5 pending flag."""

    pending_7_5: bool = field(init=False, default=False)
    """RST 7.5 pending flag."""

    sid: bool = field(init=False, default=False)
    """Serial input data pin."""

    sod: bool = field(init=False, default=False)
    """Serial output data pin."""

    trap: bool = field(init=False, default=False)
    """TRAP non-maskable hardware interrupt pin."""

    rst_7_5: bool = field(init=False, default=False)
    """RST 7.5 hardware interrupt pin."""

    rst_6_5: bool = field(init=False, default=False)
    """RST 6.5 hardware interrupt pin."""

    rst_5_5: bool = field(init=False, default=False)
    """RST 5.5 hardware interrupt pin."""

    intr: bool = field(init=False, default=False)
    """INTR maskable hardware interrupt pin."""

    _is_inta_cycle: bool = field(init=False, default=False)
    """Tracks if the current fetch is an INTA cycle."""

    _reg_src: RegisterRef = field(init=False, default_factory=lambda: RegisterRef())
    """Source (read) register to consider during execute."""

    _reg_dst: RegisterRef = field(init=False, default_factory=lambda: RegisterRef())
    """Destination (write) register to consider during execute."""

    _exec_mem: Mem = field(init=False, default=Mem(0))
    """Memory address to consider."""

    _decoder_matrix: dict[int, Callable[[SystemBus], MachineCycle]] = field(init=False, default_factory=dict)
    """Decoder decodes the instruction next cycle."""

    _dispatch_table: dict[int, Sequence[Callable[[SystemBus], None]]] = field(init=False, default_factory=dict)
    """Dispatch table for execution order of excute machine cycle."""

    def __post_init__(self):
        def _bind(
            dst: Sequence[Register] | None = None,
            src: Sequence[Register] | None = None,
        ) -> None:
            if dst is not None:
                self._reg_dst.set(dst)
            if src is not None:
                self._reg_src.set(src)

        def decode_exec(
            dst: Sequence[Register] | None = None,
            src: Sequence[Register] | None = None,
        ) -> Callable[[SystemBus], MachineCycle]:
            def _fn(bus: SystemBus) -> MachineCycle:
                _bind(dst, src)
                return MachineCycle.EXECUTE
            return _fn

        def decode_fetch(
            action: Callable[[SystemBus], None],
            dst: Sequence[Register] | None = None,
            src: Sequence[Register] | None = None,
        ) -> Callable[[SystemBus], MachineCycle]:
            def _fn(bus: SystemBus) -> MachineCycle:
                _bind(dst, src)
                action(bus)
                return MachineCycle.FETCH
            return _fn

        matrix: dict[int, Callable[[SystemBus], MachineCycle]] = {
            Opcode.MVI_A: decode_exec(dst=[self.reg_a]),
            Opcode.MVI_B: decode_exec(dst=[self.reg_b]),
            Opcode.MVI_C: decode_exec(dst=[self.reg_c]),
            Opcode.MVI_D: decode_exec(dst=[self.reg_d]),
            Opcode.MVI_E: decode_exec(dst=[self.reg_e]),
            Opcode.MVI_H: decode_exec(dst=[self.reg_h]),
            Opcode.MVI_L: decode_exec(dst=[self.reg_l]),
            Opcode.MVI_BC: decode_exec(dst=[self.reg_b, self.reg_c]),
            Opcode.MVI_DE: decode_exec(dst=[self.reg_d, self.reg_e]),
            Opcode.MVI_HL: decode_exec(dst=[self.reg_h, self.reg_l]),
            Opcode.LXI: decode_exec(dst=[self.reg_h, self.reg_l]),
            Opcode.MVI_M: decode_exec(dst=[self.reg_h, self.reg_l]),

            Opcode.MOV_M_A: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_a]),
            Opcode.MOV_M_B: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_b]),
            Opcode.MOV_M_C: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_c]),
            Opcode.MOV_M_D: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_d]),
            Opcode.MOV_M_E: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_e]),
            Opcode.MOV_M_H: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_h]),
            Opcode.MOV_M_L: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_l]),

            Opcode.MOV_A_M: decode_exec(dst=[self.reg_a]),
            Opcode.MOV_B_M: decode_exec(dst=[self.reg_b]),
            Opcode.MOV_C_M: decode_exec(dst=[self.reg_c]),
            Opcode.MOV_D_M: decode_exec(dst=[self.reg_d]),
            Opcode.MOV_E_M: decode_exec(dst=[self.reg_e]),
            Opcode.MOV_H_M: decode_exec(dst=[self.reg_h]),
            Opcode.MOV_L_M: decode_exec(dst=[self.reg_l]),

            Opcode.LDA: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.STA: decode_exec(dst=[self.reg_w, self.reg_z], src=[self.reg_a]),
            Opcode.LDA_BC: decode_exec(),
            Opcode.LDA_DE: decode_exec(),
            Opcode.STA_BC: decode_exec(),
            Opcode.STA_DE: decode_exec(),
            Opcode.LHLD: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.SHLD: decode_exec(dst=[self.reg_w, self.reg_z]),

            Opcode.ADD_M: decode_exec(),
            Opcode.ADC_M: decode_exec(),
            Opcode.SUB_M: decode_exec(),
            Opcode.SBB_M: decode_exec(),
            Opcode.INR_M: decode_exec(),
            Opcode.DCR_M: decode_exec(),
            Opcode.ANA_M: decode_exec(),
            Opcode.ANI: decode_exec(),
            Opcode.ORA_M: decode_exec(),
            Opcode.ORI: decode_exec(),
            Opcode.XRA_M: decode_exec(),
            Opcode.XRI: decode_exec(),
            Opcode.CMP_M: decode_exec(),
            Opcode.CPI: decode_exec(),
            Opcode.ADI: decode_exec(),
            Opcode.ACI: decode_exec(),
            Opcode.SUI: decode_exec(),
            Opcode.SBI: decode_exec(),

            Opcode.INX_BC: decode_exec(),
            Opcode.INX_DE: decode_exec(),
            Opcode.INX_HL: decode_exec(),
            Opcode.INX_SP: decode_exec(),
            Opcode.DCX_BC: decode_exec(),
            Opcode.DCX_DE: decode_exec(),
            Opcode.DCX_HL: decode_exec(),
            Opcode.DCX_SP: decode_exec(),
            Opcode.DAD_BC: decode_exec(),
            Opcode.DAD_DE: decode_exec(),
            Opcode.DAD_HL: decode_exec(),
            Opcode.DAD_SP: decode_exec(),

            Opcode.PUSH_BC: decode_exec(),
            Opcode.PUSH_DE: decode_exec(),
            Opcode.PUSH_HL: decode_exec(),
            Opcode.PUSH_PSW: decode_exec(),
            Opcode.POP_BC: decode_exec(),
            Opcode.POP_DE: decode_exec(),
            Opcode.POP_HL: decode_exec(),
            Opcode.POP_PSW: decode_exec(),
            Opcode.XTHL: decode_exec(),
            Opcode.SPHL: decode_exec(),

            # 4 T-State single-byte operations (Execute at T4 of FETCH)
            Opcode.NOP: decode_fetch(self._ts_exec_nop),
            Opcode.XCHG: decode_fetch(self._ts_exec_xchg),
            Opcode.RLC: decode_fetch(self._ts_exec_rlc),
            Opcode.RRC: decode_fetch(self._ts_exec_rrc),
            Opcode.RAL: decode_fetch(self._ts_exec_ral),
            Opcode.RAR: decode_fetch(self._ts_exec_rar),
            Opcode.CMA: decode_fetch(self._ts_exec_cma),
            Opcode.CMC: decode_fetch(self._ts_exec_cmc),
            Opcode.STC: decode_fetch(self._ts_exec_stc),
            Opcode.DAA: decode_fetch(self._ts_exec_daa),
            Opcode.DAS: decode_fetch(self._ts_exec_das),
            Opcode.AAA: decode_fetch(self._ts_exec_aaa),
            Opcode.AAS: decode_fetch(self._ts_exec_aas),

            Opcode.JMP: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JNZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JNC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JP: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JM: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JPE: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JPO: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CALL: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CNZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CNC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CP: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CM: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CPE: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CPO: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RET: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RNZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RNC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RP: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RM: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RPE: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RPO: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.PCHL: decode_exec(dst=[self.reg_h, self.reg_l]),
            Opcode.RST_0: decode_exec(),
            Opcode.RST_1: decode_exec(),
            Opcode.RST_2: decode_exec(),
            Opcode.RST_3: decode_exec(),
            Opcode.RST_4: decode_exec(),
            Opcode.RST_5: decode_exec(),
            Opcode.RST_6: decode_exec(),
            Opcode.RST_7: decode_exec(),
            Opcode.EI: decode_fetch(self._ts_exec_ei),
            Opcode.DI: decode_fetch(self._ts_exec_di),
            Opcode.RIM: decode_fetch(self._ts_exec_rim),
            Opcode.SIM: decode_fetch(self._ts_exec_sim),
            Opcode.IN: decode_exec(dst=[self.reg_z]),
            Opcode.OUT: decode_exec(dst=[self.reg_z], src=[self.reg_a]),
        }

        # MOV r1, r2
        regs = ("A", "B", "C", "D", "E", "H", "L")
        for r1 in regs:
            for r2 in regs:
                op = getattr(Opcode, f"MOV_{r1}_{r2}", None)
                if op:
                    matrix[op.value] = decode_fetch(
                        self._ts_exce_set_reg_from_reg,
                        dst=[self.reg(r1)],
                        src=[self.reg(r2)],
                    )

        # Register arithmetic / logic
        ops_map = [
            ("ADD", self._ts_exec_add),
            ("ADC", self._ts_exec_add_with_carry),
            ("SUB", self._ts_exec_sub),
            ("SBB", self._ts_exec_sub_with_borrow),
            ("INR", self._ts_exec_inr),
            ("DCR", self._ts_exec_dcr),
            ("ANA", self._ts_exec_ana),
            ("ORA", self._ts_exec_ora),
            ("XRA", self._ts_exec_xra),
            ("CMP", self._ts_exec_cmp),
        ]
        for prefix, handler in ops_map:
            for r in regs:
                op = getattr(Opcode, f"{prefix}_{r}", None)
                if op:
                    if prefix in ("INR", "DCR"):
                        matrix[op.value] = decode_fetch(handler, dst=[self.reg(r)])
                    else:
                        matrix[op.value] = decode_fetch(handler, src=[self.reg(r)])

        self._decoder_matrix = matrix

        set_reg: Sequence[Callable[[SystemBus], None]] = [
            self._ts_exec_set_bus_addr_from_pc,
            self._ts_exec_set_bus_mr,
            lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
        ]

        set_reg_pair: Sequence[Callable[[SystemBus], None]] = [
            self._ts_exec_set_bus_addr_from_pc,
            self._ts_exec_set_bus_mr,
            lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
            self._ts_exec_set_bus_addr_from_pc,
            self._ts_exec_set_bus_mr,
            lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
        ]

        set_reg_to_mem: Sequence[Callable[[SystemBus], None]] = [
            self._ts_exec_set_bus_addr_from_hl_reg,
            self._ts_exec_set_bus_data_from_reg,
            self._ts_exec_set_bus_mw,
        ]

        set_mem_to_reg: Sequence[Callable[[SystemBus], None]] = [
            self._ts_exec_set_bus_addr_from_hl_reg,
            self._ts_exec_set_bus_mr,
            lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
        ]

        self._dispatch_table = {
            Opcode.MVI_A: set_reg,
            Opcode.MVI_B: set_reg,
            Opcode.MVI_C: set_reg,
            Opcode.MVI_D: set_reg,
            Opcode.MVI_E: set_reg,
            Opcode.MVI_H: set_reg,
            Opcode.MVI_L: set_reg,
            Opcode.MVI_BC: set_reg_pair,
            Opcode.MVI_DE: set_reg_pair,
            Opcode.MVI_HL: set_reg_pair,
            Opcode.LXI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
            ],
            Opcode.MOV_M_A: set_reg_to_mem,
            Opcode.MOV_M_B: set_reg_to_mem,
            Opcode.MOV_M_C: set_reg_to_mem,
            Opcode.MOV_M_D: set_reg_to_mem,
            Opcode.MOV_M_E: set_reg_to_mem,
            Opcode.MOV_M_H: set_reg_to_mem,
            Opcode.MOV_M_L: set_reg_to_mem,
            Opcode.MOV_A_M: set_mem_to_reg,
            Opcode.MOV_B_M: set_mem_to_reg,
            Opcode.MOV_C_M: set_mem_to_reg,
            Opcode.MOV_D_M: set_mem_to_reg,
            Opcode.MOV_E_M: set_mem_to_reg,
            Opcode.MOV_H_M: set_mem_to_reg,
            Opcode.MOV_L_M: set_mem_to_reg,
            Opcode.MVI_M: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_tmp_reg_val_from_bus_data,
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_data_from_tmp_reg,
                self._ts_exec_set_bus_mw,
            ],
            Opcode.LDA: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_set_bus_addr_from_wz_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_a_val_from_bus_data,
            ],
            Opcode.STA: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_set_bus_addr_from_wz_reg,
                self._ts_exec_set_bus_data_from_reg,
                self._ts_exec_set_bus_mw
            ],
            Opcode.LDA_BC: [
                self._ts_exec_set_bus_addr_from_bc_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_a_val_from_bus_data,
            ],
            Opcode.LDA_DE: [
                self._ts_exec_set_bus_addr_from_de_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_a_val_from_bus_data,
            ],
            Opcode.STA_BC: [
                self._ts_exec_set_bus_addr_from_bc_reg,
                self._ts_exec_set_bus_data_from_reg_a,
                self._ts_exec_set_bus_mw,
            ],
            Opcode.STA_DE: [
                self._ts_exec_set_bus_addr_from_de_reg,
                self._ts_exec_set_bus_data_from_reg_a,
                self._ts_exec_set_bus_mw,
            ],
            Opcode.LHLD: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_set_bus_addr_from_wz_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_l_val_from_bus_data,
                self._ts_exec_set_bus_addr_from_wz_plus_1,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_h_val_from_bus_data,
            ],
            Opcode.SHLD: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_set_bus_addr_from_wz_reg,
                self._ts_exec_set_bus_data_from_reg_l,
                self._ts_exec_set_bus_mw,
                self._ts_exec_set_bus_addr_from_wz_plus_1,
                self._ts_exec_set_bus_data_from_reg_h,
                self._ts_exec_set_bus_mw,
            ],
            Opcode.ADD_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.ADC_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.SUB_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.SBB_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.INR_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_tmp_reg_val_from_bus_data,
                self._ts_exec_inr,
                self._ts_exec_set_bus_mw,
                self._ts_exec_set_bus_data_from_tmp_reg,
            ],
            Opcode.DCR_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_tmp_reg_val_from_bus_data,
                self._ts_exec_dcr,
                self._ts_exec_set_bus_mw,
                self._ts_exec_set_bus_data_from_tmp_reg,
            ],
            Opcode.ANA_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.ANI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.ORA_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.ORI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.XRA_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.XRI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.CMP_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.CPI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.ADI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.ACI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.SUI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.SBI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.INX_BC: [self._ts_exec_internal_delay, self._ts_exec_inx],
            Opcode.INX_DE: [self._ts_exec_internal_delay, self._ts_exec_inx],
            Opcode.INX_HL: [self._ts_exec_internal_delay, self._ts_exec_inx],
            Opcode.INX_SP: [self._ts_exec_internal_delay, self._ts_exec_inx],
            Opcode.DCX_BC: [self._ts_exec_internal_delay, self._ts_exec_dcx],
            Opcode.DCX_DE: [self._ts_exec_internal_delay, self._ts_exec_dcx],
            Opcode.DCX_HL: [self._ts_exec_internal_delay, self._ts_exec_dcx],
            Opcode.DCX_SP: [self._ts_exec_internal_delay, self._ts_exec_dcx],
            Opcode.DAD_BC: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_dad,
            ],
            Opcode.DAD_DE: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_dad,
            ],
            Opcode.DAD_HL: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_dad,
            ],
            Opcode.DAD_SP: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_dad,
            ],
            Opcode.PUSH_BC: [
                self._ts_exec_internal_delay,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_b,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_c,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.PUSH_DE: [
                self._ts_exec_internal_delay,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_d,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_e,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.PUSH_HL: [
                self._ts_exec_internal_delay,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_h,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_l,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.POP_BC: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_c,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_b,
            ],
            Opcode.POP_DE: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_e,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_d,
            ],
            Opcode.POP_HL: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_l,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_h,
            ],
            Opcode.PUSH_PSW: [
                self._ts_exec_internal_delay,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_a,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_flag_reg,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.POP_PSW: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_flag_reg,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_a,
            ],
            Opcode.XTHL: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_xthl_read_l,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_sp_plus_1,
                self._ts_exec_set_bus_mr,
                self._ts_exec_xthl_read_h,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.SPHL: [self._ts_exec_internal_delay, self._ts_exec_sphl],
            Opcode.PCHL: [self._ts_exec_internal_delay, self._ts_exec_pchl],
            Opcode.JMP: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_pc_from_reg_wz,
            ],
            Opcode.JZ: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JNZ: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JC: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JNC: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JP: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JM: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JPE: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JPO: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.CALL: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CZ: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CNZ: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CC: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CNC: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CP: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CM: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CPE: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CPO: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.RET: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RZ: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RNZ: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RC: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RNC: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RP: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RM: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RPE: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RPO: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
        }

        rst_steps = [
            self._ts_exec_internal_delay,
            self._ts_exec_internal_delay,
            self._ts_exec_push_step,
            self._ts_exec_set_bus_data_from_pc_high,
            self._ts_exec_set_bus_mw,
            self._ts_exec_push_step,
            self._ts_exec_set_bus_data_from_pc_low,
            self._ts_exec_set_bus_mw,
            self._ts_exec_rst_jump,
        ]
        self._dispatch_table.update({
            Opcode.RST_0: rst_steps,
            Opcode.RST_1: rst_steps,
            Opcode.RST_2: rst_steps,
            Opcode.RST_3: rst_steps,
            Opcode.RST_4: rst_steps,
            Opcode.RST_5: rst_steps,
            Opcode.RST_6: rst_steps,
            Opcode.RST_7: rst_steps,
            Opcode.IN: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_z_port,
                self._ts_exec_set_bus_ior,
                self._ts_exec_in_read_a,
            ],
            Opcode.OUT: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_z_port,
                self._ts_exec_set_bus_data_from_reg_a,
                self._ts_exec_set_bus_iow,
            ],
        })

    @property
    def pair_bc(self) -> Data:
        """Reads register B as High-byte and register C as Low-byte to form a 16-bit word."""
        val_b, val_c = self.reg_b.read(), self.reg_c.read()
        return (val_b << 8) | val_c

    @pair_bc.setter
    def pair_bc(self, value: Data):
        """Splits a 16-bit word and writes the pieces straight into B and C."""
        self.reg_b.write(value >> 8)
        self.reg_c.write(value)

    @property
    def pair_de(self) -> Data:
        """Reads register D as High-byte and register E as Low-byte to form a 16-bit word."""
        val_d, val_e = self.reg_d.read(), self.reg_e.read()
        return (val_d << 8) | val_e

    @pair_de.setter
    def pair_de(self, value: Data):
        """Splits a 16-bit word and writes the pieces straight into D and E."""
        self.reg_d.write(value >> 8)
        self.reg_e.write(value)

    @property
    def pair_hl(self) -> Data:
        """Reads register H as High-byte and register L as Low-byte to form a 16-bit word."""
        val_h, val_l = self.reg_h.read(), self.reg_l.read()
        return (val_h << 8) | val_l

    @pair_hl.setter
    def pair_hl(self, value: Data):
        """Splits a 16-bit word and writes the pieces straight into H and L."""
        self.reg_h.write(value >> 8)
        self.reg_l.write(value)

    def reg(self, name: str) -> Register:
        """Provides the register by name."""
        return getattr(self, f"reg_{name.lower()}")

    @property
    def registers(self) -> tuple[Register, ...]:
        """Provides the registers."""
        return (self.reg_a, self.reg_b, self.reg_c, self.reg_d, self.reg_e, self.reg_h, self.reg_l)

    def __repr__(self) -> str:
        return f"CPU({', '.join(repr(reg) for reg in self.registers)})"

    def process(self, bus: SystemBus):
        """Process one t-state for a machine cycle."""
        if bus.reset_in == 1:
            self.reg_pc.write(0x0000)
            self.inte = False
            self.is_halt = False
            self._cycle = MachineCycle.FETCH
            self.t_state = 1
            bus.reset_out = Data.on()
            return
        else:
            bus.reset_out = Data.off()

        if bus.hold == 1:
            bus.hlda = Data.on()
            self._cycle = MachineCycle.HOLD
            bus.mr = Data.off()
            bus.mw = Data.off()
            bus.ior = Data.off()
            bus.iow = Data.off()
            return
        elif self._cycle == MachineCycle.HOLD:
            bus.hlda = Data.off()
            self._cycle = MachineCycle.FETCH
            self.t_state = 1

        if bus.ready == 0:
            return

        if self.t_state == 100:
            bus.address = Mem(getattr(self, '_pending_int_sp', 0))
            bus.data = Data.byte(getattr(self, '_pending_int_low', 0))
            bus.mw = Data.on()
            self.reg_sp.write(getattr(self, '_pending_int_sp', 0))
            self.reg_pc.write(getattr(self, '_pending_int_vector', 0))
            self._cycle = MachineCycle.FETCH
            self.t_state = 1
            return

        if self.is_halt:
            if self.trap or (self.inte and (self.rst_7_5 or self.rst_6_5 or self.rst_5_5 or self.intr)):
                self.is_halt = False

        if self.is_halt:
            return

        if self._cycle == MachineCycle.FETCH and self.t_state <= 1:
            if self._check_hardware_interrupts(bus):
                return

        match self._cycle:
            case MachineCycle.FETCH:
                self._fetch(bus)
            case MachineCycle.EXECUTE:
                self._execute(bus)
            case _:
                pass

    def _check_hardware_interrupts(self, bus: SystemBus) -> bool:
        """Checks and services hardware interrupts based on 8085 priority."""
        if self.trap:
            self.trap = False
            self._trigger_vector_interrupt(bus, VEC_TRAP)
            return True

        if not self.inte:
            return False

        if self.rst_7_5 and not self.mask_7_5:
            self.rst_7_5 = False
            self.pending_7_5 = False
            self._trigger_vector_interrupt(bus, VEC_RST_7_5)
            return True

        if self.rst_6_5 and not self.mask_6_5:
            self.rst_6_5 = False
            self.pending_6_5 = False
            self._trigger_vector_interrupt(bus, VEC_RST_6_5)
            return True

        if self.rst_5_5 and not self.mask_5_5:
            self.rst_5_5 = False
            self.pending_5_5 = False
            self._trigger_vector_interrupt(bus, VEC_RST_5_5)
            return True

        if self.intr:
            self.intr = False
            self.inte = False
            self._is_inta_cycle = True
            return False

        return False

    def _trigger_vector_interrupt(self, bus: SystemBus, vector_addr: int):
        """Pushes current PC and jumps to fixed interrupt vector address."""
        self.inte = False
        sp_val = self.reg_sp.read().value
        pc_val = self.reg_pc.read().value

        high_byte = (pc_val >> 8) & 0xFF
        low_byte = pc_val & 0xFF

        sp_val = (sp_val - 1) & 0xFFFF
        bus.address = Mem(sp_val)
        bus.data = Data.byte(high_byte)
        bus.mw = Data.on()

        self._pending_int_sp = (sp_val - 1) & 0xFFFF
        self._pending_int_low = low_byte
        self._pending_int_vector = vector_addr
        self.t_state = 100

    def _fetch(self, bus: SystemBus):
        """Exectues one t-state fetch machine cycle."""
        if self.t_state == 0:
            self.t_state += 1

        if self.t_state == 1:
            bus.reset()
            if getattr(self, "_is_inta_cycle", False):
                bus.inta = Data.on()
            else:
                bus.address = Mem(self.reg_pc.read().value)
                self.reg_pc.increment()
            self.t_state += 1
        elif self.t_state == 2:
            if not getattr(self, "_is_inta_cycle", False):
                bus.mr = Data.on()
            self.t_state += 1
        elif self.t_state == 3:
            self.ireg.write(Opcode(bus.data.value))
            if getattr(self, "_is_inta_cycle", False):
                bus.inta = Data.off()
                self._is_inta_cycle = False
            else:
                bus.mr = Data.off()
            self.t_state += 1
        elif self.t_state == 4:
            self._decode(bus)

    def _decode(self, bus: SystemBus):
        """Executes one t-state decode machine cycle."""
        if self.t_state == 4:
            opcode = Opcode(self.ireg.read().value)

            if opcode == Opcode.HLT:
                self.is_halt = True
                self.t_state = 0
                return

            decoder_fn = self._decoder_matrix.get(opcode.value)
            if decoder_fn:
                self._cycle = decoder_fn(bus)
            else:
                self._cycle = MachineCycle.FETCH

            self.t_state = 1

    def _execute(self, bus: SystemBus):
        """Executes one t-state execute machine cycle."""
        steps = self._dispatch_table.get(self.ireg.read().value)
        if not steps:
            self._cycle = MachineCycle.FETCH
            self.t_state = 1
            return

        step_index = self.t_state - 1
        if step_index < len(steps):
            steps[step_index](bus)

        if (self.t_state - 1) >= len(steps):
            self._cycle = MachineCycle.FETCH
            self.t_state = 1

    def _ts_exec_set_bus_addr_from_pc(self, bus: SystemBus):
        """Sets the address on system bus from program counter."""
        bus.address = Mem(self.reg_pc.read().value)
        self.reg_pc.increment()
        self.t_state += 1

    def _ts_exec_set_bus_mr(self, bus: SystemBus):
        """Enables the memory read (MR) signal on system bus."""
        bus.mr = Data.on()
        self.t_state += 1

    def _ts_exec_set_bus_mw(self, bus: SystemBus):
        """Enables the memory write (MW) signal on system bus."""
        bus.mw = Data.on()
        self.t_state += 1

    def _ts_exec_set_reg_val_from_bus_data(self, bus: SystemBus, order: Literal[0, 1, 2, 3]):
        """
        Sets the register in the instruction with immediate value and disabled memory read
        signal.
        """
        self._reg_dst.write_byte(bus.data, order)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_reg_a_val_from_bus_data(self, bus: SystemBus):
        """Sets the register a in the bus data value."""
        self.reg_a.write(bus.data)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_reg_l_val_from_bus_data(self, bus: SystemBus):
        """Sets register L from bus data value."""
        self.reg_l.write(bus.data)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_reg_h_val_from_bus_data(self, bus: SystemBus):
        """Sets register H from bus data value."""
        self.reg_h.write(bus.data)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_wz_plus_1(self, bus: SystemBus):
        """Sets bus address from WZ + 1."""
        addr = Data.words(self.reg_w.read().value, self.reg_z.read().value).value + 1
        bus.address = Mem(addr)
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_l(self, bus: SystemBus):
        """Sets bus data from register L."""
        bus.data = self.reg_l.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_h(self, bus: SystemBus):
        """Sets bus data from register H."""
        bus.data = self.reg_h.read()
        self.t_state += 1

    def _ts_exec_push_step(self, bus: SystemBus):
        """Disables MW signal, decrements SP by 1, and sets bus address from SP."""
        bus.mw = Data.off()
        self.reg_sp.decrement()
        sp_val = self.reg_sp.read().value
        bus.address = Mem(sp_val)
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_sp(self, bus: SystemBus):
        """Sets bus address from SP."""
        bus.address = Mem(self.reg_sp.read().value)
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_b(self, bus: SystemBus):
        """Sets bus data from register B."""
        bus.data = self.reg_b.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_c(self, bus: SystemBus):
        """Sets bus data from register C."""
        bus.data = self.reg_c.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_d(self, bus: SystemBus):
        """Sets bus data from register D."""
        bus.data = self.reg_d.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_e(self, bus: SystemBus):
        """Sets bus data from register E."""
        bus.data = self.reg_e.read()
        self.t_state += 1

    def _ts_exec_pop_reg_c(self, bus: SystemBus):
        """Writes bus data to C and increments SP by 1."""
        self.reg_c.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_b(self, bus: SystemBus):
        """Writes bus data to B and increments SP by 1."""
        self.reg_b.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_e(self, bus: SystemBus):
        """Writes bus data to E and increments SP by 1."""
        self.reg_e.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_d(self, bus: SystemBus):
        """Writes bus data to D and increments SP by 1."""
        self.reg_d.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_l(self, bus: SystemBus):
        """Writes bus data to L and increments SP by 1."""
        self.reg_l.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_h(self, bus: SystemBus):
        """Writes bus data to H and increments SP by 1."""
        self.reg_h.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_flag_reg(self, bus: SystemBus):
        """Sets bus data from flag register."""
        bus.data = Data(self.flag_reg.value, size=DataSize.BYTE)
        self.t_state += 1

    def _ts_exec_pop_flag_reg(self, bus: SystemBus):
        """Writes bus data to flag register and increments SP by 1."""
        self.flag_reg.value = bus.data
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_a(self, bus: SystemBus):
        """Writes bus data to register A and increments SP by 1."""
        self.reg_a.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_z(self, bus: SystemBus):
        """Writes bus data to register Z and increments SP by 1."""
        self.reg_z.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_ret_jump(self, bus: SystemBus):
        """Writes bus data to register W, increments SP by 1, and sets PC = WZ."""
        self.reg_w.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.reg_pc.write(self._reg_dst.read_word())
        self.t_state += 1

    def _ts_exec_ei(self, bus: SystemBus):
        """Enables interrupts (sets INTE = True)."""
        self.inte = True
        self.t_state += 1

    def _ts_exec_di(self, bus: SystemBus):
        """Disables interrupts (clears INTE = False)."""
        self.inte = False
        self.t_state += 1

    def _ts_exec_rim(self, bus: SystemBus):
        """Reads interrupt mask, pending flags, INTE status, and SID bit into Accumulator A."""
        val = 0
        if self.mask_5_5:
            val |= (1 << 0)
        if self.mask_6_5:
            val |= (1 << 1)
        if self.mask_7_5:
            val |= (1 << 2)
        if self.inte:
            val |= (1 << 3)
        if self.pending_5_5:
            val |= (1 << 4)
        if self.pending_6_5:
            val |= (1 << 5)
        if self.pending_7_5:
            val |= (1 << 6)
        if self.sid:
            val |= (1 << 7)

        self.reg_a.write(Data.byte(val))
        self.t_state += 1

    def _ts_exec_sim(self, bus: SystemBus):
        """Sets interrupt masks, clears RST 7.5 latch, and updates SOD pin from Accumulator A."""
        val = self.reg_a.read().value
        mse = bool(val & (1 << 3))
        if mse:
            self.mask_5_5 = bool(val & (1 << 0))
            self.mask_6_5 = bool(val & (1 << 1))
            self.mask_7_5 = bool(val & (1 << 2))

        r7_5 = bool(val & (1 << 4))
        if r7_5:
            self.pending_7_5 = False

        sde = bool(val & (1 << 6))
        if sde:
            self.sod = bool(val & (1 << 7))

        self.t_state += 1

    def _ts_exec_set_bus_addr_from_z_port(self, bus: SystemBus):
        """Sets bus address from register Z for I/O port operation."""
        bus.mr = Data.off()
        port_val = self.reg_z.read().value & 0xFF
        bus.address = Mem((port_val << 8) | port_val)
        self.t_state += 1

    def _ts_exec_set_bus_ior(self, bus: SystemBus):
        """Enables I/O Read (IOR) signal on system bus."""
        bus.ior = Data.on()
        self.t_state += 1

    def _ts_exec_in_read_a(self, bus: SystemBus):
        """Reads bus data into Accumulator A and disables IOR."""
        self.reg_a.write(bus.data)
        bus.ior = Data.off()
        self._cycle = MachineCycle.FETCH
        self.t_state = 1

    def _ts_exec_set_bus_iow(self, bus: SystemBus):
        """Enables I/O Write (IOW) signal on system bus and finishes cycle."""
        bus.iow = Data.on()
        self._cycle = MachineCycle.FETCH
        self.t_state = 1

    def _ts_exec_set_bus_data_from_pc_high(self, bus: SystemBus):
        """Sets bus data from PC High byte."""
        bus.data = Data.byte(self.reg_pc.read().byte_at(1))
        self.t_state += 1

    def _ts_exec_set_bus_data_from_pc_low(self, bus: SystemBus):
        """Sets bus data from PC Low byte."""
        bus.data = Data.byte(self.reg_pc.read().byte_at(0))
        self.t_state += 1

    def _ts_exec_call_jump(self, bus: SystemBus):
        """Enables memory write for return address low byte and jumps to WZ."""
        bus.mw = Data.on()
        self.reg_pc.write(self._reg_dst.read_word())
        self.t_state += 1

    def _ts_exec_rst_jump(self, bus: SystemBus):
        """Sets PC to the restart vector address (8 * n) based on RST opcode."""
        bus.mw = Data.off()
        opcode_val = self.ireg.read().value
        vector_num = (opcode_val >> 3) & 0x07
        vectors = (VEC_RST_0, VEC_RST_1, VEC_RST_2, VEC_RST_3, VEC_RST_4, VEC_RST_5, VEC_RST_6, VEC_RST_7)
        self.reg_pc.write(vectors[vector_num])
        self._cycle = MachineCycle.FETCH
        self.t_state = 1

    def _ts_exec_xthl_read_l(self, bus: SystemBus):
        """Swaps L with bus data and puts old L on bus data for memory write."""
        old_l = self.reg_l.read()
        self.reg_l.write(bus.data)
        bus.mr = Data.off()
        bus.data = old_l
        self.t_state += 1

    def _ts_exec_xthl_read_h(self, bus: SystemBus):
        """Swaps H with bus data and puts old H on bus data for memory write."""
        old_h = self.reg_h.read()
        self.reg_h.write(bus.data)
        bus.mr = Data.off()
        bus.data = old_h
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_sp_plus_1(self, bus: SystemBus):
        """Sets bus address from SP + 1."""
        bus.mw = Data.off()
        bus.address = Mem((self.reg_sp.read().value + 1) & 0xFFFF)
        self.t_state += 1

    def _ts_exec_sphl(self, bus: SystemBus):
        """Loads stack pointer SP from register pair HL."""
        self.reg_sp.write(self.pair_hl.value)
        self.t_state += 1

    def _ts_exec_pchl(self, bus: SystemBus):
        """Loads program counter PC from register pair HL."""
        self.reg_pc.write(self.pair_hl.value)
        self.t_state += 1

    def _ts_exec_nop(self, bus: SystemBus):
        """No operation."""
        self.t_state += 1

    def _ts_exec_xchg(self, bus: SystemBus):
        """Exchanges contents of DE and HL register pairs."""
        val_d = self.reg_d.read().value
        val_e = self.reg_e.read().value
        val_h = self.reg_h.read().value
        val_l = self.reg_l.read().value

        self.reg_d.write(val_h)
        self.reg_e.write(val_l)
        self.reg_h.write(val_d)
        self.reg_l.write(val_e)

        self.t_state += 1

    def _ts_exec_set_bus_addr_from_hl_reg(self, bus: SystemBus):
        """Sets bus address from HL register pair."""
        bus.address = Mem(Data.words(
            self.reg_h.read().value,
            self.reg_l.read().value,
        ).value)
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_bc_reg(self, bus: SystemBus):
        """Sets bus address from BC register pair."""
        bus.address = Mem(Data.words(
            self.reg_b.read().value,
            self.reg_c.read().value,
        ).value)
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_de_reg(self, bus: SystemBus):
        """Sets bus address from DE register pair."""
        bus.address = Mem(Data.words(
            self.reg_d.read().value,
            self.reg_e.read().value,
        ).value)
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_wz_reg(self, bus: SystemBus):
        """Sets bus address from WZ register pair."""
        bus.address = Mem(Data.words(
            self.reg_w.read().value,
            self.reg_z.read().value,
        ).value)
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_a(self, bus: SystemBus):
        """Sets bus data from register A."""
        bus.data = self.reg_a.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg(self, bus: SystemBus):
        """Sets bus data from register."""
        bus.data = self._reg_src.read_byte(order=0)
        self.t_state += 1

    def _ts_exce_set_reg_from_reg(self, bus: SystemBus):
        """Sets register from another register."""
        data = self._reg_src.read_byte(order=0)
        self._reg_dst.write_byte(data, order=0)
        self.t_state += 1

    def _ts_exec_set_tmp_reg_val_from_bus_data(self, bus: SystemBus):
        """Sets temp register value from bus data."""
        self.reg_tmp.write(bus.data)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_tmp_reg(self, bus: SystemBus):
        """Sets bus data from temp register value."""
        bus.data = self.reg_tmp.read()
        self.t_state += 1

    def _ts_exec_internal_delay(self, bus: SystemBus):
        """Internal operation cycle / bus idle state."""
        self.t_state += 1

    def _ts_exec_read_imm_and_exec(self, bus: SystemBus):
        """Reads immediate data byte from bus and executes arithmetic operation."""
        self.reg_tmp.write(bus.data)
        bus.mr = Data.off()
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.ADI:
            self._ts_exec_add(bus)
        elif opcode == Opcode.ACI:
            self._ts_exec_add_with_carry(bus)
        elif opcode == Opcode.SUI:
            self._ts_exec_sub(bus)
        elif opcode == Opcode.SBI:
            self._ts_exec_sub_with_borrow(bus)
        elif opcode == Opcode.ANI:
            self._ts_exec_ana(bus)
        elif opcode == Opcode.ORI:
            self._ts_exec_ora(bus)
        elif opcode == Opcode.XRI:
            self._ts_exec_xra(bus)
        elif opcode == Opcode.CPI:
            self._ts_exec_cmp(bus)

    def _ts_exec_read_mem_and_exec(self, bus: SystemBus):
        """Reads memory byte from bus and executes arithmetic operation."""
        self.reg_tmp.write(bus.data)
        bus.mr = Data.off()
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.ADD_M:
            self._ts_exec_add(bus)
        elif opcode == Opcode.ADC_M:
            self._ts_exec_add_with_carry(bus)
        elif opcode == Opcode.SUB_M:
            self._ts_exec_sub(bus)
        elif opcode == Opcode.SBB_M:
            self._ts_exec_sub_with_borrow(bus)
        elif opcode == Opcode.ANA_M:
            self._ts_exec_ana(bus)
        elif opcode == Opcode.ORA_M:
            self._ts_exec_ora(bus)
        elif opcode == Opcode.XRA_M:
            self._ts_exec_xra(bus)
        elif opcode == Opcode.CMP_M:
            self._ts_exec_cmp(bus)

    def _ts_exec_add(self, bus: SystemBus):
        """Adds the register in insturction with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.ADD_M, Opcode.ADI):
            self.reg_tmp.write(self._reg_src.read_byte().value)
        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value

        res = val1 + val2
        res8 = res & 0xFF
        res4 = (val1 & 0x0F) + (val2 & 0x0F)

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res8)
        self.flag_reg.carry = (res >> 8) & 1
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = (res4 >> 4) & 1
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_add_with_carry(self, bus: SystemBus):
        """Adds the register in instruction and Carry flag with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.ADC_M, Opcode.ACI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value

        c_in = self.flag_reg.carry
        self.reg_tmp.write(val1)

        res = val1 + val2 + c_in
        res8 = res & 0xFF
        res4 = (val1 & 0x0F) + (val2 & 0x0F) + c_in

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res8)
        self.flag_reg.carry = (res >> 8) & 1
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = (res4 >> 4) & 1
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_sub(self, bus: SystemBus):
        """Subtracts the register/temp value from register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.SUB_M, Opcode.SUI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value

        res = val2 - val1
        res8 = res & 0xFF
        res4 = (val2 & 0x0F) - (val1 & 0x0F)

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res8)
        self.flag_reg.carry = 1 if res < 0 else 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1 if res4 < 0 else 0
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_sub_with_borrow(self, bus: SystemBus):
        """Subtracts the register/temp value and Carry flag from register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.SBB_M, Opcode.SBI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value
        c_in = self.flag_reg.carry

        res = val2 - val1 - c_in
        res8 = res & 0xFF
        res4 = (val2 & 0x0F) - (val1 & 0x0F) - c_in

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res8)
        self.flag_reg.carry = 1 if res < 0 else 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1 if res4 < 0 else 0
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_inr(self, bus: SystemBus):
        """Increments the register/temp value by 1 and updates status flags."""
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.INR_M:
            val = self.reg_tmp.read().value
        else:
            val = self._reg_dst.read_byte().value

        res = (val + 1) & 0xFF
        res4 = (val & 0x0F) + 1

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        if opcode == Opcode.INR_M:
            self.reg_tmp.write(res)
        else:
            self._reg_dst.write_byte(res)

        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = (res4 >> 4) & 1
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_dcr(self, bus: SystemBus):
        """Decrements the register/temp value by 1 and updates status flags."""
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.DCR_M:
            val = self.reg_tmp.read().value
        else:
            val = self._reg_dst.read_byte().value

        res = (val - 1) & 0xFF
        res4 = (val & 0x0F) - 1

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        if opcode == Opcode.DCR_M:
            self.reg_tmp.write(res)
        else:
            self._reg_dst.write_byte(res)

        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1 if res4 < 0 else 0
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_ana(self, bus: SystemBus):
        """Performs logical AND of register/temp value with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.ANA_M, Opcode.ANI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value
        res = val1 & val2

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_ora(self, bus: SystemBus):
        """Performs logical OR of register/temp value with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.ORA_M, Opcode.ORI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value
        res = val1 | val2

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 0
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_xra(self, bus: SystemBus):
        """Performs logical XOR of register/temp value with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.XRA_M, Opcode.XRI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value
        res = val1 ^ val2

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 0
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_cmp(self, bus: SystemBus):
        """Compares register/temp value with register A and updates status flags."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.CMP_M, Opcode.CPI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value

        res = val2 - val1
        res8 = res & 0xFF
        res4 = (val2 & 0x0F) - (val1 & 0x0F)

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.flag_reg.carry = 1 if res < 0 else 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1 if res4 < 0 else 0
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_rlc(self, bus: SystemBus):
        """Rotates register A left circular; updates Carry flag."""
        val = self.reg_a.read().value
        bit7 = (val >> 7) & 1
        res = ((val << 1) & 0xFE) | bit7

        self.reg_a.write(res)
        self.flag_reg.carry = bit7
        self.t_state += 1

    def _ts_exec_rrc(self, bus: SystemBus):
        """Rotates register A right circular; updates Carry flag."""
        val = self.reg_a.read().value
        bit0 = val & 1
        res = ((val >> 1) & 0x7F) | (bit0 << 7)

        self.reg_a.write(res)
        self.flag_reg.carry = bit0
        self.t_state += 1

    def _ts_exec_ral(self, bus: SystemBus):
        """Rotates register A left through Carry flag; updates Carry flag."""
        val = self.reg_a.read().value
        c_in = self.flag_reg.carry
        bit7 = (val >> 7) & 1
        res = ((val << 1) & 0xFE) | c_in

        self.reg_a.write(res)
        self.flag_reg.carry = bit7
        self.t_state += 1

    def _ts_exec_rar(self, bus: SystemBus):
        """Rotates register A right through Carry flag; updates Carry flag."""
        val = self.reg_a.read().value
        c_in = self.flag_reg.carry
        bit0 = val & 1
        res = ((val >> 1) & 0x7F) | (c_in << 7)

        self.reg_a.write(res)
        self.flag_reg.carry = bit0
        self.t_state += 1

    def _ts_exec_cma(self, bus: SystemBus):
        """Complements register A value; status flags are unchanged."""
        val = self.reg_a.read().value
        self.reg_a.write((~val) & 0xFF)
        self.t_state += 1

    def _ts_exec_cmc(self, bus: SystemBus):
        """Complements Carry flag; updates Carry flag."""
        self.flag_reg.carry = (~self.flag_reg.carry) & 1
        self.t_state += 1

    def _ts_exec_stc(self, bus: SystemBus):
        """Sets Carry flag to 1; updates Carry flag."""
        self.flag_reg.carry = 1
        self.t_state += 1

    def _ts_exec_daa(self, bus: SystemBus):
        """Decimal adjusts register A value after addition; updates status flags."""
        val = self.reg_a.read().value
        c_in = self.flag_reg.carry
        ac_in = self.flag_reg.aux

        inc = 0
        carry = c_in
        aux = 0

        if (val & 0x0F) > 9 or ac_in == 1:
            inc += 0x06
            aux = 1

        if val > 0x99 or c_in == 1:
            inc += 0x60
            carry = 1

        res = (val + inc) & 0xFF

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = carry
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = aux
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_das(self, bus: SystemBus):
        """Decimal adjusts register A value after subtraction; updates status flags."""
        val = self.reg_a.read().value
        c_in = self.flag_reg.carry
        ac_in = self.flag_reg.aux

        res = val
        carry = c_in
        aux = 0

        if (val & 0x0F) > 9 or ac_in == 1:
            res = (res - 0x06) & 0xFF
            aux = 1

        if val > 0x99 or c_in == 1:
            res = (res - 0x60) & 0xFF
            carry = 1

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = carry
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = aux
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_aaa(self, bus: SystemBus):
        """ASCII adjusts register A value after addition; updates status flags."""
        val = self.reg_a.read().value
        ac_in = self.flag_reg.aux

        if (val & 0x0F) > 9 or ac_in == 1:
            res = (val + 0x06) & 0x0F
            carry = 1
            aux = 1
        else:
            res = val & 0x0F
            carry = 0
            aux = 0

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = carry
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = aux
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_aas(self, bus: SystemBus):
        """ASCII adjusts register A value after subtraction; updates status flags."""
        val = self.reg_a.read().value
        ac_in = self.flag_reg.aux

        if (val & 0x0F) > 9 or ac_in == 1:
            res = (val - 0x06) & 0x0F
            carry = 1
            aux = 1
        else:
            res = val & 0x0F
            carry = 0
            aux = 0

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = carry
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = aux
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_inx(self, bus: SystemBus):
        """Increments 16-bit register pair by 1; status flags are unchanged."""
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.INX_BC:
            self.pair_bc = Data.word(self.pair_bc.value + 1)
        elif opcode == Opcode.INX_DE:
            self.pair_de = Data.word(self.pair_de.value + 1)
        elif opcode == Opcode.INX_HL:
            self.pair_hl = Data.word(self.pair_hl.value + 1)
        elif opcode == Opcode.INX_SP:
            self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_dcx(self, bus: SystemBus):
        """Decrements 16-bit register pair by 1; status flags are unchanged."""
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.DCX_BC:
            self.pair_bc = Data.word(self.pair_bc.value - 1)
        elif opcode == Opcode.DCX_DE:
            self.pair_de = Data.word(self.pair_de.value - 1)
        elif opcode == Opcode.DCX_HL:
            self.pair_hl = Data.word(self.pair_hl.value - 1)
        elif opcode == Opcode.DCX_SP:
            self.reg_sp.decrement()
        self.t_state += 1

    def _ts_exec_dad(self, bus: SystemBus):
        """Adds 16-bit register pair to HL pair; updates Carry flag only."""
        opcode = Opcode(self.ireg.read().value)
        val_hl = self.pair_hl.value

        if opcode == Opcode.DAD_BC:
            val_rp = self.pair_bc.value
        elif opcode == Opcode.DAD_DE:
            val_rp = self.pair_de.value
        elif opcode == Opcode.DAD_HL:
            val_rp = val_hl
        elif opcode == Opcode.DAD_SP:
            val_rp = self.reg_sp.read().value
        else:
            val_rp = 0

        res = val_hl + val_rp
        self.pair_hl = Data.word(res)
        self.flag_reg.carry = (res >> 16) & 1
        self.t_state += 1

    def _ts_exec_set_pc_from_reg_wz(self, bus: SystemBus):
        """Sets program counter register with address in WZ register pair."""
        self._reg_dst.write_byte(bus.data, 1)
        bus.mr = Data.off()
        self.reg_pc.write(self._reg_dst.read_word())
        self.t_state += 1

    def _ts_exec_cond_jump(self, bus: SystemBus):
        """Reads High byte into W, evaluates opcode jump condition, and updates PC if condition is met."""
        self._reg_dst.write_byte(bus.data, 1)
        bus.mr = Data.off()

        opcode = Opcode(self.ireg.read().value)
        jump = False
        if opcode == Opcode.JZ and self.flag_reg.zero == 1:
            jump = True
        elif opcode == Opcode.JNZ and self.flag_reg.zero == 0:
            jump = True
        elif opcode == Opcode.JC and self.flag_reg.carry == 1:
            jump = True
        elif opcode == Opcode.JNC and self.flag_reg.carry == 0:
            jump = True
        elif opcode == Opcode.JP and self.flag_reg.sign == 0:
            jump = True
        elif opcode == Opcode.JM and self.flag_reg.sign == 1:
            jump = True
        elif opcode == Opcode.JPE and self.flag_reg.parity == 1:
            jump = True
        elif opcode == Opcode.JPO and self.flag_reg.parity == 0:
            jump = True

        if jump:
            self.reg_pc.write(self._reg_dst.read_word())

        self.t_state += 1

    def _ts_exec_cond_call(self, bus: SystemBus):
        """Reads High byte into W, evaluates opcode call condition. If met, continues subroutine call; else ends cycle."""
        self._reg_dst.write_byte(bus.data, 1)
        bus.mr = Data.off()

        opcode = Opcode(self.ireg.read().value)
        call = False
        if opcode == Opcode.CZ and self.flag_reg.zero == 1:
            call = True
        elif opcode == Opcode.CNZ and self.flag_reg.zero == 0:
            call = True
        elif opcode == Opcode.CC and self.flag_reg.carry == 1:
            call = True
        elif opcode == Opcode.CNC and self.flag_reg.carry == 0:
            call = True
        elif opcode == Opcode.CP and self.flag_reg.sign == 0:
            call = True
        elif opcode == Opcode.CM and self.flag_reg.sign == 1:
            call = True
        elif opcode == Opcode.CPE and self.flag_reg.parity == 1:
            call = True
        elif opcode == Opcode.CPO and self.flag_reg.parity == 0:
            call = True

        if call:
            self.t_state += 1
        else:
            self._cycle = MachineCycle.FETCH
            self.t_state = 1

    def _ts_exec_cond_ret_check(self, bus: SystemBus):
        """Evaluates opcode return condition. If met, continues return sequence; else ends cycle."""
        opcode = Opcode(self.ireg.read().value)
        ret = False
        if opcode == Opcode.RZ and self.flag_reg.zero == 1:
            ret = True
        elif opcode == Opcode.RNZ and self.flag_reg.zero == 0:
            ret = True
        elif opcode == Opcode.RC and self.flag_reg.carry == 1:
            ret = True
        elif opcode == Opcode.RNC and self.flag_reg.carry == 0:
            ret = True
        elif opcode == Opcode.RP and self.flag_reg.sign == 0:
            ret = True
        elif opcode == Opcode.RM and self.flag_reg.sign == 1:
            ret = True
        elif opcode == Opcode.RPE and self.flag_reg.parity == 1:
            ret = True
        elif opcode == Opcode.RPO and self.flag_reg.parity == 0:
            ret = True

        if ret:
            self.t_state += 1
        else:
            self._cycle = MachineCycle.FETCH
            self.t_state = 1


class Device(ABC):
    """Abstract base class for peripheral devices."""

    @property
    @abstractmethod
    def name(self) -> str:
        """Name of the peripheral device."""
        pass

    def port_read(self, port: int) -> int:
        """Called when CPU performs I/O Read (IN port)."""
        return 0xFF

    def port_write(self, port: int, data: int) -> None:
        """Called when CPU performs I/O Write (OUT port)."""
        pass

    def mem_read(self, addr: int) -> int | None:
        """Called for memory-mapped read."""
        return None

    def mem_write(self, addr: int, data: int) -> bool:
        """Called for memory-mapped write."""
        return False

    def on_inta(self) -> int:
        """Called during INTA cycle to get vector opcode."""
        return 0xFF

    def tick(self, bus: SystemBus) -> None:
        """Simulates device logic per clock cycle."""
        pass


@dataclass(repr=False)
class DeviceManager:
    """Manages system peripheral devices and I/O routing."""

    devices: list[Device] = field(default_factory=list)
    port_map: dict[int, Device] = field(default_factory=dict)

    def attach_device(self, device: Device, ports: Sequence[int] = ()):
        """Attaches a device and maps it to optional I/O ports."""
        self.devices.append(device)
        for p in ports:
            self.port_map[p & 0xFF] = device

    def read_port(self, port: int) -> int:
        """Reads byte from port if a device is attached, else returns 0xFF."""
        dev = self.port_map.get(port & 0xFF)
        if dev:
            return dev.port_read(port & 0xFF)
        return 0xFF

    def write_port(self, port: int, data: int) -> None:
        """Writes byte to port if a device is attached."""
        dev = self.port_map.get(port & 0xFF)
        if dev:
            dev.port_write(port & 0xFF, data & 0xFF)

    def step(self, bus: SystemBus):
        """Processes bus I/O signals and ticks all attached devices on each clock cycle."""
        if bus.ior == 1:
            port = bus.address & 0xFF
            data_val = self.read_port(port)
            bus.data = Data(data_val, size=DataSize.BYTE)
        elif bus.iow == 1:
            port = bus.address & 0xFF
            self.write_port(port, bus.data.value)
        elif bus.inta == 1:
            for device in self.devices:
                val = device.on_inta()
                if val != 0xFF:
                    bus.data = Data(val, size=DataSize.BYTE)
                    break

        for device in self.devices:
            device.tick(bus)


@dataclass
class KeyboardDevice(Device):
    """Keyboard peripheral device that captures ASCII key presses (0-127)."""

    _buffer: list[int] = field(init=False, default_factory=list)
    interrupt_vector: int | None = None

    @property
    def name(self) -> str:
        """Name of the device."""
        return "KeyboardDevice"

    def trigger_key_press(self, key: str | int) -> None:
        """Triggers a keypress event for ASCII values (0-127)."""
        ascii_code = ord(key) if isinstance(key, str) else key
        if not (0 <= ascii_code <= 127):
            raise ValueError(f"ASCII code out of range (0-127): {ascii_code}")
        self._buffer.append(ascii_code)

    def port_read(self, port: int) -> int:
        """Reads the next ASCII key byte from the buffer if available, else 0x00."""
        if self._buffer:
            return self._buffer.pop(0)
        return 0x00

    def has_key(self) -> bool:
        """Returns True if there is a pending key in the buffer."""
        return len(self._buffer) > 0

    def on_inta(self) -> int:
        """Returns the RST n opcode for the configured interrupt vector (0-7)."""
        if self.interrupt_vector is not None and 0 <= self.interrupt_vector <= 7:
            return 0xC7 | ((self.interrupt_vector & 0x07) << 3)
        return 0xFF


@dataclass
class USBDevice(Device):
    """USB peripheral device that can perform DMA memory reads and writes."""

    buffer: bytearray = field(default_factory=bytearray)

    @property
    def name(self) -> str:
        """Name of the device."""
        return "USBDevice"

    def dma_read(self, machine: Any, start_addr: int, length: int) -> bytes:
        """Reads memory via DMA protocol (HOLD -> HLDA -> Memory Read -> Release HOLD)."""
        bus, ram, cpu = machine.bus, machine.ram, machine.cpu
        bus.hold = Data.on()
        machine.tick()

        data_bytes = bytearray()
        if bus.hlda == 1 or cpu._cycle == MachineCycle.HOLD:
            for i in range(length):
                addr = start_addr + i
                data_bytes.append(ram.read(Mem(addr)).value)

        bus.hold = Data.off()
        machine.tick()
        self.buffer = data_bytes
        return bytes(data_bytes)

    def dma_write(self, machine: Any, start_addr: int, data: bytes | bytearray) -> None:
        """Writes data into memory via DMA protocol (HOLD -> HLDA -> Memory Write -> Release HOLD)."""
        bus, ram, cpu = machine.bus, machine.ram, machine.cpu
        bus.hold = Data.on()
        machine.tick()

        if bus.hlda == 1 or cpu._cycle == MachineCycle.HOLD:
            for i, byte_val in enumerate(data):
                addr = start_addr + i
                ram.write(Mem(addr), Data.byte(byte_val))

        bus.hold = Data.off()
        machine.tick()


@dataclass
class PrinterDevice(Device):
    """Printer peripheral device that outputs received character bytes via callback."""

    output_callback: Callable[[str], None] = print
    history: list[str] = field(default_factory=list)

    @property
    def name(self) -> str:
        """Name of the device."""
        return "PrinterDevice"

    def port_write(self, port: int, data: int) -> None:
        """Called when CPU performs OUT to printer port."""
        char_val = chr(data & 0xFF)
        self.history.append(char_val)
        self.output_callback(char_val)


@dataclass(repr=False)
class Machine:
    """Computing machine."""

    ram: Memory
    """Random access memory."""

    cpu: CPU
    """Central processing unit."""

    bus: SystemBus
    """System bus."""

    device_manager: DeviceManager
    """Device manager for attached peripherals."""

    @classmethod
    def create(cls,
        address_lines: int = 16,
        data_lines: int = 8,
        devices: Sequence[tuple[Device, Sequence[int]]] | None = None,
    ) -> Self:
        """Creates machine parts with configs."""
        dev_mgr = DeviceManager()
        if devices:
            for dev, ports in devices:
                dev_mgr.attach_device(dev, ports)

        return cls(
            ram=Memory.from_lines(address_lines),
            cpu=CPU(),
            bus=SystemBus.create(address_lines, data_lines),
            device_manager=dev_mgr,
        )

    def load(self, program: Program, mem: Mem):
        """Loads the program in ram."""
        self.ram.write_code(program.compile(mem), mem)
        self.cpu.reg_pc.write(mem)

    def run(self):
        """Run the machine."""
        self.cpu.is_halt = False
        while not self.cpu.is_halt:
            self.tick()

    def tick(self):
        """Simulates one clock cycle."""
        self.cpu.process(self.bus)
        self.ram.step(self.bus)
        self.device_manager.step(self.bus)


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


def hexdump(mem: Memory, size: int = 16, lines: int = -1):
    """Dumps memory data to stdout."""
    lines = len(mem) // size if lines == -1 else lines
    for offset in range(0, len(mem), size):
        chunk = mem.data[offset:offset+size]
        try:
            print(f"0x{offset:04X}: {" ".join(f"{b:02X}" for b in chunk)}")
        except BrokenPipeError:
            break
        if (offset // size) > lines:
            break
