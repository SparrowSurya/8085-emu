# .e8085 Language Grammar & Syntax Reference

This document defines the formal grammar, syntax rules, directives, memory segmentation, and instruction set for `.e8085` (and `.asm`) assembly source files.

---

## 1. General Syntax Rules

- **File Extension**: Canonical assembly files use the `.e8085` extension.
- **Comments**: A semicolon (`;`) begins a comment that extends to the end of the line.
- **Case Sensitivity**:
  - Instruction mnemonics (`mov`, `MOV`, `Mov`), registers (`a`, `A`, `hl`, `HL`), and segment keywords are **case-insensitive**.
  - User-defined labels and variable names are **case-preserving** and matched cleanly.
  - By convention, `%define` constants use **`UPPERCASE_IDENTIFIERS`**.
- **Whitespace**: Flexible indentation; spaces and tabs are ignored except when separating tokens.

---

## 2. Segments

An assembly program is organized into three optional segments that may appear in any order:

```assembly
segment .data
  ; Initialized constants, strings, and byte tables

segment .bss
  ; Uninitialized zero-filled buffer reservations

segment .text
  ; Executable microprocessor code, subroutines, and ISR handlers
```

### `segment .data` (Initialized Data)
Declares named initialized memory variables.
- Data placed at `0x0040` (`DATA_BASE`).
- Supports `BYTE` and `WORD` declarations.
- Elements are whitespace-separated (no commas between values).

```assembly
segment .data
  counter BYTE 0x00
  primes  BYTE 2 3 5 7 11 13
  message BYTE "Hello, 8085!" 0x0A 0x00
  table   WORD 0x1234 0xABCD
```

### `segment .bss` (Uninitialized Space Reservations)
Reserves uninitialized, zero-filled RAM space immediately following `.data`.
- Size indicates the number of units to reserve.

```assembly
segment .bss
  input_buf  BYTE 64    ; Reserves 64 bytes
  word_stack WORD 16    ; Reserves 16 words (32 bytes)
```

### `segment .text` (Executable Code)
Contains instructions, labels, subroutines, and interrupt handlers.
- Code execution begins at `main:` (hooked to the `0x0000` reset vector).

```assembly
segment .text
main:
  lxi SP, 0xF000
  mvi A, 0x42
  hlt
```

---

## 3. Preprocessor Directives

### `%define` (Constant Definition)
Defines named numeric constants, characters, or string identifiers. Supports define chaining.

```assembly
%define PORT_DATA   0x01
%define PORT_CMD    0x02
%define DEFAULT_VAL 42
%define BACKUP_VAL  DEFAULT_VAL   ; Define chaining

segment .text
main:
  mvi A, DEFAULT_VAL
  out PORT_DATA
  hlt
```

### `%repeat` (Data Duplication)
Repeats an expression or string a specified number of times within `segment .data`.

```assembly
segment .data
  zero_table BYTE %repeat 16 0x00
  pattern    BYTE %repeat 4 0xAA 0x55
  dashes     BYTE %repeat 8 "-"
```

### `%len` (Size Evaluation)
Evaluates to the exact byte size of a previously defined `.data` array or `.bss` reservation.

```assembly
segment .data
  prompt BYTE "Enter name: "

segment .text
main:
  mvi B, %len prompt   ; B = 12
  lxi HL, prompt
  ; ...
```

### `%include` (File Inclusion)
Imports definitions, global labels, and constants from an external source file:

```assembly
%include "terminal.e8085"
; or with single quotes:
%include 'math/helpers.e8085'
```

---

## 4. Scoping & Modular Keywords

### `global` / `export`
Exports a label or subroutine globally so it can be referenced across modules and entered into the `.8085.bin` Export Symbol Table:

```assembly
segment .text

; Inline export declaration
global print:
    mvi A, 0x00
    out 0x02
    ret

; Or separate export declaration
global helper
helper:
    ret
```

### `extern`
Declares a symbol that will be resolved externally at link time (via linked `.8085.bin` container or `%include`):

