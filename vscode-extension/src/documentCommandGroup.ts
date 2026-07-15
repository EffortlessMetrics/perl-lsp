import * as vscode from 'vscode';

/** Explicit callbacks for document and workspace command registration. */
export interface DocumentCommandContext {
  readonly checkSyntax: () => Promise<void>;
  readonly formatDocument: () => Promise<void>;
  readonly showIncPaths: () => Promise<void>;
  readonly openModule: () => Promise<void>;
  readonly showParserAst: () => Promise<void>;
}

/** Register document and workspace commands without owning their handlers. */
export function registerDocumentCommandGroup(
  dependencies: DocumentCommandContext,
): vscode.Disposable[] {
  const checkSyntaxCommand = vscode.commands.registerCommand(
    'perl-lsp.checkSyntax',
    dependencies.checkSyntax,
  );
  const formatDocumentCommand = vscode.commands.registerCommand(
    'perl-lsp.formatDocument',
    dependencies.formatDocument,
  );
  const showIncPathsCommand = vscode.commands.registerCommand(
    'perl-lsp.showIncPaths',
    dependencies.showIncPaths,
  );
  const openModuleCommand = vscode.commands.registerCommand(
    'perl-lsp.openModule',
    dependencies.openModule,
  );
  const showParserAstCommand = vscode.commands.registerCommand(
    'perl-lsp.showParserAst',
    dependencies.showParserAst,
  );

  return [
    checkSyntaxCommand,
    formatDocumentCommand,
    showIncPathsCommand,
    openModuleCommand,
    showParserAstCommand,
  ];
}
