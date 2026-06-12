import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { execFile } from 'child_process';
import { LanguageClient, TransportKind, Trace } from 'vscode-languageclient/node';
import type { LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
import { PerlTestAdapter } from './testAdapter';
import {
    activateDebugger,
    hasLaunchJson,
    rewriteTestLensCommand,
    parseDebugTestLaunchTarget,
} from './debugAdapter';
import { BinaryDownloader, parseLocalVersion } from './downloader';
import { OnboardingManager } from './onboarding';
import type { HealthCheckResult } from './onboarding';
import { WhatsNewManager } from './whatsNew';
import { generateBoilerplate } from './fileCreation';
import { handleFormattingError } from './formattingErrors';
import { HealthWidget, ClientState } from './healthWidget';
import { registerPodPreview } from './podPreview';
import { registerGherkinProviders } from './gherkinProviders';
import { registerGherkinStepDefinitionSupport } from './gherkinStepDefinitions';
import { selectTestCommandAtPosition } from './runTestAtCursor';
import { StreamingCompletionController } from './streamingCompletion';
import { registerMcpSupport } from './mcpSupport';
import {
    classifyStartupError,
    formatStartupFailureDialog,
    StartupErrorKind,
} from './startupDiagnosis';
import type { StartupErrorDiagnosis } from './startupDiagnosis';
import type {
    HealthCheckCommandResult,
    HealthCheckCommandStatus,
    ManagedBinarySource,
    ReinstallCommandResult,
} from './commandResults';

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let testAdapter: PerlTestAdapter | undefined;
let currentServerPath: string | null = null;
// Set by getServerPath() when perl-lsp.serverPath is configured but the file
// does not exist, so the "not found" error can name the broken setting instead
// of failing silently.
let configuredServerPathMissing: string | null = null;
let statusBarItem: vscode.StatusBarItem | undefined;
let healthWidget: HealthWidget | undefined;
let streamingController: StreamingCompletionController | undefined;
let stateChangeDisposable: vscode.Disposable | undefined;
const COEXISTENCE_GUIDE_URL =
    'https://github.com/EffortlessMetrics/perl-lsp/blob/master/vscode-extension/README.md#extension-coexistence';
const MANAGED_BINARY_HEALTH_TIMEOUT_MS = 30_000;
/**
 * Cached startup diagnosis from the last server failure.
 *
 * Set when the LSP fails to start (`initializeLanguageClient`) or when the
 * server stops unexpectedly mid-session (`bindClientState`). Read by
 * `serverNotRunningMessage()` so every "server not running" surface shows the
 * specific root cause (e.g. "glibc mismatch") instead of a generic hint.
 *
 * Cleared to `undefined` when the server starts successfully so a stale
 * failure from a previous session is not shown after a successful restart.
 */
let lastStartupDiagnosis: StartupErrorDiagnosis | undefined;

/**
 * Return the best available "server not running" message to show the user.
 *
 * When a startup failure has been diagnosed (`lastStartupDiagnosis` is set),
 * formats and returns the specific root cause so the user sees an actionable
 * message (e.g. "glibc mismatch — install from source") instead of a generic
 * restart prompt.
 *
 * Exported so unit tests can verify the message without a full extension host.
 */
export function serverNotRunningMessage(): string {
    if (lastStartupDiagnosis) {
        return formatStartupFailureDialog(lastStartupDiagnosis, undefined);
    }
    return 'Perl Language Server is not running. Run the Health Check (Command Palette: "Perl: Run Health Check") to diagnose the issue.';
}

/**
 * Test helper — inject a cached diagnosis without going through the full
 * startup path.  Only exported for use in unit tests.
 * @internal
 */
export function _setLastStartupDiagnosisForTest(diagnosis: StartupErrorDiagnosis | undefined): void {
    lastStartupDiagnosis = diagnosis;
}

type PerlCriticSyncSettings = {
    enabled?: boolean;
    severity?: number;
    profile?: string;
    theme?: string;
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

    const theme = inspectPerlCriticOverride(config, 'perlcritic.theme');
    if (theme?.globalValue !== undefined ||
        theme?.workspaceValue !== undefined ||
        theme?.workspaceFolderValue !== undefined) {
        settings.theme = config.get<string>('perlcritic.theme', '');
    }

    return settings;
}

function buildPerlCriticConfiguration(settings: PerlCriticSyncSettings): Record<string, unknown> | undefined {
    if (
        settings.enabled === undefined &&
        settings.severity === undefined &&
        settings.profile === undefined &&
        settings.theme === undefined
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
    return ['perlcritic.enabled', 'perlcritic.severity', 'perlcritic.profile', 'perlcritic.theme'].some(key => {
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
        await activeClient.sendNotification('workspace/didChangeConfiguration', payload);
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
        await activeClient.sendNotification('workspace/didChangeConfiguration', payload);
    }

    vscode.window.showInformationMessage(`PerlCritic severity set to ${severity}.`);
}

type LspExecuteCommandClient = {
    sendRequest<T>(method: string, params: unknown): Promise<T>;
};

type ProviderDecisionQuickPickItem = vscode.QuickPickItem & {
    provider: string;
};

const PROVIDER_DECISION_OPTIONS: ProviderDecisionQuickPickItem[] = [
    { label: 'Completion', provider: 'completion' },
    { label: 'Goto definition', provider: 'goto_definition' },
    { label: 'References', provider: 'references' },
    { label: 'Hover', provider: 'hover' },
    { label: 'Diagnostics', provider: 'diagnostics' },
    { label: 'Rename', provider: 'rename' },
    { label: 'Safe delete', provider: 'safe_delete' },
    { label: 'Workspace symbols', provider: 'workspace_symbols' },
    { label: 'Document symbols', provider: 'document_symbols' },
    { label: 'Semantic tokens', provider: 'semantic_tokens' },
    { label: 'Module resolution', provider: 'module_resolution' },
    { label: 'DAP module paths', provider: 'dap_module_paths' },
    { label: 'Perl subprocess', provider: 'perl_subprocess' },
];

function activeRequestPosition(): Record<string, unknown> | undefined {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        return undefined;
    }

    return {
        uri_scheme: editor.document.uri.toString().split(':', 1)[0] || 'file',
        line: editor.selection.active.line,
        character: editor.selection.active.character,
    };
}

function providerDecisionArgument(provider: string): Record<string, unknown> {
    const argument: Record<string, unknown> = { provider };
    const requestPosition = activeRequestPosition();
    if (requestPosition) {
        argument.request_position = requestPosition;
    }
    return argument;
}

function activeSafeDeletePreviewArgument(): Record<string, unknown> | undefined {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        return undefined;
    }

    return {
        textDocument: { uri: editor.document.uri.toString() },
        position: {
            line: editor.selection.active.line,
            character: editor.selection.active.character,
        },
    };
}

async function activePackageRenamePreviewArgument(): Promise<Record<string, unknown> | undefined> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        return undefined;
    }

    const selectedText = editor.selection.isEmpty
        ? ''
        : editor.document.getText(editor.selection).trim();
    const wordRange = editor.document.getWordRangeAtPosition(editor.selection.active);
    const wordText = wordRange ? editor.document.getText(wordRange).trim() : '';
    const defaultValue = selectedText || wordText;

    const newName = await vscode.window.showInputBox({
        value: defaultValue,
        placeHolder: 'renamed_symbol',
        prompt: 'New package or symbol name for the no-edit rename preview',
        validateInput: value => {
            const trimmed = value.trim();
            if (!trimmed) {
                return 'Enter a new Perl package or symbol name.';
            }
            return isPerlModuleName(trimmed)
                ? undefined
                : 'Use a single Perl package or symbol name.';
        },
    });
    if (!newName) {
        return undefined;
    }

    return {
        textDocument: { uri: editor.document.uri.toString() },
        position: {
            line: editor.selection.active.line,
            character: editor.selection.active.character,
        },
        newName: newName.trim(),
    };
}

