import * as vscode from 'vscode';

export interface DapVariable {
    name: string;
    value: string;
    type?: string;
    variablesReference: number;
    evaluateName?: string;
    memoryReference?: string;
}

export interface DapScope {
    name: string;
    variablesReference: number;
    expensive: boolean;
}

export class E8085DebugViewItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly description?: string,
        public readonly tooltip?: string,
        iconName?: string
    ) {
        super(label, vscode.TreeItemCollapsibleState.None);
        this.description = description;
        this.tooltip = tooltip;
        if (iconName) {
            this.iconPath = new vscode.ThemeIcon(iconName);
        }
    }
}

export class E8085CustomDebugViewProvider implements vscode.TreeDataProvider<E8085DebugViewItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<E8085DebugViewItem | undefined | void> = new vscode.EventEmitter<E8085DebugViewItem | undefined | void>();
    readonly onDidChangeTreeData: vscode.Event<E8085DebugViewItem | undefined | void> = this._onDidChangeTreeData.event;

    constructor(private readonly scopePrefix: string, private readonly defaultIcon: string) {}

    public refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: E8085DebugViewItem): vscode.TreeItem {
        return element;
    }

    async getChildren(_element?: E8085DebugViewItem): Promise<E8085DebugViewItem[]> {
        const session = vscode.debug.activeDebugSession;
        if (!session || session.type !== 'e8085') {
            return [new E8085DebugViewItem('No active 8085 debug session', '', 'Start debugging to inspect', 'info')];
        }

        try {
            // Fetch scopes for top frame
            const scopesResp = await session.customRequest('scopes', { frameId: 0 });
            const scopes: DapScope[] = scopesResp?.scopes || [];

            // Find matching scope (e.g. "Data Segment", "BSS Segment", "Stack")
            const targetScope = scopes.find(s => s.name.toLowerCase().startsWith(this.scopePrefix.toLowerCase()));
            if (!targetScope) {
                return [new E8085DebugViewItem(`(${this.scopePrefix} not found)`, '', '', 'warning')];
            }

            // Fetch variables for the target scope
            const varsResp = await session.customRequest('variables', {
                variablesReference: targetScope.variablesReference
            });
            const vars: DapVariable[] = varsResp?.variables || [];

            if (vars.length === 0) {
                return [new E8085DebugViewItem(`(${this.scopePrefix} is empty)`, '', '', 'circle-outline')];
            }

            return vars.map(v => {
                const icon = this.getIconForVariable(v);
                return new E8085DebugViewItem(
                    v.name,
                    v.value,
                    `${v.name}: ${v.value}${v.memoryReference ? ` (@ ${v.memoryReference})` : ''}`,
                    icon
                );
            });
        } catch {
            return [new E8085DebugViewItem('(Paused/Unavailable)', '', '', 'debug-pause')];
        }
    }

    private getIconForVariable(v: DapVariable): string {
        if (this.scopePrefix.toLowerCase().includes('stack')) {
            return 'layers';
        }
        if (v.value.startsWith('"')) {
            return 'symbol-string';
        }
        if (v.type === 'WORD' || v.type?.startsWith('WORD')) {
            return 'symbol-numeric';
        }
        return this.defaultIcon;
    }
}
