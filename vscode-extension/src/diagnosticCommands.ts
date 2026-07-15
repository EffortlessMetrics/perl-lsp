import * as vscode from 'vscode';
import { workspaceTrustClientRuntimeState } from './workspaceTrustRuntimeState';

export type LspExecuteCommandClient = {
  sendRequest<T>(method: string, params: unknown): Promise<T>;
};

export interface DiagnosticCommandOptions {
  readonly outputChannel?: vscode.OutputChannel;
  readonly serverNotRunningMessage?: () => string;
}

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

function outputChannel(options: DiagnosticCommandOptions): vscode.OutputChannel {
  return options.outputChannel ?? vscode.window.createOutputChannel('Perl LSP Trust');
}

function asObject(value: unknown): Record<string, unknown> | undefined {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}

function stringField(
  value: Record<string, unknown> | undefined,
  field: string,
): string | undefined {
  const fieldValue = value?.[field];
  return typeof fieldValue === 'string' ? fieldValue : undefined;
}

function numberField(
  value: Record<string, unknown> | undefined,
  field: string,
): number | undefined {
  const fieldValue = value?.[field];
  return typeof fieldValue === 'number' ? fieldValue : undefined;
}

function booleanField(
  value: Record<string, unknown> | undefined,
  field: string,
): boolean | undefined {
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
    /\b(?:use|require)\s+([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)/,
  );
  return useStatement?.[1];
}

async function executeLspCommand(
  activeClient: LspExecuteCommandClient | undefined,
  command: string,
  argument: Record<string, unknown> | undefined,
  options: DiagnosticCommandOptions,
): Promise<unknown | undefined> {
  if (!activeClient) {
    vscode.window.showWarningMessage(
      options.serverNotRunningMessage?.() ??
        'Perl Language Server is not running. Run the Health Check to diagnose the issue.',
    );
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

async function showProviderDecisionResult(
  title: string,
  result: unknown,
  options: DiagnosticCommandOptions,
): Promise<void> {
  const resultObject = asObject(result);
  const message = stringField(resultObject, 'user_message') ?? `${title} completed.`;
  const channel = outputChannel(options);
  channel.appendLine('');
  channel.appendLine(`[${title}]`);
  channel.appendLine(providerDecisionJson(result));

  const action =
    stringField(resultObject, 'decision') === 'blocked'
      ? await vscode.window.showWarningMessage(message, 'Show Output')
      : await vscode.window.showInformationMessage(message, 'Show Output');

  if (action === 'Show Output') {
    channel.show();
  }
}

function appendSupportTierLines(
  lines: string[],
  supportTiers: Record<string, unknown> | undefined,
): void {
  if (!supportTiers) {
    lines.push('- support tiers: unavailable');
    return;
  }

  for (const [surface, tier] of Object.entries(supportTiers).sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
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
    stringField(report, 'claim_boundary') ??
      'This explanation is bounded to current runtime state.',
    '',
    'Raw lookup JSON',
    providerDecisionJson(result),
  );

  return lines.join('\n');
}

export async function showWorkspaceTrustReportCommand(
  activeClient: LspExecuteCommandClient | undefined,
  clientRuntimeState: () => Record<string, unknown> = workspaceTrustClientRuntimeState,
  options: DiagnosticCommandOptions = {},
): Promise<void> {
  const result = await executeLspCommand(
    activeClient,
    'perl.workspaceTrustReport',
    { client_runtime_state: clientRuntimeState() },
    options,
  );
  if (result === undefined) {
    return;
  }

  const channel = outputChannel(options);
  channel.appendLine('');
  channel.appendLine(formatWorkspaceTrustReport(result));
  channel.show();
}

export async function explainMissingModuleLookupCommand(
  activeClient: LspExecuteCommandClient | undefined,
  moduleOverride?: string,
  options: DiagnosticCommandOptions = {},
): Promise<unknown | undefined> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'perl') {
    vscode.window.showErrorMessage('Explain Missing Module Lookup requires an active Perl file.');
    return undefined;
  }

  const moduleName =
    moduleOverride ??
    moduleNameAtCursor(editor) ??
    (await vscode.window.showInputBox({
      placeHolder: 'Missing::Module',
      prompt: 'Module name to explain with perl-lsp @INC lookup state',
      validateInput: (value) => {
        if (!value.trim()) {
          return 'Enter a Perl module name.';
        }
        return isPerlModuleName(value.trim()) ? undefined : 'Enter a valid Perl module name.';
      },
    }));
  if (!moduleName) {
    return undefined;
  }

  const result = await executeLspCommand(
    activeClient,
    'perl.explainMissingModuleLookup',
    {
      module: moduleName.trim(),
      textDocument: { uri: editor.document.uri.toString() },
      position: {
        line: editor.selection.active.line,
        character: editor.selection.active.character,
      },
    },
    options,
  );
  if (result === undefined) {
    return undefined;
  }

  const resultObject = asObject(result);
  const message = stringField(resultObject, 'user_message') ?? 'Missing-module lookup explained.';
  const status = stringField(asObject(asObject(resultObject?.module_resolution)?.result), 'status');
  const channel = outputChannel(options);
  channel.appendLine('');
  channel.appendLine(formatMissingModuleLookup(result));

  const action =
    status === 'resolved'
      ? await vscode.window.showInformationMessage(message, 'Show Output')
      : await vscode.window.showWarningMessage(message, 'Show Output');

  if (action === 'Show Output') {
    channel.show();
  }

  return result;
}