function isPerlModuleName(value: string): boolean {
    return /^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*$/.test(value);
}

function moduleNameAtCursor(editor: vscode.TextEditor): string | undefined {
    const selectedText = editor.document.getText(editor.selection).trim();
    if (isPerlModuleName(selectedText)) {
        return selectedText;
    }

    const active = editor.selection.active;
    const lineText = editor.document.lineAt(active.line).text;
    const modulePattern = /[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+/g;
    for (const match of lineText.matchAll(modulePattern)) {
        const start = match.index ?? 0;
        const end = start + match[0].length;
        if (active.character >= start && active.character <= end) {
            return match[0];
        }
    }

    const useStatement = lineText.match(
        /\b(?:use|require)\s+([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)/
    );
    return useStatement?.[1];
}

function trustOutputChannel(): vscode.OutputChannel {
    return outputChannel ?? vscode.window.createOutputChannel('Perl LSP Trust');
}

function asObject(value: unknown): Record<string, unknown> | undefined {
    return value && typeof value === 'object' && !Array.isArray(value)
        ? value as Record<string, unknown>
        : undefined;
}

function stringField(value: Record<string, unknown> | undefined, field: string): string | undefined {
    const fieldValue = value?.[field];
    return typeof fieldValue === 'string' ? fieldValue : undefined;
}

function numberField(value: Record<string, unknown> | undefined, field: string): number | undefined {
    const fieldValue = value?.[field];
    return typeof fieldValue === 'number' ? fieldValue : undefined;
}

function booleanField(value: Record<string, unknown> | undefined, field: string): boolean | undefined {
    const fieldValue = value?.[field];
    return typeof fieldValue === 'boolean' ? fieldValue : undefined;
}

function arrayField(value: Record<string, unknown> | undefined, field: string): unknown[] {
    const fieldValue = value?.[field];
    return Array.isArray(fieldValue) ? fieldValue : [];
}

function providerDecisionJson(value: unknown): string {
    return JSON.stringify(value, null, 2);
}

function incrementCount(target: Record<string, number>, key: string): void {
    target[key] = (target[key] ?? 0) + 1;
}

function classifyLaunchPath(value: string): string {
    const trimmed = value.trim();
    if (!trimmed) {
        return 'empty';
    }
    if (trimmed.includes('${workspaceFolder')) {
        return 'workspace_variable';
    }
    if (trimmed.includes('${file')) {
        return 'file_variable';
    }
    if (/\$\{[^}]+}/.test(trimmed)) {
        return 'other_variable';
    }
    if (trimmed.startsWith('~')) {
        return 'home_relative';
    }
    if (path.isAbsolute(trimmed)) {
        return 'absolute';
    }
    if (!trimmed.includes('/') && !trimmed.includes('\\')) {
        return 'command';
    }
    return 'relative';
}

function collectLaunchConfigurationState(workspaceFolders: readonly vscode.WorkspaceFolder[]): Record<string, unknown> {
    const includePathKindCounts: Record<string, number> = {};
    const perlPathKindCounts: Record<string, number> = {};
    const programPathKindCounts: Record<string, number> = {};
    const cwdPathKindCounts: Record<string, number> = {};
    let configurationCount = 0;
    let perlConfigurationCount = 0;
    let launchRequestCount = 0;
    let attachRequestCount = 0;
    let perlPathConfiguredCount = 0;
    let includePathsConfiguredCount = 0;
    let includePathEntryCount = 0;
    let nonStringIncludePathCount = 0;
    let programConfiguredCount = 0;
    let cwdConfiguredCount = 0;

    for (const folder of workspaceFolders) {
        const launchConfigurations = vscode.workspace
            .getConfiguration('launch', folder.uri)
            .get<unknown[]>('configurations', []);
        if (!Array.isArray(launchConfigurations)) {
            continue;
        }

        for (const entry of launchConfigurations) {
            if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
                continue;
            }

            configurationCount += 1;
            const config = entry as Record<string, unknown>;
            if (config.type !== 'perl') {
                continue;
            }

            perlConfigurationCount += 1;
            if (config.request === 'launch') {
                launchRequestCount += 1;
            } else if (config.request === 'attach') {
                attachRequestCount += 1;
            }

            if (typeof config.perlPath === 'string') {
                perlPathConfiguredCount += 1;
                incrementCount(perlPathKindCounts, classifyLaunchPath(config.perlPath));
            }

            if (typeof config.program === 'string') {
                programConfiguredCount += 1;
                incrementCount(programPathKindCounts, classifyLaunchPath(config.program));
            }

            if (typeof config.cwd === 'string') {
                cwdConfiguredCount += 1;
                incrementCount(cwdPathKindCounts, classifyLaunchPath(config.cwd));
            }

            if (Array.isArray(config.includePaths)) {
                includePathsConfiguredCount += 1;
                for (const includePath of config.includePaths) {
                    if (typeof includePath !== 'string') {
                        nonStringIncludePathCount += 1;
                        continue;
                    }
                    includePathEntryCount += 1;
                    incrementCount(includePathKindCounts, classifyLaunchPath(includePath));
                }
            }
        }
    }

    return {
        status: 'client_launch_config_reported',
        configuration_count: configurationCount,
        perl_configuration_count: perlConfigurationCount,
        launch_request_count: launchRequestCount,
        attach_request_count: attachRequestCount,
        perl_path_configured_count: perlPathConfiguredCount,
        include_paths_configured_count: includePathsConfiguredCount,
        include_path_entry_count: includePathEntryCount,
        non_string_include_path_count: nonStringIncludePathCount,
        program_configured_count: programConfiguredCount,
        cwd_configured_count: cwdConfiguredCount,
        include_path_kind_counts: includePathKindCounts,
        perl_path_kind_counts: perlPathKindCounts,
        program_path_kind_counts: programPathKindCounts,
        cwd_path_kind_counts: cwdPathKindCounts,
        claim_boundary: 'Launch configuration state is summarized from VS Code configuration only. It reports counts and path classes, not raw paths, and does not start DAP, resolve Perl, probe modules, or change debug behavior.',
    };
}

