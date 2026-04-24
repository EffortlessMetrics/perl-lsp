/**
 * Minimal mock of the vscode module for unit testing.
 *
 * Only the surfaces actually touched by unit-testable code paths are stubbed.
 * Integration tests requiring the full VSCode runtime should use
 * @vscode/test-electron instead of this mock.
 */

export const Uri = {
  parse: (value: string) => ({ toString: () => value, fsPath: value }),
  file: (path: string) => ({ toString: () => `file://${path}`, fsPath: path }),
};

export class ThemeColor {
  constructor(public id: string) {}
}

export enum StatusBarAlignment {
  Left = 1,
  Right = 2,
}

export enum QuickPickItemKind {
  Separator = -1,
  Default = 0,
}

export enum TestRunProfileKind {
  Run = 1,
  Debug = 2,
  Coverage = 3,
}

export enum ProgressLocation {
  SourceControl = 1,
  Window = 10,
  Notification = 15,
}

export class Range {
  public start: { line: number; character: number };
  public end: { line: number; character: number };

  constructor(
    public startLine: number,
    public startCharacter: number,
    public endLine: number,
    public endCharacter: number,
  ) {
    this.start = { line: startLine, character: startCharacter };
    this.end = { line: endLine, character: endCharacter };
  }
}

export enum SymbolKind {
  File = 0,
  Module = 1,
  Namespace = 2,
  Package = 3,
  Class = 4,
  Method = 5,
  Property = 6,
  Field = 7,
  Constructor = 8,
  Enum = 9,
  Interface = 10,
  Function = 11,
  Variable = 12,
  Constant = 13,
  String = 15,
  Number = 16,
  Boolean = 17,
  Array = 18,
  Object = 19,
  Key = 20,
  Null = 21,
  EnumMember = 22,
  Struct = 23,
  Event = 24,
  Operator = 25,
  TypeParameter = 26,
}

export class TestMessage {
  constructor(public message: string) {}
}

export class CancellationTokenSource {
  token = { isCancellationRequested: false };
  dispose() {}
}

const _commands = new Map<string, (...args: any[]) => any>();

export const commands = {
  registerCommand: jest.fn((command: string, callback: (...args: any[]) => any) => {
    _commands.set(command, callback);
    return { dispose: jest.fn() };
  }),
  executeCommand: jest.fn(async (command: string, ...args: any[]) => {
    const handler = _commands.get(command);
    if (handler) return handler(...args);
  }),
};

export const window = {
  createOutputChannel: jest.fn(() => ({
    appendLine: jest.fn(),
    show: jest.fn(),
    dispose: jest.fn(),
  })),
  createStatusBarItem: jest.fn(() => ({
    text: '',
    tooltip: '',
    command: '',
    backgroundColor: undefined,
    show: jest.fn(),
    hide: jest.fn(),
    dispose: jest.fn(),
  })),
  showErrorMessage: jest.fn(async () => undefined),
  showWarningMessage: jest.fn(async () => undefined),
  showInformationMessage: jest.fn(async () => undefined),
  showQuickPick: jest.fn(async () => undefined),
  showTextDocument: jest.fn(async () => undefined),
  withProgress: jest.fn(async (_options: any, task: any) => {
    const progress = { report: jest.fn() };
    const token = { isCancellationRequested: false };
    return task(progress, token);
  }),
  activeTextEditor: undefined as any,
};

export const workspace = {
  getConfiguration: jest.fn((section?: string) => ({
    get: jest.fn((key: string, defaultValue?: any) => defaultValue),
    has: jest.fn(() => false),
    inspect: jest.fn(),
    update: jest.fn(),
  })),
  createFileSystemWatcher: jest.fn(() => ({
    onDidCreate: jest.fn(),
    onDidChange: jest.fn(),
    onDidDelete: jest.fn(),
    dispose: jest.fn(),
  })),
  onDidOpenTextDocument: jest.fn(),
  onDidChangeTextDocument: jest.fn(),
  onWillSaveTextDocument: jest.fn(),
  onDidChangeConfiguration: jest.fn(),
  textDocuments: [],
  findFiles: jest.fn(async () => []),
  openTextDocument: jest.fn(async (value: any) => ({
    uri: typeof value === 'string' ? { fsPath: value } : value,
    getText: jest.fn(() => ''),
  })),
  workspaceFolders: undefined as any[] | undefined,
};

export const tests = {
  createTestController: jest.fn(() => ({
    createRunProfile: jest.fn(),
    createTestItem: jest.fn((id: string, label: string, uri?: any) => ({
      id,
      label,
      uri,
      range: undefined,
      description: undefined,
      children: {
        add: jest.fn(),
        delete: jest.fn(),
        replace: jest.fn(),
        forEach: jest.fn(),
        get: jest.fn(),
        size: 0,
      },
    })),
    items: {
      add: jest.fn(),
      delete: jest.fn(),
      replace: jest.fn(),
      forEach: jest.fn(),
      get: jest.fn(),
      size: 0,
    },
    refreshHandler: null,
    createTestRun: jest.fn(() => ({
      started: jest.fn(),
      passed: jest.fn(),
      failed: jest.fn(),
      skipped: jest.fn(),
      errored: jest.fn(),
      end: jest.fn(),
    })),
    dispose: jest.fn(),
  })),
};

export const debug = {
  registerDebugConfigurationProvider: jest.fn(() => ({ dispose: jest.fn() })),
  registerDebugAdapterDescriptorFactory: jest.fn(() => ({ dispose: jest.fn() })),
  startDebugging: jest.fn(async () => true),
};

export class DebugAdapterExecutable {
  constructor(public command: string, public args: string[], public options?: any) {}
}

export const env = {
  clipboard: { writeText: jest.fn() },
  openExternal: jest.fn(),
};

export const extensions = {
  all: [] as any[],
  getExtension: jest.fn(() => undefined),
};

export class Disposable {
  constructor(private callOnDispose: () => void) {}
  dispose() { this.callOnDispose(); }
}

export class EventEmitter {
  event = jest.fn();
  fire = jest.fn();
  dispose = jest.fn();
}

export enum DiagnosticSeverity {
  Error = 0,
  Warning = 1,
  Information = 2,
  Hint = 3,
}

export enum ConfigurationTarget {
  Global = 1,
  Workspace = 2,
  WorkspaceFolder = 3,
}

export const languages = {
  onDidChangeDiagnostics: jest.fn(() => ({ dispose: jest.fn() })),
  getDiagnostics: jest.fn(() => [] as Array<[any, any[]]>),
  registerDocumentSymbolProvider: jest.fn(() => ({ dispose: jest.fn() })),
  registerFoldingRangeProvider: jest.fn(() => ({ dispose: jest.fn() })),
  registerCodeActionsProvider: jest.fn(() => ({ dispose: jest.fn() })),
  registerDefinitionProvider: jest.fn(() => ({ dispose: jest.fn() })),
};
