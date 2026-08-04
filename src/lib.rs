//! # emu8085
//!
//! A cycle-accurate Intel 8085 microprocessor emulator, rewritten in Rust from the
//! Python project [`SparrowSurya/8085-emu`](https://github.com/SparrowSurya/8085-emu).
//!
//! The emulator is *steppable at the T-state level*: the [`machine::Machine`] drives a
//! [`cpu::Cpu`], a [`bus::SystemBus`], flat [`memory::Memory`], and a device manager one
//! clock at a time, so machine-cycle and interrupt timing can be observed exactly.
//!
//! Where the Python original leaned on dynamic typing (an arbitrary-width `Data` value,
//! bit-packed bus integers, dicts of opcode handlers, duck-typed devices), this rewrite
//! uses closed enums ([`Opcode`], [`Reg8`], [`Reg16`]), a typed flag register
//! ([`Flags`]), newtype wrappers ([`Addr`], [`Port`]), a [`Device`] trait, and
//! `Result<T, EmuError>` in place of exceptions.
//!
//! ## Build order
//! This file only wires up the modules that exist so far. Later modules (`cpu`, `bus`,
//! `memory`, `program`, `device`, `machine`) are added as the rewrite proceeds.

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
pub use cpu::{Cpu, MachineCycle};
pub use cpu::flags::Flags;
pub use cpu::interrupts::{VEC_RST_5_5, VEC_RST_6_5, VEC_RST_7_5, VEC_TRAP};
pub use cpu::registers::{Reg16, Reg8};
pub use device::{Device, DeviceManager, KeyboardDevice, PrinterDevice, USBDevice};
pub use error::EmuError;
pub use instruction::opcode::Opcode;
pub use instruction::{Instruction, Operand};
pub use machine::Machine;
pub use memory::Memory;
pub use program::Program;
pub use value::{Addr, Port};