export function workspaceTrustClientRuntimeState(
    context?: vscode.ExtensionContext
): Record<string, unknown> {
    const workspaceFolders = vscode.workspace.workspaceFolders ?? [];
    const managedDapPath = context ? BinaryDownloader.getLocalDapPath(context) : undefined;
    const managedAdapterExists = managedDapPath ? fs.existsSync(managedDapPath) : false;
    const launchJsonWorkspaceCount = workspaceFolders.filter(folder => hasLaunchJson(folder.uri.fsPath)).length;
    const activeDebugSession = vscode.debug.activeDebugSession;

    return {
        schema_version: 'workspace_trust_client_runtime.v1',
        source: 'vscode-extension',
        perldoc: {
            status: 'client_surface_registered',
            uri_scheme: 'perldoc',
            client_surface: 'perldoc virtual documents are served by the LSP textDocumentContent path',
        },
        dap: {
            status: 'client_state_reported',
            adapter_registered: true,
            active_perl_debug_session: activeDebugSession?.type === 'perl',
            managed_adapter_exists: managedAdapterExists,
            launch_json_workspace_count: launchJsonWorkspaceCount,
            workspace_folder_count: workspaceFolders.length,
            launch_configuration: collectLaunchConfigurationState(workspaceFolders),
        },
        claim_boundary: 'VS Code client runtime state reads extension/debugger state only. It does not start DAP, run perldoc, probe Perl, or change provider behavior.',
    };
}

async function executeLspCommand(
    activeClient: LspExecuteCommandClient | undefined,
    command: string,
    argument?: Record<string, unknown>
): Promise<unknown | undefined> {
    if (!activeClient) {
        vscode.window.showWarningMessage(serverNotRunningMessage());
        return undefined;
    }

    try {
        return await activeClient.sendRequest('workspace/executeCommand', {
            command,
            arguments: argument === undefined ? [] : [argument],
        });
    } catch (error: unknown) {
        const message = error instanceof Error ? error.message : String(error);
        vscode.window.showErrorMessage(`Perl LSP command failed: ${message}`);
        return undefined;
    }
}

async function chooseProvider(providerOverride?: string): Promise<string | undefined> {
    if (providerOverride) {
        return providerOverride;
    }

    const selection = await vscode.window.showQuickPick(PROVIDER_DECISION_OPTIONS, {
        placeHolder: 'Choose a provider decision to explain',
    });
    return selection?.provider;
}

async function showProviderDecisionResult(title: string, result: unknown): Promise<void> {
    const resultObject = asObject(result);
    const message = stringField(resultObject, 'user_message') ?? `${title} completed.`;
    const channel = trustOutputChannel();
    channel.appendLine('');
    channel.appendLine(`[${title}]`);
    channel.appendLine(providerDecisionJson(result));

    const action = stringField(resultObject, 'decision') === 'blocked'
        ? await vscode.window.showWarningMessage(message, 'Show Output')
        : await vscode.window.showInformationMessage(message, 'Show Output');

    if (action === 'Show Output') {
        channel.show();
    }
}

function appendSupportTierLines(lines: string[], supportTiers: Record<string, unknown> | undefined): void {
    if (!supportTiers) {
        lines.push('- support tiers: unavailable');
        return;
    }

    for (const [surface, tier] of Object.entries(supportTiers).sort(([left], [right]) => left.localeCompare(right))) {
        if (typeof tier === 'string') {
            lines.push(`- ${surface}: ${tier}`);
        }
    }
}

