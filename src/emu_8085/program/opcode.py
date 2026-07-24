"""
This module provides the opcodes in the 8085 microprocessor.
"""

from enum import IntEnum
from typing import Self

__all__ = (
    "Opcode",
)


class Opcode(IntEnum):
    """Instruction opcode."""

    NOP = 0x00
    """No operation."""

    HLT = 0x76
    """Stops the processor execution loop."""

    # MVI (Move Immediate 8-bit)
    MVI_B = 0x06
    """Loads immediate byte value into register 'B'."""
    MVI_C = 0x0E
    """Loads immediate byte value into register 'C'."""
    MVI_D = 0x16
    """Loads immediate byte value into register 'D'."""
    MVI_E = 0x1E
    """Loads immediate byte value into register 'E'."""
    MVI_H = 0x26
    """Loads immediate byte value into register 'H'."""
    MVI_L = 0x2E
    """Loads immediate byte value into register 'L'."""
    MVI_M = 0x36
    """Writes immediate byte value to memory address pointed by 'HL'."""
    MVI_A = 0x3E
    """Loads immediate byte value into register 'A'."""

    # LXI / MVI 16-bit register pair
    MVI_BC = 0x01
    """Loads 16-bit immediate word into register pair 'BC'."""
    MVI_DE = 0x11
    """Loads 16-bit immediate word into register pair 'DE'."""
    MVI_HL = 0x21
    """Loads 16-bit immediate word into register pair 'HL'."""
    LXI = MVI_HL
    """Alternative alias: Loads 16-bit memory address into register pair 'HL'."""

    # MOV Memory (Destination = Memory M)
    MOV_M_B = 0x70
    """Writes register 'B' value to memory address pointed by 'HL'."""
    MOV_M_C = 0x71
    """Writes register 'C' value to memory address pointed by 'HL'."""
    MOV_M_D = 0x72
    """Writes register 'D' value to memory address pointed by 'HL'."""
    MOV_M_E = 0x73
    """Writes register 'E' value to memory address pointed by 'HL'."""
    MOV_M_H = 0x74
    """Writes register 'H' value to memory address pointed by 'HL'."""
    MOV_M_L = 0x75
    """Writes register 'L' value to memory address pointed by 'HL'."""
    MOV_M_A = 0x77
    """Writes register 'A' value to memory address pointed by 'HL'."""

    # MOV Memory (Source = Memory M)
    MOV_B_M = 0x46
    """Copies memory byte pointed by 'HL' into register 'B'."""
    MOV_C_M = 0x4E
    """Copies memory byte pointed by 'HL' into register 'C'."""
    MOV_D_M = 0x56
    """Copies memory byte pointed by 'HL' into register 'D'."""
    MOV_E_M = 0x5E
    """Copies memory byte pointed by 'HL' into register 'E'."""
    MOV_H_M = 0x66
    """Copies memory byte pointed by 'HL' into register 'H'."""
    MOV_L_M = 0x6E
    """Copies memory byte pointed by 'HL' into register 'L'."""
    MOV_A_M = 0x7E
    """Copies memory byte pointed by 'HL' into register 'A'."""

    # MOV Register-to-Register (Destination = A)
    MOV_A_B = 0x78
    """Copies value from register 'B' into register 'A'."""
    MOV_A_C = 0x79
    """Copies value from register 'C' into register 'A'."""
    MOV_A_D = 0x7A
    """Copies value from register 'D' into register 'A'."""
    MOV_A_E = 0x7B
    """Copies value from register 'E' into register 'A'."""
    MOV_A_H = 0x7C
    """Copies value from register 'H' into register 'A'."""
    MOV_A_L = 0x7D
    """Copies value from register 'L' into register 'A'."""

    # MOV Register-to-Register (Destination = B)
    MOV_B_B = 0x40
    """Copies value from register 'B' into register 'B'."""
    MOV_B_C = 0x41
    """Copies value from register 'C' into register 'B'."""
    MOV_B_D = 0x42
    """Copies value from register 'D' into register 'B'."""
    MOV_B_E = 0x43
    """Copies value from register 'E' into register 'B'."""
    MOV_B_H = 0x44
    """Copies value from register 'H' into register 'B'."""
    MOV_B_L = 0x45
    """Copies value from register 'L' into register 'B'."""
    MOV_B_A = 0x47
    """Copies value from register 'A' into register 'B'."""

    # MOV Register-to-Register (Destination = C)
    MOV_C_B = 0x48
    """Copies value from register 'B' into register 'C'."""
    MOV_C_C = 0x49
    """Copies value from register 'C' into register 'C'."""
    MOV_C_D = 0x4A
    """Copies value from register 'D' into register 'C'."""
    MOV_C_E = 0x4B
    """Copies value from register 'E' into register 'C'."""
    MOV_C_H = 0x4C
    """Copies value from register 'H' into register 'C'."""
    MOV_C_L = 0x4D
    """Copies value from register 'L' into register 'C'."""
    MOV_C_A = 0x4F
    """Copies value from register 'A' into register 'C'."""

    # MOV Register-to-Register (Destination = D)
    MOV_D_B = 0x50
    """Copies value from register 'B' into register 'D'."""
    MOV_D_C = 0x51
    """Copies value from register 'C' into register 'D'."""
    MOV_D_D = 0x52
    """Copies value from register 'D' into register 'D'."""
    MOV_D_E = 0x53
    """Copies value from register 'E' into register 'D'."""
    MOV_D_H = 0x54
    """Copies value from register 'H' into register 'D'."""
    MOV_D_L = 0x55
    """Copies value from register 'L' into register 'D'."""
    MOV_D_A = 0x57
    """Copies value from register 'A' into register 'D'."""

    # MOV Register-to-Register (Destination = E)
    MOV_E_B = 0x58
    """Copies value from register 'B' into register 'E'."""
    MOV_E_C = 0x59
    """Copies value from register 'C' into register 'E'."""
    MOV_E_D = 0x5A
    """Copies value from register 'D' into register 'E'."""
    MOV_E_E = 0x5B
    """Copies value from register 'E' into register 'E'."""
    MOV_E_H = 0x5C
    """Copies value from register 'H' into register 'E'."""
    MOV_E_L = 0x5D
    """Copies value from register 'L' into register 'E'."""
    MOV_E_A = 0x5F
    """Copies value from register 'A' into register 'E'."""

    # MOV Register-to-Register (Destination = H)
    MOV_H_B = 0x60
    """Copies value from register 'B' into register 'H'."""
    MOV_H_C = 0x61
    """Copies value from register 'C' into register 'H'."""
    MOV_H_D = 0x62
    """Copies value from register 'D' into register 'H'."""
    MOV_H_E = 0x63
    """Copies value from register 'E' into register 'H'."""
    MOV_H_H = 0x64
    """Copies value from register 'H' into register 'H'."""
    MOV_H_L = 0x65
    """Copies value from register 'L' into register 'H'."""
    MOV_H_A = 0x67
    """Copies value from register 'A' into register 'H'."""

    # MOV Register-to-Register (Destination = L)
    MOV_L_B = 0x68
    """Copies value from register 'B' into register 'L'."""
    MOV_L_C = 0x69
    """Copies value from register 'C' into register 'L'."""
    MOV_L_D = 0x6A
    """Copies value from register 'D' into register 'L'."""
    MOV_L_E = 0x6B
    """Copies value from register 'E' into register 'L'."""
    MOV_L_H = 0x6C
    """Copies value from register 'H' into register 'L'."""
    MOV_L_L = 0x6D
    """Copies value from register 'L' into register 'L'."""
    MOV_L_A = 0x6F
    """Copies value from register 'A' into register 'L'."""

    # Direct & Indirect Load/Store
    STA_BC = 0x02
    """Writes register 'A' value to memory address pointed by register pair 'BC'."""
    LDA_BC = 0x0A
    """Loads register 'A' with memory byte pointed by register pair 'BC'."""
    STA_DE = 0x12
    """Writes register 'A' value to memory address pointed by register pair 'DE'."""
    LDA_DE = 0x1A
    """Loads register 'A' with memory byte pointed by register pair 'DE'."""
    SHLD = 0x22
    """Writes register pair 'HL' values directly to 16-bit memory address."""
    LHLD = 0x2A
    """Loads register pair 'HL' directly with 16-bit data from memory address."""
    STA = 0x32
    """Writes register 'A' value directly to a 16-bit immediate memory address."""
    LDA = 0x3A
    """Loads register 'A' directly with data from a 16-bit immediate memory address."""
    XCHG = 0xEB
    """Exchanges 16-bit contents of register pairs 'DE' and 'HL'."""

    # ADD (Add Register / Memory to A)
    ADD_B = 0x80
    """Adds register 'B' value to register 'A'; updates all status flags."""
    ADD_C = 0x81
    """Adds register 'C' value to register 'A'; updates all status flags."""
    ADD_D = 0x82
    """Adds register 'D' value to register 'A'; updates all status flags."""
    ADD_E = 0x83
    """Adds register 'E' value to register 'A'; updates all status flags."""
    ADD_H = 0x84
    """Adds register 'H' value to register 'A'; updates all status flags."""
    ADD_L = 0x85
    """Adds register 'L' value to register 'A'; updates all status flags."""
    ADD_M = 0x86
    """Adds memory value pointed by 'HL' to register 'A'; updates all status flags."""
    ADD_A = 0x87
    """Adds register 'A' value to itself; updates all status flags."""

    # ADC (Add Register / Memory + Carry to A)
    ADC_B = 0x88
    """Adds register 'B' value and Carry flag to register 'A'; updates all status flags."""
    ADC_C = 0x89
    """Adds register 'C' value and Carry flag to register 'A'; updates all status flags."""
    ADC_D = 0x8A
    """Adds register 'D' value and Carry flag to register 'A'; updates all status flags."""
    ADC_E = 0x8B
    """Adds register 'E' value and Carry flag to register 'A'; updates all status flags."""
    ADC_H = 0x8C
    """Adds register 'H' value and Carry flag to register 'A'; updates all status flags."""
    ADC_L = 0x8D
    """Adds register 'L' value and Carry flag to register 'A'; updates all status flags."""
    ADC_M = 0x8E
    """Adds memory value pointed by 'HL' and Carry flag to register 'A'; updates all status flags."""
    ADC_A = 0x8F
    """Adds register 'A' value and the Carry flag to itself; updates all status flags."""

    # SUB (Subtract Register / Memory from A)
    SUB_B = 0x90
    """Subtracts register 'B' value from register 'A'; updates all status flags."""
    SUB_C = 0x91
    """Subtracts register 'C' value from register 'A'; updates all status flags."""
    SUB_D = 0x92
    """Subtracts register 'D' value from register 'A'; updates all status flags."""
    SUB_E = 0x93
    """Subtracts register 'E' value from register 'A'; updates all status flags."""
    SUB_H = 0x94
    """Subtracts register 'H' value from register 'A'; updates all status flags."""
    SUB_L = 0x95
    """Subtracts register 'L' value from register 'A'; updates all status flags."""
    SUB_M = 0x96
    """Subtracts memory value pointed by 'HL' from register 'A'; updates all status flags."""
    SUB_A = 0x97
    """Subtracts register 'A' value from itself; updates all status flags."""

    # SBB (Subtract Register / Memory + Carry from A)
    SBB_B = 0x98
    """Subtracts register 'B' value and Carry flag from register 'A'; updates all status flags."""
    SBB_C = 0x99
    """Subtracts register 'C' value and Carry flag from register 'A'; updates all status flags."""
    SBB_D = 0x9A
    """Subtracts register 'D' value and Carry flag from register 'A'; updates all status flags."""
    SBB_E = 0x9B
    """Subtracts register 'E' value and Carry flag from register 'A'; updates all status flags."""
    SBB_H = 0x9C
    """Subtracts register 'H' value and Carry flag from register 'A'; updates all status flags."""
    SBB_L = 0x9D
    """Subtracts register 'L' value and Carry flag from register 'A'; updates all status flags."""
    SBB_M = 0x9E
    """Subtracts memory value pointed by 'HL' and Carry flag from register 'A'; updates all status flags."""
    SBB_A = 0x9F
    """Subtracts register 'A' value and Carry flag from itself; updates all status flags."""

    # INR (Increment Register / Memory)
    INR_B = 0x04
    """Increments register 'B' value by 1; updates status flags."""
    INR_C = 0x0C
    """Increments register 'C' value by 1; updates status flags."""
    INR_D = 0x14
    """Increments register 'D' value by 1; updates status flags."""
    INR_E = 0x1C
    """Increments register 'E' value by 1; updates status flags."""
    INR_H = 0x24
    """Increments register 'H' value by 1; updates status flags."""
    INR_L = 0x2C
    """Increments register 'L' value by 1; updates status flags."""
    INR_M = 0x34
    """Increments memory value pointed by 'HL' by 1; updates status flags."""
    INR_A = 0x3C
    """Increments register 'A' value by 1; updates status flags."""

    # DCR (Decrement Register / Memory)
    DCR_B = 0x05
    """Decrements register 'B' value by 1; updates status flags."""
    DCR_C = 0x0D
    """Decrements register 'C' value by 1; updates status flags."""
    DCR_D = 0x15
    """Decrements register 'D' value by 1; updates status flags."""
    DCR_E = 0x1D
    """Decrements register 'E' value by 1; updates status flags."""
    DCR_H = 0x25
    """Decrements register 'H' value by 1; updates status flags."""
    DCR_L = 0x2D
    """Decrements register 'L' value by 1; updates status flags."""
    DCR_M = 0x35
    """Decrements memory value pointed by 'HL' by 1; updates status flags."""
    DCR_A = 0x3D
    """Decrements register 'A' value by 1; updates status flags."""

    # ANA (Logical AND Register / Memory with A)
    ANA_B = 0xA0
    """Performs logical AND of register 'B' value with register 'A'; updates status flags."""
    ANA_C = 0xA1
    """Performs logical AND of register 'C' value with register 'A'; updates status flags."""
    ANA_D = 0xA2
    """Performs logical AND of register 'D' value with register 'A'; updates status flags."""
    ANA_E = 0xA3
    """Performs logical AND of register 'E' value with register 'A'; updates status flags."""
    ANA_H = 0xA4
    """Performs logical AND of register 'H' value with register 'A'; updates status flags."""
    ANA_L = 0xA5
    """Performs logical AND of register 'L' value with register 'A'; updates status flags."""
    ANA_M = 0xA6
    """Performs logical AND of memory value pointed by 'HL' with register 'A'; updates status flags."""
    ANA_A = 0xA7
    """Performs logical AND of register 'A' value with itself; updates status flags."""
    ANI = 0xE6
    """Performs logical AND of immediate byte with register 'A'; updates status flags."""

    # XRA (Logical XOR Register / Memory with A)
    XRA_B = 0xA8
    """Performs logical XOR of register 'B' value with register 'A'; updates status flags."""
    XRA_C = 0xA9
    """Performs logical XOR of register 'C' value with register 'A'; updates status flags."""
    XRA_D = 0xAA
    """Performs logical XOR of register 'D' value with register 'A'; updates status flags."""
    XRA_E = 0xAB
    """Performs logical XOR of register 'E' value with register 'A'; updates status flags."""
    XRA_H = 0xAC
    """Performs logical XOR of register 'H' value with register 'A'; updates status flags."""
    XRA_L = 0xAD
    """Performs logical XOR of register 'L' value with register 'A'; updates status flags."""
    XRA_M = 0xAE
    """Performs logical XOR of memory value pointed by 'HL' with register 'A'; updates status flags."""
    XRA_A = 0xAF
    """Performs logical XOR of register 'A' value with itself; updates status flags."""
    XRI = 0xEE
    """Performs logical XOR of immediate byte with register 'A'; updates status flags."""

    # ORA (Logical OR Register / Memory with A)
    ORA_B = 0xB0
    """Performs logical OR of register 'B' value with register 'A'; updates status flags."""
    ORA_C = 0xB1
    """Performs logical OR of register 'C' value with register 'A'; updates status flags."""
    ORA_D = 0xB2
    """Performs logical OR of register 'D' value with register 'A'; updates status flags."""
    ORA_E = 0xB3
    """Performs logical OR of register 'E' value with register 'A'; updates status flags."""
    ORA_H = 0xB4
    """Performs logical OR of register 'H' value with register 'A'; updates status flags."""
    ORA_L = 0xB5
    """Performs logical OR of register 'L' value with register 'A'; updates status flags."""
    ORA_M = 0xB6
    """Performs logical OR of memory value pointed by 'HL' with register 'A'; updates status flags."""
    ORA_A = 0xB7
    """Performs logical OR of register 'A' value with itself; updates status flags."""
    ORI = 0xF6
    """Performs logical OR of immediate byte with register 'A'; updates status flags."""

    # CMP (Compare Register / Memory with A)
    CMP_B = 0xB8
    """Compares register 'B' value with register 'A'; updates status flags."""
    CMP_C = 0xB9
    """Compares register 'C' value with register 'A'; updates status flags."""
    CMP_D = 0xBA
    """Compares register 'D' value with register 'A'; updates status flags."""
    CMP_E = 0xBB
    """Compares register 'E' value with register 'A'; updates status flags."""
    CMP_H = 0xBC
    """Compares register 'H' value with register 'A'; updates status flags."""
    CMP_L = 0xBD
    """Compares register 'L' value with register 'A'; updates status flags."""
    CMP_M = 0xBE
    """Compares memory value pointed by 'HL' with register 'A'; updates status flags."""
    CMP_A = 0xBF
    """Compares register 'A' value with itself; updates status flags."""
    CPI = 0xFE
    """Compares immediate byte with register 'A'; updates status flags."""

    # Rotate & Special Accumulator Instructions
    RLC = 0x07
    """Rotates register 'A' value left circular; updates Carry flag."""
    RRC = 0x0F
    """Rotates register 'A' value right circular; updates Carry flag."""
    RAL = 0x17
    """Rotates register 'A' value left through Carry flag; updates Carry flag."""
    RAR = 0x1F
    """Rotates register 'A' value right through Carry flag; updates Carry flag."""
    DAA = 0x27
    """Decimal adjusts register 'A' value after addition; updates status flags."""
    CMA = 0x2F
    """Complements (inverts) register 'A' value; status flags are unchanged."""
    STC = 0x37
    """Sets Carry flag to 1; updates Carry flag."""
    CMC = 0x3F
    """Complements Carry flag; updates Carry flag."""

    # Unassigned / Custom Extensions
    DAS = 0x10
    """Decimal adjusts register 'A' value after subtraction; updates status flags."""
    AAA = 0x18
    """ASCII adjusts register 'A' value after addition; updates status flags."""
    AAS = 0x28
    """ASCII adjusts register 'A' value after subtraction; updates status flags."""

    # Immediate Arithmetic Operations
    ADI = 0xC6
    """Adds immediate byte to register 'A'; updates all status flags."""
    ACI = 0xCE
    """Adds immediate byte and Carry flag to register 'A'; updates all status flags."""
    SUI = 0xD6
    """Subtracts immediate byte from register 'A'; updates all status flags."""
    SBI = 0xDE
    """Subtracts immediate byte and Carry flag from register 'A'; updates all status flags."""

    # 16-Bit Register Pair Increment / Decrement / Add
    INX_BC = 0x03
    """Increments register pair 'BC' by 1; status flags are unchanged."""
    INX_DE = 0x13
    """Increments register pair 'DE' by 1; status flags are unchanged."""
    INX_HL = 0x23
    """Increments register pair 'HL' by 1; status flags are unchanged."""
    INX_SP = 0x33
    """Increments stack pointer 'SP' by 1; status flags are unchanged."""

    DCX_BC = 0x0B
    """Decrements register pair 'BC' by 1; status flags are unchanged."""
    DCX_DE = 0x1B
    """Decrements register pair 'DE' by 1; status flags are unchanged."""
    DCX_HL = 0x2B
    """Decrements register pair 'HL' by 1; status flags are unchanged."""
    DCX_SP = 0x3B
    """Decrements stack pointer 'SP' by 1; status flags are unchanged."""

    DAD_BC = 0x09
    """Adds register pair 'BC' to 'HL'; updates Carry flag only."""
    DAD_DE = 0x19
    """Adds register pair 'DE' to 'HL'; updates Carry flag only."""
    DAD_HL = 0x29
    """Adds register pair 'HL' to 'HL'; updates Carry flag only."""
    DAD_SP = 0x39
    """Adds stack pointer 'SP' to 'HL'; updates Carry flag only."""

    # Stack Push / Pop
    POP_BC = 0xC1
    """Pops top of stack into register pair 'BC'; status flags are unchanged."""
    PUSH_BC = 0xC5
    """Pushes register pair 'BC' onto the stack; status flags are unchanged."""
    POP_DE = 0xD1
    """Pops top of stack into register pair 'DE'; status flags are unchanged."""
    PUSH_DE = 0xD5
    """Pushes register pair 'DE' onto the stack; status flags are unchanged."""
    POP_HL = 0xE1
    """Pops top of stack into register pair 'HL'; status flags are unchanged."""
    PUSH_HL = 0xE5
    """Pushes register pair 'HL' onto the stack; status flags are unchanged."""
    POP_PSW = 0xF1
    """Pops top of stack into Program Status Word (Accumulator 'A' and Flags)."""
    PUSH_PSW = 0xF5
    """Pushes Program Status Word (Accumulator 'A' and Flags) onto the stack."""

    XTHL = 0xE3
    """Exchanges 16-bit contents of top of stack with register pair 'HL'."""
    SPHL = 0xF9
    """Loads stack pointer 'SP' with 16-bit contents of register pair 'HL'."""
    PCHL = 0xE9
    """Loads program counter 'PC' with 16-bit contents of register pair 'HL'."""

    # Unconditional Jump & Call & Return
    JMP = 0xC3
    """Unconditional jump; jumps to the memory address."""
    CALL = 0xCD
    """Unconditional call subroutine."""
    RET = 0xC9
    """Unconditional return from subroutine."""

    # Conditional Jumps
    JNZ = 0xC2
    """Jump if zero flag is 0."""
    JZ = 0xCA
    """Jump if zero flag is 1."""
    JNC = 0xD2
    """Jump if carry flag is 0."""
    JC = 0xDA
    """Jump if carry flag is 1."""
    JPO = 0xE2
    """Jump if parity flag is 0 (parity odd)."""
    JPE = 0xEA
    """Jump if parity flag is 1 (parity even)."""
    JP = 0xF2
    """Jump if sign flag is 0 (positive)."""
    JM = 0xFA
    """Jump if sign flag is 1 (minus)."""

    # Conditional Calls
    CNZ = 0xC4
    """Call subroutine if zero flag is 0."""
    CZ = 0xCC
    """Call subroutine if zero flag is 1."""
    CNC = 0xD4
    """Call subroutine if carry flag is 0."""
    CC = 0xDC
    """Call subroutine if carry flag is 1."""
    CPO = 0xE4
    """Call subroutine if parity flag is 0 (parity odd)."""
    CPE = 0xEC
    """Call subroutine if parity flag is 1 (parity even)."""
    CP = 0xF4
    """Call subroutine if sign flag is 0 (positive)."""
    CM = 0xFC
    """Call subroutine if sign flag is 1 (minus)."""

    # Conditional Returns
    RNZ = 0xC0
    """Return from subroutine if zero flag is 0."""
    RZ = 0xC8
    """Return from subroutine if zero flag is 1."""
    RNC = 0xD0
    """Return from subroutine if carry flag is 0."""
    RC = 0xD8
    """Return from subroutine if carry flag is 1."""
    RPO = 0xE0
    """Return from subroutine if parity flag is 0 (parity odd)."""
    RPE = 0xE8
    """Return from subroutine if parity flag is 1 (parity even)."""
    RP = 0xF0
    """Return from subroutine if sign flag is 0 (positive)."""
    RM = 0xF8
    """Return from subroutine if sign flag is 1 (minus)."""

    # Software Restarts / Interrupts
    RST_0 = 0xC7
    """Restart 0: Push PC and jump to 0x0000."""
    RST_1 = 0xCF
    """Restart 1: Push PC and jump to 0x0008."""
    RST_2 = 0xD7
    """Restart 2: Push PC and jump to 0x0010."""
    RST_3 = 0xDF
    """Restart 3: Push PC and jump to 0x0018."""
    RST_4 = 0xE7
    """Restart 4: Push PC and jump to 0x0020."""
    RST_5 = 0xEF
    """Restart 5: Push PC and jump to 0x0028."""
    RST_6 = 0xF7
    """Restart 6: Push PC and jump to 0x0030."""
    RST_7 = 0xFF
    """Restart 7: Push PC and jump to 0x0038."""

    # I/O & Interrupt Control
    IN = 0xDB
    """Reads 8-bit byte from I/O port into Accumulator A."""
    OUT = 0xD3
    """Writes 8-bit Accumulator A byte to I/O port."""
    EI = 0xFB
    """Enables maskable interrupts (sets INTE flag)."""
    DI = 0xF3
    """Disables maskable interrupts (clears INTE flag)."""
    RIM = 0x20
    """Read Interrupt Mask into Accumulator A."""
    SIM = 0x30
    """Set Interrupt Mask from Accumulator A."""

    @classmethod
    def from_name(cls, name: str) -> Self:
        """Provides opcode from opcode name."""
        try:
            return cls[name]
        except KeyError:
            raise ValueError(f"'{name}' is not a valid {cls.__name__} name")

    def __repr__(self) -> str:
        return f"Opcode(name={self.name}, value={int(self)})"

    def __str__(self) -> str:
        return self.name
