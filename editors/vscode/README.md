# 8085 Assembly (`.e8085`) VS Code Extension

This extension provides rich syntax highlighting, language configuration, and semantic token coloring for the Intel 8085 assembly language dialect defined in `SPEC_v2.md` for files ending in `.e8085`.

---

## Features

### 1. Directives & Preprocessor
- `%define IDENTIFIER VALUE` — Macro and constant definitions.
- `%repeat COUNT VALUE` — Repeated data expansion in `.data`.
- `%len IDENTIFIER` — Byte-length evaluation for variables.
- `%include "path/to/file"` / `%include 'path/to/file'` — File inclusion with circular protection.

### 2. Scoping & Modular Keywords
- `global <label>:` / `export <label>:` — Export labels globally for linking.
- `extern <symbol>` — External symbol declarations.
- `.label:` / `jz .label` — Subroutine-scoped local labels.

### 3. Segments & Memory Layout
- `segment .data` — Initialized static data.
- `segment .bss` — Uninitialized, zero-filled reservations.
- `segment .text` — Executable microprocessor code.

### 4. Data Types & Declarations
- `BYTE` — 8-bit units.
- `WORD` — 16-bit little-endian units.
- Automatic variable declaration highlighting (`name BYTE ...` or `table WORD ...`).

### 5. Registers & Special Symbols
- **8-bit General Registers**: `A` (Accumulator), `B`, `C`, `D`, `E`, `H`, `L`.
- **Memory Pointer**: `M` (virtual register representing memory location at `[HL]`).
- **16-bit Register Pairs**: `BC`, `DE`, `HL`.
- **Stack & Status**: `SP` (Stack Pointer), `PSW` (Program Status Word / Flags).

### 6. Constants & Variables
- **Constants**: UPPERCASE identifiers (such as `CMD_WRITE`, `DATA_PORT`, `BUFFER_SIZE`) are automatically highlighted as constants.
- **Variables**: Identifiers referenced in data transfer operations (e.g. `lxi HL, prompt`, `lhld ptr`) are styled as variables.

### 7. Labels & Control Flow
- **Label Definitions**: Standalone line declarations (e.g. `send_prompt:`).
- **Branch / Jump Targets**: Target operands in control instructions (e.g. `jnz send_prompt`, `call add_three`, `jmp main`) are highlighted as label/function references.

### 8. Complete 8085 Instruction Set (Case-Insensitive)
- **Data Transfer**: `MOV`, `MVI`, `LXI`, `LDA`, `STA`, `LDAX`, `STAX`, `LHLD`, `SHLD`, `XCHG`, `XTHL`, `SPHL`, `PCHL`
- **Arithmetic**: `ADD`, `ADI`, `ADC`, `ACI`, `SUB`, `SUI`, `SBB`, `SBI`, `INR`, `DCR`, `INX`, `DCX`, `DAD`, `DAA`
- **Logical**: `ANA`, `ANI`, `XRA`, `XRI`, `ORA`, `ORI`, `CMP`, `CPI`, `CMA`, `CMC`, `STC`, `RLC`, `RRC`, `RAL`, `RAR`
- **Branch & Control**: `JMP`, `JZ`, `JNZ`, `JC`, `JNC`, `JP`, `JM`, `JPE`, `JPO`, `CALL`, `CZ`, `CNZ`, `CC`, `CNC`, `CP`, `CM`, `CPE`, `CPO`, `RET`, `RZ`, `RNZ`, `RC`, `RNC`, `RP`, `RM`, `RPE`, `RPO`, `RST`
- **Stack Operations**: `PUSH`, `POP`
- **I/O Ports**: `IN`, `OUT`
- **Machine Control**: `NOP`, `HLT`, `EI`, `DI`, `RIM`, `SIM`

### 9. Numeric Literals & Comments
- **Hexadecimal**: `0xFF`, `0x1234`
- **Binary**: `0b1010`, `0B11001100`
- **Octal**: `0o77`, `0O377`
- **Decimal**: `0`, `42`, `255`
- **Strings & Characters**: `"Hello, World!"`, `'A'`
- **Comments**: Semicolon comments (`; ...`) extending to the end of the line.

---

## How to Use & Open in VS Code

### Quick Launch (Extension Development Mode)
Run the bundled script from the project root:
```bash
bash open.sh
```
This starts VS Code with the local extension loaded for the workspace.

### Permanent Installation (Symlink)
To make the extension globally available across all VS Code sessions:
```bash
ln -s "$(pwd)/editors/vscode" $HOME/.vscode/extensions/e8085-assembly
```
