# Language Server Protocol (`e8085-lsp`) Documentation

The `e8085-lsp` is a high-performance, asynchronous Language Server implementing the [Language Server Protocol (LSP 3.17)](https://microsoft.github.io/language-server-protocol/) for Intel 8085 assembly programs (`.e8085`).

It provides real-time semantic analysis, documentation hover tooltips, definition navigation, contextual auto-completion, refactoring, live error squiggles, cycle inlay hints, and automated quick fixes directly inside modern code editors.

---

## 1. Features & Capabilities

### 1.1 Documentation on Hover (`textDocument/hover`)
Hovering over any token in an `.e8085` source file displays rich markdown documentation:
- **8085 Instructions**: Shows opcode summary, byte size, hardware T-state execution cycles (e.g. `MOV` $\rightarrow$ `4T`, `CALL` $\rightarrow$ `18T`), flags affected (`Z, S, P, CY, AC`), and operation pseudocode.
- **Registers**: Shows 8-bit registers (`A`, `B`, `C`, `D`, `E`, `H`, `L`, `M`), 16-bit register pairs (`BC`, `DE`, `HL`, `SP`), and Program Status Word (`PSW` flags `[S Z 0 AC 0 P 1 CY]`).
- **Directives**: Documents `%define`, `%include`, `%repeat`, `%len`, `segment .text/.data/.bss`, `global`, `extern`, `BYTE`, and `WORD`.
- **User Symbols**: Displays symbol type (Global Subroutine, Local Code Label, Variable), memory segment, definition line number, and extracts doc-comments written with `;` above the definition.

### 1.2 Goto Definition (`textDocument/definition`)
Pressing `F12` or `Ctrl+Click` on an identifier navigates directly to its definition:
- **Global Labels**: `call print` $\rightarrow$ jumps to `print:` / `global print:`.
- **Scoped Local Labels**: `jnz .loop` $\rightarrow$ jumps to `.loop:` within the same parent subroutine scope without label collisions.
- **Variables**: `lxi HL, prompt` $\rightarrow$ jumps to `prompt "..."` in `segment .data` or `prompt BYTE 32` in `.bss`.
- **Include Files**: Clicking on `%include "devices/terminal.e8085"` opens the referenced `.e8085` file.

### 1.3 Auto-Completion (`textDocument/completion`)
Intelligent suggestions with snippet placeholders:
- **Instruction Mnemonics**: Expands mnemonics with tab-stops (e.g. `lxi` $\rightarrow$ `lxi ${1:pair}, ${2:addr16}`, `mvi` $\rightarrow$ `mvi ${1:dest}, ${2:byte}`).
- **Context-Sensitive Registers**:
  - After `MOV `, `ADD `, `SUB `, `INR `, `DCR `, `CMP `: suggests 8-bit registers (`A, B, C, D, E, H, L, M`).
  - After `LXI `, `INX `, `DCX `, `DAD `: suggests 16-bit pairs (`BC, DE, HL, SP`).
  - After `PUSH `, `POP `: suggests `BC, DE, HL, PSW`.
  - After `LDAX `, `STAX `: suggests `BC, DE`.
- **Directives & Segments**: `%define`, `%include`, `%repeat`, `%len`, `segment .text`, `segment .data`, `segment .bss`, `global`, `extern`.
- **In-Scope Symbols**: Auto-completes defined labels, subroutines, variables, and constants.

### 1.4 Rename Symbol (`textDocument/rename` & `prepareRename`)
Pressing `F2` safely renames a symbol across all definitions and call/jump references:
- Supports global subroutine labels and data variables.
- Supports scoped local labels (`.loop`), renaming only occurrences within the enclosing parent subroutine.
- Rejects renaming reserved mnemonics, registers, and keywords.

### 1.5 Live Diagnostics (`textDocument/publishDiagnostics`)
Real-time compiler feedback as you type (no need to save):
- Converts assembler errors (`AsmError`) into editor squiggles with exact line and column spans.
- Clears diagnostics immediately when syntax errors are corrected.

### 1.6 Inlay Hints (`textDocument/inlayHint`)
Displays hardware execution timing inline next to instruction mnemonics:
```assembly
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
- **`hints`** (`src/lsp/hints.rs`): Instruction cycle calculator.
- **`code_actions`** (`src/lsp/code_actions.rs`): Quick fix provider.