```assembly
segment .text

extern print
extern input

main:
    call print
    hlt
```

### Local Labels (`.name:`, `jz .name`)
Scoped directly to the preceding parent (non-local) label. Prevents name collisions across separate subroutines:

```assembly
segment .text

func_a:
    mvi B, 5
.loop:
    dcr B
    jnz .loop
    ret

func_b:
    mvi C, 10
.loop:               ; Scoped to func_b, does not conflict with func_a.loop
    dcr C
    jnz .loop
    ret
```

---

## 5. Data Types & Number Formats

### Data Types
- `BYTE`: 8-bit unsigned integer (`0x00` .. `0xFF`).
- `WORD`: 16-bit unsigned integer (`0x0000` .. `0xFFFF`), stored in **Little-Endian** format (least significant byte first).

### Number Bases
| Format | Prefix | Example | Decimal Equivalent |
|:---|:---|:---|:---|
| **Hexadecimal** | `0x` or `0X` | `0xFF`, `0x1234` | 255, 4660 |
| **Binary** | `0b` or `0B` | `0b1010_0101`, `0B1111` | 165, 15 |
| **Octal** | `0o` or `0O` | `0o77`, `0O123` | 63, 83 |
| **Decimal** | *(None)* | `42`, `1000` | 42, 1000 |

### Literals
- **String Literals**: `"Hello World!"` — ASCII byte sequence.
- **Character Literals**: `'A'`, `'Z'` — Single ASCII byte value.

---

## 6. Registers & Operands

### 8-Bit Registers
- `A` — Accumulator (primary arithmetic/logical operand).
- `B`, `C`, `D`, `E`, `H`, `L` — General purpose 8-bit registers.
- `M` — **Memory Pseudo-Register**: Represents the byte in memory at the address pointed to by register pair `HL` (`[HL]`).

### 16-Bit Register Pairs
- `BC` (or `B` in pair instructions like `INX B`, `PUSH B`).
- `DE` (or `D`).
- `HL` (or `H`) — Primary memory pointer register pair.
- `SP` — 16-bit Stack Pointer.
- `PSW` — Program Status Word (Accumulator `A` + Flags byte).

---

## 7. Interrupts & ISR Subroutines

The assembler automatically wires 3-byte `JMP <isr>` hooks into the 8085 Interrupt Vector Table (`0x0000`..`0x003F`) when standardized ISR label names are defined:

```assembly
; ==========================================================
; Interrupt Service Routine Conventions
; ==========================================================

; Software Interrupt 1 (0x0008) — triggered by `rst 1`
isr_rst1:
  inr A
  ret

; Software Interrupt 2 (0x0010) — triggered by `rst 2`
isr_rst2:
  adi 5
  ret

; Hardware TRAP (0x0024) — triggered on illegal opcode or memory fault
isr_trap:
  mvi A, 0xEE
  hlt

; Hardware Maskable RST 5.5 (0x002C)
isr_rst55:
  in 0x01
  ret
```

---

## 8. Instruction Set Reference

### 1. Data Transfer Instructions
| Mnemonic | Operands | Description | Example |
|:---|:---|:---|:---|
| `MOV` | `r1, r2` | Move data from register `r2` to `r1` | `mov A, B`, `mov M, A` |
| `MVI` | `r, imm8` | Move 8-bit immediate into register `r` | `mvi A, 0x55`, `mvi M, 0` |
| `LXI` | `rp, imm16`| Load 16-bit immediate into register pair | `lxi HL, 0x1234`, `lxi SP, 0xF000` |
| `LDA` | `addr` | Load Accumulator directly from memory | `lda 0x2000`, `lda counter` |
| `STA` | `addr` | Store Accumulator directly to memory | `sta 0x2000`, `sta result` |
| `LHLD`| `addr` | Load `HL` pair directly from memory (2 bytes) | `lhld ptr` |
| `SHLD`| `addr` | Store `HL` pair directly to memory (2 bytes) | `shld ptr` |
| `LDAX`| `B` / `D` | Load Accumulator indirect from `[BC]` or `[DE]` | `ldax BC`, `ldax D` |
| `STAX`| `B` / `D` | Store Accumulator indirect to `[BC]` or `[DE]` | `stax BC`, `stax D` |
| `XCHG`| *(None)* | Exchange contents of `HL` and `DE` pairs | `xchg` |

