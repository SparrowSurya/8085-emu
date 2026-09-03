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

const INSTRUCTIONS = new Set([
    'MOV', 'MVI', 'LXI', 'LDA', 'STA', 'LDAX', 'STAX', 'LHLD', 'SHLD', 'XCHG',
    'XTHL', 'SPHL', 'PCHL', 'ADD', 'ADI', 'ADC', 'ACI', 'SUB', 'SUI', 'SBB',
    'SBI', 'INR', 'DCR', 'INX', 'DCX', 'DAD', 'DAA', 'ANA', 'ANI', 'XRA',
    'XRI', 'ORA', 'ORI', 'CMP', 'CPI', 'CMA', 'CMC', 'STC', 'RLC', 'RRC',
    'RAL', 'RAR', 'JMP', 'JZ', 'JNZ', 'JC', 'JNC', 'JP', 'JM', 'JPE',
    'JPO', 'CALL', 'CZ', 'CNZ', 'CC', 'CNC', 'CP', 'CM', 'CPE', 'CPO',
    'RET', 'RZ', 'RNZ', 'RC', 'RNC', 'RP', 'RM', 'RPE', 'RPO', 'RST',
    'PUSH', 'POP', 'IN', 'OUT', 'NOP', 'HLT', 'EI', 'DI', 'RIM', 'SIM'
]);

function escapeHtml(str: string): string {
    return str
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

function highlightE8085(code: string): string {
    const lines = code.split('\n');
    const highlightedLines = lines.map(line => {
        let out = '';
        let i = 0;
        while (i < line.length) {
            // Semicolon comment
            if (line[i] === ';') {
                out += `<span class="hljs-comment">${escapeHtml(line.slice(i))}</span>`;
                break;
            }
            // Double-quoted string
            if (line[i] === '"') {
                let end = i + 1;
                while (end < line.length) {
                    if (line[end] === '\\') {
                        end += 2;
                    } else if (line[end] === '"') {
                        end++;
                        break;
                    } else {
                        end++;
                    }
                }
                const strToken = line.slice(i, end);
                out += `<span class="hljs-string">${escapeHtml(strToken)}</span>`;
                i = end;
                continue;
            }
            // Single-quoted character literal
            if (line[i] === "'") {
                let end = i + 1;
                while (end < line.length) {
                    if (line[end] === '\\') {
                        end += 2;
                    } else if (line[end] === "'") {
                        end++;
                        break;
                    } else {
                        end++;
                    }
                }
                const charToken = line.slice(i, end);
                out += `<span class="hljs-literal">${escapeHtml(charToken)}</span>`;
                i = end;
                continue;
            }
            // Preprocessor directives (%define, %include, %repeat, %len)
            if (line[i] === '%') {
                const match = line.slice(i).match(/^%[a-zA-Z_]+/);
                if (match) {
                    out += `<span class="hljs-meta">${escapeHtml(match[0])}</span>`;
                    i += match[0].length;
                    continue;
                }
            }
            // Scoped local labels (.loop:, .loop)
            if (line[i] === '.') {
                const match = line.slice(i).match(/^\.[a-zA-Z_][a-zA-Z0-9_]*(:?)/);
                if (match) {
                    if (match[1] === ':') {
                        out += `<span class="hljs-title">${escapeHtml(match[0])}</span>`;
                    } else {
                        out += `<span class="hljs-symbol">${escapeHtml(match[0])}</span>`;
                    }
                    i += match[0].length;
                    continue;
                }
            }
            // Numbers (Hex, Binary, Octal, Decimal)
            const numMatch = line.slice(i).match(/^(?:0[xX][0-9a-fA-F]+|0[bB][01]+|0[oO][0-7]+|[0-9]+)\b/);
            if (numMatch) {
                out += `<span class="hljs-number">${escapeHtml(numMatch[0])}</span>`;
                i += numMatch[0].length;
                continue;
            }
            // Words (Instructions, Registers, Directives, Labels, Constants, Variables)
            const wordMatch = line.slice(i).match(/^[a-zA-Z_][a-zA-Z0-9_]*(:?)/);
            if (wordMatch) {
                const word = wordMatch[0];
                const upper = word.toUpperCase();
                if (word.endsWith(':')) {
                    out += `<span class="hljs-title">${escapeHtml(word)}</span>`;
                } else if (['BYTE', 'WORD'].includes(upper)) {
                    out += `<span class="hljs-type">${escapeHtml(word)}</span>`;
                } else if (['SEGMENT', 'GLOBAL', 'EXTERN'].includes(upper)) {
                    out += `<span class="hljs-meta">${escapeHtml(word)}</span>`;
                } else if (['A', 'B', 'C', 'D', 'E', 'H', 'L', 'M', 'BC', 'DE', 'HL', 'SP', 'PSW'].includes(upper)) {
                    out += `<span class="hljs-built_in">${escapeHtml(word)}</span>`;
                } else if (INSTRUCTIONS.has(upper)) {
                    out += `<span class="hljs-keyword">${escapeHtml(word)}</span>`;
                } else if (/^[A-Z][A-Z0-9_]*$/.test(word)) {
                    out += `<span class="hljs-literal">${escapeHtml(word)}</span>`;
                } else {
                    out += `<span class="hljs-variable">${escapeHtml(word)}</span>`;
                }
                i += word.length;
                continue;
            }
            // Normal character (whitespace, commas, etc.)
            out += escapeHtml(line[i]);
            i++;
        }
        return out;
    });
    return highlightedLines.join('\n');
}

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

    return {
        extendMarkdownIt(md: any) {
            const originalHighlight = md.options.highlight;
            md.options.highlight = (str: string, lang: string) => {
                const normalized = (lang || '').trim().toLowerCase();
                if (['e8085', '8085', 'assembly', 'asm', 'asm8085', '8085asm', 'e8085-asm'].includes(normalized)) {
                    const highlighted = highlightE8085(str);
                    return `<pre class="hljs"><code class="language-${normalized}">${highlighted}</code></pre>`;
                }
                if (originalHighlight) {
                    return originalHighlight(str, lang);
                }
                return '';
            };
            return md;
        }
    };
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
