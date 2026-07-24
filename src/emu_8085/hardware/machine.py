"""
This module provides a computation machine.
"""

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Self

from emu_8085.core import Mem
from emu_8085.program import Program

from .cpu import CPU
from .device import Device, DeviceManager
from .memory import Memory
from .system_bus import SystemBus

__all__ = (
    "Machine",
)


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
