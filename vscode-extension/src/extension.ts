import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { execFile } from 'child_process';
import { LanguageClient, TransportKind, Trace } from 'vscode-languageclient/node';
import type { LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
import { PerlTestAdapter } from './testAdapter';
import { activateDebugger, rewriteDebugTestLensCommand } from './debugAdapter';
import { BinaryDownloader } from './downloader';
import { OnboardingManager } from './onboarding';
import { WhatsNewManager } from './whatsNew';
import { generateBoilerplate } from './fileCreation';
import { handleFormattingError } from './formattingErrors';
import { HealthWidget, ClientState } from './healthWidget';
import { registerPodPreview } from './podPreview';
import { registerGherkinProviders } from './gherkinProviders';
import { registerGherkinStepDefinitionSupport } from './gherkinStepDefinitions';
import { selectTestCommandAtPosition } from './runTestAtCursor';
import { StreamingCompletionController } from './streamingCompletion';
import {
    classifyStartupError,
    formatStartupFailureDialog,
    StartupErrorKind,
} from './startupDiagnosis';
import type { StartupErrorDiagnosis } from './startupDiagnosis';

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let testAdapter: PerlTestAdapter | undefined;
let currentServerPath: string | null = null;
let statusBarItem: vscode.StatusBarItem | undefined;
let healthWidget: HealthWidget | undefined;
let streamingController: StreamingCompletionController | undefined;
let stateChangeDisposable: vscode.Disposable | undefined;
const COEXISTENCE_GUIDE_URL =
    'https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/README.md#extension-coexistence';
/**
 * Cached diagnostic message from the last startup failure.
 * Set by `initializeLanguageClient` when the LSP fails to start; read by
 * command handlers that need to surface a "server not running" message so
 * they can report the specific root cause rather than a generic "restart" message.
 */
let lastStartupDiagnostic: string | undefined;

/**
 * Return the best available "server not running" message to show the user.
 *
 * When a startup failure has been diagnosed (i.e. `lastStartupDiagnostic` is
 * set), that specific message is returned so the user can see the root cause
 * (e.g. "Perl interpreter not found" rather than a generic restart prompt).
 * Falls back to a brief actionable message for the case where the server was
 * never started in this session.
 */
function serverNotRunningMessage(): string {
    return lastStartupDiagnostic ??
        'Perl Language Server is not running. Run the Health Check (Command Palette: "Perl: Run Health Check") to diagnose the issue.';
}

type PerlCriticSettings = {
    enabled: boolean;
    severity: number;
    profile: string;
    theme: string;
};

function getPerlCriticSettings(documentUri?: vscode.Uri): PerlCriticSettings {
    const config = vscode.workspace.getConfiguration('perl-lsp', documentUri);
    return {
        enabled: config.get<boolean>('perlcritic.enabled', false),
        severity: config.get<number>('perlcritic.severity', 3),
        profile: config.get<string>('perlcritic.profile', ''),
        theme: config.get<string>('perlcritic.theme', ''),
    };
}

type PerlCriticSyncSettings = {
    enabled?: boolean;
    severity?: number;
    profile?: string;
};

function inspectPerlCriticOverride(
    config: vscode.WorkspaceConfiguration,
    key: string
): { globalValue?: unknown; workspaceValue?: unknown; workspaceFolderValue?: unknown } | undefined {
    return config.inspect(key) as {
        globalValue?: unknown;
        workspaceValue?: unknown;
        workspaceFolderValue?: unknown;
    } | undefined;
}

function getPerlCriticSyncSettings(
    documentUri?: vscode.Uri,
    severityOverride?: number
): PerlCriticSyncSettings {
    const config = vscode.workspace.getConfiguration('perl-lsp', documentUri);
    const settings: PerlCriticSyncSettings = {};

    const enabled = inspectPerlCriticOverride(config, 'perlcritic.enabled');
    if (enabled?.globalValue !== undefined ||
        enabled?.workspaceValue !== undefined ||
        enabled?.workspaceFolderValue !== undefined) {
        settings.enabled = config.get<boolean>('perlcritic.enabled', false);
    }

    const severity = inspectPerlCriticOverride(config, 'perlcritic.severity');
    if (severityOverride !== undefined) {
        settings.severity = severityOverride;
    } else if (severity?.globalValue !== undefined ||
        severity?.workspaceValue !== undefined ||
        severity?.workspaceFolderValue !== undefined) {
        settings.severity = config.get<number>('perlcritic.severity', 3);
    }

    const profile = inspectPerlCriticOverride(config, 'perlcritic.profile');
    if (profile?.globalValue !== undefined ||
        profile?.workspaceValue !== undefined ||
        profile?.workspaceFolderValue !== undefined) {
        settings.profile = config.get<string>('perlcritic.profile', '');
    }

    return settings;
}

function buildPerlCriticConfiguration(settings: PerlCriticSyncSettings): Record<string, unknown> | undefined {
    if (
        settings.enabled === undefined &&
        settings.severity === undefined &&
        settings.profile === undefined
    ) {
        return undefined;
    }

    return {
        settings: {
            perl: {
                perlcritic: settings,
            },
        },
    };
}

function hasExplicitPerlCriticOverrides(documentUri?: vscode.Uri): boolean {
    const config = vscode.workspace.getConfiguration('perl-lsp', documentUri);
    return ['perlcritic.enabled', 'perlcritic.severity', 'perlcritic.profile'].some(key => {
        const value = config.inspect(key) as {
            globalValue?: unknown;
            workspaceValue?: unknown;
            workspaceFolderValue?: unknown;
        } | undefined;
        return Boolean(
            value &&
            (value.globalValue !== undefined ||
                value.workspaceValue !== undefined ||
                value.workspaceFolderValue !== undefined)
        );
    });
}

export async function syncPerlCriticConfiguration(
    activeClient: Pick<LanguageClient, 'sendNotification'> | undefined = client,
    documentUri?: vscode.Uri
): Promise<void> {
    if (!activeClient) {
        return;
    }

    const payload = buildPerlCriticConfiguration(getPerlCriticSyncSettings(documentUri));
    if (payload) {
        activeClient.sendNotification('workspace/didChangeConfiguration', payload);
    }
}

export async function runPerlCriticOnActiveFile(
    activeClient: Pick<LanguageClient, 'sendRequest' | 'sendNotification'> | undefined = client
): Promise<void> {
    const channel = outputChannel ?? vscode.window.createOutputChannel('Perl Language Server');
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        vscode.window.showErrorMessage('No active Perl file to run PerlCritic on');
        return;
    }

    if (editor.document.isDirty) {
        await editor.document.save();
    }

    if (!activeClient) {
        vscode.window.showWarningMessage(serverNotRunningMessage());
        return;
    }

    if (hasExplicitPerlCriticOverrides(editor.document.uri)) {
        await syncPerlCriticConfiguration(activeClient, editor.document.uri);
    }

    let result: unknown;
    try {
        result = await activeClient.sendRequest('workspace/executeCommand', {
            command: 'perl.runCritic',
            arguments: [editor.document.uri.toString()],
        });
    } catch (error: unknown) {
        const message = error instanceof Error ? error.message : String(error);
        vscode.window.showErrorMessage(`Failed to run PerlCritic: ${message}`);
        return;
    }

    const response = (result && typeof result === 'object') ? result as Record<string, unknown> : {};
    const status = typeof response.status === 'string' ? response.status : 'unknown';
    const violationCount = typeof response.violationCount === 'number'
        ? response.violationCount
        : Array.isArray(response.violations)
            ? response.violations.length
            : 0;
    const analyzerUsed = typeof response.analyzerUsed === 'string' ? response.analyzerUsed : 'unknown';
    const fileName = path.basename(editor.document.uri.fsPath);

    channel.appendLine(
        `[perlcritic] ${fileName}: status=${status} violations=${violationCount} analyzer=${analyzerUsed}`
    );

    if (status === 'error' || typeof response.error === 'string') {
        const message = typeof response.error === 'string'
            ? response.error
            : 'PerlCritic returned an error';
        vscode.window.showErrorMessage(message, 'Show Output').then(selection => {
            if (selection === 'Show Output') {
                channel.show();
            }
        });
        return;
    }

    if (violationCount > 0) {
        vscode.window.showWarningMessage(
            `PerlCritic found ${violationCount} issue${violationCount === 1 ? '' : 's'} in ${fileName}.`,
            'Show Output'
        ).then(selection => {
            if (selection === 'Show Output') {
                channel.show();
            }
        });
        return;
    }

    vscode.window.showInformationMessage(
        `PerlCritic passed for ${fileName} using ${analyzerUsed}.`,
        'Show Output'
    ).then(selection => {
        if (selection === 'Show Output') {
            channel.show();
        }
    });
}

