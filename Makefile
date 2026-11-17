# ==============================================================================
# emu8085 - Cycle-Accurate Intel 8085 Emulator & Toolchain
# ==============================================================================

# Detect Host Operating System
UNAME_S := $(shell uname -s 2>/dev/null || echo Windows_NT)

ifeq ($(UNAME_S),Darwin)
    # macOS user-local configuration (no sudo required)
    PREFIX     ?= $(HOME)/.local
    BIN_DIR    ?= $(PREFIX)/bin
    SHARE_DIR  ?= $(PREFIX)/share/emu8085
    STDLIB_DIR ?= $(SHARE_DIR)/stdlib
else ifeq ($(UNAME_S),Linux)
    # Linux user-local configuration (no sudo required)
    PREFIX     ?= $(HOME)/.local
    BIN_DIR    ?= $(PREFIX)/bin
    SHARE_DIR  ?= $(PREFIX)/share/emu8085
    STDLIB_DIR ?= $(SHARE_DIR)/stdlib
else
    # Windows / Other (MSYS, MINGW, etc.)
    PREFIX     ?= $(USERPROFILE)/.local
    BIN_DIR    ?= $(PREFIX)/bin
    SHARE_DIR  ?= $(PREFIX)/share/emu8085
    STDLIB_DIR ?= $(SHARE_DIR)/stdlib
endif

DESTDIR     ?=
CARGO       ?= cargo
CARGO_FLAGS ?= --release

TARGET_BIN_DIR    = $(DESTDIR)$(BIN_DIR)
TARGET_STDLIB_DIR = $(DESTDIR)$(STDLIB_DIR)
TARGET_SHARE_DIR  = $(DESTDIR)$(SHARE_DIR)

BINARIES    = e8085 e8085-lsp e8085-dap

.PHONY: all help code build install clean-build clean-install install-extension test

all: help

## help: Show this help message
help:
	@echo "emu8085 Makefile targets (user-local, no sudo required):"
	@echo ""
	@echo "  make code             Open VS Code in extension development mode (replaces open.sh)"
	@echo "  make build            Build all release binaries (e8085, e8085-lsp, e8085-dap)"
	@echo "  make install          Install binaries to $(BIN_DIR) and stdlib to $(STDLIB_DIR)"
	@echo "  make clean-build      Clean cargo build artifacts and extension compiler outputs"
	@echo "  make clean-install    Uninstall binaries and stdlib from $(PREFIX)"
	@echo "  make test             Run test suite"
	@echo "  make install-extension Install/symlink VS Code extension to ~/.vscode/extensions"
	@echo ""
	@echo "Configuration (OS detected: $(UNAME_S)):"
	@echo "  PREFIX=$(PREFIX)"
	@echo "  BIN_DIR=$(BIN_DIR)"
	@echo "  STDLIB_DIR=$(STDLIB_DIR)"

## code: Opens the code editor in extension development mode (replaces open.sh)
code:
	$(CARGO) build --bin e8085-lsp --bin e8085-dap
	@if [ ! -d "editors/vscode/node_modules" ]; then \
		echo "Installing extension dependencies..."; \
		(cd editors/vscode && npm install); \
	fi
	(cd editors/vscode && npm run compile)
	code --extensionDevelopmentPath="$$(pwd)/editors/vscode" "$$(pwd)"

## build: Builds the release executables
build:
	$(CARGO) build $(CARGO_FLAGS) --bins

## install: Installs the program and stdlib in user-local directories (no sudo needed)
install: build
	install -d $(TARGET_BIN_DIR)
	@for bin in $(BINARIES); do \
		echo "Installing $$bin to $(TARGET_BIN_DIR)/..."; \
		install -m 755 target/release/$$bin $(TARGET_BIN_DIR)/$$bin; \
	done
	install -d $(TARGET_STDLIB_DIR)
	@echo "Installing stdlib to $(TARGET_STDLIB_DIR)/..."; \
	cp -R stdlib/* $(TARGET_STDLIB_DIR)/
	@echo ""
	@echo "Installation complete!"
	@echo "  Binaries: $(TARGET_BIN_DIR)"
	@echo "  Stdlib:   $(TARGET_STDLIB_DIR)"
	@echo ""
	@echo "Note: Ensure '$(BIN_DIR)' is in your PATH."

## clean-build: Cleans cargo build artifacts and VS Code extension outputs
clean-build:
	$(CARGO) clean
	rm -rf editors/vscode/out

## clean-install: Cleans the installation (uninstalls user-local binaries and stdlib)
clean-install:
	@for bin in $(BINARIES); do \
		echo "Removing $(TARGET_BIN_DIR)/$$bin..."; \
		rm -f $(TARGET_BIN_DIR)/$$bin; \
	done
	@echo "Removing $(TARGET_SHARE_DIR)..."; \
	rm -rf $(TARGET_SHARE_DIR)
	@echo "Uninstallation complete."

## test: Runs the full test suite
test:
	$(CARGO) test

## install-extension: Permanently links the VS Code extension into ~/.vscode/extensions
install-extension:
	@if [ ! -d "editors/vscode/node_modules" ]; then \
		(cd editors/vscode && npm install); \
	fi
	(cd editors/vscode && npm run compile)
	@mkdir -p $$HOME/.vscode/extensions
	@rm -rf $$HOME/.vscode/extensions/e8085-assembly
	ln -s "$$(pwd)/editors/vscode" $$HOME/.vscode/extensions/e8085-assembly
	@echo "VS Code extension linked into $$HOME/.vscode/extensions/e8085-assembly"
