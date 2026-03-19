import * as vscode from 'vscode';
import * as path from 'path';
import { BinaryDownloader } from './downloader';

export class PerlDebugAdapterDescriptorFactory implements vscode.DebugAdapterDescriptorFactory {
    constructor(private readonly context: vscode.ExtensionContext) {}

    createDebugAdapterDescriptor(
        session: vscode.DebugSession,
        executable: vscode.DebugAdapterExecutable | undefined
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        // Try to find perl-dap in PATH or use bundled version
        const dapPath = this.findDebugAdapter();
        
        if (!dapPath) {
            vscode.window.showErrorMessage(
                'Perl Debug Adapter (perl-dap) not found. It ships with perl-lsp — re-download from the release page or install via: cargo install perl-dap'
            );
            return undefined;
        }

        return new vscode.DebugAdapterExecutable(dapPath, [], {
            env: { ...process.env, RUST_LOG: 'debug' }
        });
    }

    private findDebugAdapter(): string | undefined {
        // First, check the auto-download directory (ships with perl-lsp)
        const downloadedDap = BinaryDownloader.getLocalDapPath(this.context);
        if (this.isExecutable(downloadedDap)) {
            return downloadedDap;
        }

        // Next, try to find perl-dap in PATH
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
    /**
     * Returns the workspace root URI path, or undefined if no workspace is open.
     */
    private getWorkspaceRoot(folder?: vscode.WorkspaceFolder): string | undefined {
        if (folder) {
            return folder.uri.fsPath;
        }
        const folders = vscode.workspace.workspaceFolders;
        if (folders && folders.length > 0) {
            return folders[0].uri.fsPath;
        }
        return undefined;
    }

    /**
     * Returns the configured Perl executable path from extension settings,
     * defaulting to "perl" if not set.
     */
    private getPerlPath(): string {
        const config = vscode.workspace.getConfiguration('perl-lsp');
        const perlPath = config.get<string>('perlPath');
        return perlPath || 'perl';
    }

    /**
     * Returns configured include paths from perl-lsp.includePaths,
     * resolved relative to the workspace root.
     */
    private getIncludePaths(folder?: vscode.WorkspaceFolder): string[] {
        const config = vscode.workspace.getConfiguration('perl-lsp');
        const paths = config.get<string[]>('includePaths', ['lib', 'local/lib/perl5']);
        const root = this.getWorkspaceRoot(folder);
        if (!root) {
            return paths;
        }
        return paths.map(p => path.isAbsolute(p) ? p : path.join(root, p));
    }

    /**
     * Builds -I flags from include paths for use in args arrays.
     */
    private buildIncludeArgs(folder?: vscode.WorkspaceFolder): string[] {
        return this.getIncludePaths(folder).flatMap(p => ['-I', p]);
    }

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

        // Inject perlPath from settings if not explicitly set
        if (!config.perlPath) {
            config.perlPath = this.getPerlPath();
        }

        // Inject include paths from settings if not explicitly set
        if (!config.includePaths) {
            config.includePaths = this.getIncludePaths(folder);
        }

        // Set cwd to workspace root if not explicitly set
        if (!config.cwd) {
            const root = this.getWorkspaceRoot(folder);
            if (root) {
                config.cwd = root;
            }
        }

        return config;
    }

    provideDebugConfigurations(
        folder: vscode.WorkspaceFolder | undefined,
        token?: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.DebugConfiguration[]> {
        const perlPath = this.getPerlPath();
        const includePaths = this.getIncludePaths(folder);
        const root = this.getWorkspaceRoot(folder);
        const cwd = root || '${workspaceFolder}';

        return [
            {
                type: 'perl',
                request: 'launch',
                name: 'Launch Perl Script',
                program: '${file}',
                perlPath,
                includePaths,
                cwd,
                stopOnEntry: true,
                args: []
            },
            {
                type: 'perl',
                request: 'launch',
                name: 'Launch Perl Test',
                program: '${file}',
                perlPath,
                includePaths,
                cwd,
                stopOnEntry: false,
                args: [],
                env: {
                    'PERL_TEST_HARNESS_DUMP_TAP': '1'
                }
            },
            {
                type: 'perl',
                request: 'launch',
                name: 'Debug Current Test (prove)',
                program: 'prove',
                perlPath,
                cwd,
                stopOnEntry: false,
                args: [
                    '-v',
                    ...includePaths.flatMap(p => ['-I', p]),
                    '${file}'
                ]
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

    const factory = new PerlDebugAdapterDescriptorFactory(context);
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