export async function setPerlCriticSeverity(
    activeClient: Pick<LanguageClient, 'sendNotification'> | undefined = client
): Promise<void> {
    const resourceUri = vscode.window.activeTextEditor?.document.uri;
    const selection = await vscode.window.showQuickPick(
        [
            { label: '1', description: 'Very permissive' },
            { label: '2', description: 'Permissive' },
            { label: '3', description: 'Balanced default' },
            { label: '4', description: 'Strict' },
            { label: '5', description: 'Very strict' },
        ],
        {
            placeHolder: 'Choose a PerlCritic severity level',
        }
    );

    if (!selection) {
        return;
    }

    const severity = Number(selection.label);
    const config = vscode.workspace.getConfiguration('perl-lsp', resourceUri);
    const target = vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0
        ? vscode.ConfigurationTarget.Workspace
        : vscode.ConfigurationTarget.Global;
    await config.update('perlcritic.severity', severity, target);
    const payload = buildPerlCriticConfiguration(getPerlCriticSyncSettings(resourceUri, severity));
    if (activeClient && payload) {
        activeClient.sendNotification('workspace/didChangeConfiguration', payload);
    }

    vscode.window.showInformationMessage(`PerlCritic severity set to ${severity}.`);
}

export async function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.window.createOutputChannel('Perl Language Server');
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.command = 'perl-lsp.showStatusMenu';
    statusBarItem.show();
    healthWidget = new HealthWidget(statusBarItem);
    healthWidget.onStateChange(ClientState.Starting);
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

    const runPerlCriticCommand = vscode.commands.registerCommand('perl-lsp.runPerlCritic', async () => {
        await runPerlCriticOnActiveFile();
    });

    const setPerlCriticSeverityCommand = vscode.commands.registerCommand('perl-lsp.setPerlCriticSeverity', async () => {
        await setPerlCriticSeverity();
    });
    
    const showVersionCommand = vscode.commands.registerCommand('perl-lsp.showVersion', async () => {
        if (!currentServerPath) {
            vscode.window.showErrorMessage(
                serverNotRunningMessage(),
                'Restart Server', 'Show Output', 'Run Health Check'
            ).then(sel => {
                if (sel === 'Restart Server') { void vscode.commands.executeCommand('perl-lsp.restart'); }
                if (sel === 'Show Output') { outputChannel.show(); }
                if (sel === 'Run Health Check') { void vscode.commands.executeCommand('perl-lsp.runHealthCheck'); }
            });
            return;
        }

        execFile(currentServerPath, ['--version'], (error: Error | null, stdout: string) => {
            if (error) {
                vscode.window.showErrorMessage(
                    `Could not get Perl LSP version: ${error.message}. The server binary may be missing or corrupt — try reinstalling.`,
                    'Reinstall'
                ).then(sel => {
                    if (sel === 'Reinstall') { void vscode.commands.executeCommand('perl-lsp.reinstall'); }
                });
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
            args?: unknown[];
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
                label: '$(checklist) Run PerlCritic',
                detail: isPerl ? 'Run PerlCritic on the active file' : 'Run PerlCritic on the active file (Only available for Perl files)',
                command: 'perl-lsp.runPerlCritic',
                disabled: !isPerl
            },
            {
                label: '$(symbol-numeric) Set PerlCritic Severity',
                detail: isPerl ? 'Choose a PerlCritic severity level' : 'Choose a PerlCritic severity level (Only available for Perl files)',
                command: 'perl-lsp.setPerlCriticSeverity',
                disabled: !isPerl
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
            { label: '$(info) Show Version', detail: 'Check installed perllsp version', command: 'perl-lsp.showVersion' },
            { label: '$(pulse) Run Health Check', detail: 'Check Perl, perltidy, and LSP binary', command: 'perl-lsp.runHealthCheck' },
            { label: '$(cloud-download) Reinstall Server Binary', detail: 'Re-download the managed perllsp binary', command: 'perl-lsp.reinstall' },

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

    const runHealthCheckCommand = vscode.commands.registerCommand('perl-lsp.runHealthCheck', async (serverPath?: string | null) => {
        const resolvedPath = serverPath !== undefined ? serverPath : currentServerPath;
        const onboarding = new OnboardingManager(context, outputChannel);
        const results = await onboarding.runSetupHealthCheck(resolvedPath ?? null);

        const errors = results.filter(r => !r.ok && r.status === 'error');
        const warnings = results.filter(r => !r.ok && r.status === 'warning');

        const lines = results.map(r => {
            const icon = r.ok ? '$(check)' : r.status === 'warning' ? '$(warning)' : '$(error)';
            return `${icon} ${r.label}: ${r.detail}`;
        });

        outputChannel.appendLine('[health-check] Results:');
        for (const line of lines) {
            outputChannel.appendLine(`  ${line.replace(/\$\(\w[^)]*\)/g, '')}`);
        }

        if (errors.length > 0) {
            const msg = `Health check failed: ${errors.map(e => e.label).join(', ')}`;
            vscode.window.showErrorMessage(msg, 'Show Output').then(sel => {
                if (sel === 'Show Output') { outputChannel.show(); }
            });
        } else if (warnings.length > 0) {
            const msg = `Health check passed with warnings: ${warnings.map(w => w.detail).join(' | ')}`;
            vscode.window.showWarningMessage(msg, 'Show Output').then(sel => {
                if (sel === 'Show Output') { outputChannel.show(); }
            });
        } else {
            vscode.window.showInformationMessage('Perl LSP health check passed.', 'Show Output').then(sel => {
                if (sel === 'Show Output') { outputChannel.show(); }
            });
        }
    });

    const checkSyntaxCommand = vscode.commands.registerCommand('perl-lsp.checkSyntax', async () => {
        await runCheckSyntax();
    });

    const runCurrentTestCommand = vscode.commands.registerCommand('perl-lsp.runCurrentTest', async () => {
        await runCurrentTestWithProve();
    });

    const runTestAtCursorCommand = vscode.commands.registerCommand('perl-lsp.runTestAtCursor', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'perl') {
            vscode.window.showErrorMessage('Run Test at Cursor requires an active Perl file');
            return;
        }

        if (editor.document.isDirty) {
            await editor.document.save();
        }

        if (!client) {
            vscode.window.showWarningMessage(serverNotRunningMessage());
            return;
        }

        const lenses = await client.sendRequest<Array<{
            range?: { start: { line: number; character: number }; end: { line: number; character: number } };
            command?: { command: string; arguments?: unknown[] };
        }> | null>('textDocument/codeLens', {
            textDocument: { uri: editor.document.uri.toString() },
        });

        const command = selectTestCommandAtPosition(lenses ?? [], editor.selection.active);
        if (!command) {
            vscode.window.showWarningMessage('No runnable test was found at the cursor position');
            return;
        }

        await vscode.commands.executeCommand(command.command, ...(command.arguments ?? []));
    });

    const runAllTestsCommand = vscode.commands.registerCommand('perl-lsp.runAllTests', async () => {
        await runAllTestsWithProve();
    });

    const formatDocumentCommand = vscode.commands.registerCommand('perl-lsp.formatDocument', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'perl') {
            vscode.window.showErrorMessage('No active Perl file to format');
            return;
        }
        await vscode.commands.executeCommand('editor.action.formatDocument');
    });

    const showIncPathsCommand = vscode.commands.registerCommand('perl-lsp.showIncPaths', async () => {
        await showIncPaths();
    });

    const openModuleCommand = vscode.commands.registerCommand('perl-lsp.openModule', async () => {
        await openPerlModule();
    });

    const showParserAstCommand = vscode.commands.registerCommand('perl-lsp.showParserAst', async () => {
        await showParserAst();
    });

    const whatsNewManager = new WhatsNewManager(context, outputChannel);
    const showWhatsNewCommand = vscode.commands.registerCommand('perl-lsp.showWhatsNew', async () => {
        await whatsNewManager.showWhatsNew();
    });

    const openConfigurationGuideCommand = vscode.commands.registerCommand(
        'perl-lsp.openConfigurationGuide',
        () => {
            void vscode.commands.executeCommand(
                'workbench.action.openSettings',
                '@ext:EffortlessMetrics.perl-lsp-rs'
            );
        }
    );

    const extractVariableCommand = vscode.commands.registerCommand('perl-lsp.extractVariable', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'perl') {
            vscode.window.showErrorMessage('Extract Variable requires an active Perl file with a selection');
            return;
        }
        if (editor.selection.isEmpty) {
            vscode.window.showWarningMessage('Select an expression to extract as a variable');
            return;
        }
        if (!client) {
            vscode.window.showWarningMessage(serverNotRunningMessage());
            return;
        }
        const range = editor.selection;
        const params = {
            textDocument: { uri: editor.document.uri.toString() },
            range: {
                start: { line: range.start.line, character: range.start.character },
                end: { line: range.end.line, character: range.end.character },
            },
            context: { diagnostics: [], only: ['refactor.extract'], triggerKind: 2 },
        };
        type CodeActionResult = Array<{ title: string; kind?: string; edit?: unknown; command?: unknown }> | null;
        const actions = await client.sendRequest<CodeActionResult>('textDocument/codeAction', params);
        if (!actions || actions.length === 0) {
            vscode.window.showInformationMessage('No extract actions available for the selected expression');
            return;
        }
        const variableAction = actions.find(a => a.title.toLowerCase().includes('variable'));
        const action = variableAction ?? actions[0];
        if (action.edit) {
            const workspaceEdit = await client.protocol2CodeConverter.asWorkspaceEdit(
                action.edit as Parameters<typeof client.protocol2CodeConverter.asWorkspaceEdit>[0]
            );
            if (workspaceEdit) {
                await vscode.workspace.applyEdit(workspaceEdit);
            }
        } else if (action.command) {
            const cmd = action.command as { command: string; arguments?: unknown[] };
            await vscode.commands.executeCommand(cmd.command, ...(cmd.arguments ?? []));
        } else {
            vscode.window.showInformationMessage('No extract variable action is available for the current selection');
        }
    });

    const extractMethodCommand = vscode.commands.registerCommand('perl-lsp.extractMethod', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'perl') {
            vscode.window.showErrorMessage('Extract Method requires an active Perl file with a selection');
            return;
        }
        if (editor.selection.isEmpty) {
            vscode.window.showWarningMessage('Select code to extract as a method');
            return;
        }
        if (!client) {
            vscode.window.showWarningMessage(serverNotRunningMessage());
            return;
        }
        const range = editor.selection;
        const params = {
            textDocument: { uri: editor.document.uri.toString() },
            range: {
                start: { line: range.start.line, character: range.start.character },
                end: { line: range.end.line, character: range.end.character },
            },
            context: { diagnostics: [], only: ['refactor.extract'], triggerKind: 2 },
        };
        type CodeActionResult = Array<{ title: string; kind?: string; edit?: unknown; command?: unknown }> | null;
        const actions = await client.sendRequest<CodeActionResult>('textDocument/codeAction', params);
        if (!actions || actions.length === 0) {
            vscode.window.showInformationMessage('No extract actions available for the selected code');
            return;
        }
        const subroutineAction = actions.find(
            a => a.title.toLowerCase().includes('subroutine') || a.title.toLowerCase().includes('method') || a.title.toLowerCase().includes('function')
        );
        const action = subroutineAction ?? actions[actions.length - 1];
        if (action.edit) {
            const workspaceEdit = await client.protocol2CodeConverter.asWorkspaceEdit(
                action.edit as Parameters<typeof client.protocol2CodeConverter.asWorkspaceEdit>[0]
            );
            if (workspaceEdit) {
                await vscode.workspace.applyEdit(workspaceEdit);
            }
        } else if (action.command) {
            const cmd = action.command as { command: string; arguments?: unknown[] };
            await vscode.commands.executeCommand(cmd.command, ...(cmd.arguments ?? []));
        } else {
            vscode.window.showInformationMessage('No extract method action is available for the current selection');
        }
    });

    const showRefactoringOptionsCommand = vscode.commands.registerCommand('perl-lsp.showRefactoringOptions', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'perl') {
            vscode.window.showErrorMessage('Refactoring options require an active Perl file');
            return;
        }

        interface RefactorAction extends vscode.QuickPickItem {
            command: string;
            args?: unknown[];
        }

        const items: RefactorAction[] = [
            {
                label: '$(symbol-variable) Extract Variable',
                description: 'Shift+Alt+V',
                detail: editor.selection.isEmpty
                    ? 'Select an expression first to extract it as a variable'
                    : 'Extract selected expression as a local variable',
                command: 'perl-lsp.extractVariable',
            },
            {
                label: '$(symbol-method) Extract Method',
                description: 'Shift+Alt+M',
                detail: editor.selection.isEmpty
                    ? 'Select code first to extract it as a subroutine'
                    : 'Extract selected code as a named subroutine',
                command: 'perl-lsp.extractMethod',
            },
            {
                label: '$(organization) Organize Imports',
                description: 'Shift+Alt+O',
                detail: 'Sort and deduplicate use statements',
                command: 'perl-lsp.organizeImports',
            },
        ];

        const selection = await vscode.window.showQuickPick(items, {
            placeHolder: 'Perl Refactoring Options',
        });

        if (selection) {
            await vscode.commands.executeCommand(selection.command, ...(selection.args ?? []));
        }
    });

    const reportIssueCommand = vscode.commands.registerCommand('perl-lsp.reportIssue', async () => {
        const extensionVersion = context.extension.packageJSON.version as string ?? 'unknown';
        const vscodeVersion = vscode.version;
        const platform = process.platform;
        const arch = process.arch;

        const getServerVersion = (): Promise<string> =>
            new Promise(resolve => {
                if (!currentServerPath) {
                    resolve('unavailable');
                    return;
                }
                execFile(currentServerPath, ['--version'], { timeout: 3000 }, (err: Error | null, stdout: string) => {
                    if (err) {
                        resolve('unavailable');
                        return;
                    }
                    const firstLine = stdout.trim().split('\n')[0] ?? '';
                    resolve(firstLine.trim() || 'unavailable');
                });
            });

        const serverVersion = await getServerVersion();

        const diagnosticInfo = [
            `perl-lsp server: ${serverVersion}`,
            `Extension: ${extensionVersion}`,
            `VS Code: ${vscodeVersion}`,
            `Platform: ${platform}/${arch}`,
        ].join('\n');

        const selection = await vscode.window.showInformationMessage(
            'Open a GitHub issue to report a bug or request a feature.',
            'Copy Diagnostic Info',
            'Open Issue Form'
        );

        if (selection === 'Copy Diagnostic Info') {
            try {
                await vscode.env.clipboard.writeText(diagnosticInfo);
                vscode.window.showInformationMessage('Diagnostic info copied. Paste it into the issue form.');
            } catch {
                // Clipboard unavailable — continue to open browser anyway
            }
        }

        if (selection === 'Copy Diagnostic Info' || selection === 'Open Issue Form') {
            const url = vscode.Uri.parse(
                'https://github.com/EffortlessMetrics/perl-lsp/issues/new?template=bug_report.yml'
            );
            await vscode.env.openExternal(url);
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
            void client.setTrace(newTrace);
            outputChannel.appendLine(`Trace level changed to: ${newTrace}`);
        }

        if (
            event.affectsConfiguration('perl-lsp.aiCompletion.enabled') ||
            event.affectsConfiguration('perl-lsp.aiCompletion.streaming.enabled')
        ) {
            refreshStreamingController(client);
        }

        if (event.affectsConfiguration('perl-lsp.includePaths')) {
            await validateIncludePaths(context);
        }

        if (
            event.affectsConfiguration('perl-lsp.perlcritic.enabled') ||
            event.affectsConfiguration('perl-lsp.perlcritic.severity') ||
            event.affectsConfiguration('perl-lsp.perlcritic.profile')
        ) {
            await syncPerlCriticConfiguration(client);
        }

        if (requiresClientRefresh(event)) {
            await promptForClientRefresh(context);
        }
    });

    const fileCreationWatcher = vscode.workspace.onDidCreateFiles(async (event) => {
        const config = vscode.workspace.getConfiguration('perl-lsp');
        if (!config.get<boolean>('autoPopulateNewFiles', true)) {
            return;
        }

        for (const uri of event.files) {
            const boilerplate = generateBoilerplate(uri.fsPath);
            if (!boilerplate) {
                continue;
            }

            const doc = await vscode.workspace.openTextDocument(uri);
            if (doc.getText().length > 0) {
                // File already has content — don't overwrite
                continue;
            }

            const edit = new vscode.WorkspaceEdit();
            edit.insert(uri, new vscode.Position(0, 0), boilerplate.content);
            await vscode.workspace.applyEdit(edit);
        }
    });

    const checkForUpdateCommand = vscode.commands.registerCommand('perl-lsp.checkForUpdate', async () => {
        const downloader = new BinaryDownloader(context, outputChannel);
        // Reset the lastUpdateCheck timestamp so the interval guard is bypassed
        await context.globalState.update('perl-lsp.lastUpdateCheck', 0);
        await downloader.checkForUpdateSilent();
    });

    const arrowCompletionWatcher = vscode.workspace.onDidChangeTextDocument((event) => {
        maybeNudgeArrowCompletion(event);
    });

    context.subscriptions.push(
        showOutputCommand,
        restartCommand,
        organizeImportsCommand,
        runTestsCommand,
        runPerlCriticCommand,
        setPerlCriticSeverityCommand,
        checkSyntaxCommand,
        runCurrentTestCommand,
        runTestAtCursorCommand,
        runAllTestsCommand,
        formatDocumentCommand,
        showIncPathsCommand,
        openModuleCommand,
        showParserAstCommand,
        showVersionCommand,
        statusMenuCommand,
        reinstallCommand,
        checkForUpdateCommand,
        runHealthCheckCommand,
        showWhatsNewCommand,
        openConfigurationGuideCommand,
        extractVariableCommand,
        extractMethodCommand,
        showRefactoringOptionsCommand,
        reportIssueCommand,
        formatOnSaveDisposable,
        configurationWatcher,
        fileCreationWatcher,
        arrowCompletionWatcher,
        ...registerGherkinProviders(),
        ...registerGherkinStepDefinitionSupport(),
        ...registerPodPreview(context),
    );

    // Initialize debug adapter
    activateDebugger(context);
    await initializeLanguageClient(context);
    await validateIncludePaths(context);
    await warnAboutPerlExtensionConflicts(context);

    // Background update check — fire-and-forget after startup completes.
    // Runs at most once per updateCheckInterval hours; no-ops when serverPath
    // is user-managed, channel='tag', or updateCheckInterval=0.
    const updateDownloader = new BinaryDownloader(context, outputChannel);
    updateDownloader.checkForUpdateSilent().catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        outputChannel.appendLine(`[update-check] Error: ${msg}`);
    });

    // First-run onboarding: show welcome notification once per installation
    const onboarding = new OnboardingManager(context, outputChannel);
    if (onboarding.shouldShowWelcome()) {
        // Fire-and-forget; failures must not block extension startup
        onboarding.showWelcomeNotification(currentServerPath).catch((err: unknown) => {
            const msg = err instanceof Error ? err.message : String(err);
            outputChannel.appendLine(`[onboarding] Error showing welcome: ${msg}`);
        });
        // Mark the version seen on first install so the next activation
        // (after an update) triggers the What's New panel instead of welcome.
        whatsNewManager.markVersionSeen().catch((err: unknown) => {
            const msg = err instanceof Error ? err.message : String(err);
            outputChannel.appendLine(`[whats-new] Error marking version seen: ${msg}`);
        });
    } else if (whatsNewManager.shouldShowWhatsNew()) {
        // Extension was updated — show What's New panel.
        // Fire-and-forget; failures must not block extension startup.
        whatsNewManager.markVersionSeen().then(() => {
            return whatsNewManager.showWhatsNew();
        }).catch((err: unknown) => {
            const msg = err instanceof Error ? err.message : String(err);
            outputChannel.appendLine(`[whats-new] Error showing What's New: ${msg}`);
        });
    }
}

