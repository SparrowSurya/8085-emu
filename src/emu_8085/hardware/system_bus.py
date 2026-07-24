"""
This module provides system bus hardware component.
"""

from dataclasses import dataclass, field
from typing import Self

from emu_8085.core import Data, Mask, MaskedData, Mem

__all__ = (
    "SystemBus",
)


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