function formatWorkspaceTrustReport(result: unknown): string {
    const report = asObject(result);
    const workspace = asObject(report?.workspace);
    const moduleResolution = asObject(report?.module_resolution);
    const globalConfig = asObject(moduleResolution?.global_workspace_config);
    const index = asObject(report?.index);
    const providers = asObject(report?.providers);
    const supportTiers = asObject(providers?.support_tiers);
    const dynamicBoundaries = asObject(report?.dynamic_boundaries);
    const setupHints = asObject(report?.setup_hints);
    const clientRuntime = asObject(report?.client_runtime_state);
    const setupHintItems = arrayField(setupHints, 'hints');
    const perlBinary = asObject(setupHints?.perl_binary);
    const perldoc = asObject(setupHints?.perldoc);
    const dap = asObject(setupHints?.dap);
    const clientPerldoc = asObject(clientRuntime?.perldoc);
    const clientDap = asObject(clientRuntime?.dap);
    const launchConfiguration = asObject(clientDap?.launch_configuration);

    const rootPath = stringField(workspace, 'root_path') ?? '(none)';
    const folderCount = numberField(workspace, 'workspace_folder_count') ?? 0;
    const openDocumentCount = numberField(workspace, 'open_document_count') ?? 0;
    const includePaths = arrayField(globalConfig, 'include_paths');
    const effectiveIncludePaths = arrayField(globalConfig, 'effective_include_paths');
    const usePerl5lib = booleanField(globalConfig, 'use_perl5lib');
    const perl5libCount = numberField(globalConfig, 'perl5lib_entry_count') ?? 0;
    const indexState = stringField(index, 'state') ?? 'unknown';
    const indexAvailability = stringField(index, 'availability') ?? 'unknown';
    const indexedFileCount = numberField(index, 'indexed_file_count') ?? 0;
    const indexedSymbolCount = numberField(index, 'indexed_symbol_count') ?? 0;
    const traceCount = numberField(providers, 'decision_trace_count') ?? 0;

    const lines = [
        'Perl LSP Trust Report',
        '',
        `Schema: ${stringField(report, 'schema_version') ?? 'unknown'}`,
        `Root: ${rootPath}`,
        `Workspace folders: ${folderCount}`,
        `Open documents: ${openDocumentCount}`,
        '',
        'Module resolution / @INC',
        `- configured include paths: ${includePaths.length}`,
        `- effective include paths: ${effectiveIncludePaths.length}`,
        `- system @INC: ${stringField(globalConfig, 'system_inc_status') ?? 'unknown'}`,
        `- PERL5LIB enabled: ${usePerl5lib === undefined ? 'unknown' : String(usePerl5lib)}`,
        `- PERL5LIB entries: ${perl5libCount}`,
        `- perl.path: ${stringField(globalConfig, 'perl_path') ?? '(unconfigured)'}`,
        '',
        'Setup hints',
        `- status: ${stringField(setupHints, 'status') ?? 'unknown'}`,
        `- hint count: ${numberField(setupHints, 'hint_count') ?? setupHintItems.length}`,
        `- Perl binary: ${stringField(perlBinary, 'resolution_status') ?? 'unknown'}`,
        `- Perl version: ${stringField(perlBinary, 'version_status') ?? 'unknown'}`,
        `- perldoc: ${stringField(perldoc, 'status') ?? 'unknown'}`,
        `- DAP Perl: ${stringField(dap, 'status') ?? 'unknown'}`,
        '',
        'Client runtime state',
        `- source: ${stringField(clientRuntime, 'source') ?? 'unknown'}`,
        `- perldoc surface: ${stringField(clientPerldoc, 'status') ?? 'unknown'}`,
        `- DAP adapter: ${stringField(clientDap, 'status') ?? 'unknown'}`,
        `- DAP managed adapter exists: ${String(booleanField(clientDap, 'managed_adapter_exists') ?? false)}`,
        `- DAP active Perl session: ${String(booleanField(clientDap, 'active_perl_debug_session') ?? false)}`,
        `- DAP launch.json workspaces: ${numberField(clientDap, 'launch_json_workspace_count') ?? 0}`,
        `- DAP launch configs: ${numberField(launchConfiguration, 'configuration_count') ?? 0}`,
        `- DAP Perl configs: ${numberField(launchConfiguration, 'perl_configuration_count') ?? 0}`,
        `- DAP includePaths configs: ${numberField(launchConfiguration, 'include_paths_configured_count') ?? 0}`,
        `- DAP includePaths entries: ${numberField(launchConfiguration, 'include_path_entry_count') ?? 0}`,
        `- DAP perlPath configs: ${numberField(launchConfiguration, 'perl_path_configured_count') ?? 0}`,
    ];

    for (const item of setupHintItems.slice(0, 5)) {
        const hint = asObject(item);
        if (!hint) {
            continue;
        }
        const severity = stringField(hint, 'severity') ?? 'info';
        const message = stringField(hint, 'message') ?? 'Setup hint did not include a message.';
        lines.push(`- ${severity}: ${message}`);
        const action = stringField(hint, 'action');
        if (action) {
            lines.push(`  action: ${action}`);
        }
    }

    const setupBoundary = stringField(setupHints, 'claim_boundary');
    if (setupBoundary) {
        lines.push(`- boundary: ${setupBoundary}`);
    }
    const launchConfigBoundary = stringField(launchConfiguration, 'claim_boundary');
    if (launchConfigBoundary) {
        lines.push(`- launch config boundary: ${launchConfigBoundary}`);
    }

    lines.push(
        '',
        'Index',
        `- state: ${indexState}`,
        `- availability: ${indexAvailability}`,
        `- indexed files: ${indexedFileCount}`,
        `- indexed symbols: ${indexedSymbolCount}`,
        '',
        'Provider support tiers',
    );
    appendSupportTierLines(lines, supportTiers);

    lines.push(
        '',
        'Provider decision traces',
        `- persisted trace keys: ${traceCount}`,
        '',
        'Dynamic boundaries',
        `- ${stringField(dynamicBoundaries, 'policy') ?? 'Generated, dynamic, stale, low-confidence, ambiguous, and fallback facts remain bounded by provider policy.'}`,
        '',
        'Claim boundary',
        stringField(report, 'claim_boundary') ?? 'This report is bounded to current runtime state.',
        '',
        'Raw report JSON',
        providerDecisionJson(result),
    );

    return lines.join('\n');
}

function formatMissingModuleLookup(result: unknown): string {
    const report = asObject(result);
    const moduleResolution = asObject(report?.module_resolution);
    const lookupResult = asObject(moduleResolution?.result);
    const includePaths = arrayField(moduleResolution, 'effective_include_paths');

    const lines = [
        'Perl LSP Missing Module Lookup',
        '',
        `Module: ${stringField(report, 'requested_module') ?? '(unknown)'}`,
        `Expected path: ${stringField(report, 'expected_relative_path') ?? '(unknown)'}`,
        `Result: ${stringField(lookupResult, 'status') ?? 'unknown'}`,
        `Why: ${stringField(lookupResult, 'why') ?? 'No lookup reason reported.'}`,
        '',
        'Module resolution / @INC',
        `- PERL5LIB policy: ${stringField(moduleResolution, 'perl5lib_policy') ?? 'unknown'}`,
        `- system @INC enabled: ${String(booleanField(moduleResolution, 'use_system_inc') ?? false)}`,
        `- include roots: ${includePaths.length}`,
    ];

    for (const entry of includePaths.slice(0, 8)) {
        const root = asObject(entry);
        if (!root) {
            continue;
        }

        const source = stringField(root, 'source') ?? 'unknown source';
        const kind = stringField(root, 'kind') ?? 'unknown kind';
        lines.push(`- ${stringField(root, 'path') ?? '(unknown path)'} (${source}, ${kind})`);

        for (const candidate of arrayField(root, 'candidate_paths').slice(0, 2)) {
            const candidateObject = asObject(candidate);
            if (!candidateObject) {
                continue;
            }
            const exists = booleanField(candidateObject, 'exists') === true ? 'exists' : 'missing';
            lines.push(`  candidate: ${stringField(candidateObject, 'path') ?? '(unknown)'} [${exists}]`);
        }
    }

    if (includePaths.length > 8) {
        lines.push(`- ... and ${includePaths.length - 8} more include roots`);
    }

    lines.push(
        '',
        'Claim boundary',
        stringField(report, 'claim_boundary') ?? 'This explanation is bounded to current runtime state.',
        '',
        'Raw lookup JSON',
        providerDecisionJson(result),
    );

    return lines.join('\n');
}