export async function deactivate() {
    await disposeLanguageClient();
}

async function getServerPath(context: vscode.ExtensionContext): Promise<string | null> {
    // First check user settings
    const config = vscode.workspace.getConfiguration('perl-lsp');
    const userPath = config.get<string>('serverPath');
    
    if (userPath && fs.existsSync(userPath)) {
        outputChannel.appendLine(`Using user-configured Perl LSP binary: ${userPath}`);
        return userPath;
    }
    
    // Check bundled binary
    const platform = process.platform;
    const arch = process.arch;
    const binaryNames = platform === 'win32'
        ? ['perllsp.exe', 'perl-lsp.exe']
        : ['perllsp', 'perl-lsp'];

    for (const binaryName of binaryNames) {
        const bundledPath = path.join(
            context.extensionPath,
            'bin',
            `${platform}-${arch}`,
            binaryName
        );

        if (fs.existsSync(bundledPath)) {
            outputChannel.appendLine(`Using bundled Perl LSP binary: ${bundledPath}`);
            if (platform !== 'win32') {
                fs.chmodSync(bundledPath, 0o755);
            }
            return bundledPath;
        }
    }
    
    // Try to find in PATH
    const pathDirs = process.env.PATH?.split(path.delimiter) || [];
    for (const dir of pathDirs) {
        for (const binaryName of binaryNames) {
            const fullPath = path.join(dir, binaryName);
            if (fs.existsSync(fullPath)) {
                outputChannel.appendLine(`Found Perl LSP binary in PATH: ${fullPath}`);
                return fullPath;
            }
        }
    }
    
    // Check if auto-download is enabled
    const autoDownload = config.get<boolean>('autoDownload', true);
    
    if (autoDownload) {
        outputChannel.appendLine('Perl LSP binary not found, attempting to download...');
        const downloader = new BinaryDownloader(context, outputChannel);
        const downloadedPath = await downloader.ensureBinary();
        
        if (downloadedPath) {
            outputChannel.appendLine(`Downloaded Perl LSP binary to: ${downloadedPath}`);
            return downloadedPath;
        }
    } else {
        outputChannel.appendLine('Perl LSP binary not found and auto-download is disabled');
    }
    
    outputChannel.appendLine('Failed to obtain a Perl LSP binary');
    return null;
}

