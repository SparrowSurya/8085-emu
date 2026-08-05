//! Every opcode the 8085 understands, one variant per encoded byte.
//!
//! `#[repr(u8)]` pins each variant to its real machine-code value, so
//! encoding is a cast and decoding is a single generated match, matching
//! the reference behavioral spec.

use crate::error::EmuError;

/// An 8085 instruction opcode. The discriminant *is* the machine-code byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum Opcode {
    /// No operation.
    NOP = 0x0,
    /// Loads 16-bit immediate word into register pair 'BC'.
    MVI_BC = 0x1,
    /// Writes register 'A' value to memory address pointed by register pair 'BC'.
    STA_BC = 0x2,
    /// Increments register pair 'BC' by 1; status flags are unchanged.
    INX_BC = 0x3,
    /// Increments register 'B' value by 1; updates status flags.
    INR_B = 0x4,
    /// Decrements register 'B' value by 1; updates status flags.
    DCR_B = 0x5,
    /// Loads immediate byte value into register 'B'.
    MVI_B = 0x6,
    /// Rotates register 'A' value left circular; updates Carry flag.
    RLC = 0x7,
    /// Adds register pair 'BC' to 'HL'; updates Carry flag only.
    DAD_BC = 0x9,
    /// Loads register 'A' with memory byte pointed by register pair 'BC'.
    LDA_BC = 0xa,
    /// Decrements register pair 'BC' by 1; status flags are unchanged.
    DCX_BC = 0xb,
    /// Increments register 'C' value by 1; updates status flags.
    INR_C = 0xc,
    /// Decrements register 'C' value by 1; updates status flags.
    DCR_C = 0xd,
    /// Loads immediate byte value into register 'C'.
    MVI_C = 0xe,
    /// Rotates register 'A' value right circular; updates Carry flag.
    RRC = 0xf,

    /// Loads 16-bit immediate word into register pair 'DE'.
    MVI_DE = 0x11,
    /// Writes register 'A' value to memory address pointed by register pair 'DE'.
    STA_DE = 0x12,
    /// Increments register pair 'DE' by 1; status flags are unchanged.
    INX_DE = 0x13,
    /// Increments register 'D' value by 1; updates status flags.
    INR_D = 0x14,
    /// Decrements register 'D' value by 1; updates status flags.
    DCR_D = 0x15,
    /// Loads immediate byte value into register 'D'.
    MVI_D = 0x16,
    /// Rotates register 'A' value left through Carry flag; updates Carry flag.
    RAL = 0x17,

    /// Adds register pair 'DE' to 'HL'; updates Carry flag only.
    DAD_DE = 0x19,
    /// Loads register 'A' with memory byte pointed by register pair 'DE'.
    LDA_DE = 0x1a,
    /// Decrements register pair 'DE' by 1; status flags are unchanged.
    DCX_DE = 0x1b,
    /// Increments register 'E' value by 1; updates status flags.
    INR_E = 0x1c,
    /// Decrements register 'E' value by 1; updates status flags.
    DCR_E = 0x1d,
    /// Loads immediate byte value into register 'E'.
    MVI_E = 0x1e,
    /// Rotates register 'A' value right through Carry flag; updates Carry flag.
    RAR = 0x1f,
    /// Read Interrupt Mask into Accumulator A.
    RIM = 0x20,
    /// Loads 16-bit immediate word into register pair 'HL'.
    MVI_HL = 0x21,
    /// Writes register pair 'HL' values directly to 16-bit memory address.
    SHLD = 0x22,
    /// Increments register pair 'HL' by 1; status flags are unchanged.
    INX_HL = 0x23,
    /// Increments register 'H' value by 1; updates status flags.
    INR_H = 0x24,
    /// Decrements register 'H' value by 1; updates status flags.
    DCR_H = 0x25,
    /// Loads immediate byte value into register 'H'.
    MVI_H = 0x26,
    /// Decimal adjusts register 'A' value after addition; updates status flags.
    DAA = 0x27,

    /// Adds register pair 'HL' to 'HL'; updates Carry flag only.
    DAD_HL = 0x29,
    /// Loads register pair 'HL' directly with 16-bit data from memory address.
    LHLD = 0x2a,
    /// Decrements register pair 'HL' by 1; status flags are unchanged.
    DCX_HL = 0x2b,
    /// Increments register 'L' value by 1; updates status flags.
    INR_L = 0x2c,
    /// Decrements register 'L' value by 1; updates status flags.
    DCR_L = 0x2d,
    /// Loads immediate byte value into register 'L'.
    MVI_L = 0x2e,
    /// Complements (inverts) register 'A' value; status flags are unchanged.
    CMA = 0x2f,
    /// Set Interrupt Mask from Accumulator A.
    SIM = 0x30,
    /// LXI SP, d16 — load the stack pointer with a 16-bit immediate.
    LXI_SP = 0x31,
    /// Writes register 'A' value directly to a 16-bit immediate memory address.
    STA = 0x32,
    /// Increments stack pointer 'SP' by 1; status flags are unchanged.
    INX_SP = 0x33,
    /// Increments memory value pointed by 'HL' by 1; updates status flags.
    INR_M = 0x34,
    /// Decrements memory value pointed by 'HL' by 1; updates status flags.
    DCR_M = 0x35,
    /// Writes immediate byte value to memory address pointed by 'HL'.
    MVI_M = 0x36,
    /// Sets Carry flag to 1; updates Carry flag.
    STC = 0x37,
    /// Adds stack pointer 'SP' to 'HL'; updates Carry flag only.
    DAD_SP = 0x39,
    /// Loads register 'A' directly with data from a 16-bit immediate memory address.
    LDA = 0x3a,
    /// Decrements stack pointer 'SP' by 1; status flags are unchanged.
    DCX_SP = 0x3b,
    /// Increments register 'A' value by 1; updates status flags.
    INR_A = 0x3c,
    /// Decrements register 'A' value by 1; updates status flags.
    DCR_A = 0x3d,
    /// Loads immediate byte value into register 'A'.
    MVI_A = 0x3e,
    /// Complements Carry flag; updates Carry flag.
    CMC = 0x3f,
    /// Copies value from register 'B' into register 'B'.
    MOV_BB = 0x40,
    /// Copies value from register 'C' into register 'B'.
    MOV_BC = 0x41,
    /// Copies value from register 'D' into register 'B'.
    MOV_BD = 0x42,
    /// Copies value from register 'E' into register 'B'.
    MOV_BE = 0x43,
    /// Copies value from register 'H' into register 'B'.
    MOV_BH = 0x44,
    /// Copies value from register 'L' into register 'B'.
    MOV_BL = 0x45,
    /// Copies memory byte pointed by 'HL' into register 'B'.
    MOV_BM = 0x46,
    /// Copies value from register 'A' into register 'B'.
    MOV_BA = 0x47,
    /// Copies value from register 'B' into register 'C'.
    MOV_CB = 0x48,
    /// Copies value from register 'C' into register 'C'.
    MOV_CC = 0x49,
    /// Copies value from register 'D' into register 'C'.
    MOV_CD = 0x4a,
    /// Copies value from register 'E' into register 'C'.
    MOV_CE = 0x4b,
    /// Copies value from register 'H' into register 'C'.
    MOV_CH = 0x4c,
    /// Copies value from register 'L' into register 'C'.
    MOV_CL = 0x4d,
    /// Copies memory byte pointed by 'HL' into register 'C'.
    MOV_CM = 0x4e,
    /// Copies value from register 'A' into register 'C'.
    MOV_CA = 0x4f,
    /// Copies value from register 'B' into register 'D'.
    MOV_DB = 0x50,
    /// Copies value from register 'C' into register 'D'.
    MOV_DC = 0x51,
    /// Copies value from register 'D' into register 'D'.
    MOV_DD = 0x52,
    /// Copies value from register 'E' into register 'D'.
    MOV_DE = 0x53,
    /// Copies value from register 'H' into register 'D'.
    MOV_DH = 0x54,
    /// Copies value from register 'L' into register 'D'.
    MOV_DL = 0x55,
    /// Copies memory byte pointed by 'HL' into register 'D'.
    MOV_DM = 0x56,
    /// Copies value from register 'A' into register 'D'.
    MOV_DA = 0x57,
    /// Copies value from register 'B' into register 'E'.
    MOV_EB = 0x58,
    /// Copies value from register 'C' into register 'E'.
    MOV_EC = 0x59,
    /// Copies value from register 'D' into register 'E'.
    MOV_ED = 0x5a,
    /// Copies value from register 'E' into register 'E'.
    MOV_EE = 0x5b,
    /// Copies value from register 'H' into register 'E'.
    MOV_EH = 0x5c,
    /// Copies value from register 'L' into register 'E'.
    MOV_EL = 0x5d,
    /// Copies memory byte pointed by 'HL' into register 'E'.
    MOV_EM = 0x5e,
    /// Copies value from register 'A' into register 'E'.
    MOV_EA = 0x5f,
    /// Copies value from register 'B' into register 'H'.
    MOV_HB = 0x60,
    /// Copies value from register 'C' into register 'H'.
    MOV_HC = 0x61,
    /// Copies value from register 'D' into register 'H'.
    MOV_HD = 0x62,
    /// Copies value from register 'E' into register 'H'.
    MOV_HE = 0x63,
    /// Copies value from register 'H' into register 'H'.
    MOV_HH = 0x64,
    /// Copies value from register 'L' into register 'H'.
    MOV_HL = 0x65,
    /// Copies memory byte pointed by 'HL' into register 'H'.
    MOV_HM = 0x66,
    /// Copies value from register 'A' into register 'H'.
    MOV_HA = 0x67,
    /// Copies value from register 'B' into register 'L'.
    MOV_LB = 0x68,
    /// Copies value from register 'C' into register 'L'.
    MOV_LC = 0x69,
    /// Copies value from register 'D' into register 'L'.
    MOV_LD = 0x6a,
    /// Copies value from register 'E' into register 'L'.
    MOV_LE = 0x6b,
    /// Copies value from register 'H' into register 'L'.
    MOV_LH = 0x6c,
    /// Copies value from register 'L' into register 'L'.
    MOV_LL = 0x6d,
    /// Copies memory byte pointed by 'HL' into register 'L'.
    MOV_LM = 0x6e,
    /// Copies value from register 'A' into register 'L'.
    MOV_LA = 0x6f,
    /// Writes register 'B' value to memory address pointed by 'HL'.
    MOV_MB = 0x70,
    /// Writes register 'C' value to memory address pointed by 'HL'.
    MOV_MC = 0x71,
    /// Writes register 'D' value to memory address pointed by 'HL'.
    MOV_MD = 0x72,
    /// Writes register 'E' value to memory address pointed by 'HL'.
    MOV_ME = 0x73,
    /// Writes register 'H' value to memory address pointed by 'HL'.
    MOV_MH = 0x74,
    /// Writes register 'L' value to memory address pointed by 'HL'.
    MOV_ML = 0x75,
    /// Stops the processor execution loop.
    HLT = 0x76,
    /// Writes register 'A' value to memory address pointed by 'HL'.
    MOV_MA = 0x77,
    /// Copies value from register 'B' into register 'A'.
    MOV_AB = 0x78,
    /// Copies value from register 'C' into register 'A'.
    MOV_AC = 0x79,
    /// Copies value from register 'D' into register 'A'.
    MOV_AD = 0x7a,
    /// Copies value from register 'E' into register 'A'.
    MOV_AE = 0x7b,
    /// Copies value from register 'H' into register 'A'.
    MOV_AH = 0x7c,
    /// Copies value from register 'L' into register 'A'.
    MOV_AL = 0x7d,
    /// Copies memory byte pointed by 'HL' into register 'A'.
    MOV_AM = 0x7e,
    /// MOV A, A — copy A into A (a one-cycle no-op).
    MOV_AA = 0x7F,
    /// Adds register 'B' value to register 'A'; updates all status flags.
    ADD_B = 0x80,
    /// Adds register 'C' value to register 'A'; updates all status flags.
    ADD_C = 0x81,
    /// Adds register 'D' value to register 'A'; updates all status flags.
    ADD_D = 0x82,
    /// Adds register 'E' value to register 'A'; updates all status flags.
    ADD_E = 0x83,
    /// Adds register 'H' value to register 'A'; updates all status flags.
    ADD_H = 0x84,
    /// Adds register 'L' value to register 'A'; updates all status flags.
    ADD_L = 0x85,
    /// Adds memory value pointed by 'HL' to register 'A'; updates all status flags.
    ADD_M = 0x86,
    /// Adds register 'A' value to itself; updates all status flags.
    ADD_A = 0x87,
    /// Adds register 'B' value and Carry flag to register 'A'; updates all status flags.
    ADC_B = 0x88,
    /// Adds register 'C' value and Carry flag to register 'A'; updates all status flags.
    ADC_C = 0x89,
    /// Adds register 'D' value and Carry flag to register 'A'; updates all status flags.
    ADC_D = 0x8a,
    /// Adds register 'E' value and Carry flag to register 'A'; updates all status flags.
    ADC_E = 0x8b,
    /// Adds register 'H' value and Carry flag to register 'A'; updates all status flags.
    ADC_H = 0x8c,
    /// Adds register 'L' value and Carry flag to register 'A'; updates all status flags.
    ADC_L = 0x8d,
    /// Adds memory value pointed by 'HL' and Carry flag to register 'A'; updates all status flags.
    ADC_M = 0x8e,
    /// Adds register 'A' value and the Carry flag to itself; updates all status flags.
    ADC_A = 0x8f,
    /// Subtracts register 'B' value from register 'A'; updates all status flags.
    SUB_B = 0x90,
    /// Subtracts register 'C' value from register 'A'; updates all status flags.
    SUB_C = 0x91,
    /// Subtracts register 'D' value from register 'A'; updates all status flags.
    SUB_D = 0x92,
    /// Subtracts register 'E' value from register 'A'; updates all status flags.
    SUB_E = 0x93,
    /// Subtracts register 'H' value from register 'A'; updates all status flags.
    SUB_H = 0x94,
    /// Subtracts register 'L' value from register 'A'; updates all status flags.
    SUB_L = 0x95,
    /// Subtracts memory value pointed by 'HL' from register 'A'; updates all status flags.
    SUB_M = 0x96,
    /// Subtracts register 'A' value from itself; updates all status flags.
    SUB_A = 0x97,
    /// Subtracts register 'B' value and Carry flag from register 'A'; updates all status flags.
    SBB_B = 0x98,
    /// Subtracts register 'C' value and Carry flag from register 'A'; updates all status flags.
    SBB_C = 0x99,
    /// Subtracts register 'D' value and Carry flag from register 'A'; updates all status flags.
    SBB_D = 0x9a,
    /// Subtracts register 'E' value and Carry flag from register 'A'; updates all status flags.
    SBB_E = 0x9b,
    /// Subtracts register 'H' value and Carry flag from register 'A'; updates all status flags.
    SBB_H = 0x9c,
    /// Subtracts register 'L' value and Carry flag from register 'A'; updates all status flags.
    SBB_L = 0x9d,
    /// Subtracts memory value pointed by 'HL' and Carry flag from register 'A'; updates all status flags.
    SBB_M = 0x9e,
    /// Subtracts register 'A' value and Carry flag from itself; updates all status flags.
    SBB_A = 0x9f,
    /// Performs logical AND of register 'B' value with register 'A'; updates status flags.
    ANA_B = 0xa0,
    /// Performs logical AND of register 'C' value with register 'A'; updates status flags.
    ANA_C = 0xa1,
    /// Performs logical AND of register 'D' value with register 'A'; updates status flags.
    ANA_D = 0xa2,
    /// Performs logical AND of register 'E' value with register 'A'; updates status flags.
    ANA_E = 0xa3,
    /// Performs logical AND of register 'H' value with register 'A'; updates status flags.
    ANA_H = 0xa4,
    /// Performs logical AND of register 'L' value with register 'A'; updates status flags.
    ANA_L = 0xa5,
    /// Performs logical AND of memory value pointed by 'HL' with register 'A'; updates status flags.
    ANA_M = 0xa6,
    /// Performs logical AND of register 'A' value with itself; updates status flags.
    ANA_A = 0xa7,
    /// Performs logical XOR of register 'B' value with register 'A'; updates status flags.
    XRA_B = 0xa8,
    /// Performs logical XOR of register 'C' value with register 'A'; updates status flags.
    XRA_C = 0xa9,
    /// Performs logical XOR of register 'D' value with register 'A'; updates status flags.
    XRA_D = 0xaa,
    /// Performs logical XOR of register 'E' value with register 'A'; updates status flags.
    XRA_E = 0xab,
    /// Performs logical XOR of register 'H' value with register 'A'; updates status flags.
    XRA_H = 0xac,
    /// Performs logical XOR of register 'L' value with register 'A'; updates status flags.
    XRA_L = 0xad,
    /// Performs logical XOR of memory value pointed by 'HL' with register 'A'; updates status flags.
    XRA_M = 0xae,
    /// Performs logical XOR of register 'A' value with itself; updates status flags.
    XRA_A = 0xaf,
    /// Performs logical OR of register 'B' value with register 'A'; updates status flags.
    ORA_B = 0xb0,
    /// Performs logical OR of register 'C' value with register 'A'; updates status flags.
    ORA_C = 0xb1,
    /// Performs logical OR of register 'D' value with register 'A'; updates status flags.
    ORA_D = 0xb2,
    /// Performs logical OR of register 'E' value with register 'A'; updates status flags.
    ORA_E = 0xb3,
    /// Performs logical OR of register 'H' value with register 'A'; updates status flags.
    ORA_H = 0xb4,
    /// Performs logical OR of register 'L' value with register 'A'; updates status flags.
    ORA_L = 0xb5,
    /// Performs logical OR of memory value pointed by 'HL' with register 'A'; updates status flags.
    ORA_M = 0xb6,
    /// Performs logical OR of register 'A' value with itself; updates status flags.
    ORA_A = 0xb7,
    /// Compares register 'B' value with register 'A'; updates status flags.
    CMP_B = 0xb8,
    /// Compares register 'C' value with register 'A'; updates status flags.
    CMP_C = 0xb9,
    /// Compares register 'D' value with register 'A'; updates status flags.
    CMP_D = 0xba,
    /// Compares register 'E' value with register 'A'; updates status flags.
    CMP_E = 0xbb,
    /// Compares register 'H' value with register 'A'; updates status flags.
    CMP_H = 0xbc,
    /// Compares register 'L' value with register 'A'; updates status flags.
    CMP_L = 0xbd,
    /// Compares memory value pointed by 'HL' with register 'A'; updates status flags.
    CMP_M = 0xbe,
    /// Compares register 'A' value with itself; updates status flags.
    CMP_A = 0xbf,
    /// Return from subroutine if zero flag is 0.
    RNZ = 0xc0,
    /// Pops top of stack into register pair 'BC'; status flags are unchanged.
    POP_BC = 0xc1,
    /// Jump if zero flag is 0.
    JNZ = 0xc2,
    /// Unconditional jump; jumps to the memory address.
    JMP = 0xc3,
    /// CALL subroutine if zero flag is 0.
    CNZ = 0xc4,
    /// Pushes register pair 'BC' onto the stack; status flags are unchanged.
    PUSH_BC = 0xc5,
    /// Adds immediate byte to register 'A'; updates all status flags.
    ADI = 0xc6,
    /// Restart 0: Push PC and jump to 0x0000.
    RST_0 = 0xc7,
    /// Return from subroutine if zero flag is 1.
    RZ = 0xc8,
    /// Unconditional return from subroutine.
    RET = 0xc9,
    /// Jump if zero flag is 1.
    JZ = 0xca,
    /// CALL subroutine if zero flag is 1.
    CZ = 0xcc,
    /// Unconditional call subroutine.
    CALL = 0xcd,
    /// Adds immediate byte and Carry flag to register 'A'; updates all status flags.
    ACI = 0xce,
    /// Restart 1: Push PC and jump to 0x0008.
    RST_1 = 0xcf,
    /// Return from subroutine if carry flag is 0.
    RNC = 0xd0,
    /// Pops top of stack into register pair 'DE'; status flags are unchanged.
    POP_DE = 0xd1,
    /// Jump if carry flag is 0.
    JNC = 0xd2,
    /// Writes 8-bit Accumulator A byte to I/O port.
    OUT = 0xd3,
    /// CALL subroutine if carry flag is 0.
    CNC = 0xd4,
    /// Pushes register pair 'DE' onto the stack; status flags are unchanged.
    PUSH_DE = 0xd5,
    /// Subtracts immediate byte from register 'A'; updates all status flags.
    SUI = 0xd6,
    /// Restart 2: Push PC and jump to 0x0010.
    RST_2 = 0xd7,
    /// Return from subroutine if carry flag is 1.
    RC = 0xd8,
    /// Jump if carry flag is 1.
    JC = 0xda,
    /// Reads 8-bit byte from I/O port into Accumulator A.
    IN = 0xdb,
    /// CALL subroutine if carry flag is 1.
    CC = 0xdc,
    /// Subtracts immediate byte and Carry flag from register 'A'; updates all status flags.
    SBI = 0xde,
    /// Restart 3: Push PC and jump to 0x0018.
    RST_3 = 0xdf,
    /// Return from subroutine if parity flag is 0 (parity odd).
    RPO = 0xe0,
    /// Pops top of stack into register pair 'HL'; status flags are unchanged.
    POP_HL = 0xe1,
    /// Jump if parity flag is 0 (parity odd).
    JPO = 0xe2,
    /// Exchanges 16-bit contents of top of stack with register pair 'HL'.
    XTHL = 0xe3,
    /// CALL subroutine if parity flag is 0 (parity odd).
    CPO = 0xe4,
    /// Pushes register pair 'HL' onto the stack; status flags are unchanged.
    PUSH_HL = 0xe5,
    /// Performs logical AND of immediate byte with register 'A'; updates status flags.
    ANI = 0xe6,
    /// Restart 4: Push PC and jump to 0x0020.
    RST_4 = 0xe7,
    /// Return from subroutine if parity flag is 1 (parity even).
    RPE = 0xe8,
    /// Loads program counter 'PC' with 16-bit contents of register pair 'HL'.
    PCHL = 0xe9,
    /// Jump if parity flag is 1 (parity even).
    JPE = 0xea,
    /// Exchanges 16-bit contents of register pairs 'DE' and 'HL'.
    XCHG = 0xeb,
    /// CALL subroutine if parity flag is 1 (parity even).
    CPE = 0xec,
    /// Performs logical XOR of immediate byte with register 'A'; updates status flags.
    XRI = 0xee,
    /// Restart 5: Push PC and jump to 0x0028.
    RST_5 = 0xef,
    /// Return from subroutine if sign flag is 0 (positive).
    RP = 0xf0,
    /// Pops top of stack into Program Status Word (Accumulator 'A' and Flags).
    POP_PSW = 0xf1,
    /// Jump if sign flag is 0 (positive).
    JP = 0xf2,
    /// Disables maskable interrupts (clears INTE flag).
    DI = 0xf3,
    /// CALL subroutine if sign flag is 0 (positive).
    CP = 0xf4,
    /// Pushes Program Status Word (Accumulator 'A' and Flags) onto the stack.
    PUSH_PSW = 0xf5,
    /// Performs logical OR of immediate byte with register 'A'; updates status flags.
    ORI = 0xf6,
    /// Restart 6: Push PC and jump to 0x0030.
    RST_6 = 0xf7,
    /// Return from subroutine if sign flag is 1 (minus).
    RM = 0xf8,
    /// Loads stack pointer 'SP' with 16-bit contents of register pair 'HL'.
    SPHL = 0xf9,
    /// Jump if sign flag is 1 (minus).
    JM = 0xfa,
    /// Enables maskable interrupts (sets INTE flag).
    EI = 0xfb,
    /// CALL subroutine if sign flag is 1 (minus).
    CM = 0xfc,
    /// Compares immediate byte with register 'A'; updates status flags.
    CPI = 0xfe,
    /// Restart 7: Push PC and jump to 0x0038.
    RST_7 = 0xff,
}

