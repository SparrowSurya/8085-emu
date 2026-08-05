//! # emu8085
//!
//! A cycle-accurate Intel 8085 microprocessor emulator.
//!
//! The emulator is *steppable at the T-state level*: the [`machine::Machine`] drives a
//! [`cpu::Cpu`], a [`bus::SystemBus`], flat [`memory::Memory`], and a device manager one
//! clock at a time, so machine-cycle and interrupt timing can be observed exactly.
//!
//! It utilizes strict typing with closed enums ([`Opcode`], [`Reg8`], [`Reg16`]), a typed flag register
//! ([`Flags`]), newtype wrappers ([`Addr`], [`Port`]), a [`Device`] trait, and
//! `Result<T, EmuError>` for error handling.

pub mod bus;
pub mod device;
pub mod error;
pub mod instruction;
pub mod machine;
pub mod memory;
pub mod program;
pub mod value;

pub mod cpu;

pub use bus::{ControlLines, SystemBus};
pub use cpu::flags::Flags;
pub use cpu::interrupts::{VEC_RST_5_5, VEC_RST_6_5, VEC_RST_7_5, VEC_TRAP};
pub use cpu::registers::{Reg8, Reg16};
pub use cpu::{Cpu, MachineCycle};
pub use device::{Device, DeviceManager, KeyboardDevice, PrinterDevice, TerminalDevice, USBDevice};
pub use error::EmuError;
pub use instruction::opcode::Opcode;
pub use instruction::{Instruction, Operand};
pub use machine::Machine;
pub use memory::Memory;
pub use program::Program;
pub use value::{Addr, Port};
