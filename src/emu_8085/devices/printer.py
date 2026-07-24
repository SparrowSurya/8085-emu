"""
This module provides printer device.
"""

from dataclasses import dataclass, field
from typing import Callable

from emu_8085.hardware.device import Device

__all__ = (
    "PrinterDevice",
)


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