impl Opcode {
    /// Decodes a raw byte into an `Opcode`, or reports it as invalid/undefined.
    pub fn from_byte(byte: u8) -> Result<Self, EmuError> {
        Ok(match byte {
            0x0 => Opcode::NOP,
            0x1 => Opcode::MVI_BC,
            0x2 => Opcode::STA_BC,
            0x3 => Opcode::INX_BC,
            0x4 => Opcode::INR_B,
            0x5 => Opcode::DCR_B,
            0x6 => Opcode::MVI_B,
            0x7 => Opcode::RLC,
            0x9 => Opcode::DAD_BC,
            0xa => Opcode::LDA_BC,
            0xb => Opcode::DCX_BC,
            0xc => Opcode::INR_C,
            0xd => Opcode::DCR_C,
            0xe => Opcode::MVI_C,
            0xf => Opcode::RRC,

            0x11 => Opcode::MVI_DE,
            0x12 => Opcode::STA_DE,
            0x13 => Opcode::INX_DE,
            0x14 => Opcode::INR_D,
            0x15 => Opcode::DCR_D,
            0x16 => Opcode::MVI_D,
            0x17 => Opcode::RAL,

            0x19 => Opcode::DAD_DE,
            0x1a => Opcode::LDA_DE,
            0x1b => Opcode::DCX_DE,
            0x1c => Opcode::INR_E,
            0x1d => Opcode::DCR_E,
            0x1e => Opcode::MVI_E,
            0x1f => Opcode::RAR,
            0x20 => Opcode::RIM,
            0x21 => Opcode::MVI_HL,
            0x22 => Opcode::SHLD,
            0x23 => Opcode::INX_HL,
            0x24 => Opcode::INR_H,
            0x25 => Opcode::DCR_H,
            0x26 => Opcode::MVI_H,
            0x27 => Opcode::DAA,

            0x29 => Opcode::DAD_HL,
            0x2a => Opcode::LHLD,
            0x2b => Opcode::DCX_HL,
            0x2c => Opcode::INR_L,
            0x2d => Opcode::DCR_L,
            0x2e => Opcode::MVI_L,
            0x2f => Opcode::CMA,
            0x30 => Opcode::SIM,
            0x31 => Opcode::LXI_SP,
            0x32 => Opcode::STA,
            0x33 => Opcode::INX_SP,
            0x34 => Opcode::INR_M,
            0x35 => Opcode::DCR_M,
            0x36 => Opcode::MVI_M,
            0x37 => Opcode::STC,
            0x39 => Opcode::DAD_SP,
            0x3a => Opcode::LDA,
            0x3b => Opcode::DCX_SP,
            0x3c => Opcode::INR_A,
            0x3d => Opcode::DCR_A,
            0x3e => Opcode::MVI_A,
            0x3f => Opcode::CMC,
            0x40 => Opcode::MOV_BB,
            0x41 => Opcode::MOV_BC,
            0x42 => Opcode::MOV_BD,
            0x43 => Opcode::MOV_BE,
            0x44 => Opcode::MOV_BH,
            0x45 => Opcode::MOV_BL,
            0x46 => Opcode::MOV_BM,
            0x47 => Opcode::MOV_BA,
            0x48 => Opcode::MOV_CB,
            0x49 => Opcode::MOV_CC,
            0x4a => Opcode::MOV_CD,
            0x4b => Opcode::MOV_CE,
            0x4c => Opcode::MOV_CH,
            0x4d => Opcode::MOV_CL,
            0x4e => Opcode::MOV_CM,
            0x4f => Opcode::MOV_CA,
            0x50 => Opcode::MOV_DB,
            0x51 => Opcode::MOV_DC,
            0x52 => Opcode::MOV_DD,
            0x53 => Opcode::MOV_DE,
            0x54 => Opcode::MOV_DH,
            0x55 => Opcode::MOV_DL,
            0x56 => Opcode::MOV_DM,
            0x57 => Opcode::MOV_DA,
            0x58 => Opcode::MOV_EB,
            0x59 => Opcode::MOV_EC,
            0x5a => Opcode::MOV_ED,
            0x5b => Opcode::MOV_EE,
            0x5c => Opcode::MOV_EH,
            0x5d => Opcode::MOV_EL,
            0x5e => Opcode::MOV_EM,
            0x5f => Opcode::MOV_EA,
            0x60 => Opcode::MOV_HB,
            0x61 => Opcode::MOV_HC,
            0x62 => Opcode::MOV_HD,
            0x63 => Opcode::MOV_HE,
            0x64 => Opcode::MOV_HH,
            0x65 => Opcode::MOV_HL,
            0x66 => Opcode::MOV_HM,
            0x67 => Opcode::MOV_HA,
            0x68 => Opcode::MOV_LB,
            0x69 => Opcode::MOV_LC,
            0x6a => Opcode::MOV_LD,
            0x6b => Opcode::MOV_LE,
            0x6c => Opcode::MOV_LH,
            0x6d => Opcode::MOV_LL,
            0x6e => Opcode::MOV_LM,
            0x6f => Opcode::MOV_LA,
            0x70 => Opcode::MOV_MB,
            0x71 => Opcode::MOV_MC,
            0x72 => Opcode::MOV_MD,
            0x73 => Opcode::MOV_ME,
            0x74 => Opcode::MOV_MH,
            0x75 => Opcode::MOV_ML,
            0x76 => Opcode::HLT,
            0x77 => Opcode::MOV_MA,
            0x78 => Opcode::MOV_AB,
            0x79 => Opcode::MOV_AC,
            0x7a => Opcode::MOV_AD,
            0x7b => Opcode::MOV_AE,
            0x7c => Opcode::MOV_AH,
            0x7d => Opcode::MOV_AL,
            0x7e => Opcode::MOV_AM,
            0x7F => Opcode::MOV_AA,
            0x80 => Opcode::ADD_B,
            0x81 => Opcode::ADD_C,
            0x82 => Opcode::ADD_D,
            0x83 => Opcode::ADD_E,
            0x84 => Opcode::ADD_H,
            0x85 => Opcode::ADD_L,
            0x86 => Opcode::ADD_M,
            0x87 => Opcode::ADD_A,
            0x88 => Opcode::ADC_B,
            0x89 => Opcode::ADC_C,
            0x8a => Opcode::ADC_D,
            0x8b => Opcode::ADC_E,
            0x8c => Opcode::ADC_H,
            0x8d => Opcode::ADC_L,
            0x8e => Opcode::ADC_M,
            0x8f => Opcode::ADC_A,
            0x90 => Opcode::SUB_B,
            0x91 => Opcode::SUB_C,
            0x92 => Opcode::SUB_D,
            0x93 => Opcode::SUB_E,
            0x94 => Opcode::SUB_H,
            0x95 => Opcode::SUB_L,
            0x96 => Opcode::SUB_M,
            0x97 => Opcode::SUB_A,
            0x98 => Opcode::SBB_B,
            0x99 => Opcode::SBB_C,
            0x9a => Opcode::SBB_D,
            0x9b => Opcode::SBB_E,
            0x9c => Opcode::SBB_H,
            0x9d => Opcode::SBB_L,
            0x9e => Opcode::SBB_M,
            0x9f => Opcode::SBB_A,
            0xa0 => Opcode::ANA_B,
            0xa1 => Opcode::ANA_C,
            0xa2 => Opcode::ANA_D,
            0xa3 => Opcode::ANA_E,
            0xa4 => Opcode::ANA_H,
            0xa5 => Opcode::ANA_L,
            0xa6 => Opcode::ANA_M,
            0xa7 => Opcode::ANA_A,
            0xa8 => Opcode::XRA_B,
            0xa9 => Opcode::XRA_C,
            0xaa => Opcode::XRA_D,
            0xab => Opcode::XRA_E,
            0xac => Opcode::XRA_H,
            0xad => Opcode::XRA_L,
            0xae => Opcode::XRA_M,
            0xaf => Opcode::XRA_A,
            0xb0 => Opcode::ORA_B,
            0xb1 => Opcode::ORA_C,
            0xb2 => Opcode::ORA_D,
            0xb3 => Opcode::ORA_E,
            0xb4 => Opcode::ORA_H,
            0xb5 => Opcode::ORA_L,
            0xb6 => Opcode::ORA_M,
            0xb7 => Opcode::ORA_A,
            0xb8 => Opcode::CMP_B,
            0xb9 => Opcode::CMP_C,
            0xba => Opcode::CMP_D,
            0xbb => Opcode::CMP_E,
            0xbc => Opcode::CMP_H,
            0xbd => Opcode::CMP_L,
            0xbe => Opcode::CMP_M,
            0xbf => Opcode::CMP_A,
            0xc0 => Opcode::RNZ,
            0xc1 => Opcode::POP_BC,
            0xc2 => Opcode::JNZ,
            0xc3 => Opcode::JMP,
            0xc4 => Opcode::CNZ,
            0xc5 => Opcode::PUSH_BC,
            0xc6 => Opcode::ADI,
            0xc7 => Opcode::RST_0,
            0xc8 => Opcode::RZ,
            0xc9 => Opcode::RET,
            0xca => Opcode::JZ,
            0xcc => Opcode::CZ,
            0xcd => Opcode::CALL,
            0xce => Opcode::ACI,
            0xcf => Opcode::RST_1,
            0xd0 => Opcode::RNC,
            0xd1 => Opcode::POP_DE,
            0xd2 => Opcode::JNC,
            0xd3 => Opcode::OUT,
            0xd4 => Opcode::CNC,
            0xd5 => Opcode::PUSH_DE,
            0xd6 => Opcode::SUI,
            0xd7 => Opcode::RST_2,
            0xd8 => Opcode::RC,
            0xda => Opcode::JC,
            0xdb => Opcode::IN,
            0xdc => Opcode::CC,
            0xde => Opcode::SBI,
            0xdf => Opcode::RST_3,
            0xe0 => Opcode::RPO,
            0xe1 => Opcode::POP_HL,
            0xe2 => Opcode::JPO,
            0xe3 => Opcode::XTHL,
            0xe4 => Opcode::CPO,
            0xe5 => Opcode::PUSH_HL,
            0xe6 => Opcode::ANI,
            0xe7 => Opcode::RST_4,
            0xe8 => Opcode::RPE,
            0xe9 => Opcode::PCHL,
            0xea => Opcode::JPE,
            0xeb => Opcode::XCHG,
            0xec => Opcode::CPE,
            0xee => Opcode::XRI,
            0xef => Opcode::RST_5,
            0xf0 => Opcode::RP,
            0xf1 => Opcode::POP_PSW,
            0xf2 => Opcode::JP,
            0xf3 => Opcode::DI,
            0xf4 => Opcode::CP,
            0xf5 => Opcode::PUSH_PSW,
            0xf6 => Opcode::ORI,
            0xf7 => Opcode::RST_6,
            0xf8 => Opcode::RM,
            0xf9 => Opcode::SPHL,
            0xfa => Opcode::JM,
            0xfb => Opcode::EI,
            0xfc => Opcode::CM,
            0xfe => Opcode::CPI,
            0xff => Opcode::RST_7,
            other => return Err(EmuError::InvalidOpcode(other)),
        })
    }

    /// The machine-code byte for this opcode.
    #[inline]
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// `LXI H,d16` shares its encoding with `MVI HL` (0x21); provided as an alias.
    pub const LXI: Opcode = Opcode::MVI_HL;
}
