"""
This module provides hardware devices.
"""

from .keyboard import KeyboardDevice
from .printer import PrinterDevice
from .usb import USBDevice

__all__ = (
    "KeyboardDevice",
    "PrinterDevice",
    "USBDevice",
)
