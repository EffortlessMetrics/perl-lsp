import * as vscode from 'vscode';
import {
  registerDiagnosticCommandGroup,
  type DiagnosticCommandContext,
} from '../diagnosticCommandGroup';

function makeDependencies(): DiagnosticCommandContext & {
  explainProviderDecision: jest.Mock;
  previewSafeDelete: jest.Mock;
  previewPackageRename: jest.Mock;
  copyProviderDecisionReceipt: jest.Mock;
  showWorkspaceTrustReport: jest.Mock;
  explainMissingModuleLookup: jest.Mock;
  explainDiagnostic: jest.Mock;
} {
  return {
    explainProviderDecision: jest.fn(async () => undefined),
    previewSafeDelete: jest.fn(async () => undefined),
    previewPackageRename: jest.fn(async () => undefined),
    copyProviderDecisionReceipt: jest.fn(async () => undefined),
    showWorkspaceTrustReport: jest.fn(async () => undefined),
    explainMissingModuleLookup: jest.fn(async () => undefined),
    explainDiagnostic: jest.fn(async () => undefined),
  };
}

let registeredDisposables: vscode.Disposable[] = [];

afterEach(() => {
  for (const disposable of registeredDisposables) {
    disposable.dispose();
  }
  registeredDisposables = [];
});

describe('registerDiagnosticCommandGroup', () => {
  test('registers and delegates every provider-diagnostics command', async () => {
    const dependencies = makeDependencies();
    registeredDisposables = registerDiagnosticCommandGroup(dependencies);

    expect(registeredDisposables).toHaveLength(7);
    await vscode.commands.executeCommand('perl-lsp.explainProviderDecision', 'hover');
    await vscode.commands.executeCommand('perl-lsp.previewSafeDelete');
    await vscode.commands.executeCommand('perl-lsp.previewPackageRename');
    await vscode.commands.executeCommand('perl-lsp.copyProviderDecisionReceipt', 'completion');
    await vscode.commands.executeCommand('perl-lsp.showWorkspaceTrustReport');
    await vscode.commands.executeCommand('perl-lsp.explainMissingModuleLookup', 'Missing::Module');
    await vscode.commands.executeCommand('perl-lsp.explainDiagnostic', { provider: 'diagnostics' });

    expect(dependencies.explainProviderDecision).toHaveBeenCalledWith('hover');
    expect(dependencies.previewSafeDelete).toHaveBeenCalledTimes(1);
    expect(dependencies.previewPackageRename).toHaveBeenCalledTimes(1);
    expect(dependencies.copyProviderDecisionReceipt).toHaveBeenCalledWith('completion');
    expect(dependencies.showWorkspaceTrustReport).toHaveBeenCalledTimes(1);
    expect(dependencies.explainMissingModuleLookup).toHaveBeenCalledWith('Missing::Module');
    expect(dependencies.explainDiagnostic).toHaveBeenCalledWith({ provider: 'diagnostics' });
  });

  test('does not invoke feature callbacks during registration', () => {
    const dependencies = makeDependencies();
    registeredDisposables = registerDiagnosticCommandGroup(dependencies);

    expect(dependencies.explainProviderDecision).not.toHaveBeenCalled();
    expect(dependencies.previewSafeDelete).not.toHaveBeenCalled();
    expect(dependencies.previewPackageRename).not.toHaveBeenCalled();
    expect(dependencies.copyProviderDecisionReceipt).not.toHaveBeenCalled();
    expect(dependencies.showWorkspaceTrustReport).not.toHaveBeenCalled();
    expect(dependencies.explainMissingModuleLookup).not.toHaveBeenCalled();
    expect(dependencies.explainDiagnostic).not.toHaveBeenCalled();
  });
});
