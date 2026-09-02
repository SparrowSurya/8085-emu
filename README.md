# emu8085

A cycle-accurate **Intel 8085 Microprocessor Emulator & Assembler Toolchain** implemented in modern Rust.

The emulator is **steppable at the T-state clock level**: the [`Machine`](src/machine.rs) drives a cycle-accurate [`Cpu`](src/cpu/mod.rs), a typed [`SystemBus`](src/bus/lines.rs), flat [`Memory`](src/memory/mod.rs) with fault interception, and attached peripheral devices one clock cycle at a time.

---

## Quick Start

### 1. Running Programs (`run`)
Run any assembly source file (`.e8085`) or precompiled binary (`.8085.bin`):

```bash
# Run assembly file directly
cargo run --bin e8085 -- run programs/demo.e8085

# Run Hello World program
cargo run --bin e8085 -- run programs/hello_world.e8085

# Run precompiled standalone binary (no extra linker flags needed)
cargo run --bin e8085 -- run greet.8085.bin

# Run assembly file with external library linking on-the-fly
cargo run --bin e8085 -- run programs/greet.e8085 -l terminal.8085.bin
```

### 2. Compiling Programs & Linking Libraries (`compile`)
Compile `.e8085` source into a self-contained `.8085.bin` binary image:

```bash
# 1. Compile reusable subroutine library (terminal helper)
cargo run --bin e8085 -- compile programs/terminal.e8085 -o terminal.8085.bin

# 2. Statically link library into a standalone executable binary
cargo run --bin e8085 -- compile programs/greet.e8085 -l terminal.8085.bin -o greet.8085.bin

# 3. Run the standalone binary directly
cargo run --bin e8085 -- run greet.8085.bin
```

### 3. Disassembling Binary Images (`disassemble`)
Disassemble a `.8085.bin` machine code file into standard 8085 assembly instructions with exported symbol and entry-point annotations:

```bash
cargo run --bin e8085 -- disassemble greet.8085.bin
```

### 4. Inspecting Binary Containers (`inspect` & `strings`)
Analyze container structure, segment memory maps, exported symbol tables, and extract printable strings:

```bash
# Full diagnostic report (default --all)
cargo run --bin e8085 -- inspect greet.8085.bin

# Inspect specific sections
cargo run --bin e8085 -- inspect greet.8085.bin --header
cargo run --bin e8085 -- inspect greet.8085.bin --segments
cargo run --bin e8085 -- inspect greet.8085.bin --symbols
cargo run --bin e8085 -- inspect greet.8085.bin --strings

# Extract printable ASCII strings (shortcut)
cargo run --bin e8085 -- strings greet.8085.bin -n 4
```

### 5. Running Rust API Programmatic Examples
Run any of the Rust API examples demonstrating direct hardware and emulator interaction:

```bash
# Hello World through the Rust builder API
cargo run --example hello_world

# USB DMA memory transfer
cargo run --example usb_dma_transfer

# Keyboard interrupt handling
cargo run --example keyboard_input_interrupt
```

### 6. Running the Test Suite
```bash
# Run all unit tests, integration tests, and doc-tests
cargo test --all-targets && cargo test --doc
```

---

## File Structure

