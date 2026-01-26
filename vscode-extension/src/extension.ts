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
            vscode.env.openExternal(vscode.Uri.parse('https://github.com/EffortlessSteven/tree-sitter-perl'));
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
            await testAdapter.runFileTests(editor.document.uri);
        } else {
            vscode.window.showWarningMessage('Test adapter is not available. It might still be initializing.');
        }
    });
    
    const showOutputCommand = vscode.commands.registerCommand('perl-lsp.showOutput', () => {
        outputChannel.show();
    });
    
    const showVersionCommand = vscode.commands.registerCommand('perl-lsp.showVersion', async () => {
        const { execFile } = require('child_process');
        execFile(serverPath, ['--version'], (error: any, stdout: string, stderr: string) => {
            if (error) {
                vscode.window.showErrorMessage(`Failed to get version: ${error.message}`);
            } else {
                vscode.window.showInformationMessage(`Perl LSP Version: ${stdout.trim()}`);
            }
        });
    });

    const statusMenuCommand = vscode.commands.registerCommand('perl-lsp.showStatusMenu', async () => {
        interface MenuAction extends vscode.QuickPickItem {
            command: string;
        }

        const items: MenuAction[] = [
            { label: '$(refresh) Restart Server', description: 'Restart the language server', command: 'perl-lsp.restart' },
            { label: '$(beaker) Run Tests in Current File', description: 'Run tests for the active file', command: 'perl-lsp.runTests' },
            { label: '$(output) Show Output', description: 'Open the extension output channel', command: 'perl-lsp.showOutput' },
            { label: '$(info) Show Version', description: 'Check installed perl-lsp version', command: 'perl-lsp.showVersion' }
        ];

        const selection = await vscode.window.showQuickPick(items, {
            placeHolder: 'Perl Language Server Actions'
        });

        if (selection) {
            vscode.commands.executeCommand(selection.command);
        }
    });
    
    context.subscriptions.push(restartCommand, runTestsCommand, showOutputCommand, showVersionCommand, statusMenuCommand);
    
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
    vscode.window.showInformationMessage('Perl Language Server restarted');
}