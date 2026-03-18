import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { execFile } from 'child_process';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    State
} from 'vscode-languageclient/node';
import { PerlTestAdapter } from './testAdapter';
import { activateDebugger } from './debugAdapter';
import { BinaryDownloader } from './downloader';

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let testAdapter: PerlTestAdapter | undefined;
let resolvedServerPath: string | null = null;

export async function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.window.createOutputChannel('Perl Language Server');

    // Register showOutput command early so it's available during binary download and initialization
    const showOutputCommand = vscode.commands.registerCommand('perl-lsp.showOutput', () => {
        outputChannel.show();
    });
    context.subscriptions.push(showOutputCommand);
    
    // Get the path to perl-lsp
    const serverPath = await getServerPath(context);
    resolvedServerPath = serverPath;
    if (!serverPath) {
        vscode.window.showErrorMessage(
            'Perl Language Server (perl-lsp) not found.',
            'Install (cargo install perl-lsp)',
            'Open Settings'
        ).then((choice: string | undefined) => {
            if (choice === 'Install (cargo install perl-lsp)') {
                vscode.window.showInformationMessage(
                    'Run in your terminal: cargo install perl-lsp\nThen reload VS Code.'
                );
            } else if (choice === 'Open Settings') {
                vscode.commands.executeCommand('workbench.action.openSettings', 'perl-lsp.serverPath');
            }
        });
        return;
    }

    // Validate that the binary is functional before starting the LSP client.
    // This catches corrupted downloads, platform-incompatible binaries, and
    // missing shared libraries with an actionable error message.
    const healthOk = await runHealthCheck(serverPath);
    if (!healthOk) {
        const choice = await vscode.window.showErrorMessage(
            `perl-lsp health check failed. The binary at '${serverPath}' does not respond to --health. ` +
            'It may be corrupted or incompatible with your platform.',
            'Show Output',
            'Reinstall'
        );
        if (choice === 'Show Output') {
            outputChannel.show();
        } else if (choice === 'Reinstall') {
            await vscode.commands.executeCommand('perl-lsp.reinstall');
        }
        return;
    }

    // Server options
    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            args: getServerArgs(['--stdio']),
            transport: TransportKind.stdio
        },
        debug: {
            command: serverPath,
            args: getServerArgs(['--stdio', '--log']),
            transport: TransportKind.stdio
        }
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'perl' },
            { scheme: 'untitled', language: 'perl' }
        ],
        synchronize: {
            // Notify the server about file changes to .perltidyrc files
            fileEvents: vscode.workspace.createFileSystemWatcher('**/.perltidyrc')
        },
        outputChannel
    };

    // Create and start the language client
    client = new LanguageClient(
        'perl-language-server',
        'Perl Language Server',
        serverOptions,
        clientOptions
    );

    // Create status bar item - show immediately with starting state
    const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.command = 'perl-lsp.showStatusMenu';
    statusBarItem.text = '$(sync~spin) Perl LSP';
    statusBarItem.tooltip = 'Perl Language Server is starting... (click for options)';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    client.onDidChangeState(event => {
        switch (event.newState) {
            case State.Running:
                statusBarItem.text = '$(check) Perl LSP';
                statusBarItem.tooltip = 'Perl Language Server is running (click for options)';
                statusBarItem.backgroundColor = undefined;
                break;
            case State.Starting:
                statusBarItem.text = '$(sync~spin) Perl LSP';
                statusBarItem.tooltip = 'Perl Language Server is starting... (click for options)';
                statusBarItem.backgroundColor = undefined;
                break;
            case State.Stopped:
                statusBarItem.text = '$(error) Perl LSP';
                statusBarItem.tooltip = 'Perl Language Server is stopped (click for options)';
                statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
                break;
        }
    });

    // Start the client
    await client.start();
    
    // Initialize test adapter
    testAdapter = new PerlTestAdapter(client);
    context.subscriptions.push(testAdapter);
    
    // Initialize debug adapter
    activateDebugger(context);

    // Register commands
    const restartCommand = vscode.commands.registerCommand('perl-lsp.restart', async () => {
        await restartServer(context);
    });

    const reinstallCommand = vscode.commands.registerCommand('perl-lsp.reinstall', async () => {
        await reinstallServer(context);
    });

    const organizeImportsCommand = vscode.commands.registerCommand('perl-lsp.organizeImports', async () => {
        await vscode.commands.executeCommand('editor.action.organizeImports');
    });

    const runTestsCommand = vscode.commands.registerCommand('perl-lsp.runTests', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'perl') {
            vscode.window.showErrorMessage('No active Perl file to test');
            return;
        }

        // Restrict to test files (.t, .pl) - .pm files are modules, not test scripts
        const filePath = editor.document.uri.fsPath;
        if (!filePath.endsWith('.t') && !filePath.endsWith('.pl')) {
            vscode.window.showWarningMessage('Run Tests is only available for .t and .pl files');
            return;
        }

        if (testAdapter) {
            // Store original state
            const originalText = statusBarItem.text;
            const originalTooltip = statusBarItem.tooltip;

            // Show running state
            statusBarItem.text = '$(beaker~spin) Running Tests...';
            statusBarItem.tooltip = 'Executing Perl tests in current file';

            try {
                await testAdapter.runFileTests(editor.document.uri);
            } finally {
                // Restore original state
                statusBarItem.text = originalText;
                statusBarItem.tooltip = originalTooltip;
            }
        } else {
            vscode.window.showWarningMessage('Test adapter is not available. It might still be initializing.');
        }
    });
    
    const showVersionCommand = vscode.commands.registerCommand('perl-lsp.showVersion', async () => {
        const { execFile } = require('child_process');
        execFile(serverPath, ['--version'], (error: any, stdout: string, stderr: string) => {
            if (error) {
                vscode.window.showErrorMessage(`Failed to get version: ${error.message}`);
            } else {
                const version = stdout.trim();
                vscode.window.showInformationMessage(`Perl LSP Version: ${version}`, 'Copy').then(selection => {
                    if (selection === 'Copy') {
                        vscode.env.clipboard.writeText(version);
                    }
                });
            }
        });
    });

    const statusMenuCommand = vscode.commands.registerCommand('perl-lsp.showStatusMenu', async () => {
        const editor = vscode.window.activeTextEditor;
        const isPerl = editor ? editor.document.languageId === 'perl' : false;
        const filePath = editor ? editor.document.uri.fsPath : '';
        const isTestFile = isPerl && (filePath.endsWith('.t') || filePath.endsWith('.pl'));

        interface MenuAction extends vscode.QuickPickItem {
            command?: string;
            args?: any[];
            disabled?: boolean;
        }

        const items: MenuAction[] = [
            { label: 'Actions', kind: vscode.QuickPickItemKind.Separator },
            {
                label: '$(refresh) Restart Server',
                description: 'Shift+Alt+R',
                detail: 'Restart the language server',
                command: 'perl-lsp.restart'
            },
            {
                label: '$(cloud-download) Reinstall Server Binary',
                detail: 'Clear the cached download and fetch perl-lsp again',
                command: 'perl-lsp.reinstall'
            },
            {
                label: '$(organization) Organize Imports',
                description: 'Shift+Alt+O',
                detail: isPerl ? 'Sort and organize use statements' : 'Sort and organize use statements (Only available for Perl files)',
                command: 'perl-lsp.organizeImports',
                disabled: !isPerl
            },
            {
                label: '$(beaker) Run Tests in Current File',
                description: 'Shift+Alt+T',
                detail: isTestFile ? 'Run tests for the active file' : 'Run tests for the active file (Only available for .t/.pl files)',
                command: 'perl-lsp.runTests',
                disabled: !isTestFile
            },
            {
                label: '$(list-flat) Format Document',
                description: 'Shift+Alt+F',
                detail: isPerl ? 'Format using perltidy' : 'Format using perltidy (Only available for Perl files)',
                command: 'editor.action.formatDocument',
                disabled: !isPerl
            },

            { label: 'Information', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(output) Show Output', detail: 'Open the extension output channel', command: 'perl-lsp.showOutput' },
            { label: '$(info) Show Version', detail: 'Check installed perl-lsp version', command: 'perl-lsp.showVersion' },

            { label: 'Configuration', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(gear) Configure Settings', detail: 'Open Perl LSP settings', command: 'workbench.action.openSettings', args: ['@ext:EffortlessMetrics.perl-lsp-rs'] }
        ];

        const selection = await vscode.window.showQuickPick(items, {
            placeHolder: 'Perl Language Server Actions'
        });

        if (selection && selection.command && !selection.disabled) {
            vscode.commands.executeCommand(selection.command, ...(selection.args || []));
        }
    });
    
    context.subscriptions.push(
        restartCommand,
        reinstallCommand,
        organizeImportsCommand,
        runTestsCommand,
        showVersionCommand,
        statusMenuCommand
    );
    
    outputChannel.appendLine('Perl Language Server started successfully');
}

export async function deactivate() {
    if (testAdapter) {
        testAdapter.dispose();
    }
    if (client) {
        await client.stop();
    }
}

async function getServerPath(context: vscode.ExtensionContext): Promise<string | null> {
    // First check user settings
    const config = vscode.workspace.getConfiguration('perl-lsp');
    const userPath = config.get<string>('serverPath');
    
    if (userPath && fs.existsSync(userPath)) {
        outputChannel.appendLine(`Using user-configured perl-lsp: ${userPath}`);
        return userPath;
    }
    
    // Check bundled binary
    const platform = process.platform;
    const arch = process.arch;
    let binaryName = 'perl-lsp';
    
    if (platform === 'win32') {
        binaryName = 'perl-lsp.exe';
    }
    
    const bundledPath = path.join(
        context.extensionPath,
        'bin',
        `${platform}-${arch}`,
        binaryName
    );
    
    if (fs.existsSync(bundledPath)) {
        outputChannel.appendLine(`Using bundled perl-lsp: ${bundledPath}`);
        // Make sure it's executable on Unix-like systems
        if (platform !== 'win32') {
            fs.chmodSync(bundledPath, 0o755);
        }
        return bundledPath;
    }
    
    // Try to find in PATH
    const pathDirs = process.env.PATH?.split(path.delimiter) || [];
    for (const dir of pathDirs) {
        const fullPath = path.join(dir, binaryName);
        if (fs.existsSync(fullPath)) {
            outputChannel.appendLine(`Found perl-lsp in PATH: ${fullPath}`);
            return fullPath;
        }
    }
    
    // Check if auto-download is enabled
    const autoDownload = config.get<boolean>('autoDownload', true);
    
    if (autoDownload) {
        outputChannel.appendLine('perl-lsp not found, attempting to download...');
        const downloader = new BinaryDownloader(context, outputChannel);
        const downloadedPath = await downloader.ensureBinary();
        
        if (downloadedPath) {
            outputChannel.appendLine(`Downloaded perl-lsp to: ${downloadedPath}`);
            return downloadedPath;
        }
    } else {
        outputChannel.appendLine('perl-lsp not found and auto-download is disabled');
    }
    
    outputChannel.appendLine('Failed to obtain perl-lsp');
    return null;
}

async function reinstallServer(context: vscode.ExtensionContext): Promise<void> {
    outputChannel.show(true);
    outputChannel.appendLine('Starting perl-lsp reinstall...');

    const downloader = new BinaryDownloader(context, outputChannel);
    const previousPath = resolvedServerPath;

    try {
        if (client) {
            await client.stop();
        }

        downloader.clearCachedBinaries();
        resolvedServerPath = await getServerPath(context);

        if (!resolvedServerPath) {
            throw new Error('perl-lsp could not be located after reinstall.');
        }

        const healthOk = await runHealthCheck(resolvedServerPath);
        if (!healthOk) {
            throw new Error(`Health check failed for reinstalled binary: ${resolvedServerPath}`);
        }

        if (client) {
            await client.start();
        }

        const detail = previousPath && previousPath !== resolvedServerPath
            ? `Reinstalled perl-lsp and switched from ${previousPath} to ${resolvedServerPath}.`
            : `Reinstalled perl-lsp at ${resolvedServerPath}.`;

        vscode.window.showInformationMessage(detail, 'Show Output').then(selection => {
            if (selection === 'Show Output') {
                outputChannel.show();
            }
        });
    } catch (error: unknown) {
        const message = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`Failed to reinstall perl-lsp: ${message}`);
        vscode.window.showErrorMessage(`Failed to reinstall Perl Language Server: ${message}`, 'Show Output').then(selection => {
            if (selection === 'Show Output') {
                outputChannel.show();
            }
        });

        if (client) {
            try {
                await client.start();
            } catch (restartError: unknown) {
                const restartMessage = restartError instanceof Error ? restartError.message : String(restartError);
                outputChannel.appendLine(`Failed to restart client after reinstall error: ${restartMessage}`);
            }
        }
    }
}

