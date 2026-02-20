import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
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
            'Perl Language Server (perl-lsp) not found. Please install it or set perl.lsp.path in settings.'
        );
        return;
    }

    // Server options
    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            args: ['--stdio'],
            transport: TransportKind.stdio
        },
        debug: {
            command: serverPath,
            args: ['--stdio', '--log'],
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

    // Helper for unimplemented refactoring commands
    const handleMissingRefactor = async (title: string) => {
        const selection = await vscode.window.showInformationMessage(
            `The '${title}' feature is currently in development.`,
            'View Roadmap'
        );
        if (selection === 'View Roadmap') {
            vscode.env.openExternal(vscode.Uri.parse('https://github.com/EffortlessMetrics/perl-lsp'));
        }
    };

    // Register placeholder refactoring commands
    const extractSubCommand = vscode.commands.registerCommand('perl-lsp.extractSubroutine', () =>
        handleMissingRefactor('Extract Subroutine')
    );

    const extractVarCommand = vscode.commands.registerCommand('perl-lsp.extractVariable', () =>
        handleMissingRefactor('Extract Variable')
    );

    const inlineVarCommand = vscode.commands.registerCommand('perl-lsp.inlineVariable', () =>
        handleMissingRefactor('Inline Variable')
    );

    context.subscriptions.push(extractSubCommand, extractVarCommand, inlineVarCommand);
    
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
        interface MenuAction extends vscode.QuickPickItem {
            command?: string;
            args?: any[];
            disabled?: boolean; // Explicitly add for older @types/vscode if needed, though 1.82+ supports it
        }

        const editor = vscode.window.activeTextEditor;
        const isPerl = editor?.document.languageId === 'perl';
        const fileName = editor?.document.uri.fsPath || '';
        const isTest = isPerl && (fileName.endsWith('.t') || fileName.endsWith('.pl'));

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
                detail: isPerl ? 'Sort and organize use statements' : 'Only available for Perl files',
                command: 'perl-lsp.organizeImports',
                disabled: !isPerl
            },
            {
                label: '$(beaker) Run Tests in Current File',
                description: 'Shift+Alt+T',
                detail: isTest ? 'Run tests for the active file' : 'Only available for .t or .pl files',
                command: 'perl-lsp.runTests',
                disabled: !isTest
            },
            {
                label: '$(list-flat) Format Document',
                description: 'Shift+Alt+F',
                detail: isPerl ? 'Format using perltidy' : 'Only available for Perl files',
                command: 'editor.action.formatDocument',
                disabled: !isPerl
            },

            { label: 'Information', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(output) Show Output', detail: 'Open the extension output channel', command: 'perl-lsp.showOutput' },
            { label: '$(info) Show Version', detail: 'Check installed perl-lsp version', command: 'perl-lsp.showVersion' },

            { label: 'Configuration', kind: vscode.QuickPickItemKind.Separator },
            { label: '$(gear) Configure Settings', detail: 'Open Perl LSP settings', command: 'workbench.action.openSettings', args: ['@ext:effortlesssteven.perl-lsp'] }
        ];

        const selection = await vscode.window.showQuickPick(items, {
            placeHolder: 'Perl Language Server Actions'
        });

        if (selection && selection.command) {
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

async function restartServer(context: vscode.ExtensionContext) {
    if (client) {
        await client.stop();
    }
    
    await activate(context);
    vscode.window.showInformationMessage('Perl Language Server restarted', 'Show Output').then(selection => {
        if (selection === 'Show Output') {
            outputChannel.show();
        }
    });
}
