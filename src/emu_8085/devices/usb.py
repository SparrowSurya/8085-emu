"""
This module provides usb device.
"""

from dataclasses import dataclass, field
from typing import Any

from emu_8085.core import Data, Mem
from emu_8085.hardware.cpu import MachineCycle
from emu_8085.hardware.device import Device
from emu_8085.hardware.machine import Machine

__all__ = (
    "USBDevice",
)


@dataclass
class USBDevice(Device):
    """USB peripheral device that can perform DMA memory reads and writes."""

    buffer: bytearray = field(default_factory=bytearray)

    @property
    def name(self) -> str:
        """Name of the device."""
        return "USBDevice"

    def dma_read(self, machine: Machine, start_addr: int, length: int) -> bytes:
        """Reads memory via DMA protocol (HOLD -> HLDA -> Memory Read -> Release HOLD)."""
        bus, ram, cpu = machine.bus, machine.ram, machine.cpu
        bus.hold = Data.on()
        machine.tick()

        data_bytes = bytearray()
        if bus.hlda == 1 or cpu.cycle == MachineCycle.HOLD:
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

