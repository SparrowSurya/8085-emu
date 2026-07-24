"""
This module provides the keyboard device.
"""


from dataclasses import dataclass, field

from emu_8085.hardware.device import Device

__all__ = (
    "KeyboardDevice",
)


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