```text
emu8085/
├── programs/           # 8085 user assembly programs (.e8085)
│   ├── demo.e8085              # Interactive terminal I/O demo
│   ├── terminal.e8085          # Terminal I/O subroutine library (print, input, putch, endl)
│   ├── greet.e8085             # Interactive greeting using extern subroutine linking
│   ├── hello_world.e8085       # Terminal device string display
│   ├── array_sum.e8085         # Array traversal & summation (ADD M)
│   ├── directives.e8085        # %define, %repeat, %len, and .bss
│   ├── hardware_trap.e8085     # Hardware TRAP exception handling
│   ├── print_stars.e8085       # Printer port loop counter
│   ├── software_interrupts.e8085 # Custom RST 1 & RST 2 ISR routines
│   └── subroutine.e8085        # CALL and RET subroutine execution
│
├── examples/           # Rust programmatic API examples (.rs)
│   ├── arithmetic_immediate.rs # Immediate arithmetic operations
│   ├── arithmetic_register.rs  # Register-to-register arithmetic
│   ├── bcd_arithmetic.rs       # DAA and BCD arithmetic
│   ├── branching_control.rs    # Conditional jumps and flags
│   ├── data_transfer.rs        # MOV, MVI, LDA, STA, LHLD, SHLD
│   ├── hello_world.rs          # Basic machine initialization
│   ├── hello_world_labels.rs   # Label resolution in Program compiler
│   ├── keyboard_input_interrupt.rs # Keyboard hardware interrupt & INTA
│   ├── logical_operations.rs   # ANA, ORA, XRA, CMA, CMP
│   ├── loop_multiplication_labels.rs # Loop arithmetic with labels
│   ├── printer_output.rs       # Direct port output streaming
│   ├── register_pair_arithmetic.rs # DAD, INX, DCX operations
│   ├── stack_operations.rs     # PUSH, POP, XTHL, SPHL
│   ├── system_control_pins.rs  # READY, HOLD/HLDA, RESET_IN
│   └── usb_dma_transfer.rs     # Bus mastering & DMA transfer
│
├── src/                # Core library and binaries
│   ├── bin/
│   │   └── e8085.rs            # Unified CLI binary (run, compile, disassemble)
│   ├── asm/                    # Two-pass 8085 Assembler & Static Linker toolchain
│   │   ├── assemble.rs         # Layout, vector table, static linking, and image generation
│   │   ├── include.rs          # Source preprocessor for %include resolution
│   │   ├── container.rs        # .8085.bin binary container encoding/decoding
│   │   ├── lexer.rs            # Lexical tokenizer (4 number bases, strings, directives)
│   │   ├── parser.rs           # Recursive-descent AST parser
│   │   ├── encode.rs           # Opcode & operand instruction encoder
│   │   ├── token.rs            # Token definitions and spans
│   │   └── ast.rs              # AST types for segments, directives, and instructions
│   ├── cpu/                    # 8085 CPU core
│   │   ├── mod.rs              # Steppable CPU state machine
│   │   ├── execute.rs          # Instruction decoder & execution steps
│   │   ├── alu.rs              # Full ALU with 8-bit arithmetic & DAA
│   │   ├── flags.rs            # Typed flags (S, Z, AC, P, CY) & PSW
│   │   ├── registers.rs        # Reg8 (A..L), Reg16 (BC, DE, HL, SP, PC, WZ)
│   │   └── interrupts.rs       # Priority interrupts (TRAP, RST 7.5..5.5, INTR)
│   ├── bus/                    # Shared system bus & control lines
│   ├── memory/                 # RAM with boundary & fault protection
│   ├── device/                 # Peripherals (Terminal, Printer, Keyboard, USB)
│   ├── instruction/            # Instruction types and Opcode enum
│   ├── machine.rs              # Unified system facade (run, step, tick, DMA)
│   └── lib.rs                  # Crate root and documentation
│
├── tests/              # Integration and verification test suites
│   ├── linking_integration.rs  # Static linking, local labels, %include & extern tests
│   ├── programs_integration.rs # Tests all programs/ files end-to-end
│   ├── interrupt_software_and_hardware.rs # Software RST and Hardware TRAP tests
│   ├── asm_coverage.rs         # Assembler directives & layout coverage
│   ├── asm_end_to_end.rs       # Assembler end-to-end integration tests
│   ├── arith_integration.rs    # Differential ALU tests against 8085 reference
│   ├── control_integration.rs  # Branching & stack differential tests
│   ├── device_integration.rs   # Peripheral device tests
│   ├── examples_integration.rs # Automated example verification
│   ├── fuzz_integration.rs     # Randomized fuzzing against reference
│   ├── interrupt_integration.rs# Interrupt priority & timing tests
│   ├── program_label_tests.rs  # Label resolution integration tests
│   └── terminal_integration.rs # Terminal device I/O tests
│
└── doc/                # Detailed technical documentation
    ├── ASSEMBLER.md            # In-depth Assembler pipeline, container format & static linker
    └── GRAMMAR.md              # Complete .e8085 language reference & syntax
```

---

## Architectural Features

- **Cycle-Accurate T-State Execution**: Step one clock cycle at a time via `machine.tick()` or step per-instruction via `machine.step()`. Per-opcode timing matches hardware reference specifications.
- **Hardware & Software Interrupts**:
  - Highest priority non-maskable **TRAP** (`0x0024`) for hardware exceptions (illegal opcode, memory faults).
  - Maskable hardware interrupts: **RST 7.5** (`0x003C`), **RST 6.5** (`0x0034`), **RST 5.5** (`0x002C`), and **INTR** (`INTA` vectoring).
  - Software interrupts: **RST 0** (`0x0000`) through **RST 7** (`0x0038`) with automatic Interrupt Vector Table mapping to `isr_rst<n>` subroutines.
- **Direct Memory Access (DMA)**: Hardware `HOLD` / `HLDA` bus-master handshake allowing peripherals like `USBDevice` to stream directly to/from memory.
- **Modular Assembler & Static Linker**:
  - Two-pass assembler supporting `%define`, `%repeat`, `%len`, segments (`.data`, `.bss`, `.text`), 4 number bases, and string literals.
  - Source inclusion with `%include "file.e8085"`.
  - Subroutine-scoped local labels (`.name:` and `jz .name`).
  - Modular library export (`global` / `export`) with export symbol tables.
  - External referencing (`extern <symbol>`) with static binary linking (`-l <library.8085.bin>`) to produce standalone executables.
  - Entry-point verification: non-executable library binaries (without `main`) are rejected from execution.
- **Rich Peripheral Set**:
  - `TerminalDevice`: Two-port virtual terminal supporting line-buffered input and output.
  - `PrinterDevice`: Character stream capture device with callbacks.
  - `KeyboardDevice`: FIFO-buffered keyboard input device with INTR/INTA interrupt vectoring.
  - `USBDevice`: DMA-capable high-speed transfer controller.

---

## Detailed Documentation

For comprehensive technical documentation, refer to:
- [**doc/ASSEMBLER.md**](doc/ASSEMBLER.md) — Detailed guide to the assembler pipeline, container layout, static linking, and symbol resolution.
- [**doc/GRAMMAR.md**](doc/GRAMMAR.md) — Full language reference for `.e8085` assembly programs, syntax rules, directives, registers, and instructions.
