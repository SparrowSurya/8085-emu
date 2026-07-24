"""
This module provides abstract device base class and device manager hardware component.
"""

from abc import ABC, abstractmethod
from collections.abc import Sequence
from dataclasses import dataclass, field

from emu_8085.core import Data, DataSize

from .system_bus import SystemBus

__all__ = (
    "Device",
)


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
