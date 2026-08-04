# emu8085

A cycle-accurate Intel 8085 microprocessor emulator in Rust, rewritten from the Python
project [`SparrowSurya/8085-emu`](https://github.com/SparrowSurya/8085-emu). The Python
source was treated as a *behavioral specification*: this is an idiomatic Rust design
(closed enums, traits, `Result`, newtypes, explicit ownership), not a line-by-line port,
but its observable behavior is verified against the reference.

## Quick start

```rust
use emu8085::{Addr, Instruction, Machine, Opcode, Operand, Program};

let program = Program::new(vec![
    Instruction::with(Opcode::MviA, Operand::byte(0x05)),
    Instruction::with(Opcode::MviB, Operand::byte(0x03)),
    Instruction::new(Opcode::AddB),
    Instruction::new(Opcode::Hlt),
]);

let mut machine = Machine::create(16, 8);
machine.load(&program, Addr(0x0000)).unwrap();
machine.run();
assert_eq!(machine.cpu.regs.a, 0x08);
```

Run the ported examples:

```
cargo run --example hello_world
cargo run --example hello_world_labels
cargo test
```

## What it models

- **CPU**: A–L, hidden W/Z, PC, SP, the five flags, and the PSW (unused bits preserved
  byte-for-byte across `PUSH`/`POP PSW`, matching the reference).
- **T-state accuracy**: `Machine::step`/`tick` advances one T-state; per-opcode timing is
  extracted verbatim from the reference so instruction and interrupt cycle counts match.
- **Full instruction set**: every documented 8085 opcode, across data transfer,
  arithmetic/logical, stack/subroutine, branching, and machine/I-O control.
- **Interrupts**: TRAP, RST 7.5/6.5/5.5 (with masking via `SIM`/`RIM`), INTR + INTA, and
  software `RST 0`–`RST 7`, in hardware priority order.
- **DMA**: HOLD/HLDA bus-master handshake, plus `READY` wait states and `RESET_IN`.
- **Devices**: a `Device` trait with a `DeviceManager`, and keyboard, USB (DMA), and
  printer peripherals.
- **Assembly with labels**: a `Program` compiler with two-pass label resolution.

## Module layout

```
src/
  lib.rs            crate root + public re-exports
  error.rs          EmuError
  value.rs          Addr / Port newtypes
  bus/              SystemBus + typed control lines
  memory/           flat RAM
  cpu/              Cpu, registers, flags, alu, execute, interrupts
  instruction/      Opcode enum, Instruction, Operand
  program/          Program + two-pass label compiler
  device/           Device trait, DeviceManager, keyboard/usb/printer
  machine.rs        Machine facade (create/load/run/step/tick + DMA)
tests/              differential + integration suites
examples/           runnable ported example programs
```

## How it's verified

Behavior is checked against the original Python emulator, not just against hand-written
expectations:

- **1,160 ALU vectors** — every arithmetic/logical/rotate/DAA operation over an
  edge-case input grid, exact on result byte and full PSW.
- **~1,100 randomized fuzz programs** — pseudo-random non-control instruction streams,
  exact on all registers, probed memory, and T-state count. (This suite caught the one
  real divergence during development: PSW unused-bit preservation.)
- **Curated program suites** — arithmetic, control/stack/branch, interrupts (with cycle
  counts), devices (including the INTR→INTA round-trip), and the reference's own labeled
  example programs compiling to byte-identical output.

All checks pass with zero compiler warnings.

## Note on the reference spec

A couple of the reference's quirks are reproduced deliberately for fidelity: `LXI SP`
(opcode `0x31`) is undefined in the spec (while `INX/DCX/DAD SP` exist), and `PUSH`/`POP
PSW` preserve whatever unused bits were loaded rather than forcing hardware values.
