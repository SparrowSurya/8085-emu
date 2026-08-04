//! The single-bit control, status, and handshake lines carried by the system bus.
//!
//! The Python bus packed all of these into one integer behind bit-masks; here each
//! line is a named `bool`, which is what the CPU, memory, and devices actually want
//! to read and toggle.

/// Every one-bit line on the bus other than the address and data buses.
///
/// Grouped into one struct so the bus can offer a single `lines` field and so the
/// per-fetch [`ControlLines::reset`] (which mirrors the Python `bus.reset()`) has an
/// obvious home.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlLines {
    /// Memory read strobe.
    pub mr: bool,
    /// Memory write strobe.
    pub mw: bool,
    /// I/O read strobe.
    pub ior: bool,
    /// I/O write strobe.
    pub iow: bool,
    /// Interrupt acknowledge (driven during an `INTA` cycle).
    pub inta: bool,
    /// DMA hold request, asserted by a peripheral that wants the bus.
    pub hold: bool,
    /// DMA hold acknowledge, asserted by the CPU once it has released the bus.
    pub hlda: bool,
    /// Ready line; when low the CPU inserts wait states.
    pub ready: bool,
    /// Hardware reset input.
    pub reset_in: bool,
    /// Hardware reset output, asserted while the CPU is being reset.
    pub reset_out: bool,
}

impl ControlLines {
    /// Power-on line state: everything low except `ready`, which idles high so the CPU
    /// runs unless a peripheral explicitly pulls it low.
    pub fn new() -> Self {
        ControlLines {
            ready: true,
            ..Default::default()
        }
    }

    /// Clear the four bus-transaction strobes and `INTA` at the start of a fetch,
    /// matching the Python `SystemBus.reset()`. Handshake/reset lines are left alone.
    pub fn reset(&mut self) {
        self.mr = false;
        self.mw = false;
        self.ior = false;
        self.iow = false;
        self.inta = false;
    }
}
