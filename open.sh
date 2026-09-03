#!/usr/bin/env bash
set -e

# 1. Ensure the e8085-lsp binary is compiled
cargo build --bin e8085-lsp

# 2. Ensure extension dependencies are installed and compiled
if [ ! -d "editors/vscode/node_modules" ]; then
    (cd editors/vscode && npm install)
fi
(cd editors/vscode && npm run compile)

# 3. Launch VS Code in extension development mode
code --extensionDevelopmentPath="$(pwd)/editors/vscode" "$(pwd)"

