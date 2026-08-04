//! Peripheral devices and the manager that routes bus I/O to them.
//!
//! The Python original duck-typed devices; here [`Device`] is a trait with defaulted
//! methods, so a concrete device overrides only what it uses. [`DeviceManager`] owns its
//! devices as `Box<dyn Device>` (heterogeneous and open-ended) and maps I/O ports to
//! them by index, avoiding shared-mutability wrappers.

pub mod keyboard;
pub mod printer;
pub mod usb;

pub use keyboard::KeyboardDevice;
pub use printer::PrinterDevice;
pub use usb::USBDevice;

use crate::bus::SystemBus;
use std::collections::HashMap;

/// A peripheral attached to the system. All hooks are defaulted so a device implements
/// only the ones it needs (an output device overrides `port_write`, an interrupting
/// device overrides `on_inta`, and so on).
pub trait Device {
    /// Human-readable device name.
    fn name(&self) -> &str;

    /// Respond to an `IN` from `port`. Defaults to the idle bus value `0xFF`.
    fn port_read(&mut self, _port: u8) -> u8 {
        0xFF
    }

    /// Handle an `OUT` of `data` to `port`. Defaults to ignoring it.
    fn port_write(&mut self, _port: u8, _data: u8) {}

    /// Supply a restart opcode during an INTA acknowledge cycle, or `0xFF` for "none".
    fn on_inta(&mut self) -> u8 {
        0xFF
    }

    /// Per-clock hook for devices that need to observe the bus each tick.
    fn tick(&mut self, _bus: &mut SystemBus) {}
}

/// Owns the attached devices and routes I/O-port and INTA traffic to them, mirroring
/// the Python `DeviceManager`.
#[derive(Default)]
pub struct DeviceManager {
    devices: Vec<Box<dyn Device>>,
    port_map: HashMap<u8, usize>,
}

impl DeviceManager {
    /// An empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a device and map it to the given I/O ports.
    pub fn attach(&mut self, device: Box<dyn Device>, ports: &[u8]) {
        let idx = self.devices.len();
        self.devices.push(device);
        for &p in ports {
            self.port_map.insert(p, idx);
        }
    }

    /// Read a byte from whatever device owns `port`, or `0xFF` if none.
    pub fn read_port(&mut self, port: u8) -> u8 {
        match self.port_map.get(&port).copied() {
            Some(i) => self.devices[i].port_read(port),
            None => 0xFF,
        }
    }

    /// Write a byte to whatever device owns `port` (ignored if none).
    pub fn write_port(&mut self, port: u8, data: u8) {
        if let Some(&i) = self.port_map.get(&port) {
            self.devices[i].port_write(port, data);
        }
    }

    /// Borrow an attached device by insertion order (handy for inspecting state in tests).
    pub fn device(&self, idx: usize) -> Option<&dyn Device> {
        self.devices.get(idx).map(|b| b.as_ref())
    }

    /// Service one bus cycle: fulfil an I/O read/write, or during INTA let the first
    /// device offering a vector drive it. Then tick every device.
    pub fn step(&mut self, bus: &mut SystemBus) {
        if bus.lines.ior {
            let port = bus.address().low();
            let v = self.read_port(port);
            bus.set_data(v);
        } else if bus.lines.iow {
            let port = bus.address().low();
            self.write_port(port, bus.data());
        } else if bus.lines.inta {
            for dev in self.devices.iter_mut() {
                let v = dev.on_inta();
                if v != 0xFF {
                    bus.set_data(v);
                    break;
                }
            }
        }
        for dev in self.devices.iter_mut() {
            dev.tick(bus);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo(String);
    impl Device for Echo {
        fn name(&self) -> &str {
            &self.0
        }
        fn port_read(&mut self, port: u8) -> u8 {
            port ^ 0xAA
        }
    }

    #[test]
    fn unmapped_port_reads_ff() {
        let mut m = DeviceManager::new();
        assert_eq!(m.read_port(0x10), 0xFF);
    }

    #[test]
    fn routes_read_to_mapped_device() {
        let mut m = DeviceManager::new();
        m.attach(Box::new(Echo("echo".into())), &[0x10]);
        assert_eq!(m.read_port(0x10), 0x10 ^ 0xAA);
        assert_eq!(m.read_port(0x11), 0xFF); // unmapped
    }
}