export async function showWorkspaceTrustReportCommand(
    activeClient: LspExecuteCommandClient | undefined = client,
    clientRuntimeState: () => Record<string, unknown> = workspaceTrustClientRuntimeState
): Promise<void> {
    const result = await executeLspCommand(activeClient, 'perl.workspaceTrustReport', {
        client_runtime_state: clientRuntimeState(),
    });
    if (result === undefined) {
        return;
    }

    const channel = trustOutputChannel();
    channel.appendLine('');
    channel.appendLine(formatWorkspaceTrustReport(result));
    channel.show();
}

export async function explainMissingModuleLookupCommand(
    activeClient: LspExecuteCommandClient | undefined = client,
    moduleOverride?: string
): Promise<unknown | undefined> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        vscode.window.showErrorMessage('Explain Missing Module Lookup requires an active Perl file.');
        return undefined;
    }

    const moduleName = moduleOverride ?? moduleNameAtCursor(editor) ?? await vscode.window.showInputBox({
        placeHolder: 'Missing::Module',
        prompt: 'Module name to explain with perl-lsp @INC lookup state',
        validateInput: value => {
            if (!value.trim()) {
                return 'Enter a Perl module name.';
            }
            return isPerlModuleName(value.trim()) ? undefined : 'Enter a valid Perl module name.';
        },
    });
    if (!moduleName) {
        return undefined;
    }

    const result = await executeLspCommand(activeClient, 'perl.explainMissingModuleLookup', {
        module: moduleName.trim(),
        textDocument: { uri: editor.document.uri.toString() },
        position: {
            line: editor.selection.active.line,
            character: editor.selection.active.character,
        },
    });
    if (result === undefined) {
        return undefined;
    }

    const resultObject = asObject(result);
    const message = stringField(resultObject, 'user_message') ?? 'Missing-module lookup explained.';
    const status = stringField(asObject(asObject(resultObject?.module_resolution)?.result), 'status');
    const channel = trustOutputChannel();
    channel.appendLine('');
    channel.appendLine(formatMissingModuleLookup(result));

    const action = status === 'resolved'
        ? await vscode.window.showInformationMessage(message, 'Show Output')
        : await vscode.window.showWarningMessage(message, 'Show Output');

    if (action === 'Show Output') {
        channel.show();
    }

    return result;
}

export async function explainProviderDecisionCommand(
    activeClient: LspExecuteCommandClient | undefined = client,
    providerOverride?: string
): Promise<void> {
    const provider = await chooseProvider(providerOverride);
    if (!provider) {
        return;
    }

    const result = await executeLspCommand(
        activeClient,
        'perl.explainProviderDecision',
        providerDecisionArgument(provider)
    );
    if (result !== undefined) {
        await showProviderDecisionResult('Provider decision explanation', result);
    }
}

export async function explainDiagnosticCommand(
    activeClient: LspExecuteCommandClient | undefined = client,
    request?: unknown
): Promise<void> {
    const requestObject = asObject(request);
    const argument: Record<string, unknown> = requestObject
        ? { ...requestObject }
        : providerDecisionArgument('diagnostics');

    if (typeof argument.provider !== 'string') {
        argument.provider = 'diagnostics';
    }

    const result = await executeLspCommand(
        activeClient,
        'perl.explainProviderDecision',
        argument
    );
    if (result !== undefined) {
        await showProviderDecisionResult('Diagnostic explanation', result);
    }
}

export async function previewSafeDeleteCommand(
    activeClient: LspExecuteCommandClient | undefined = client
): Promise<void> {
    const argument = activeSafeDeletePreviewArgument();
    if (!argument) {
        vscode.window.showErrorMessage('Preview Safe Delete requires an active Perl file.');
        return;
    }

    const result = await executeLspCommand(activeClient, 'perl.previewSafeDelete', argument);
    if (result !== undefined) {
        await showProviderDecisionResult('Safe-delete preview', result);
    }
}

export async function previewPackageRenameCommand(
    activeClient: LspExecuteCommandClient | undefined = client
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'perl') {
        vscode.window.showErrorMessage('Preview Package Rename requires an active Perl file.');
        return;
    }

    const argument = await activePackageRenamePreviewArgument();
    if (!argument) {
        return;
    }

    const result = await executeLspCommand(activeClient, 'perl.previewPackageRename', argument);
    if (result !== undefined) {
        await showProviderDecisionResult('Package rename preview', result);
    }
}

export async function copyProviderDecisionReceiptCommand(
    activeClient: LspExecuteCommandClient | undefined = client,
    providerOverride?: string
): Promise<void> {
    const provider = await chooseProvider(providerOverride);
    if (!provider) {
        return;
    }

    const result = await executeLspCommand(
        activeClient,
        'perl.explainProviderDecision',
        providerDecisionArgument(provider)
    );
    const resultObject = asObject(result);
    if (!resultObject) {
        return;
    }

    const payload = resultObject.copyable_payload ?? resultObject;
    await vscode.env.clipboard.writeText(providerDecisionJson(payload));
    vscode.window.showInformationMessage('Provider decision receipt copied.');
}