async function initializeLanguageClient(context: vscode.ExtensionContext): Promise<boolean> {
    healthWidget?.onStateChange(ClientState.Starting);

    currentServerPath = await getServerPath(context);
    if (!currentServerPath) {
        healthWidget?.onStateChange(ClientState.Stopped);
        const choice = await vscode.window.showErrorMessage(
            'Perl Language Server (perllsp) not found.',
            'Install (cargo install perllsp)',
            'Open Settings'
        );

        if (choice === 'Install (cargo install perllsp)') {
            void vscode.window.showInformationMessage(
                'Run in your terminal: cargo install perllsp\nThen reload VS Code.'
            );
        } else if (choice === 'Open Settings') {
            void vscode.commands.executeCommand('workbench.action.openSettings', 'perl-lsp.serverPath');
        }

        return false;
    }

    client = createLanguageClient(currentServerPath);
    bindClientState(client);
    try {
        await client.start();
    } catch (startError: unknown) {
        const msg = startError instanceof Error ? startError.message : String(startError);
        outputChannel.appendLine(`[startup] Language client failed to start: ${msg}`);
        stateChangeDisposable?.dispose();
        stateChangeDisposable = undefined;
        try { void client.dispose(); } catch { /* already dead */ }
        client = undefined;
        healthWidget?.onStateChange(ClientState.Stopped);

        // Probe the binary to get an actionable OS-level diagnosis (#3280).
        // If the probe result is Unknown (binary gave no useful output), fall
        // back to the health check (#3312) which can detect missing Perl etc.
        // lastStartupDiagnostic is updated so that serverNotRunningMessage() in
        // command handlers surfaces the specific root cause rather than a generic prompt.
        const probeResult = currentServerPath
            ? await probeStartupFailure(currentServerPath)
            : classifyStartupError('');
        let healthMsg: string | undefined;
        if (probeResult.kind === StartupErrorKind.Unknown) {
            const onboarding = new OnboardingManager(context, outputChannel);
            healthMsg = await onboarding.runStartupDiagnostics(currentServerPath ?? null);
        }
        const dialogMessage = formatStartupFailureDialog(probeResult, healthMsg);
        lastStartupDiagnostic = dialogMessage;

        const choice = await vscode.window.showErrorMessage(
            dialogMessage,
            'View Logs',
            'Run Health Check',
            'Reinstall',
            'Check serverPath Setting'
        );
        if (choice === 'View Logs') {
            outputChannel.show();
        } else if (choice === 'Run Health Check') {
            await vscode.commands.executeCommand('perl-lsp.runHealthCheck', currentServerPath);
        } else if (choice === 'Reinstall') {
            await reinstallServerBinary(context);
        } else if (choice === 'Check serverPath Setting') {
            void vscode.commands.executeCommand('workbench.action.openSettings', 'perl-lsp.serverPath');
        }
        return false;
    }
    // Expose the server version in the widget tooltip once the handshake completes.
    const serverVersion = client.initializeResult?.serverInfo?.version;
    if (serverVersion) {
        healthWidget?.setVersion(serverVersion);
    }

    await refreshTestAdapter(context);

    // Initialize streaming inline completion controller (config-gated)
    refreshStreamingController(client);

    // Clear any stale startup diagnostic — the server started successfully so
    // the root cause (e.g. missing Perl) no longer applies.
    lastStartupDiagnostic = undefined;
    outputChannel.appendLine('Perl Language Server started successfully');
    return true;
}