/**
 * Run `perl-lsp --health` and return `true` if the binary responds with `ok`.
 *
 * Waits up to 5 seconds. Returns `false` on timeout, non-zero exit, or if
 * stdout does not start with `ok`.
 */
async function runHealthCheck(serverPath: string): Promise<boolean> {
    return new Promise(resolve => {
        execFile(serverPath, ['--health'], { timeout: 5000 }, (err: Error | null, stdout: string) => {
            if (err) {
                outputChannel.appendLine(`[health-check] Failed: ${err.message}`);
                resolve(false);
                return;
            }
            const ok = stdout.trim().startsWith('ok');
            if (!ok) {
                outputChannel.appendLine(`[health-check] Unexpected output: ${stdout.trim()}`);
            }
            resolve(ok);
        });
    });
}

function getServerArgs(baseArgs: string[]): string[] {
    const config = vscode.workspace.getConfiguration('perl-lsp');
    const featureProfile = config.get<string>('featureProfile', 'auto');
    const canonicalProfile = normalizeFeatureProfile(featureProfile || 'auto');

    if (!canonicalProfile || canonicalProfile === 'auto') {
        return baseArgs;
    }

    return [...baseArgs, `--feature-profile=${canonicalProfile}`];
}

function normalizeFeatureProfile(rawProfile: string): string | null {
    const normalized = rawProfile.trim().toLowerCase();
    if (!normalized) {
        return 'auto';
    }

    const normalizedProfile = normalized.replace(/_/g, '-');
    const knownProfiles = getSupportedFeatureProfiles();

    if (!knownProfiles.includes(normalizedProfile)) {
        outputChannel.appendLine(`Unsupported featureProfile '${rawProfile}'. Falling back to 'auto'.`);
        return null;
    }

    return normalizedProfile;
}

