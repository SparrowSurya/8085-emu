"""
emu_8085: A High-Fidelity, Cycle-Accurate 8085 Microprocessor Emulator.

This library models the internal CPU registers, flag registers, system bus control lines,
memory space, T-states, machine cycles, peripheral hardware devices, interrupts, and
direct memory access (DMA) transfers of the Intel 8085 microprocessor.

Key Features:
    * Cycle-Accurate Timing: Simulates state-by-state execution timing (T-states).
    * Comprehensive Instruction Set: Supports data transfer, arithmetic, logic, branching,
        stack, and control instructions.
    * Interrupt Controller: Supports TRAP, RST 7.5, RST 6.5, RST 5.5, INTR, and software interrupts.
    * DMA Support: Simulates HOLD/HLDA bus master takeover handshaking.
    * Custom Peripheral Devices: Simple mapping of custom status/data register devices.

Modules & Exports:
    - Core Datatypes: Data, DataSize, Mem, Mask, MaskedData
    - Interrupt Vectors: VEC_RST_0 through VEC_RST_7, VEC_TRAP, VEC_RST_5_5, etc.
    - Hardware Models: Machine, Memory, Device
    - Peripheral Devices: KeyboardDevice, PrinterDevice, USBDevice
    - Program Structure: Instruction, Opcode

Quick Example:
    >>> from emu_8085 import Machine, Program, Instruction, Opcode, Data, Mem
    >>> # 1. Create a machine instance
    >>> machine = Machine.create(address_lines=16, data_lines=8)
    >>> # 2. Define assembly instructions
    >>> program = Program([
    ...     Instruction(Opcode.MVI_A, Data.byte(0x05)),
    ...     Instruction(Opcode.HLT)
    ... ])
    >>> # 3. Load program and run
    >>> machine.load(program, Mem(0x0000))
    >>> machine.run()
    >>> print(machine.cpu.reg_a.value)
    5
"""

from .core import (
    VEC_RST_0,
    VEC_RST_1,
    VEC_RST_2,
    VEC_RST_3,
    VEC_RST_4,
    VEC_RST_5,
    VEC_RST_5_5,
    VEC_RST_6,
    VEC_RST_6_5,
    VEC_RST_7,
    VEC_RST_7_5,
    VEC_TRAP,
    Data,
    DataSize,
    Mask,
    MaskedData,
    Mem,
)
from .devices import KeyboardDevice, PrinterDevice, USBDevice
from .hardware import Device, Machine, Memory
from .program import Instruction, Opcode

__all__ = (
    "Data",
    "DataSize",
    "Mask",
    "MaskedData",
    "Mem",
    "VEC_RST_0",
    "VEC_RST_1",
    "VEC_RST_2",
    "VEC_RST_3",
    "VEC_RST_4",
    "VEC_RST_5",
    "VEC_RST_6",
    "VEC_RST_7",
    "VEC_TRAP",
    "VEC_RST_5_5",
    "VEC_RST_6_5",
    "VEC_RST_7_5",
    "KeyboardDevice",
    "PrinterDevice",
    "USBDevice",
    "Device",
    "Machine",
    "Memory",
    "Opcode",
    "Instruction"
)