function createLanguageClient(serverPath: string): LanguageClient {
    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            args: getLanguageServerLaunchArgs(false),
            transport: TransportKind.stdio
        },
        debug: {
            command: serverPath,
            args: getLanguageServerLaunchArgs(true),
            transport: TransportKind.stdio
        }
    };

    const disabledFeatures = vscode.workspace.getConfiguration('perl-lsp')
        .get<string[]>('disabledFeatures', []);

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'perl' },
            { scheme: 'untitled', language: 'perl' }
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/.perltidyrc')
        },
        outputChannel,
        traceOutputChannel: outputChannel,
        middleware: {
            provideCodeLenses: async (document, token, next) => {
                const lenses = await next(document, token);
                return lenses?.map(rewriteDebugTestLensCommand);
            },
            resolveCodeLens: async (codeLens, token, next) => {
                const resolved = await next(codeLens, token);
                return rewriteDebugTestLensCommand(resolved ?? codeLens);
            },
            provideDocumentFormattingEdits: async (document, options, token, next) => {
                try {
                    return await next(document, options, token);
                } catch (err: unknown) {
                    const code = err && typeof err === 'object' && 'code' in err
                        ? (err as { code: unknown }).code
                        : undefined;
                    // Do not notify for request cancellations (code -32800)
                    if (code !== -32800) {
                        const msg = err instanceof Error ? err.message : String(err);
                        handleFormattingError(msg, outputChannel);
                    }
                    return null;
                }
            },
            provideDocumentRangeFormattingEdits: async (document, range, options, token, next) => {
                try {
                    return await next(document, range, options, token);
                } catch (err: unknown) {
                    const code = err && typeof err === 'object' && 'code' in err
                        ? (err as { code: unknown }).code
                        : undefined;
                    if (code !== -32800) {
                        const msg = err instanceof Error ? err.message : String(err);
                        handleFormattingError(msg, outputChannel);
                    }
                    return null;
                }
            },
            handleWorkDoneProgress: (token, params, next) => {
                healthWidget?.onProgress(token, params);
                next(token, params);
            },
        },
        initializationOptions: {
            disabledFeatures,
        },
    };

    const lc = new LanguageClient(
        'perl-language-server',
        'Perl Language Server',
        serverOptions,
        clientOptions
    );
    void lc.setTrace(getTraceLevel());
    return lc;
}

