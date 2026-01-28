import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

export class PerlDebugAdapterDescriptorFactory implements vscode.DebugAdapterDescriptorFactory {
    createDebugAdapterDescriptor(
        session: vscode.DebugSession,
        executable: vscode.DebugAdapterExecutable | undefined
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        // Try to find perl-dap in PATH or use bundled version
        const dapPath = this.findDebugAdapter();
        
        if (!dapPath) {
            vscode.window.showErrorMessage(
                'Perl Debug Adapter not found. Please install it with: cargo install --path crates/perl-parser --bin perl-dap'
            );
            return undefined;
        }

        return new vscode.DebugAdapterExecutable(dapPath, [], {
            env: { ...process.env, RUST_LOG: 'debug' }
        });
    }

    private findDebugAdapter(): string | undefined {
        // First, try to find perl-dap in PATH
        const pathDap = this.findInPath('perl-dap');
        if (pathDap) {
            return pathDap;
        }

        // Otherwise, check common installation locations
        const possiblePaths = [
            path.join(process.env.HOME || '', '.cargo', 'bin', 'perl-dap'),
            path.join(process.env.CARGO_HOME || '', 'bin', 'perl-dap'),
            '/usr/local/bin/perl-dap',
            '/usr/bin/perl-dap',
        ];

        for (const p of possiblePaths) {
            if (this.fileExists(p)) {
                return p;
            }
        }

        return undefined;
    }

    private findInPath(command: string): string | undefined {
        const pathDirs = process.env.PATH?.split(path.delimiter) || [];
        const isWindows = process.platform === 'win32';
        const extensions = isWindows ? ['.exe', '.cmd', '.bat', ''] : [''];

        for (const dir of pathDirs) {
            for (const ext of extensions) {
                const fullPath = path.join(dir, command + ext);
                if (this.fileExists(fullPath)) {
                    return fullPath;
                }
            }
        }
        return undefined;
    }

    private fileExists(path: string): boolean {
        try {
            return fs.existsSync(path) && fs.statSync(path).isFile();
        } catch {
            return false;
        }
    }
}

export class PerlDebugConfigurationProvider implements vscode.DebugConfigurationProvider {
    resolveDebugConfiguration(
        folder: vscode.WorkspaceFolder | undefined,
        config: vscode.DebugConfiguration,
        token?: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.DebugConfiguration> {
        // If launch.json is missing or empty
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            if (editor && editor.document.languageId === 'perl') {
                config.type = 'perl';
                config.name = 'Launch Perl';
                config.request = 'launch';
                config.program = '${file}';
            }
        }

        if (!config.program) {
            return vscode.window.showInformationMessage('Cannot find a Perl file to debug').then(_ => {
                return undefined;
            });
        }

        return config;
    }

    provideDebugConfigurations(
        folder: vscode.WorkspaceFolder | undefined,
        token?: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.DebugConfiguration[]> {
        return [
            {
                type: 'perl',
                request: 'launch',
                name: 'Launch Perl Script',
                program: '${file}',
                stopOnEntry: true,
                args: []
            },
            {
                type: 'perl',
                request: 'launch',
                name: 'Launch Perl Test',
                program: '${file}',
                stopOnEntry: false,
                args: [],
                env: {
                    'PERL_TEST_HARNESS_DUMP_TAP': '1'
                }
            }
        ];
    }
}

export function activateDebugger(context: vscode.ExtensionContext) {
    // Register the debug adapter
    const provider = new PerlDebugConfigurationProvider();
    context.subscriptions.push(
        vscode.debug.registerDebugConfigurationProvider('perl', provider)
    );

    const factory = new PerlDebugAdapterDescriptorFactory();
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory('perl', factory)
    );

    // Register debug commands
    context.subscriptions.push(
        vscode.commands.registerCommand('perl.debugTest', (test: any) => {
            const config: vscode.DebugConfiguration = {
                type: 'perl',
                name: `Debug ${test.label}`,
                request: 'launch',
                program: test.uri.fsPath,
                stopOnEntry: false,
                args: test.args || []
            };

            vscode.debug.startDebugging(undefined, config);
        })
    );
}