export async function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.window.createOutputChannel('Perl Language Server');
    const mcpDisposable = registerMcpSupport(outputChannel);
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
        return reinstallServerBinary(context);
    });

    // Register commands
    const restartCommand = vscode.commands.registerCommand('perl-lsp.restart', async () => {
        await restartServer(context);
    });

    const organizeImportsCommand = vscode.commands.registerCommand('perl-lsp.organizeImports', async () => {
        await vscode.commands.executeCommand('editor.action.organizeImports');
    });

    const runTestsCommand = vscode.commands.registerCommand('perl-lsp.runTests', async (test?: unknown) => {
        let targetUri: vscode.Uri | undefined;

        if (test) {
            const target = parseDebugTestLaunchTarget(test);
            if (target && target.program) {
                targetUri = vscode.Uri.file(target.program);
            }
        }

        if (!targetUri) {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'perl') {
                vscode.window.showErrorMessage('No active Perl file to test');
                return;
            }
            targetUri = editor.document.uri;
        }

        // Restrict to test files (.t, .pl) - .pm files are modules, not test scripts
        const filePath = targetUri.fsPath;
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
                await testAdapter.runFileTests(targetUri);
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
        const commandResult = toHealthCheckCommandResult(results);

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

        return commandResult;
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

    const explainProviderDecisionCommandDisposable = vscode.commands.registerCommand(
        'perl-lsp.explainProviderDecision',
        async (provider?: unknown) => {
            await explainProviderDecisionCommand(client, typeof provider === 'string' ? provider : undefined);
        }
    );

    const previewSafeDeleteCommandDisposable = vscode.commands.registerCommand(
        'perl-lsp.previewSafeDelete',
        async () => {
            await previewSafeDeleteCommand(client);
        }
    );

    const previewPackageRenameCommandDisposable = vscode.commands.registerCommand(
        'perl-lsp.previewPackageRename',
        async () => {
            await previewPackageRenameCommand(client);
        }
    );

    const copyProviderDecisionReceiptCommandDisposable = vscode.commands.registerCommand(
        'perl-lsp.copyProviderDecisionReceipt',
        async (provider?: unknown) => {
            await copyProviderDecisionReceiptCommand(client, typeof provider === 'string' ? provider : undefined);
        }
    );

    const showWorkspaceTrustReportCommandDisposable = vscode.commands.registerCommand(
        'perl-lsp.showWorkspaceTrustReport',
        async () => {
            await showWorkspaceTrustReportCommand(client, () => workspaceTrustClientRuntimeState(context));
        }
    );

    const explainMissingModuleLookupCommandDisposable = vscode.commands.registerCommand(
        'perl-lsp.explainMissingModuleLookup',
        async (moduleName?: unknown) => {
            await explainMissingModuleLookupCommand(
                client,
                typeof moduleName === 'string' ? moduleName : undefined
            );
        }
    );

    const explainDiagnosticCommandDisposable = vscode.commands.registerCommand(
        'perl-lsp.explainDiagnostic',
        async (request?: unknown) => {
            await explainDiagnosticCommand(client, request);
        }
    );

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
        const editorVersion = vscode.version;
        const editorName = (vscode.env as unknown as { appName?: string }).appName;
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

        const diagnosticInfo = formatIssueDiagnosticInfo({
            serverVersion,
            extensionVersion,
            editorVersion,
            platform,
            arch,
            editorName,
        });

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
        explainProviderDecisionCommandDisposable,
        previewSafeDeleteCommandDisposable,
        previewPackageRenameCommandDisposable,
        copyProviderDecisionReceiptCommandDisposable,
        showWorkspaceTrustReportCommandDisposable,
        explainMissingModuleLookupCommandDisposable,
        explainDiagnosticCommandDisposable,
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
        ...(mcpDisposable ? [mcpDisposable] : []),
        ...registerGherkinProviders(),
        ...registerGherkinStepDefinitionSupport(),
        ...registerPodPreview(context),
    );

    // Initialize debug adapter
    activateDebugger(context);

    if (
        context.extensionMode === vscode.ExtensionMode.Test &&
        process.env.PERL_LSP_EXTENSION_TEST_SKIP_STARTUP === '1'
    ) {
        outputChannel.appendLine('[extension-test] Skipping automatic server startup.');
        return;
    }

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

/**
 * If `userPath` (the configured perl-lsp.serverPath) is set but does not exist,
 * log a diagnostic and return it so callers can surface an actionable error —
 * instead of silently falling through to PATH/bundled and leaving the user with
 * a generic "not found" that gives no hint their setting was the cause. Returns
 * null when no configured path was rejected. Exported for testing.
 */
export function diagnoseConfiguredServerPath(
    userPath: string | undefined,
    pathExists: boolean,
    channel: vscode.OutputChannel,
): string | null {
    if (!userPath || pathExists) {
        return null;
    }
    channel.appendLine(
        `[startup] perl-lsp.serverPath is configured but does not exist: ${userPath}. ` +
        `Falling back to PATH/bundled binary (or auto-download).`,
    );
    return userPath;
}