### 2. Arithmetic Instructions
| Mnemonic | Operands | Description | Example |
|:---|:---|:---|:---|
| `ADD` | `r` / `M` | Add register/memory to Accumulator | `add B`, `add M` |
| `ADI` | `imm8` | Add immediate to Accumulator | `adi 10` |
| `ADC` | `r` / `M` | Add register/memory to Accumulator with Carry | `adc C` |
| `ACI` | `imm8` | Add immediate to Accumulator with Carry | `aci 1` |
| `SUB` | `r` / `M` | Subtract register/memory from Accumulator | `sub B`, `sub M` |
| `SUI` | `imm8` | Subtract immediate from Accumulator | `sui 5` |
| `SBB` | `r` / `M` | Subtract register/memory with Borrow | `sbb D` |
| `SBI` | `imm8` | Subtract immediate with Borrow | `sbi 0` |
| `INR` | `r` / `M` | Increment register/memory by 1 | `inr A`, `inr M` |
| `DCR` | `r` / `M` | Decrement register/memory by 1 | `dcr C`, `dcr M` |
| `INX` | `rp` | Increment 16-bit register pair by 1 | `inx HL`, `inx SP` |
| `DCX` | `rp` | Decrement 16-bit register pair by 1 | `dcx BC` |
| `DAD` | `rp` | Add 16-bit register pair to `HL` | `dad BC`, `dad SP` |
| `DAA` | *(None)* | Decimal Adjust Accumulator (for BCD) | `daa` |

