#!/usr/bin/env bash
set -e

# 1. Ensure the e8085-lsp binary is compiled
cargo build --bin e8085-lsp

# 2. Ensure extension dependencies are installed and compiled
if [ ! -d "editors/vscode/node_modules" ] || [ ! -f "editors/vscode/out/extension.js" ]; then
    echo "Compiling VS Code extension..."
    (cd editors/vscode && npm install && npm run compile)
fi

# 3. Launch VS Code in extension development mode
code --extensionDevelopmentPath="$(pwd)/editors/vscode" "$(pwd)"