export function shouldNudgeArrowCompletion(linePrefix: string): boolean {
    if (!linePrefix.endsWith('-')) {
        return false;
    }

    const beforeDash = linePrefix.slice(0, -1);
    if (beforeDash.length === 0 || /\s$/.test(beforeDash) || beforeDash.endsWith(':')) {
        return false;
    }

    return /(?:\$[\w:]+|[@%][\w:]+|[A-Z]\w*)$/.test(beforeDash);
}

export function maybeNudgeArrowCompletion(event: vscode.TextDocumentChangeEvent): void {
    const editor = vscode.window.activeTextEditor;
    if (!editor || event.document !== editor.document || event.document.languageId !== 'perl') {
        return;
    }

    if (event.contentChanges.length !== 1) {
        return;
    }

    const change = event.contentChanges[0];
    if (change.rangeLength !== 0 || change.text !== '-') {
        return;
    }

    const lineText = event.document.lineAt(change.range.start.line).text;
    const linePrefix = lineText.slice(0, change.range.start.character + change.text.length);
    if (!shouldNudgeArrowCompletion(linePrefix)) {
        return;
    }

    void vscode.commands.executeCommand('editor.action.triggerSuggest');
}

/**
 * Probe the LSP binary directly and return diagnostic information.
 *
 * Runs the binary with `--version` (fast probe, 3s timeout). On failure,
 * classifies the stderr output into an actionable diagnosis.
 *
 * When execFile fails with no stderr (e.g., ENOEXEC for wrong-arch or EACCES
 * for permission denied), the OS never writes to stderr — the error code lives
 * in err.code instead.  We synthesize a recognisable string so that
 * classifyStartupError() returns the right kind rather than Unknown.
 */
async function probeStartupFailure(serverPath: string): Promise<StartupErrorDiagnosis> {
    return new Promise(resolve => {
        execFile(serverPath, ['--version'], { timeout: 3000 }, (err: Error | null, stdout: string, stderr: string) => {
            const combined = [stderr, stdout].filter(Boolean).join('\n').trim();
            if (err) {
                outputChannel.appendLine(`[startup-probe] Binary probe failed: ${err.message}`);
                if (combined) {
                    outputChannel.appendLine(`[startup-probe] stderr: ${combined}`);
                }

                // When stderr is empty, infer from the OS error code so the
                // classifier returns an actionable kind instead of Unknown.
                const errCode = (err as NodeJS.ErrnoException).code;
                let diagInput = combined;
                if (!diagInput) {
                    if (errCode === 'ENOEXEC') {
                        // Kernel refused execve — wrong ELF machine type (arch mismatch)
                        diagInput = 'cannot execute binary file: Exec format error';
                    } else if (errCode === 'EACCES') {
                        // Kernel refused execve — execute bit not set
                        diagInput = 'Permission denied';
                    } else {
                        diagInput = err.message;
                    }
                }
                resolve(classifyStartupError(diagInput));
            } else {
                // Binary responded fine — classify as unknown (client-level issue)
                resolve(classifyStartupError(''));
            }
        });
    });
}

