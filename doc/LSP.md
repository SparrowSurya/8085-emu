# Language Server Protocol (`e8085-lsp`) Documentation

The `e8085-lsp` is a high-performance, asynchronous Language Server implementing the [Language Server Protocol (LSP 3.17)](https://microsoft.github.io/language-server-protocol/) for Intel 8085 assembly programs (`.e8085`).

It provides real-time semantic analysis, documentation hover tooltips, definition navigation, contextual auto-completion, refactoring, live error squiggles, cycle inlay hints, and automated quick fixes directly inside modern code editors.

---

## 1. Features & Capabilities

### 1.1 Documentation on Hover (`textDocument/hover`)
Hovering over tokens in an `.e8085` source file displays rich markdown documentation:
- **8085 Instructions**: Shows opcode summary, byte size, hardware T-state execution cycles (e.g. `MOV` $\rightarrow$ `4T`, `CALL` $\rightarrow$ `18T`), flags affected (`Z, S, P, CY, AC`), and operation pseudocode.
- **Registers**: Documents 8-bit registers (`A`, `B`, `C`, `D`, `E`, `H`, `L`, `M`), 16-bit register pairs (`BC`, `DE`, `HL`, `SP`), and Program Status Word (`PSW` flags `[S Z 0 AC 0 P 1 CY]`).
- **Numeric & Character Literals**: Hovering over numbers (`0x41`, `65`, `0b1000001`, `0o101`) or character literals (`'A'`, `'\n'`, `'\t'`, `'\0'`, `'\\'`) displays a multi-radix breakdown:
  ```text
  Decimal:     65
  Hexadecimal: 0x41
  Binary:      0b1000001
  Octal:       0o101
  ASCII:       'A'
  ```
- **Directives & Segments**: Documents `%define`, `%include`, `%repeat`, `%len`, `segment .text/.data/.bss`, `global`, `extern`, `BYTE`, and `WORD`.
- **`%include` Directives & Modules**: Hovering over `%include "..."` displays module documentation extracted from doc-comments at the top of the imported file.
- **User Symbols**: Displays symbol kind (Global Subroutine, Local Code Label, Variable), memory segment, definition line/col, and doc-comments written with `;` preceding the definition (including symbols imported across `%include` files).

### 1.2 Goto Definition (`textDocument/definition`)
Pressing `F12` or `Ctrl+Click` on an identifier navigates directly to its definition:
- **Global Labels**: `call print` $\rightarrow$ jumps to `print:` / `global print:`.
- **Scoped Local Labels**: `jnz .loop` $\rightarrow$ jumps to `.loop:` within the enclosing parent subroutine without name collisions.
- **Variables**: `lxi HL, prompt` $\rightarrow$ jumps to `prompt "..."` in `segment .data` or `prompt BYTE 32` in `.bss`.
- **Constants**: `mvi A, MAX_LIMIT` $\rightarrow$ jumps to `%define MAX_LIMIT 100`.
- **Include Files**: Clicking on `%include "devices/terminal.e8085"` opens the referenced `.e8085` file.
- **Cross-File Symbols**: Symbols defined in an included file jump directly into the source location within the included file.

### 1.3 Auto-Completion (`textDocument/completion`)
Context-aware suggestions and snippet expansions:
- **Instruction Mnemonics**: Suggested only inside `segment .text`. Expands mnemonics with tab-stops (e.g. `lxi` $\rightarrow$ `lxi ${1:pair}, ${2:addr16}`, `mvi` $\rightarrow$ `mvi ${1:dest}, ${2:byte}`).
- **Data Segment Completions**: Inside `segment .data` and `segment .bss`, instructions are suppressed; only data types (`BYTE`, `WORD`), `%repeat ${1:count} ${2:value}`, `%len ${1:var_name}`, string templates, and defined constants are suggested.
- **`%include` Path Suggestions**: When typing inside `%include "..."`, suggests relative directories and `.e8085` / `.inc` files, suppressing unrelated keywords.
- **Context-Sensitive Registers**:
  - After `MOV `, `ADD `, `SUB `, `INR `, `DCR `, `CMP `, `MVI `: suggests 8-bit registers (`A, B, C, D, E, H, L, M`).
  - After `LXI `, `INX `, `DCX `, `DAD `: suggests 16-bit pairs (`BC, DE, HL, SP`).
  - After `PUSH `, `POP `: suggests `BC, DE, HL, PSW`.
  - After `LDAX `, `STAX `: suggests `BC, DE`.
- **Target Operand Filtering**:
  - After `CALL`, `JMP`, and condition branches (`JZ`, `JNZ`, `CC`, `CNC`...): suggests code labels and `extern` subroutines. Data variables and defined constants are filtered out.
  - After `LDA`, `STA`, `LHLD`, `SHLD`: suggests data/BSS variables and defined constants.
- **Space-Separated `%len`**: Formats suggestions as `%len variable` without parentheses.
- **Comment Filtering**: Inline comments (`; ...`) are stripped so commented-out code is never suggested as variables or labels.

### 1.4 Rename Symbol (`textDocument/rename` & `prepareRename`)
Pressing `F2` safely renames a symbol across all definitions and call/jump references:
- Supports global subroutine labels and data variables.
- Supports scoped local labels (`.loop`), renaming only occurrences within the enclosing parent subroutine.
- Rejects renaming reserved mnemonics, registers, and keywords.

### 1.5 Live Diagnostics & Static Analysis (`textDocument/publishDiagnostics`)
Real-time compiler feedback and semantic checks as you type (no need to save):
- **Assembler Syntax Errors**: Spans exact line and column locations for syntax and semantic mistakes.
- **Duplicate Definition Tracking**: When a symbol is defined more than once, diagnostics report the duplicate definition and attach `DiagnosticRelatedInformation` pointing directly to where the symbol was first defined.
- **Unused Variable Warnings**: Flags variables declared in `segment .data` or `segment .bss` that are never referenced across `.text`, `.data`, or `%define` as `DiagnosticSeverity::WARNING` (`DiagnosticTag::UNNECESSARY`).
- **Unused Label Warnings**: Flags unreferenced local labels.
- **CFG Reachability Analysis**: Flags dead/unreachable code following unconditional branch instructions (`HLT`, `RET`, `JMP`) as `DiagnosticTag::UNNECESSARY`.
- **Missing Halt Check**: Warns if the entry point `main:` has execution paths that do not terminate with `HLT` or an infinite loop. (Skipped for library modules without `main:`).
- **Extern Resolution**: Local definitions satisfy `extern` declarations without throwing duplicate definition errors.

### 1.6 Inlay Hints (`textDocument/inlayHint`)
Displays hardware execution timing inline next to instruction mnemonics:
```e8085
main:
    lxi HL, prompt     /* [10T] */
    call print         /* [18T] */
    hlt                /* [5T]  */
```

### 1.7 Quick Fixes (`textDocument/codeAction`)
Automated single-click code optimizations and repairs:
- **Accumulator Clear Optimization**: Recommends replacing `mvi A, 0` with `xra A` (saving 1 byte of memory and 3 T-states).
- **Mnemonic Casing**: Automatically corrects mixed-cased mnemonics (e.g. `Mvi` $\rightarrow$ `mvi`).

---

## 2. Running the Language Server

The language server communicates over standard input/output (`stdio`) using the standard JSON-RPC LSP protocol.

### Standalone Binary
```bash
cargo run --bin e8085-lsp
```

### Unified CLI Subcommand
```bash
cargo run --bin e8085 -- lsp
```

---

## 3. Editor Integrations

### 3.1 Visual Studio Code
A dedicated extension is provided in [`editors/vscode/`](../editors/vscode/).

To test in VS Code development mode:
```bash
./open.sh
```

Or manually:
```bash
# 1. Build the Rust LSP binary
cargo build --bin e8085-lsp

# 2. Compile the extension
cd editors/vscode
npm install
npm run compile
cd ../..

# 3. Launch VS Code with extension
code --extensionDevelopmentPath=$(pwd)/editors/vscode $(pwd)
```

#### Settings (`settings.json`)
```json
{
  "e8085.serverPath": "e8085-lsp"
}
```

---

### 3.2 Neovim (`nvim-lspconfig`)

Add the following to your `init.lua`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.e8085_lsp then
  configs.e8085_lsp = {
    default_config = {
      cmd = { 'e8085-lsp' },
      filetypes = { 'e8085' },
      root_dir = lspconfig.util.root_pattern('.git', 'Cargo.toml'),
      settings = {},
    },
  }
end

lspconfig.e8085_lsp.setup{}
```

---

### 3.3 Helix Editor

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "e8085"
scope = "source.e8085"
file-types = ["e8085"]
roots = [".git"]
language-servers = ["e8085-lsp"]

[language-server.e8085-lsp]
command = "e8085-lsp"
```

---

## 4. Architectural Design

```
┌─────────────────────────────────────────────────────────────┐
│                 Editor Client (VS Code / Neovim)            │
└──────────────────────────────┬──────────────────────────────┘
                               │ JSON-RPC (stdio)
┌──────────────────────────────▼──────────────────────────────┐
│                    e8085-lsp Language Server                │
│                                                             │
│   ┌──────────────────────────┐    ┌─────────────────────┐   │
│   │   tower-lsp Handler      │<──>│    DocumentStore    │   │
│   │   (Protocol Engine)      │    │  (In-Memory VFS)    │   │
│   └────────────┬─────────────┘    └──────────┬──────────┘   │
│                │                             │              │
│   ┌────────────▼─────────────┐    ┌──────────▼──────────┐   │
│   │   AST & Error Collector  │<──>│  Symbol & Ref Index │   │
│   │   (emu8085::asm)         │    │  (Hover, Def, Nav)  │   │
│   └──────────────────────────┘    └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

- **`DocumentStore`** (`src/lsp/document.rs`): Thread-safe in-memory VFS mapping `Url -> Document` with fast line-offset indexing.
- **`hover`** (`src/lsp/hover.rs`): Built-in 8085 instruction and register reference database.
- **`definition`** (`src/lsp/definition.rs`): Symbol resolver supporting global, local, data, and `%include` path definitions.
- **`completion`** (`src/lsp/completion.rs`): Contextual snippet and symbol completion engine.
- **`rename`** (`src/lsp/rename.rs`): Workspace refactoring engine with parent-scoped local label isolation.
- **`diagnostics`** (`src/lsp/diagnostics.rs`): Diagnostic bridge parsing front-end errors into LSP diagnostics.
<!-- - **`hints`** (`src/lsp/hints.rs`): Instruction cycle calculator. -->
- **`code_actions`** (`src/lsp/code_actions.rs`): Quick fix provider.
