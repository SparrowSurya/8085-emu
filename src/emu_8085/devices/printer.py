"""
This module provides printer device.
"""

from dataclasses import dataclass, field
from typing import Callable

from emu_8085.core import Data, Mem
from emu_8085.hardware import Memory
from emu_8085.hardware.device import Device

__all__ = (
    "PrinterDevice",
)


@dataclass
class PrinterDevice(Device):
    """Printer peripheral device that outputs received character bytes via callback."""

    output_callback: Callable[[str], None] = print
    history: list[str] = field(default_factory=list)
    memory: Memory = field(default_factory=lambda: Memory.from_lines(8))
    _write_ptr: int = field(init=False, default=0)

    @property
    def name(self) -> str:
        """Name of the device."""
        return "PrinterDevice"

    def port_write(self, port: int, data: int) -> None:
        """Called when CPU performs OUT to printer port."""
        self.memory.write(Mem(self._write_ptr), Data.byte(data))
        self._write_ptr = (self._write_ptr + 1) % len(self.memory)

        char_val = chr(data & 0xFF)
        self.history.append(char_val)
        self.output_callback(char_val)
