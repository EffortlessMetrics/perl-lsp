import * as vscode from 'vscode';

/**
 * Explicit dependencies for provider-diagnostics and explainability commands.
 *
 * The group owns command registration only. Handler behavior and the active
 * language client remain owned by the composition layer that supplies these
 * callbacks.
 */
export interface DiagnosticCommandContext {
  readonly explainProviderDecision: (provider?: unknown) => Promise<void>;
  readonly previewSafeDelete: () => Promise<void>;
  readonly previewPackageRename: () => Promise<void>;
  readonly copyProviderDecisionReceipt: (provider?: unknown) => Promise<void>;
  readonly showWorkspaceTrustReport: () => Promise<void>;
  readonly explainMissingModuleLookup: (moduleName?: unknown) => Promise<unknown>;
  readonly explainDiagnostic: (request?: unknown) => Promise<void>;
}

/** Register commands owned by the provider-diagnostics group. */
export function registerDiagnosticCommandGroup(
  dependencies: DiagnosticCommandContext,
): vscode.Disposable[] {
  const explainProviderDecisionCommand = vscode.commands.registerCommand(
    'perl-lsp.explainProviderDecision',
    dependencies.explainProviderDecision,
  );
  const previewSafeDeleteCommand = vscode.commands.registerCommand(
    'perl-lsp.previewSafeDelete',
    dependencies.previewSafeDelete,
  );
  const previewPackageRenameCommand = vscode.commands.registerCommand(
    'perl-lsp.previewPackageRename',
    dependencies.previewPackageRename,
  );
  const copyProviderDecisionReceiptCommand = vscode.commands.registerCommand(
    'perl-lsp.copyProviderDecisionReceipt',
    dependencies.copyProviderDecisionReceipt,
  );
  const showWorkspaceTrustReportCommand = vscode.commands.registerCommand(
    'perl-lsp.showWorkspaceTrustReport',
    dependencies.showWorkspaceTrustReport,
  );
  const explainMissingModuleLookupCommand = vscode.commands.registerCommand(
    'perl-lsp.explainMissingModuleLookup',
    dependencies.explainMissingModuleLookup,
  );
  const explainDiagnosticCommand = vscode.commands.registerCommand(
    'perl-lsp.explainDiagnostic',
    dependencies.explainDiagnostic,
  );

  return [
    explainProviderDecisionCommand,
    previewSafeDeleteCommand,
    previewPackageRenameCommand,
    copyProviderDecisionReceiptCommand,
    showWorkspaceTrustReportCommand,
    explainMissingModuleLookupCommand,
    explainDiagnosticCommand,
  ];
}