### 3. Logical Instructions
| Mnemonic | Operands | Description | Example |
|:---|:---|:---|:---|
| `ANA` | `r` / `M` | Logical AND with Accumulator | `ana B`, `ana M` |
| `ANI` | `imm8` | Logical AND immediate with Accumulator | `ani 0x0F` |
| `ORA` | `r` / `M` | Logical OR with Accumulator | `ora C` |
| `ORI` | `imm8` | Logical OR immediate with Accumulator | `ori 0x80` |
| `XRA` | `r` / `M` | Logical XOR with Accumulator (e.g. clear A) | `xra A` |
| `XRI` | `imm8` | Logical XOR immediate with Accumulator | `xri 0xFF` |
| `CMP` | `r` / `M` | Compare register/memory with Accumulator | `cmp B`, `cmp M` |
| `CPI` | `imm8` | Compare immediate with Accumulator | `cpi 0x00` |
| `RLC` | *(None)* | Rotate Accumulator Left | `rlc` |
| `RRC` | *(None)* | Rotate Accumulator Right | `rrc` |
| `RAL` | *(None)* | Rotate Accumulator Left through Carry | `ral` |
| `RAR` | *(None)* | Rotate Accumulator Right through Carry | `rar` |
| `CMA` | *(None)* | Complement Accumulator (1's complement) | `cma` |
| `CMC` | *(None)* | Complement Carry Flag | `cmc` |
| `STC` | *(None)* | Set Carry Flag to 1 | `stc` |

### 4. Branching & Control Instructions
| Mnemonic | Operands | Description | Example |
|:---|:---|:---|:---|
| `JMP` | `label` / `addr` | Unconditional Jump | `jmp loop`, `jmp 0x0040` |
| `JNZ` | `label` / `addr` | Jump if Not Zero (`Z == 0`) | `jnz loop` |
| `JZ`  | `label` / `addr` | Jump if Zero (`Z == 1`) | `jz exit` |
| `JNC` | `label` / `addr` | Jump if No Carry (`CY == 0`) | `jnc continue` |
| `JC`  | `label` / `addr` | Jump if Carry (`CY == 1`) | `jc overflow` |
| `JPO` | `label` / `addr` | Jump if Parity Odd (`P == 0`) | `jpo odd_handler` |
| `JPE` | `label` / `addr` | Jump if Parity Even (`P == 1`) | `jpe even_handler` |
| `JP`  | `label` / `addr` | Jump if Positive (`S == 0`) | `jp positive` |
| `JM`  | `label` / `addr` | Jump if Minus (`S == 1`) | `jm negative` |
| `PCHL`| *(None)* | Jump indirect to address in `HL` | `pchl` |

### 5. Call, Return & Subroutines
| Mnemonic | Operands | Description | Example |
|:---|:---|:---|:---|
| `CALL` | `label` / `addr` | Unconditional Subroutine Call | `call print_str` |
| `CNZ`, `CZ`, `CNC`, `CC`, `CPO`, `CPE`, `CP`, `CM` | `label` / `addr` | Conditional Subroutine Calls | `cnz retry`, `cz zero_fn` |
| `RET`  | *(None)* | Unconditional Return from Subroutine | `ret` |
| `RNZ`, `RZ`, `RNC`, `RC`, `RPO`, `RPE`, `RP`, `RM` | *(None)* | Conditional Returns | `rnz`, `rz` |
| `RST`  | `0` .. `7` | Software Interrupt Restart (calls `0x0008 * n`) | `rst 1`, `rst 7` |

### 6. Stack & System Control
| Mnemonic | Operands | Description | Example |
|:---|:---|:---|:---|
| `PUSH` | `rp` / `PSW` | Push 16-bit register pair or PSW onto Stack | `push BC`, `push PSW` |
| `POP`  | `rp` / `PSW` | Pop 16-bit register pair or PSW from Stack | `pop BC`, `pop PSW` |
| `XTHL` | *(None)* | Exchange top of stack with `HL` pair | `xthl` |
| `SPHL` | *(None)* | Move `HL` pair to Stack Pointer `SP` | `sphl` |
| `IN`   | `port` | Input byte from peripheral port into `A` | `in 0x01` |
| `OUT`  | `port` | Output byte from `A` to peripheral port | `out 0x02` |
| `EI`   | *(None)* | Enable Interrupts | `ei` |
| `DI`   | *(None)* | Disable Interrupts | `di` |
| `SIM`  | *(None)* | Set Interrupt Mask | `sim` |
| `RIM`  | *(None)* | Read Interrupt Mask | `rim` |
| `NOP`  | *(None)* | No Operation (4 T-states) | `nop` |
| `HLT`  | *(None)* | Halt CPU execution | `hlt` |

---

## 9. Complete Annotated Program Example

```e8085
; ==========================================================
; Example .e8085 Program: String Echo with Software ISR
; ==========================================================

%define TERM_DATA_PORT 0x01
%define TERM_CMD_PORT  0x02
%define CMD_WRITE      0x00
%define CMD_DISPLAY    0x01

segment .data
  greeting BYTE "8085 Online!" 0x0A
  counter  BYTE 0x00

segment .bss
  buffer   BYTE 32

segment .text
main:
  lxi SP, 0xF000      ; Initialize stack pointer

  ; Invoke custom Software Interrupt 1
  rst 1

  ; Write greeting to terminal device
  mvi A, CMD_WRITE
  out TERM_CMD_PORT
  mvi A, %len greeting
  out TERM_DATA_PORT

  lxi HL, greeting
  mvi B, %len greeting
send_loop:
  mov A, M
  out TERM_DATA_PORT
  inx HL
  dcr B
  jnz send_loop

  ; Display the buffered message
  mvi A, CMD_DISPLAY
  out TERM_CMD_PORT
  hlt

; --- Custom ISR for RST 1 (Vector 0x0008) ---
isr_rst1:
  lda counter
  inr A
  sta counter
  ret
```