export async function explainProviderDecisionCommand(
  activeClient: LspExecuteCommandClient | undefined,
  providerOverride?: string,
  options: DiagnosticCommandOptions = {},
): Promise<void> {
  const provider = await chooseProvider(providerOverride);
  if (!provider) {
    return;
  }

  const result = await executeLspCommand(
    activeClient,
    'perl.explainProviderDecision',
    providerDecisionArgument(provider),
    options,
  );
  if (result !== undefined) {
    await showProviderDecisionResult('Provider decision explanation', result, options);
  }
}

export async function explainDiagnosticCommand(
  activeClient: LspExecuteCommandClient | undefined,
  request?: unknown,
  options: DiagnosticCommandOptions = {},
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
    argument,
    options,
  );
  if (result !== undefined) {
    await showProviderDecisionResult('Diagnostic explanation', result, options);
  }
}

export async function previewSafeDeleteCommand(
  activeClient: LspExecuteCommandClient | undefined,
  options: DiagnosticCommandOptions = {},
): Promise<void> {
  const argument = activeSafeDeletePreviewArgument();
  if (!argument) {
    vscode.window.showErrorMessage('Preview Safe Delete requires an active Perl file.');
    return;
  }

  const result = await executeLspCommand(activeClient, 'perl.previewSafeDelete', argument, options);
  if (result !== undefined) {
    await showProviderDecisionResult('Safe-delete preview', result, options);
  }
}

export async function previewPackageRenameCommand(
  activeClient: LspExecuteCommandClient | undefined,
  options: DiagnosticCommandOptions = {},
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

  const result = await executeLspCommand(
    activeClient,
    'perl.previewPackageRename',
    argument,
    options,
  );
  if (result !== undefined) {
    await showProviderDecisionResult('Package rename preview', result, options);
  }
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
    validateInput: (value) => {
      const trimmed = value.trim();
      if (!trimmed) {
        return 'Enter a new Perl package or symbol name.';
      }
      return isPerlModuleName(trimmed) ? undefined : 'Use a single Perl package or symbol name.';
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

export async function copyProviderDecisionReceiptCommand(
  activeClient: LspExecuteCommandClient | undefined,
  providerOverride?: string,
  options: DiagnosticCommandOptions = {},
): Promise<void> {
  const provider = await chooseProvider(providerOverride);
  if (!provider) {
    return;
  }

  const result = await executeLspCommand(
    activeClient,
    'perl.explainProviderDecision',
    providerDecisionArgument(provider),
    options,
  );
  const resultObject = asObject(result);
  if (!resultObject) {
    return;
  }

  const payload = resultObject.copyable_payload ?? resultObject;
  await vscode.env.clipboard.writeText(providerDecisionJson(payload));
  vscode.window.showInformationMessage('Provider decision receipt copied.');
}
