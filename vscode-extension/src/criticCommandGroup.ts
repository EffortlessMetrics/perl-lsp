import * as vscode from 'vscode';

/**
 * Explicit dependencies for Critic command registration.
 *
 * Critic behavior remains in the existing tested extension functions for this
 * slice. The command group owns only registration and delegates through this
 * context, so it does not acquire a second client or configuration owner.
 */
export interface CriticCommandContext {
  readonly runPerlCriticOnActiveFile: () => Promise<void>;
  readonly setPerlCriticSeverity: () => Promise<void>;
}

/** Register the Critic commands owned by the diagnostics/critic group. */
export function registerCriticCommandGroup(
  dependencies: CriticCommandContext,
): vscode.Disposable[] {
  const runPerlCriticCommand = vscode.commands.registerCommand(
    'perl-lsp.runPerlCritic',
    async () => {
      await dependencies.runPerlCriticOnActiveFile();
    },
  );

  const setPerlCriticSeverityCommand = vscode.commands.registerCommand(
    'perl-lsp.setPerlCriticSeverity',
    async () => {
      await dependencies.setPerlCriticSeverity();
    },
  );

  return [runPerlCriticCommand, setPerlCriticSeverityCommand];
}
