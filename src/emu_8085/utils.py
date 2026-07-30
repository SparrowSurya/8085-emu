"""
This module provide helper utility functions.
"""

from typing import Iterator

from emu_8085.hardware.memory import Memory

__all__ = (
    "hexview",
)


def hexview(
    mem: Memory,
    size: int = 16,
    lines: int = -1,
) -> Iterator[tuple[int, tuple[int, ...]]]:
    """Reads the memory at once for given size."""
    lines = len(mem) // size if lines == -1 else lines
    for offset in range(0, len(mem), size):
        chunk = mem.data[offset:offset+size]
        yield (offset, tuple(b for b in chunk))
        if (offset // size) > lines:
            break
