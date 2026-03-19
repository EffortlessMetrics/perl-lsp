import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { execFile } from 'child_process';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    State,
    Trace
} from 'vscode-languageclient/node';
import { PerlTestAdapter } from './testAdapter';
import { activateDebugger } from './debugAdapter';
import { BinaryDownloader } from './downloader';
import { PerlTaskProvider } from './taskProvider';

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let testAdapter: PerlTestAdapter | undefined;
let currentServerPath: string | null = null;
let statusBarItem: vscode.StatusBarItem | undefined;
let stateChangeDisposable: vscode.Disposable | undefined;
let taskProvider: PerlTaskProvider | undefined;

export async function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.window.createOutputChannel('Perl Language Server');
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.command = 'perl-lsp.showStatusMenu';
    statusBarItem.show();
    setStatusBarState(State.Starting);
    context.subscriptions.push(statusBarItem);

    // Register showOutput command early so it's available during binary download and initialization
    const showOutputCommand = vscode.commands.registerCommand('perl-lsp.showOutput', () => {
        outputChannel.show();
    });
    const reinstallCommand = vscode.commands.registerCommand('perl-lsp.reinstall', async () => {
        await reinstallServerBinary(context);
    });

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
            const originalText = statusBarItem?.text;
            const originalTooltip = statusBarItem?.tooltip;

            // Show running state
            if (statusBarItem) {
                statusBarItem.text = '$(beaker~spin) Running Tests...';
                statusBarItem.tooltip = 'Executing Perl tests in current file';
            }

            try {
                await testAdapter.runFileTests(editor.document.uri);
            } finally {
                // Restore original state
                if (statusBarItem && originalText) {
                    statusBarItem.text = originalText;
                    statusBarItem.tooltip = originalTooltip;
                }
            }
        } else {
            vscode.window.showWarningMessage('Test adapter is not available. It might still be initializing.');
        }
    });
    
    const showVersionCommand = vscode.commands.registerCommand('perl-lsp.showVersion', async () => {
        if (!currentServerPath) {
            vscode.window.showErrorMessage('Perl LSP server path is unavailable.');
            return;
        }

        execFile(currentServerPath, ['--version'], (error: Error | null, stdout: string) => {
            if (error) {
                vscode.window.showErrorMessage(`Failed to get version: ${error.message}`);
                return;
            }

            const version = stdout.trim();
            vscode.window.showInformationMessage(`Perl LSP Version: ${version}`, 'Copy').then(selection => {
                if (selection === 'Copy') {
                    void vscode.env.clipboard.writeText(version);
                }
            });
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
            { label: '$(cloud-download) Reinstall Server Binary', detail: 'Re-download the managed perl-lsp binary', command: 'perl-lsp.reinstall' },

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

    const formatOnSaveDisposable = vscode.workspace.onWillSaveTextDocument((event) => {
        if (!shouldFormatOnSave(event.document)) {
            return;
        }

        event.waitUntil(formatDocumentOnSave(event.document));
    });

    const configurationWatcher = vscode.workspace.onDidChangeConfiguration(async (event) => {
        if (event.affectsConfiguration('perl-lsp.enableTestIntegration')) {
            await refreshTestAdapter(context);
        }

        if (event.affectsConfiguration('perl-lsp.trace.server') && client) {
            const newTrace = getTraceLevel();
            client.setTrace(newTrace);
            outputChannel.appendLine(`Trace level changed to: ${newTrace}`);
        }

        if (requiresClientRefresh(event)) {
            await promptForClientRefresh(context);
        }
    });

    context.subscriptions.push(
        showOutputCommand,
        restartCommand,
        organizeImportsCommand,
        runTestsCommand,
        showVersionCommand,
        statusMenuCommand,
        reinstallCommand,
        formatOnSaveDisposable,
        configurationWatcher,
    );

    // Initialize task auto-detection
    taskProvider = new PerlTaskProvider();
    context.subscriptions.push(
        vscode.tasks.registerTaskProvider(PerlTaskProvider.type, taskProvider),
        taskProvider
    );

    // Initialize debug adapter
    activateDebugger(context);
    await initializeLanguageClient(context);
}

export async function deactivate() {
    await disposeLanguageClient();
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

async function initializeLanguageClient(context: vscode.ExtensionContext): Promise<boolean> {
    setStatusBarState(State.Starting);

    currentServerPath = await getServerPath(context);
    if (!currentServerPath) {
        setStatusBarState(State.Stopped);
        const choice = await vscode.window.showErrorMessage(
            'Perl Language Server (perl-lsp) not found.',
            'Install (cargo install perl-lsp)',
            'Open Settings'
        );

        if (choice === 'Install (cargo install perl-lsp)') {
            void vscode.window.showInformationMessage(
                'Run in your terminal: cargo install perl-lsp\nThen reload VS Code.'
            );
        } else if (choice === 'Open Settings') {
            void vscode.commands.executeCommand('workbench.action.openSettings', 'perl-lsp.serverPath');
        }

        return false;
    }

    const healthOk = await runHealthCheck(currentServerPath);
    if (!healthOk) {
        setStatusBarState(State.Stopped);
        const choice = await vscode.window.showErrorMessage(
            `perl-lsp health check failed. The binary at '${currentServerPath}' does not respond to --health. ` +
            'It may be corrupted or incompatible with your platform.',
            'Show Output',
            'Reinstall'
        );
        if (choice === 'Show Output') {
            outputChannel.show();
        } else if (choice === 'Reinstall') {
            await reinstallServerBinary(context);
        }
        return false;
    }

    client = createLanguageClient(currentServerPath);
    bindClientState(client);
    await client.start();
    await refreshTestAdapter(context);
    outputChannel.appendLine('Perl Language Server started successfully');
    return true;
}

function createLanguageClient(serverPath: string): LanguageClient {
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

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'perl' },
            { scheme: 'untitled', language: 'perl' }
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/.perltidyrc')
        },
        outputChannel,
        traceOutputChannel: outputChannel
    };

    const lc = new LanguageClient(
        'perl-language-server',
        'Perl Language Server',
        serverOptions,
        clientOptions
    );
    lc.setTrace(getTraceLevel());
    return lc;
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

function getTraceLevel(): Trace {
    const traceSetting = vscode.workspace.getConfiguration('perl-lsp').get<string>('trace.server', 'off');

    switch ((traceSetting || 'off').toLowerCase()) {
        case 'messages':
            return Trace.Messages;
        case 'verbose':
            return Trace.Verbose;
        default:
            return Trace.Off;
    }
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
    if (!client && !currentServerPath) {
        vscode.window.showWarningMessage('Perl Language Server is not initialized yet.');
        return;
    }

    try {
        await disposeLanguageClient();
        const started = await initializeLanguageClient(context);
        if (!started) {
            return;
        }
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

function shouldFormatOnSave(document: vscode.TextDocument): boolean {
    if (document.languageId !== 'perl') {
        return false;
    }

    const config = vscode.workspace.getConfiguration('perl-lsp', document.uri);
    return config.get<boolean>('formatOnSave', false);
}

async function formatDocumentOnSave(document: vscode.TextDocument): Promise<vscode.TextEdit[]> {
    const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
        'vscode.executeFormatDocumentProvider',
        document.uri
    );

    return edits ?? [];
}

async function refreshTestAdapter(context: vscode.ExtensionContext) {
    if (testAdapter) {
        testAdapter.dispose();
        testAdapter = undefined;
    }

    const config = vscode.workspace.getConfiguration('perl-lsp');
    if (!config.get<boolean>('enableTestIntegration', true) || !client) {
        outputChannel.appendLine('Perl test integration disabled.');
        return;
    }

    testAdapter = new PerlTestAdapter(client);
    context.subscriptions.push(testAdapter);
    outputChannel.appendLine('Perl test integration enabled.');
}

async function reinstallServerBinary(context: vscode.ExtensionContext) {
    outputChannel.show(true);
    outputChannel.appendLine('Reinstalling perl-lsp binary...');

    const downloader = new BinaryDownloader(context, outputChannel);
    const downloadedPath = await downloader.ensureBinary(true);

    if (!downloadedPath) {
        vscode.window.showErrorMessage('Failed to reinstall perl-lsp. See output for details.', 'Show Output').then(selection => {
            if (selection === 'Show Output') {
                outputChannel.show();
            }
        });
        return;
    }

    currentServerPath = downloadedPath;
    const healthOk = await runHealthCheck(downloadedPath);
    if (!healthOk) {
        vscode.window.showErrorMessage('Downloaded perl-lsp failed its health check.', 'Show Output').then(selection => {
            if (selection === 'Show Output') {
                outputChannel.show();
            }
        });
        return;
    }

    const choice = await vscode.window.showInformationMessage(
        'perl-lsp was reinstalled successfully.',
        client ? 'Restart Server' : 'OK'
    );

    if (choice === 'Restart Server' && client) {
        await restartServer(context);
    }
}

function bindClientState(languageClient: LanguageClient) {
    stateChangeDisposable?.dispose();
    stateChangeDisposable = languageClient.onDidChangeState(event => {
        setStatusBarState(event.newState);
    });
}

function setStatusBarState(state: State) {
    if (!statusBarItem) {
        return;
    }

    switch (state) {
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
}

function requiresClientRefresh(event: vscode.ConfigurationChangeEvent): boolean {
    return [
        'perl-lsp.serverPath',
        'perl-lsp.autoDownload',
        'perl-lsp.channel',
        'perl-lsp.versionTag',
        'perl-lsp.downloadBaseUrl',
        'perl-lsp.featureProfile',
    ].some(setting => event.affectsConfiguration(setting));
}

async function promptForClientRefresh(context: vscode.ExtensionContext) {
    const choice = await vscode.window.showInformationMessage(
        'Perl LSP settings changed. Restart the language server to apply the new configuration.',
        'Restart Now',
        'Later'
    );

    if (choice === 'Restart Now') {
        await restartServer(context);
    }
}

async function disposeLanguageClient() {
    if (testAdapter) {
        testAdapter.dispose();
        testAdapter = undefined;
    }

    stateChangeDisposable?.dispose();
    stateChangeDisposable = undefined;

    if (client) {
        const activeClient = client;
        client = undefined;
        await activeClient.stop();
        activeClient.dispose();
    }
}
