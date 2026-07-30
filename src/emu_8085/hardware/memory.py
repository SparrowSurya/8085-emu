"""
This module provides memory hardware component.
"""


import math
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Iterable, Self

from emu_8085.core import Data, DataSize, MachineCode, Mask, Mem

from .system_bus import SystemBus

__all__ = (
    "Memory",
)

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

    def write_code(self, machine_code: MachineCode, mem: Mem) -> int:
        """Write the code into memroy from location. Returns the bytes wrote."""
        i = int(mem)
        for code in machine_code:
            values = bytes(code)
            for j in range(len(values)):
                self.write(Mem(i+j), values[j])
            i += len(values)
        return i - int(mem)

    def write_seq(self, seq: Data | Sequence[int] | str | bytes, mem: Mem) -> int:
        """Writes the sequence into memory from location. Returns the bytes wrote."""
        data: Iterable[int] = (
            map(int, bytes(seq)) if isinstance(seq, Data)
            else map(int, seq.encode('utf-8')) if isinstance(seq, str)
            else map(int, seq)
        )

        i = int(mem)
        for val in data:
            self.write(Mem(i), val)
            i += 1
        return i - int(mem)
