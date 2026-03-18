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


type ServerVisualState = 'starting' | 'running' | 'stopped' | 'runningTests';

interface ActivePerlContext {
    isPerl: boolean;
    fileLabel?: string;
    fileKind?: 'test' | 'module' | 'script' | 'pod' | 'perl';
    canRunTests: boolean;
}

function getActivePerlContext(): ActivePerlContext {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        return { isPerl: false, canRunTests: false };
    }

    const fileName = path.basename(editor.document.uri.fsPath || editor.document.fileName || 'untitled');
    const normalized = fileName.toLowerCase();

    let fileKind: ActivePerlContext['fileKind'] = 'perl';
    if (normalized.endsWith('.t')) {
        fileKind = 'test';
    } else if (normalized.endsWith('.pm')) {
        fileKind = 'module';
    } else if (normalized.endsWith('.pl')) {
        fileKind = 'script';
    } else if (normalized.endsWith('.pod')) {
        fileKind = 'pod';
    }

    return {
        isPerl: true,
        fileLabel: fileName,
        fileKind,
        canRunTests: fileKind === 'test' || fileKind === 'script'
    };
}

function buildStatusBarPresentation(state: ServerVisualState): {
    text: string;
    tooltip: vscode.MarkdownString;
    backgroundColor?: vscode.ThemeColor;
} {
    const activeContext = getActivePerlContext();

    if (state === 'runningTests') {
        const tooltip = new vscode.MarkdownString(undefined, true);
        tooltip.appendMarkdown('$(beaker~spin) **Perl Language Server**\n\n');
        tooltip.appendMarkdown('Running tests for the active file.\n\n');
        if (activeContext.fileLabel) {
            tooltip.appendMarkdown(`- Active file: \`${activeContext.fileLabel}\`\n`);
        }
        tooltip.appendMarkdown('- Click to open the action menu.');
        tooltip.isTrusted = false;
        return {
            text: '$(beaker~spin) Perl LSP · tests',
            tooltip
        };
    }

    const statusIcon = state === 'running' ? 'check' : state === 'starting' ? 'sync~spin' : 'error';
    const statusLabel = state === 'running' ? 'Running' : state === 'starting' ? 'Starting' : 'Stopped';
    const contextIcon = !activeContext.isPerl
        ? 'code'
        : activeContext.fileKind === 'test'
            ? 'beaker'
            : activeContext.fileKind === 'module'
                ? 'package'
                : activeContext.fileKind === 'script'
                    ? 'terminal'
                    : activeContext.fileKind === 'pod'
                        ? 'book'
                        : 'symbol-file';
    const contextLabel = !activeContext.isPerl
        ? 'No Perl file'
        : activeContext.fileLabel ?? 'Perl file';

    const tooltip = new vscode.MarkdownString(undefined, true);
    tooltip.appendMarkdown(`$(${statusIcon}) **Perl Language Server**\n\n`);
    tooltip.appendMarkdown(`- Status: **${statusLabel}**\n`);
    tooltip.appendMarkdown(`- Active editor: $(${contextIcon}) **${contextLabel}**\n`);
    if (activeContext.isPerl) {
        const testStatus = activeContext.canRunTests ? 'Available' : 'Open a `.t` or `.pl` file';
        tooltip.appendMarkdown(`- Run tests: **${testStatus}**\n`);
    }
    tooltip.appendMarkdown('\nClick to open the action menu.');
    tooltip.isTrusted = false;

    return {
        text: `$(${statusIcon}) Perl LSP · $(${contextIcon})`,
        tooltip,
        backgroundColor: state === 'stopped'
            ? new vscode.ThemeColor('statusBarItem.errorBackground')
            : undefined
    };
}

function renderStatusBar(statusBarItem: vscode.StatusBarItem, state: ServerVisualState) {
    const presentation = buildStatusBarPresentation(state);
    statusBarItem.text = presentation.text;
    statusBarItem.tooltip = presentation.tooltip;
    statusBarItem.backgroundColor = presentation.backgroundColor;
}

export async function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.window.createOutputChannel('Perl Language Server');

    // Register showOutput command early so it's available during binary download and initialization
    const showOutputCommand = vscode.commands.registerCommand('perl-lsp.showOutput', () => {
        outputChannel.show();
    });
    context.subscriptions.push(showOutputCommand);
    
    // Get the path to perl-lsp
    const serverPath = await getServerPath(context);
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
    renderStatusBar(statusBarItem, 'starting');
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    client.onDidChangeState(event => {
        switch (event.newState) {
            case State.Running:
                renderStatusBar(statusBarItem, 'running');
                break;
            case State.Starting:
                renderStatusBar(statusBarItem, 'starting');
                break;
            case State.Stopped:
                renderStatusBar(statusBarItem, 'stopped');
                break;
        }
    });

    const activeEditorListener = vscode.window.onDidChangeActiveTextEditor(() => {
        const clientState = client?.state;
        if (clientState === State.Running) {
            renderStatusBar(statusBarItem, 'running');
        } else if (clientState === State.Starting) {
            renderStatusBar(statusBarItem, 'starting');
        } else {
            renderStatusBar(statusBarItem, 'stopped');
        }
    });
    context.subscriptions.push(activeEditorListener);

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
            const originalBackgroundColor = statusBarItem.backgroundColor;

            // Show running state
            renderStatusBar(statusBarItem, 'runningTests');

            try {
                await testAdapter.runFileTests(editor.document.uri);
            } finally {
                // Restore original state
                statusBarItem.text = originalText;
                statusBarItem.tooltip = originalTooltip;
                statusBarItem.backgroundColor = originalBackgroundColor;
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
            title: 'Perl Language Server',
            placeHolder: 'Choose an action or review the current Perl context'
        });

        if (selection && selection.command && !selection.disabled) {
            vscode.commands.executeCommand(selection.command, ...(selection.args || []));
        }
    });
    
    context.subscriptions.push(restartCommand, organizeImportsCommand, runTestsCommand, showVersionCommand, statusMenuCommand);
    
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
