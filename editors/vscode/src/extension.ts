import * as path from 'path';
import * as fs from 'fs';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('e8085');
    let customPath = config.get<string>('serverPath');

    let serverExecutable = 'e8085-lsp';
    let serverArgs: string[] = [];

    if (customPath && customPath !== 'e8085-lsp' && fs.existsSync(customPath)) {
        serverExecutable = customPath;
    } else {
        // Look in cargo target directory if running in workspace
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders && workspaceFolders.length > 0) {
            const root = workspaceFolders[0].uri.fsPath;
            const debugLsp = path.join(root, 'target', 'debug', 'e8085-lsp');
            const releaseLsp = path.join(root, 'target', 'release', 'e8085-lsp');
            const debugCli = path.join(root, 'target', 'debug', 'e8085');

            if (fs.existsSync(debugLsp)) {
                serverExecutable = debugLsp;
            } else if (fs.existsSync(releaseLsp)) {
                serverExecutable = releaseLsp;
            } else if (fs.existsSync(debugCli)) {
                serverExecutable = debugCli;
                serverArgs = ['lsp'];
            }
        }
    }

    const serverOptions: ServerOptions = {
        command: serverExecutable,
        args: serverArgs,
        transport: TransportKind.stdio
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'e8085' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.e8085')
        }
    };

    client = new LanguageClient(
        'e8085LanguageServer',
        '8085 Assembly Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