async function getServerPath(context: vscode.ExtensionContext): Promise<string | null> {
    // First check user settings
    const config = vscode.workspace.getConfiguration('perl-lsp');
    const userPath = config.get<string>('serverPath');
    const userPathExists = userPath ? fs.existsSync(userPath) : false;

    if (userPath && userPathExists) {
        outputChannel.appendLine(`Using user-configured Perl LSP binary: ${userPath}`);
        configuredServerPathMissing = null;
        return userPath;
    }

    configuredServerPathMissing = diagnoseConfiguredServerPath(userPath, userPathExists, outputChannel);

    const platform = process.platform;
    const arch = process.arch;
    const binaryNames = platform === 'win32'
        ? ['perllsp.exe', 'perl-lsp.exe']
        : ['perllsp', 'perl-lsp'];

    const findInPath = (): string | null => {
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
        return null;
    };

    const findBundled = (): string | null => {
        for (const binaryName of binaryNames) {
            const bundledPath = path.join(
                context.extensionPath,
                'bin',
                `${platform}-${arch}`,
                binaryName
            );

            if (!fs.existsSync(bundledPath)) {
                continue;
            }

            outputChannel.appendLine(`Using bundled Perl LSP binary: ${bundledPath}`);
            if (platform !== 'win32') {
                try {
                    fs.chmodSync(bundledPath, 0o755);
                } catch (chmodError: unknown) {
                    const msg = chmodError instanceof Error ? chmodError.message : String(chmodError);
                    outputChannel.appendLine(
                        `[startup] Could not update executable permissions for bundled binary: ${msg}`
                    );
                }
            }
            return bundledPath;
        }
        return null;
    };

    // Firebase Studio (and IDX) run in remote containers where extension install
    // paths may be mounted read-only or noexec. Prefer PATH there so users can
    // provide perllsp from their workspace/toolchain.
    const remoteName = vscode.env.remoteName?.toLowerCase() ?? '';
    const preferPathBeforeBundled = remoteName.includes('firebase') || remoteName.includes('idx');
    if (preferPathBeforeBundled) {
        const pathCandidate = findInPath();
        if (pathCandidate) {
            return pathCandidate;
        }
    }

    const bundledCandidate = findBundled();
    if (bundledCandidate) {
        return bundledCandidate;
    }

    const pathCandidate = findInPath();
    if (pathCandidate) {
        return pathCandidate;
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
        const notFoundMessage = configuredServerPathMissing
            ? `Perl Language Server not found: your perl-lsp.serverPath points to "${configuredServerPathMissing}", which does not exist. Fix the path or clear the setting to auto-download.`
            : 'Perl Language Server (perllsp) not found.';
        const choice = await vscode.window.showErrorMessage(
            notFoundMessage,
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
        // lastStartupDiagnosis is updated so that serverNotRunningMessage() in
        // command handlers surfaces the specific root cause rather than a generic prompt.
        const probeResult = currentServerPath
            ? await probeStartupFailure(currentServerPath)
            : classifyStartupError('');
        let healthMsg: string | undefined;
        if (probeResult.kind === StartupErrorKind.Unknown) {
            const onboarding = new OnboardingManager(context, outputChannel);
            healthMsg = await onboarding.runStartupDiagnostics(currentServerPath ?? null);
        }
        // Cache the structured diagnosis so serverNotRunningMessage() can format
        // it; when healthMsg overrides the hint, wrap it as a synthetic diagnosis.
        lastStartupDiagnosis = healthMsg && probeResult.kind === StartupErrorKind.Unknown
            ? { kind: StartupErrorKind.Unknown, hint: healthMsg, remediation: probeResult.remediation }
            : probeResult;
        const dialogMessage = formatStartupFailureDialog(probeResult, healthMsg);

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

    // Clear any stale startup diagnosis — the server started successfully so
    // the root cause (e.g. missing Perl) no longer applies.
    lastStartupDiagnosis = undefined;
    outputChannel.appendLine('Perl Language Server started successfully');
    return true;
}

export function buildDisabledFeaturesFromConfig(
    config: { get<T>(key: string, defaultValue: T): T }
): string[] {
    const base = config.get<string[]>('disabledFeatures', []).slice();
    if (!config.get<boolean>('enableSemanticTokens', true) && !base.includes('lsp.semantic_tokens')) {
        base.push('lsp.semantic_tokens');
    }
    if (!config.get<boolean>('enableFormatting', true) && !base.includes('lsp.formatting')) {
        base.push('lsp.formatting');
    }
    return base;
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

    const disabledFeatures = buildDisabledFeaturesFromConfig(
        vscode.workspace.getConfiguration('perl-lsp')
    );

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
                return lenses?.map(rewriteTestLensCommand);
            },
            resolveCodeLens: async (codeLens, token, next) => {
                const resolved = await next(codeLens, token);
                return rewriteTestLensCommand(resolved ?? codeLens);
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
 * Waits up to 30 seconds. Returns `false` on timeout, non-zero exit, or if
 * stdout does not start with `ok`.
 */
async function runHealthCheck(serverPath: string): Promise<boolean> {
    return new Promise(resolve => {
        execFile(
            serverPath,
            ['--health'],
            { timeout: MANAGED_BINARY_HEALTH_TIMEOUT_MS },
            (err: Error | null, stdout: string, stderr: string) => {
                if (err) {
                    outputChannel.appendLine(`[health-check] Failed: ${err.message}`);
                    const stderrText = stderr.trim();
                    if (stderrText) {
                        outputChannel.appendLine(`[health-check] stderr: ${stderrText}`);
                    }
                    const stdoutText = stdout.trim();
                    if (stdoutText) {
                        outputChannel.appendLine(`[health-check] stdout: ${stdoutText}`);
                    }
                    resolve(false);
                    return;
                }
                const ok = stdout.trim().startsWith('ok');
                if (!ok) {
                    outputChannel.appendLine(`[health-check] Unexpected output: ${stdout.trim()}`);
                }
                resolve(ok);
            }
        );
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

export function formatIssueDiagnosticInfo(params: {
    serverVersion: string;
    extensionVersion: string;
    editorVersion: string;
    platform: string;
    arch: string;
    editorName?: string;
}): string {
    const editorName = (params.editorName ?? 'VS Code').trim() || 'VS Code';
    return [
        `perl-lsp server: ${params.serverVersion}`,
        `Extension: ${params.extensionVersion}`,
        `${editorName}: ${params.editorVersion}`,
        `Platform: ${params.platform}/${params.arch}`,
    ].join('\n');
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

    const isWithinBasePath = (basePath: string, targetPath: string): boolean => {
        const relative = path.relative(basePath, targetPath);
        return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative));
    };

    const hasSafeExistingAncestor = (workspaceRealPath: string, candidatePath: string): boolean => {
        let current = candidatePath;
        while (!fs.existsSync(current)) {
            const parent = path.dirname(current);
            if (parent === current) {
                return false;
            }
            current = parent;
        }

        try {
            const ancestorRealPath = fs.realpathSync(current);
            return isWithinBasePath(workspaceRealPath, ancestorRealPath);
        } catch {
            return false;
        }
    };

    for (const folder of workspaceFolders) {
        const cacheKey = `perl-lsp.includePathsWarning.${encodeURIComponent(folder.uri.toString())}`;
        const config = vscode.workspace.getConfiguration('perl-lsp', folder.uri);
        const includePaths: string[] = config.get('includePaths', ['lib', 'local/lib/perl5']);

        // Origin-aware validation: built-in default include paths (e.g. "lib",
        // "local/lib/perl5") are optional search hints: used if present,
        // silently ignored if missing. Only explicitly-configured paths are
        // expectations worth a user-facing warning. Otherwise a fresh project
        // without a lib/ directory looks broken on first run when it is not.
        const inspected =
            typeof config.inspect === 'function'
                ? config.inspect<string[]>('includePaths')
                : undefined;
        const defaultPaths = new Set<string>([
            // The workspace root "." always exists, but treat it as a default
            // hint defensively so it can never trigger the warning.
            '.',
            ...(inspected?.defaultValue ?? ['lib', 'local/lib/perl5']),
        ]);
        const isDefaultPath = (includePath: string): boolean => defaultPaths.has(includePath);

        let workspaceRealPath: string;
        try {
            workspaceRealPath = fs.realpathSync(folder.uri.fsPath);
        } catch {
            continue;
        }
        const missingPaths = includePaths.filter(includePath => {
            // Missing built-in defaults are optional hints: never reported.
            if (isDefaultPath(includePath)) {
                return false;
            }
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

        const creatablePaths = missingPaths.filter(includePath => {
            if (path.isAbsolute(includePath)) {
                return false;
            }
            const resolved = path.resolve(folder.uri.fsPath, includePath);
            const relative = path.relative(folder.uri.fsPath, resolved);
            if (relative === '' || relative.startsWith('..') || path.isAbsolute(relative)) {
                return false;
            }

            return hasSafeExistingAncestor(workspaceRealPath, resolved);
        });
        const actions = ['Open Settings'];
        if (creatablePaths.length > 0) {
            actions.push('Create Missing Directories');
        }

        const choice = await vscode.window.showWarningMessage(
            `Perl LSP: configured include path "${firstMissing}" (${relativeNote}) does not exist.${suffix}`,
            ...actions
        );

        if (choice === 'Open Settings') {
            void vscode.commands.executeCommand(
                'workbench.action.openSettings',
                '@ext:EffortlessMetrics.perl-lsp-rs perl-lsp.includePaths'
            );
        } else if (choice === 'Create Missing Directories') {
            const createdPaths: string[] = [];
            for (const includePath of creatablePaths) {
                const resolved = path.resolve(folder.uri.fsPath, includePath);
                if (!fs.existsSync(resolved) && hasSafeExistingAncestor(workspaceRealPath, resolved)) {
                    fs.mkdirSync(resolved, { recursive: true });
                    createdPaths.push(includePath);
                }
            }

            if (createdPaths.length > 0) {
                vscode.window.showInformationMessage(
                    `Created ${createdPaths.length} include director${createdPaths.length === 1 ? 'y' : 'ies'}: ${createdPaths.join(', ')}.`
                );
                await context.globalState.update(cacheKey, undefined);
                continue;
            }
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

function getManagedBinarySource(): ManagedBinarySource {
    const downloadBaseUrl = vscode.workspace.getConfiguration('perl-lsp').get<string>('downloadBaseUrl', '');
    return downloadBaseUrl ? 'internal-base-url' : 'github-release';
}

function toHealthCheckCommandResult(results: HealthCheckResult[]): HealthCheckCommandResult {
    const checks = results.map(result => ({
        label: result.label,
        status: result.status as HealthCheckCommandStatus,
        detail: result.detail,
    }));

    return {
        ok: checks.every(check => check.status !== 'error'),
        checks,
    };
}

async function readInstalledServerVersion(serverPath: string): Promise<string | undefined> {
    return new Promise(resolve => {
        execFile(serverPath, ['--version'], { timeout: 3000 }, (error: Error | null, stdout: string) => {
            if (error) {
                resolve(undefined);
                return;
            }
            resolve(parseLocalVersion(stdout) ?? undefined);
        });
    });
}

async function reinstallServerBinary(context: vscode.ExtensionContext): Promise<ReinstallCommandResult> {
    outputChannel.show(true);
    outputChannel.appendLine('Reinstalling perllsp binary...');

    const downloader = new BinaryDownloader(context, outputChannel);
    const target = downloader.getTargetTriple();
    const source = getManagedBinarySource();

    // Lifecycle snapshot: stop a running language client before install so
    // Windows releases its handle on the existing perllsp.exe. On failure
    // we restart with the previous binary so the user is never left worse
    // off than before they invoked Reinstall.
    const wasRunning = client !== undefined;
    const previousServerPath = currentServerPath;

    if (wasRunning) {
        outputChannel.appendLine('[reinstall] stopping language client to release the running binary');
        try {
            await disposeLanguageClient();
        } catch (stopErr: unknown) {
            const msg = stopErr instanceof Error ? stopErr.message : String(stopErr);
            outputChannel.appendLine(`[reinstall] language client stop reported: ${msg}`);
        }
        // Brief grace period: Windows can lag a few ms releasing the
        // executable file handle after the LSP child process exits.
        await new Promise(resolve => setTimeout(resolve, 250));
    }

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
        if (wasRunning && previousServerPath) {
            outputChannel.appendLine('[reinstall] restoring previous binary after failed download');
            currentServerPath = previousServerPath;
            try {
                await restartServer(context);
            } catch {
                // restartServer surfaces its own dialog/log; nothing to add here.
            }
        }
        return {
            ok: false,
            serverPath: previousServerPath ?? '',
            target,
            source,
            error: downloader.getLastErrorMessage() ?? 'download failed',
        };
    }

    const healthOk = await runHealthCheck(downloadedPath);
    const version = await readInstalledServerVersion(downloadedPath);
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
        if (wasRunning && previousServerPath) {
            outputChannel.appendLine('[reinstall] restoring previous binary after failed health check');
            currentServerPath = previousServerPath;
            try {
                await restartServer(context);
            } catch {
                // restartServer surfaces its own dialog/log.
            }
        }
        return {
            ok: false,
            serverPath: downloadedPath,
            target,
            source,
            version,
            checksumVerified: true,
            error: 'health check failed',
        };
    }

    currentServerPath = downloadedPath;

    if (wasRunning) {
        outputChannel.appendLine('[reinstall] restarting language client with the freshly installed binary');
        try {
            await restartServer(context);
        } catch {
            // restartServer surfaces its own dialog/log.
        }
    } else {
        vscode.window.showInformationMessage('perl-lsp was reinstalled successfully.', 'OK');
    }

    return {
        ok: true,
        serverPath: downloadedPath,
        target,
        source,
        version,
        checksumVerified: true,
    };
}

function bindClientState(languageClient: LanguageClient) {
    stateChangeDisposable?.dispose();
    stateChangeDisposable = languageClient.onDidChangeState(event => {
        // vscode-languageclient State values match ClientState numeric values:
        // Stopped = 1, Running = 2, Starting = 3
        healthWidget?.onStateChange(event.newState as unknown as ClientState);

        // When the server stops unexpectedly after a successful start (mid-session
        // crash), capture a generic diagnosis so serverNotRunningMessage() returns
        // an actionable hint instead of the stale "not running" fallback.
        // We can't run probeStartupFailure here (async) so we set a generic
        // "server stopped" diagnosis; the user can run Health Check for details.
        //
        // Note: event.newState / oldState are vscode-languageclient's `State` enum
        // which shares numeric values with ClientState (Stopped=1, Running=2) but
        // is a distinct nominal type, so we cast to number for comparison.
        const newStateNum = event.newState as unknown as number;
        const oldStateNum = event.oldState as unknown as number;
        if (newStateNum === ClientState.Stopped && oldStateNum === ClientState.Running) {
            lastStartupDiagnosis = {
                kind: StartupErrorKind.Unknown,
                hint: 'The Perl Language Server stopped unexpectedly. Check the Output panel for details.',
                remediation: 'Try restarting the server (Command Palette: "Perl: Restart Server") or run the Health Check.',
            };
        }
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
        'perl-lsp.enableSemanticTokens',
        'perl-lsp.enableFormatting',
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