function getSupportedFeatureProfiles(): string[] {
    const extension = vscode.extensions.getExtension('EffortlessMetrics.perl-lsp-rs');
    const schemaEnum =
        extension?.packageJSON?.contributes?.configuration?.properties?.['perl-lsp.featureProfile']?.enum;

    if (Array.isArray(schemaEnum)) {
        return schemaEnum.map((value: unknown) => `${value}`).map((profile) => profile.toLowerCase().replace(/_/g, '-'));
    }

    return [
        'auto',
        'ga-lock',
        'ga',
        'prod',
        'production',
        'all',
    ];
}

async function restartServer(context: vscode.ExtensionContext) {
    if (!client) {
        vscode.window.showWarningMessage('Perl Language Server is not initialized yet.');
        return;
    }

    try {
        await client.stop();
        await client.start();
        vscode.window.showInformationMessage('Perl Language Server restarted', 'Show Output').then(selection => {
            if (selection === 'Show Output') {
                outputChannel.show();
            }
        });
    } catch (error: any) {
        const message = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`Failed to restart perl-lsp: ${message}`);
        vscode.window.showErrorMessage(`Failed to restart Perl Language Server: ${message}`, 'Show Output').then(selection => {
            if (selection === 'Show Output') {
                outputChannel.show();
            }
        });
    }
}
