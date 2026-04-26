import * as vscode from 'vscode';

type McpServerConfig = {
    label: string;
    command: string;
    args?: string[];
    cwd?: string;
    env?: Record<string, string | number | null>;
    version?: string;
    enabled?: boolean;
};

type VscodeWithMcp = typeof vscode & {
    lm?: {
        registerMcpServerDefinitionProvider?: (
            id: string,
            provider: vscode.McpServerDefinitionProvider<vscode.McpStdioServerDefinition>
        ) => vscode.Disposable;
    };
};

function readConfiguredMcpServers(): McpServerConfig[] {
    const config = vscode.workspace.getConfiguration('perl-lsp');
    const configured = config.get<McpServerConfig[]>('mcp.servers', []);
    return configured.filter(server => server.enabled !== false);
}

function toMcpDefinition(server: McpServerConfig): vscode.McpStdioServerDefinition {
    const definition = new vscode.McpStdioServerDefinition(
        server.label,
        server.command,
        server.args ?? [],
        server.env ?? {},
        server.version
    );
    if (server.cwd) {
        definition.cwd = vscode.Uri.file(server.cwd);
    }
    return definition;
}

export function registerMcpSupport(outputChannel: vscode.OutputChannel): vscode.Disposable | undefined {
    const mcpApi = (vscode as VscodeWithMcp).lm;
    if (!mcpApi?.registerMcpServerDefinitionProvider) {
        outputChannel.appendLine('[mcp] VS Code MCP API unavailable in this editor build.');
        return undefined;
    }

    const provider: vscode.McpServerDefinitionProvider<vscode.McpStdioServerDefinition> = {
        provideMcpServerDefinitions: () => readConfiguredMcpServers().map(toMcpDefinition),
    };

    outputChannel.appendLine('[mcp] Registered MCP server definition provider.');
    return mcpApi.registerMcpServerDefinitionProvider('perl-lsp.mcp-servers', provider);
}
