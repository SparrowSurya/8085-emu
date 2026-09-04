//! # emu8085
//!
//! A cycle-accurate Intel 8085 microprocessor emulator and assembler toolchain.
//!
//! The emulator is *steppable at the T-state level*: the [`Machine`] drives a
//! [`Cpu`], a [`SystemBus`], flat [`Memory`], and attached peripherals one
//! clock at a time, so machine-cycle and interrupt timing can be observed exactly.
//!
//! ## Core Components
//! - [`asm`]: The complete 8085 assembler toolchain supporting macros/defines (`%define`),
//!   data repetition (`%repeat`), length resolution (`%len`), memory segments (`.data`, `.bss`, `.text`),
//!   hardware/software interrupt vector table generation, and assembling into a [`asm::LoadImage`].
//! - [`cpu`]: Cycle-accurate CPU core modeling registers ([`Reg8`], [`Reg16`]), flags ([`Flags`]),
//!   ALU operations, hardware/software interrupts, and machine cycles.
//! - [`bus`]: Shared 16-bit address and 8-bit data bus with control strobes and handshakes ([`SystemBus`], [`ControlLines`]).
//! - [`memory`]: Byte-addressable RAM with bus-driven access and illegal address fault detection ([`Memory`]).
//! - [`device`]: Peripheral device management ([`Device`], [`DeviceManager`]) with built-in
//!   devices like [`TerminalDevice`], [`PrinterDevice`], [`KeyboardDevice`], and [`USBDevice`].
//! - [`machine`]: Top-level system facade ([`Machine`]) clocking all subsystems together.
//! - [`instruction`]: Opcode enumerations ([`Opcode`]) and instruction representations ([`Instruction`]).
//!
//! It utilizes strict typing with closed enums ([`Opcode`], [`Reg8`], [`Reg16`]), a typed flag register
//! ([`Flags`]), newtype wrappers ([`Addr`], [`Port`]), a [`Device`] trait, and
//! `Result<T, EmuError>` for error handling.

pub mod asm;
pub mod dap;
pub mod lsp;

pub mod bus;
pub mod device;
pub mod error;
pub mod instruction;
pub mod machine;
pub mod memory;
pub mod program;
pub mod value;

pub mod cpu;

pub use asm::{
    BinaryContainer, ContainerHeader, ExtractedString, InspectOptions, ListingRow, LoadImage,
    SegmentRecord, assemble, assemble_and_link, assemble_full, assemble_listing,
    assemble_with_options, assemble_with_symbols, extract_strings, format_header, format_segments,
    format_strings, format_symbols, get_segments, inspect_container, load,
};
pub use bus::{ControlLines, SystemBus};
pub use cpu::flags::Flags;
pub use cpu::interrupts::{VEC_RST_5_5, VEC_RST_6_5, VEC_RST_7_5, VEC_TRAP};
pub use cpu::registers::{Reg8, Reg16};
pub use cpu::{Cpu, MachineCycle};
pub use device::{Device, DeviceManager, KeyboardDevice, PrinterDevice, TerminalDevice, USBDevice};
pub use error::EmuError;
pub use instruction::opcode::Opcode;
pub use instruction::{
    DisassembleOptions, DisassemblyRow, Instruction, Operand, disassemble_bytes,
    disassemble_container, disassemble_container_with_options, opcode_t_states,
};
pub use machine::Machine;
pub use memory::Memory;
pub use program::Program;
pub use value::{Addr, Port};