/**
 * Run `perllsp --health` and return `true` if the binary responds with `ok`.
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

export function getLanguageServerLaunchArgs(enableLogging: boolean): string[] {
    const baseArgs = enableLogging ? ['--log'] : [];
    return getServerArgs(baseArgs);
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
    } catch (error: unknown) {
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
    if (!config.get<boolean>('enableTestIntegration', true)) {
        outputChannel.appendLine('Perl test integration disabled.');
        return;
    }

    testAdapter = new PerlTestAdapter();
    context.subscriptions.push(testAdapter);
    outputChannel.appendLine('Perl test integration enabled.');
}

/**
 * Create or dispose the streaming inline completion controller based on config.
 *
 * The controller is only active when both `aiCompletion.enabled` and
 * `aiCompletion.streaming.enabled` are true and a language client is running.
 */
function refreshStreamingController(activeClient: LanguageClient | undefined): void {
    // Always dispose any existing controller first
    if (streamingController) {
        streamingController.dispose();
        streamingController = undefined;
    }

    if (!activeClient) {
        return;
    }

    const config = vscode.workspace.getConfiguration('perl-lsp');
    const aiEnabled = config.get<boolean>('aiCompletion.enabled', false);
    const streamingEnabled = config.get<boolean>('aiCompletion.streaming.enabled', true);

    if (aiEnabled && streamingEnabled) {
        streamingController = new StreamingCompletionController(activeClient);
        outputChannel.appendLine('Streaming inline completion controller enabled.');
    }
}

async function runCheckSyntax(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        vscode.window.showErrorMessage('No active Perl file to check syntax');
        return;
    }

    if (editor.document.isDirty) {
        await editor.document.save();
    }

    const filePath = editor.document.uri.fsPath;
    const config = vscode.workspace.getConfiguration('perl-lsp');
    const includePaths: string[] = config.get('includePaths', ['lib', 'local/lib/perl5']);
    const workspaceRoot = vscode.workspace.getWorkspaceFolder(editor.document.uri)?.uri.fsPath;

    const perlArgs: string[] = [];
    for (const inc of includePaths) {
        const resolved = workspaceRoot && !path.isAbsolute(inc) ? path.join(workspaceRoot, inc) : inc;
        perlArgs.push('-I', resolved);
    }
    perlArgs.push('-c', filePath);

    return new Promise(resolve => {
        execFile('perl', perlArgs, { timeout: 10000 }, (error, stdout, stderr) => {
            const output = (stdout + stderr).trim();
            if (error) {
                vscode.window.showErrorMessage(
                    `Syntax error: ${output}`,
                    'Show Output'
                ).then(sel => {
                    if (sel === 'Show Output') {
                        outputChannel.appendLine(`[check-syntax] ${output}`);
                        outputChannel.show();
                    }
                    resolve();
                });
            } else {
                vscode.window.showInformationMessage(`Syntax OK: ${path.basename(filePath)}`).then(() => {
                    resolve();
                });
            }
        });
    });
}

/**
 * Validate configured include paths for each workspace folder and warn once
 * per workspace when a path does not exist.
 */
export async function validateIncludePaths(context: vscode.ExtensionContext): Promise<void> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return;
    }

    for (const folder of workspaceFolders) {
        const cacheKey = `perl-lsp.includePathsWarning.${encodeURIComponent(folder.uri.toString())}`;
        const config = vscode.workspace.getConfiguration('perl-lsp', folder.uri);
        const includePaths: string[] = config.get('includePaths', ['lib', 'local/lib/perl5']);
        const missingPaths = includePaths.filter(includePath => {
            const resolved = path.resolve(folder.uri.fsPath, includePath);
            return !fs.existsSync(resolved);
        });

        if (missingPaths.length === 0) {
            await context.globalState.update(cacheKey, undefined);
            continue;
        }

        const missingSignature = missingPaths.join('\n');
        const warnedSignature = context.globalState.get<string | undefined>(cacheKey);
        if (warnedSignature === missingSignature) {
            continue;
        }

        const firstMissing = missingPaths[0];
        const relativeNote = path.isAbsolute(firstMissing)
            ? 'absolute path'
            : 'relative to the workspace';
        const suffix =
            missingPaths.length > 1
                ? ` ${missingPaths.length} include paths are missing.`
                : '';

        const choice = await vscode.window.showWarningMessage(
            `Perl LSP: include path "${firstMissing}" (${relativeNote}) does not exist.${suffix}`,
            'Open Settings'
        );

        if (choice === 'Open Settings') {
            void vscode.commands.executeCommand(
                'workbench.action.openSettings',
                '@ext:EffortlessMetrics.perl-lsp-rs perl-lsp.includePaths'
            );
        }

        await context.globalState.update(cacheKey, missingSignature);
    }
}

type ExtensionPackage = {
    publisher?: string;
    name?: string;
    version?: string;
    displayName?: string;
    description?: string;
    keywords?: string[];
    contributes?: {
        languages?: Array<{ id?: string }>;
    };
};

type InstalledExtension = {
    id?: string;
    packageJSON?: ExtensionPackage;
};

function isPerlLanguageExtension(extension: InstalledExtension): boolean {
    const packageJSON = extension.packageJSON;
    if (!packageJSON) {
        return false;
    }

    if ((packageJSON.contributes?.languages ?? []).some(language => language.id === 'perl')) {
        return true;
    }

    const haystack = [
        extension.id,
        packageJSON.publisher && packageJSON.name
            ? `${packageJSON.publisher}.${packageJSON.name}`
            : undefined,
        packageJSON.displayName,
        packageJSON.name,
        packageJSON.description,
        ...(packageJSON.keywords ?? []),
    ]
        .filter((value): value is string => typeof value === 'string' && value.length > 0)
        .join(' ')
        .toLowerCase();

    return /\bperl(?:\b|[-:]|navigator|critic|tidy|lsp)/i.test(haystack);
}

/**
 * Warn once per major version when conflicting Perl extensions are installed.
 */
