import * as vscode from 'vscode';

/**
 * Explicit command callbacks for test and debugger integration.
 *
 * This slice extracts command registration only. The existing handlers keep
 * their behavior and dependencies in the composition layer until the next
 * implementation-focused extraction.
 */
export interface TestCommandContext {
  readonly runTests: (test?: unknown) => Promise<void>;
  readonly runCurrentTest: () => Promise<void>;
  readonly runTestAtCursor: () => Promise<void>;
  readonly runAllTests: () => Promise<void>;
}

/** Register commands owned by the test/debugger feature group. */
export function registerTestCommandGroup(dependencies: TestCommandContext): vscode.Disposable[] {
  const runTestsCommand = vscode.commands.registerCommand(
    'perl-lsp.runTests',
    async (test?: unknown) => {
      await dependencies.runTests(test);
    },
  );

  const runCurrentTestCommand = vscode.commands.registerCommand(
    'perl-lsp.runCurrentTest',
    async () => {
      await dependencies.runCurrentTest();
    },
  );

  const runTestAtCursorCommand = vscode.commands.registerCommand(
    'perl-lsp.runTestAtCursor',
    async () => {
      await dependencies.runTestAtCursor();
    },
  );

  const runAllTestsCommand = vscode.commands.registerCommand('perl-lsp.runAllTests', async () => {
    await dependencies.runAllTests();
  });

  return [runTestsCommand, runCurrentTestCommand, runTestAtCursorCommand, runAllTestsCommand];
}
