//! A USB-like peripheral that moves data to and from RAM by direct memory access,
//! using the standard HOLD/HLDA bus-master handshake.

use super::Device;
use crate::bus::SystemBus;
use crate::cpu::Cpu;
use crate::memory::Memory;
use crate::value::Addr;

/// A high-speed device that takes over the bus (via HOLD) to read or write RAM without
/// the CPU. It keeps a mirror of the most recent transfer for inspection.
///
/// DMA is a bus-master operation, not port I/O, so these methods drive the CPU, bus, and
/// RAM directly rather than going through the [`DeviceManager`](super::DeviceManager).
#[derive(Debug, Default)]
pub struct USBDevice {
    /// A copy of the bytes moved by the most recent DMA transfer.
    pub mirror: Vec<u8>,
}

impl USBDevice {
    /// A fresh USB device with an empty mirror.
    pub fn new() -> Self {
        Self::default()
    }

    /// DMA-write `data` into RAM starting at `start`: assert HOLD, wait for the CPU to
    /// grant the bus (HLDA), write directly, then release HOLD. Two CPU ticks bracket the
    /// transfer, matching the reference.
    pub fn dma_write(
        &mut self,
        cpu: &mut Cpu,
        bus: &mut SystemBus,
        ram: &mut Memory,
        start: u16,
        data: &[u8],
    ) {
        bus.lines.hold = true;
        tick(cpu, bus, ram);
        if bus.lines.hlda || cpu.in_hold() {
            self.mirror.clear();
            for (i, &b) in data.iter().enumerate() {
                ram.write(Addr(start.wrapping_add(i as u16)), b);
                self.mirror.push(b);
            }
        }
        bus.lines.hold = false;
        tick(cpu, bus, ram);
    }

    /// DMA-read `length` bytes from RAM starting at `start`, using the same handshake.
    pub fn dma_read(
        &mut self,
        cpu: &mut Cpu,
        bus: &mut SystemBus,
        ram: &mut Memory,
        start: u16,
        length: usize,
    ) -> Vec<u8> {
        bus.lines.hold = true;
        tick(cpu, bus, ram);
        let mut out = Vec::with_capacity(length);
        if bus.lines.hlda || cpu.in_hold() {
            self.mirror.clear();
            for i in 0..length {
                let v = ram.read(Addr(start.wrapping_add(i as u16)));
                out.push(v);
                self.mirror.push(v);
            }
        }
        bus.lines.hold = false;
        tick(cpu, bus, ram);
        out
    }
}

impl Device for USBDevice {
    fn name(&self) -> &str {
        "USBDevice"
    }
}

/// One machine tick's worth of CPU + memory stepping (the DMA driver's clock).
fn tick(cpu: &mut Cpu, bus: &mut SystemBus, ram: &mut Memory) {
    cpu.process(bus);
    ram.step(bus);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dma_write_then_read_round_trips_through_ram() {
        let mut cpu = Cpu::new();
        let mut ram = Memory::from_lines(16);
        let mut bus = SystemBus::default();
        // A tiny program so the CPU is live; DMA should still preempt it.
        ram.load_bytes(&[0x00, 0x76], Addr(0x00A0)).unwrap();
        cpu.start_at(Addr(0x00A0));

        let mut usb = USBDevice::new();
        let payload = b"USB_DMA_PACKET";
        usb.dma_write(&mut cpu, &mut bus, &mut ram, 0x0200, payload);

        // RAM now holds the payload.
        for (i, &b) in payload.iter().enumerate() {
            assert_eq!(ram.read(Addr(0x0200 + i as u16)), b);
        }
        // And a DMA read returns it verbatim.
        let back = usb.dma_read(&mut cpu, &mut bus, &mut ram, 0x0200, payload.len());
        assert_eq!(back, payload);
    }

    #[test]
    fn cpu_asserts_hlda_during_transfer() {
        let mut cpu = Cpu::new();
        let mut ram = Memory::from_lines(16);
        let mut bus = SystemBus::default();
        cpu.start_at(Addr(0));
        let mut usb = USBDevice::new();
        usb.dma_write(&mut cpu, &mut bus, &mut ram, 0x0100, &[1, 2, 3]);
        assert_eq!(usb.mirror, vec![1, 2, 3]);
    }
}