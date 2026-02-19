import * as vscode from 'vscode';
import * as path from 'path';

export class PerlDebugAdapterDescriptorFactory implements vscode.DebugAdapterDescriptorFactory {
    createDebugAdapterDescriptor(
        session: vscode.DebugSession,
        executable: vscode.DebugAdapterExecutable | undefined
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        // Try to find perl-dap in PATH or use bundled version
        const dapPath = this.findDebugAdapter();
        
        if (!dapPath) {
            vscode.window.showErrorMessage(
                'Perl Debug Adapter not found. Please install it with: cargo install --path crates/perl-dap --bin perl-dap'
            );
            return undefined;
        }

        return new vscode.DebugAdapterExecutable(dapPath, [], {
            env: { ...process.env, RUST_LOG: 'debug' }
        });
    }

    private findDebugAdapter(): string | undefined {
        // First, try to find perl-dap in PATH
        const pathDap = this.findExecutable('perl-dap');
        if (pathDap) {
            return pathDap;
        }

        // Otherwise, check common installation locations
        const binary = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';
        const possiblePaths: string[] = [
            path.join(process.env.HOME || '', '.cargo', 'bin', binary),
            path.join(process.env.CARGO_HOME || '', 'bin', binary),
        ];
        if (process.platform !== 'win32') {
            possiblePaths.push('/usr/local/bin/perl-dap', '/usr/bin/perl-dap');
        }

        for (const p of possiblePaths) {
            if (this.isExecutable(p)) {
                return p;
            }
        }

        return undefined;
    }

    private findExecutable(command: string): string | undefined {
        // If it's already an absolute path, check it
        if (path.isAbsolute(command)) {
            return this.isExecutable(command) ? command : undefined;
        }

        const pathEnv = process.env.PATH || '';
        const pathDirs = pathEnv.split(path.delimiter);

        // On Windows, we need to check extensions
        const isWindows = process.platform === 'win32';
        const extensions = isWindows
            ? (process.env.PATHEXT ? process.env.PATHEXT.split(';') : ['.EXE', '.CMD', '.BAT', '.COM'])
            : [''];

        for (const dir of pathDirs) {
            if (!dir) continue;

            for (const ext of extensions) {
                const fullPath = path.join(dir, command + ext);
                if (this.isExecutable(fullPath)) {
                    return fullPath;
                }
            }
        }

        return undefined;
    }

    private isExecutable(filePath: string): boolean {
        try {
            const fs = require('fs');
            // Check if file exists and is a file
            const stats = fs.statSync(filePath);
            if (!stats.isFile()) return false;

            // On Windows, existence is enough (permissions are complex)
            // On Unix, check for execute permission
            if (process.platform !== 'win32') {
                fs.accessSync(filePath, fs.constants.X_OK);
            }
            return true;
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

        if (config.request === 'attach') {
            // Attach supports either processId or host/port.
            if (config.processId === undefined || config.processId === null) {
                if (!config.host) {
                    config.host = 'localhost';
                }
                if (config.port === undefined || config.port === null) {
                    config.port = 13603;
                }
            }
            return config;
        }

        if (!config.program) {
            return vscode.window.showInformationMessage('Cannot find a Perl file to debug').then(() => {
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
            },
            {
                type: 'perl',
                request: 'attach',
                name: 'Attach by TCP',
                host: 'localhost',
                port: 13603,
                timeout: 5000
            },
            {
                type: 'perl',
                request: 'attach',
                name: 'Attach by Process ID',
                processId: 12345
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
