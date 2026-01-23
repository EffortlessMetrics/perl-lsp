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
let statusBarItem: vscode.StatusBarItem;

export async function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.window.createOutputChannel('Perl Language Server');
    
    // Status Bar initialization
    if (statusBarItem) {
        statusBarItem.dispose();
    }
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    context.subscriptions.push(statusBarItem);
    statusBarItem.command = 'perl-lsp.showStatusMenu';
    updateStatusBar(State.Starting);
    statusBarItem.show();

    // Get the path to perl-lsp
    const serverPath = await getServerPath(context);
    if (!serverPath) {
        vscode.window.showErrorMessage(
            'Perl Language Server (perl-lsp) not found. Please install it or set perl.lsp.path in settings.'
        );
        updateStatusBar(State.Stopped);
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

    // Update status bar on state change
    client.onDidChangeState((event) => {
        updateStatusBar(event.newState);
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
    
    const showOutputCommand = vscode.commands.registerCommand('perl.showOutputChannel', () => {
        outputChannel.show();
    });
    
    const showVersionCommand = vscode.commands.registerCommand('perl-lsp.showVersion', async () => {
        const { exec } = require('child_process');
        exec(`${serverPath} --version`, (error: any, stdout: string, stderr: string) => {
            if (error) {
                vscode.window.showErrorMessage(`Failed to get version: ${error.message}`);
            } else {
                vscode.window.showInformationMessage(`Perl LSP Version: ${stdout.trim()}`);
            }
        });
    });

    const showStatusMenuCommand = vscode.commands.registerCommand('perl-lsp.showStatusMenu', async () => {
        await showStatusMenu();
    });
    
    context.subscriptions.push(restartCommand, showOutputCommand, showVersionCommand, showStatusMenuCommand);
    
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

function updateStatusBar(state: State) {
    if (!statusBarItem) {
        return;
    }

    switch (state) {
        case State.Starting:
            statusBarItem.text = '$(sync~spin) Perl LSP: Starting...';
            statusBarItem.tooltip = 'Perl Language Server is starting';
            break;
        case State.Running:
            statusBarItem.text = '$(check) Perl LSP: Ready';
            statusBarItem.tooltip = 'Perl Language Server is running';
            break;
        case State.Stopped:
            statusBarItem.text = '$(circle-slash) Perl LSP: Stopped';
            statusBarItem.tooltip = 'Perl Language Server is stopped';
            break;
    }
}

async function showStatusMenu() {
    const items = [
        { label: '$(refresh) Restart Server', description: 'Restart the language server', command: 'perl-lsp.restart' },
        { label: '$(output) Show Output', description: 'Show the language server output channel', command: 'perl.showOutputChannel' },
        { label: '$(info) Show Version', description: 'Show the current version of the language server', command: 'perl-lsp.showVersion' },
        { label: '$(settings) Configure Settings', description: 'Open Perl LSP settings', command: 'workbench.action.openSettings', args: ['perl-lsp'] }
    ];

    const selection = await vscode.window.showQuickPick(items, {
        placeHolder: 'Perl Language Server Actions'
    });

    if (selection) {
        if (selection.args) {
            vscode.commands.executeCommand(selection.command, ...selection.args);
        } else {
            vscode.commands.executeCommand(selection.command);
        }
    }
}