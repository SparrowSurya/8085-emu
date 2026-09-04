# 8085 Assembly (`.e8085`) VS Code Extension

This extension provides rich syntax highlighting, language configuration, and semantic token coloring for the Intel 8085 assembly language dialect defined in `SPEC_v2.md` for files ending in `.e8085`.

---

## Features

### 1. Directives & Preprocessor
- `%define IDENTIFIER VALUE` — 1-byte constant definitions ($\le \text{0xFF}$, 1-byte character literal `'x'`, or 1-byte string).
- `%repeat COUNT VALUE` — Repeated data expansion in `.data`.
- `%len IDENTIFIER` — Byte-length evaluation for variables (`%len var`).
- `%include "path/to/file.e8085"` — File inclusion in double quotes with circular protection.

### 2. Scoping & Modular Keywords
- `global <label>:` — Export labels globally for linking (disallowing `global main`).
- `extern <symbol>` — External symbol declarations (satisfied by local definition if included).
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

### 9. Numeric Literals, Escape Sequences & Comments
- **Hexadecimal**: `0xFF`, `0x1234`
- **Binary**: `0b1010`, `0B11001100`
- **Octal**: `0o77`, `0O377`
- **Decimal**: `0`, `42`, `255`
- **Character Literals**: Single-quoted 1-byte constant literals (`'A'`, `'\n'`, `'\t'`, `'\0'`, `'\''`, `'\\'`, `\xHH`).
- **Strings with Escapes**: `"Hello, World!\n\0"`
- **Comments**: Semicolon comments (`; ...`) extending to the end of the line.

### 10. Markdown Code Blocks & Preview Integration
Full syntax highlighting support inside Markdown files (`.md`) and the VS Code **Markdown Preview**:
- **Editor Fenced Blocks**: Embedded `source.e8085` syntax highlighting in `.md` files for ```` ```e8085 ```` code fences.
- **Markdown Preview Webview**: Built-in `markdown-it` plugin and theme-adaptive preview stylesheet (`markdown.previewStyles`) ensuring rich 8085 color styling in both light and dark preview modes.

### 11. Interactive Cycle-Accurate Debugging (DAP 1.6+)
Integrated Debug Adapter Protocol (DAP) server support:
- **Stepping**: Step In (`F11`), Step Over (`F10`), Step Out (`Shift+F11`), and Time Travel / Step Back (`Ctrl+F10` / reverse continue).
- **Breakpoints**: Line breakpoints, conditional expressions (e.g., `A == 0x05`, `*0x2000 > 10`), and hit counts (`> 5`, `== 3`).
- **Scopes & Variable Inspection**:
  - **CPU Registers**: 8-bit registers (`A`, `B`, `C`, `D`, `E`, `H`, `L`), `SP`, `PC`, and T-States / Cycles.
  - **Register Pairs**: 16-bit views (`BC`, `DE`, `HL`, `PSW`).
  - **Flags (PSW)**: Individual flag bits (`S`, `Z`, `AC`, `P`, `CY`).
  - **Data Variables**: Live values for variables defined in `.data` and `.bss`.
  - **Peripherals**: Real-time inspection of attached devices (`Terminal`, `Printer`, `Keyboard`).
- **Live State Mutation**: Edit register and memory values on the fly during debugging.
- **Debug Console / REPL**: Interactive expression evaluation, memory dereferencing (`*(HL)`, `*0x2000`), and REPL commands (`:mem 0x2000 16`, `:regs`).

---

## Debugging Setup (`launch.json`)

Press `F5` in any open `.e8085` file, or create `.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "e8085",
      "name": "Debug 8085 Assembly",
      "request": "launch",
      "program": "${file}",
      "stopOnEntry": true,
      "maxHistory": 256
    }
  ]
}
```

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
