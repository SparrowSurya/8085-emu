//! The `Machine`: the facade that owns the CPU, system bus, RAM, and device manager and
//! clocks them together. This is the Rust analogue of the Python `Machine.create(...)`.

use crate::bus::SystemBus;
use crate::cpu::Cpu;
use crate::device::{Device, DeviceManager, USBDevice};
use crate::error::EmuError;
use crate::memory::Memory;
use crate::program::Program;
use crate::value::Addr;

/// A complete emulated computer. Public fields let callers inspect or poke the parts
/// (as the reference's tests do); the methods drive them as a unit.
pub struct Machine {
    /// Random-access memory.
    pub ram: Memory,
    /// The processor.
    pub cpu: Cpu,
    /// The shared system bus.
    pub bus: SystemBus,
    /// Attached peripherals.
    pub devices: DeviceManager,
}

impl Default for Machine {
    fn default() -> Self {
        Machine::create(16, 8)
    }
}

impl Machine {
    /// Build a machine with the given address- and data-bus widths (16/8 for a stock 8085).
    pub fn create(address_lines: u32, data_lines: u32) -> Self {
        Machine {
            ram: Memory::from_lines(address_lines),
            cpu: Cpu::new(),
            bus: SystemBus::new(address_lines, data_lines),
            devices: DeviceManager::new(),
        }
    }

    /// Attach a peripheral, mapping it to the given I/O ports.
    pub fn attach_device(&mut self, device: Box<dyn Device>, ports: &[u8]) {
        self.devices.attach(device, ports);
    }

    /// Compile `program` for load address `at`, write it into RAM, and point the PC at it.
    pub fn load(&mut self, program: &Program, at: Addr) -> Result<(), EmuError> {
        let code = program.compile(at)?;
        self.ram.load_bytes(&code, at)?;
        self.cpu.regs.pc = at;
        Ok(())
    }

    /// Advance the whole machine by one clock (one T-state): step the CPU, then let RAM
    /// and devices service whatever the CPU drove onto the bus.
    pub fn tick(&mut self) {
        self.cpu.process(&mut self.bus);
        self.ram.step(&mut self.bus);
        self.devices.step(&mut self.bus);
    }

    /// Alias for [`tick`](Self::tick): the machine is steppable one T-state at a time.
    pub fn step(&mut self) {
        self.tick();
    }

    /// Release the CPU from halt and run until it halts again (or a fault is recorded).
    pub fn run(&mut self) {
        self.cpu.is_halt = false;
        let mut guard = 0u64;
        while !self.cpu.is_halt && self.cpu.fault.is_none() && guard < 100_000_000 {
            self.tick();
            guard += 1;
        }
    }

    /// DMA-write `data` into RAM at `start` on behalf of `usb` (bus-master transfer).
    pub fn dma_write(&mut self, usb: &mut USBDevice, start: u16, data: &[u8]) {
        usb.dma_write(&mut self.cpu, &mut self.bus, &mut self.ram, start, data);
    }

    /// DMA-read `length` bytes from RAM at `start` on behalf of `usb`.
    pub fn dma_read(&mut self, usb: &mut USBDevice, start: u16, length: usize) -> Vec<u8> {
        usb.dma_read(&mut self.cpu, &mut self.bus, &mut self.ram, start, length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{Instruction, Opcode, Operand};

    #[test]
    fn load_and_run_a_small_program() {
        // MVI A, 0x05 ; MVI B, 0x03 ; ADD B ; HLT  ->  A == 8
        let prog = Program::new(vec![
            Instruction::with(Opcode::MVI_A, Operand::byte(0x05)),
            Instruction::with(Opcode::MVI_B, Operand::byte(0x03)),
            Instruction::new(Opcode::ADD_B),
            Instruction::new(Opcode::HLT),
        ]);
        let mut m = Machine::default();
        m.load(&prog, Addr(0x0000)).unwrap();
        m.run();
        assert_eq!(m.cpu.regs.a, 0x08);
        assert!(m.cpu.is_halt);
    }

    #[test]
    fn load_reports_unresolved_labels() {
        let prog = Program::new(vec![Instruction::with(Opcode::JMP, Operand::label("MISSING"))]);
        let mut m = Machine::default();
        assert!(m.load(&prog, Addr(0)).is_err());
    }

    #[test]
    fn usb_dma_round_trips_through_the_machine() {
        let mut m = Machine::default();
        let prog = Program::new(vec![Instruction::new(Opcode::NOP), Instruction::new(Opcode::HLT)]);
        m.load(&prog, Addr(0x00A0)).unwrap();
        let mut usb = USBDevice::new();
        let payload = b"DMA!";
        m.dma_write(&mut usb, 0x0300, payload);
        assert_eq!(m.dma_read(&mut usb, 0x0300, payload.len()), payload);
    }
}