export async function warnAboutPerlExtensionConflicts(
    context: vscode.ExtensionContext
): Promise<void> {
    const packageJSON = context.extension.packageJSON as ExtensionPackage;
    const currentMajor = String(packageJSON.version ?? '0').split('.')[0] ?? '0';
    const warnedMajor = context.globalState.get<string>('perl-lsp.conflictWarningMajorVersion');
    if (warnedMajor === currentMajor) {
        return;
    }

    const selfId = `${packageJSON.publisher ?? 'EffortlessMetrics'}.${packageJSON.name ?? 'perl-lsp-rs'}`;
    const conflicts = (vscode.extensions.all as unknown as InstalledExtension[]).filter(extension => {
        if (!extension || extension.id === selfId) {
            return false;
        }
        return isPerlLanguageExtension(extension);
    });

    if (conflicts.length === 0) {
        return;
    }

    const names = conflicts
        .map(extension => extension.packageJSON?.displayName ?? extension.id ?? 'unknown extension')
        .slice(0, 3);
    const label = names.length === 1
        ? names[0]
        : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
    const extra = conflicts.length > names.length ? ` (+${conflicts.length - names.length} more)` : '';
    const choice = await vscode.window.showWarningMessage(
        `Perl LSP detected ${conflicts.length} other Perl extension${conflicts.length === 1 ? '' : 's'}: ${label}${extra}. These can conflict with completion, hover, diagnostics, or formatting. See the coexistence guide for details.`,
        'Open Coexistence Guide'
    );

    if (choice === 'Open Coexistence Guide') {
        await vscode.env.openExternal(vscode.Uri.parse(COEXISTENCE_GUIDE_URL));
    }

    await context.globalState.update('perl-lsp.conflictWarningMajorVersion', currentMajor);
}

async function runProveTask(name: string, args: string[], cwd?: string): Promise<void> {
    const scope = cwd
        ? vscode.workspace.getWorkspaceFolder(vscode.Uri.file(cwd)) ?? vscode.TaskScope.Global
        : vscode.TaskScope.Global;
    const execution = new vscode.ProcessExecution('prove', args, cwd ? { cwd } : undefined);
    const task = new vscode.Task(
        { type: 'perl-lsp' },
        scope,
        name,
        'perl-lsp',
        execution,
    );
    task.presentationOptions = {
        reveal: vscode.TaskRevealKind.Always,
        panel: vscode.TaskPanelKind.Shared,
        clear: false,
        showReuseMessage: false,
    };
    await vscode.tasks.executeTask(task);
}

async function runCurrentTestWithProve(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        vscode.window.showErrorMessage('No active Perl file to run');
        return;
    }

    if (editor.document.isDirty) {
        await editor.document.save();
    }

    const filePath = editor.document.uri.fsPath;
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(editor.document.uri);
    const cwd = workspaceFolder?.uri.fsPath;

    await runProveTask('Perl Tests: Current File', ['-v', filePath], cwd);
}

async function runAllTestsWithProve(): Promise<void> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const cwd = workspaceFolders[0].uri.fsPath;
    await runProveTask('Perl Tests: All', ['-r', 't/'], cwd);
}

async function showIncPaths(): Promise<void> {
    return new Promise(resolve => {
        execFile('perl', ['-e', 'print join("\\n", @INC)'], { timeout: 5000 }, (error, stdout) => {
            if (error) {
                vscode.window.showErrorMessage(
                    `Could not read Perl @INC paths: ${error.message}. ` +
                    `Make sure 'perl' is installed and on your PATH, or set perl-lsp.includePaths in settings.`
                ).then(() => {
                    resolve();
                });
                return;
            }

            const lines = stdout.trim().split('\n').filter(l => l.length > 0);
            const panel = vscode.window.createOutputChannel('Perl @INC');
            panel.clear();
            panel.appendLine('Perl @INC paths:');
            panel.appendLine('');
            for (const line of lines) {
                panel.appendLine(`  ${line}`);
            }
            panel.show();
            resolve();
        });
    });
}

async function openPerlModule(): Promise<void> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const pmFiles = await vscode.workspace.findFiles('**/*.pm', '{**/node_modules/**,**/blib/**}', 500);
    if (pmFiles.length === 0) {
        vscode.window.showInformationMessage('No .pm module files found in workspace');
        return;
    }

    const items = pmFiles.map(uri => {
        const rel = vscode.workspace.asRelativePath(uri);
        // Convert path to module name: lib/Foo/Bar.pm -> Foo::Bar
        const moduleName = rel
            .replace(/^(lib|local\/lib\/perl5)\//, '')
            .replace(/\.pm$/, '')
            .replace(/\//g, '::');
        return {
            label: moduleName,
            description: rel,
            uri
        };
    }).sort((a, b) => a.label.localeCompare(b.label));

    const selected = await vscode.window.showQuickPick(items, {
        placeHolder: 'Search Perl modules...',
        matchOnDescription: true
    });

    if (selected) {
        const doc = await vscode.workspace.openTextDocument(selected.uri);
        await vscode.window.showTextDocument(doc);
    }
}

async function showParserAst(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        vscode.window.showErrorMessage('No active Perl file to show AST');
        return;
    }

    if (!client) {
        vscode.window.showWarningMessage(serverNotRunningMessage());
        return;
    }

    try {
        const result = await client.sendRequest<string | null>(
            'perl/showAst',
            { uri: editor.document.uri.toString() }
        );

        if (!result) {
            vscode.window.showInformationMessage('No AST available for this file');
            return;
        }

        const panel = vscode.window.createOutputChannel('Perl Parser AST');
        panel.clear();
        panel.appendLine(`AST for: ${vscode.workspace.asRelativePath(editor.document.uri)}`);
        panel.appendLine('');
        panel.appendLine(result);
        panel.show();
    } catch {
        vscode.window.showWarningMessage(
            'Show Parser AST is not supported by the current perllsp version'
        );
    }
}

async function reinstallServerBinary(context: vscode.ExtensionContext) {
    outputChannel.show(true);
    outputChannel.appendLine('Reinstalling perllsp binary...');

    const downloader = new BinaryDownloader(context, outputChannel);
    const downloadedPath = await downloader.ensureBinary(true);

    if (!downloadedPath) {
        vscode.window.showErrorMessage(
            'Could not reinstall perl-lsp. Check your internet connection and proxy settings, then try again.',
            'Show Output', 'Open Settings'
        ).then(selection => {
            if (selection === 'Show Output') { outputChannel.show(); }
            if (selection === 'Open Settings') {
                void vscode.commands.executeCommand('workbench.action.openSettings', 'http.proxy');
            }
        });
        return;
    }

    currentServerPath = downloadedPath;
    const healthOk = await runHealthCheck(downloadedPath);
    if (!healthOk) {
        vscode.window.showErrorMessage(
            'The downloaded perl-lsp binary failed its health check — it may be corrupted or incompatible with your platform.',
            'Show Output', 'Report Issue'
        ).then(selection => {
            if (selection === 'Show Output') { outputChannel.show(); }
            if (selection === 'Report Issue') {
                void vscode.env.openExternal(vscode.Uri.parse('https://github.com/EffortlessMetrics/perl-lsp/issues'));
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
        // vscode-languageclient State values match ClientState numeric values:
        // Stopped = 1, Running = 2, Starting = 3
        healthWidget?.onStateChange(event.newState as unknown as ClientState);
    });
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
    if (streamingController) {
        streamingController.dispose();
        streamingController = undefined;
    }

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
        void activeClient.dispose();
    }
}
