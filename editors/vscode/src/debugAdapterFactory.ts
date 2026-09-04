import * as path from 'path';
import * as fs from 'fs';
import * as net from 'net';
import * as vscode from 'vscode';

export class E8085Pseudoterminal implements vscode.Pseudoterminal {
    private writeEmitter = new vscode.EventEmitter<string>();
    readonly onDidWrite: vscode.Event<string> = this.writeEmitter.event;
    private closeEmitter = new vscode.EventEmitter<number>();
    readonly onDidClose?: vscode.Event<number> = this.closeEmitter.event;
    private socket?: net.Socket;

    open(): void {
        this.writeEmitter.fire('\x1b[1;36m=== 8085 Microprocessor Terminal ===\x1b[0m\r\n\r\n');
    }

    close(): void {
        if (this.socket) {
            this.socket.destroy();
            this.socket = undefined;
        }
    }

    handleInput(data: string): void {
        if (!this.socket || this.socket.destroyed) {
            return;
        }

        for (let i = 0; i < data.length; i++) {
            const ch = data[i];
            if (ch === '\r') {
                // Enter pressed: Echo newline and send \n to emulator
                this.writeEmitter.fire('\r\n');
                this.socket.write('\n');
            } else if (ch === '\x7f' || ch === '\b') {
                // Backspace pressed: Erase character on terminal screen and send backspace
                this.writeEmitter.fire('\b \b');
                this.socket.write('\x08');
            } else if (ch === '\x03') {
                // Ctrl+C
                this.writeEmitter.fire('^C\r\n');
                this.socket.write('\x03');
            } else {
                // Normal printable character: Echo and send to emulator
                this.writeEmitter.fire(ch);
                this.socket.write(ch);
            }
        }
    }

    setSocket(socket: net.Socket): void {
        if (this.socket && !this.socket.destroyed) {
            this.socket.destroy();
        }
        this.socket = socket;
        this.writeEmitter.fire('\x1b[1;36m[8085 Debug Session Connected]\x1b[0m\r\n');
        socket.on('data', (buf: Buffer) => {
            const str = buf.toString().replace(/\r?\n/g, '\r\n');
            this.writeEmitter.fire(str);
        });
        socket.on('close', () => {
            this.writeEmitter.fire('\r\n\x1b[1;33m[Program Terminated]\x1b[0m\r\n');
        });
        socket.on('error', (err) => {
            this.writeEmitter.fire(`\r\n\x1b[1;31m[Terminal Error: ${err.message}]\x1b[0m\r\n`);
        });
    }
}

export class E8085TerminalManager {
    private static instance: E8085TerminalManager;
    private server?: net.Server;
    private pty?: E8085Pseudoterminal;
    private terminal?: vscode.Terminal;

    public static getInstance(): E8085TerminalManager {
        if (!E8085TerminalManager.instance) {
            E8085TerminalManager.instance = new E8085TerminalManager();
        }
        return E8085TerminalManager.instance;
    }

    public focusTerminal(): void {
        if (this.terminal) {
            this.terminal.show(false);
            vscode.commands.executeCommand('workbench.action.terminal.focus');
        }
    }

    public async startTerminalServer(port: number = 8085): Promise<number> {
        // Recreate terminal if not created or if closed
        if (!this.pty || !this.terminal || this.terminal.exitStatus !== undefined) {
            this.pty = new E8085Pseudoterminal();
            this.terminal = vscode.window.createTerminal({
                name: '8085 Terminal',
                pty: this.pty
            });
        }
        this.focusTerminal();

        if (this.server) {
            this.server.close();
            this.server = undefined;
        }

        return new Promise<number>((resolve) => {
            this.server = net.createServer((socket) => {
                if (this.pty) {
                    this.pty.setSocket(socket);
                }
            });

            this.server.listen(port, '127.0.0.1', () => {
                resolve(port);
            });

            this.server.on('error', (_err) => {
                // If requested port (e.g. 8085) is in use, fallback to ephemeral port
                if (this.server) {
                    this.server.listen(0, '127.0.0.1', () => {
                        const addr = this.server?.address();
                        if (addr && typeof addr !== 'string') {
                            resolve(addr.port);
                        } else {
                            resolve(8085);
                        }
                    });
                }
            });
        });
    }

    public cleanup(): void {
        if (this.server) {
            this.server.close();
            this.server = undefined;
        }
    }
}

export class E8085DebugAdapterFactory implements vscode.DebugAdapterDescriptorFactory {
    createDebugAdapterDescriptor(
        _session: vscode.DebugSession,
        _executable: vscode.DebugAdapterExecutable | undefined
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        const config = vscode.workspace.getConfiguration('e8085');
        const customPath = config.get<string>('dapServerPath');

        let serverExecutable = 'e8085-dap';
        let serverArgs: string[] = [];

        if (customPath && customPath !== 'e8085-dap' && fs.existsSync(customPath)) {
            serverExecutable = customPath;
        } else {
            // Look in cargo target directory if running in workspace
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (workspaceFolders && workspaceFolders.length > 0) {
                const root = workspaceFolders[0].uri.fsPath;
                const debugDap = path.join(root, 'target', 'debug', 'e8085-dap');
                const releaseDap = path.join(root, 'target', 'release', 'e8085-dap');
                const debugCli = path.join(root, 'target', 'debug', 'e8085');
                const releaseCli = path.join(root, 'target', 'release', 'e8085');

                if (fs.existsSync(debugDap)) {
                    serverExecutable = debugDap;
                } else if (fs.existsSync(releaseDap)) {
                    serverExecutable = releaseDap;
                } else if (fs.existsSync(debugCli)) {
                    serverExecutable = debugCli;
                    serverArgs = ['dap'];
                } else if (fs.existsSync(releaseCli)) {
                    serverExecutable = releaseCli;
                    serverArgs = ['dap'];
                }
            }
        }

        return new vscode.DebugAdapterExecutable(serverExecutable, serverArgs);
    }
}

export class E8085DebugConfigurationProvider implements vscode.DebugConfigurationProvider {
    async resolveDebugConfiguration(
        _folder: vscode.WorkspaceFolder | undefined,
        config: vscode.DebugConfiguration,
        _token?: vscode.CancellationToken
    ): Promise<vscode.DebugConfiguration | undefined> {
        // If launch.json is missing or empty
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'e8085') {
                config.type = 'e8085';
                config.name = 'Debug Current 8085 File';
                config.request = 'launch';
                config.program = '${file}';
                config.stopOnEntry = true;
                config.console = 'integratedTerminal';
                config.terminalPort = 8085;
            }
        }

        if (!config.program) {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'e8085') {
                config.program = editor.document.uri.fsPath;
            } else {
                vscode.window.showInformationMessage('Cannot find a .e8085 file to debug');
                return undefined; // abort launch
            }
        }

        if (config.stopOnEntry === undefined) {
            config.stopOnEntry = true;
        }

        if (config.console === undefined) {
            config.console = 'integratedTerminal';
        }

        if (config.internalConsoleOptions === undefined) {
            config.internalConsoleOptions = 'neverOpen';
        }

        if (config.console !== 'internalConsole') {
            const requestedPort = config.terminalPort || 8085;
            const actualPort = await E8085TerminalManager.getInstance().startTerminalServer(requestedPort);
            config.terminalPort = actualPort;
        }

        return config;
    }
}

