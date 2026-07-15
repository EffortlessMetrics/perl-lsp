import * as vscode from 'vscode';
jest.mock('vscode-languageclient/node', () => ({
  LanguageClient: class {},
  Trace: {
    Off: 'off',
    Messages: 'messages',
    Verbose: 'verbose',
  },
  TransportKind: {
    stdio: 0,
  },
}));

import { maybeNudgeArrowCompletion, shouldNudgeArrowCompletion } from '../extension';

const testWindow = vscode.window as unknown as {
  activeTextEditor: vscode.TextEditor | undefined;
};

describe('arrow completion nudge', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    testWindow.activeTextEditor = undefined;
  });

  test('nudges completion for variable arrow intent on dash', () => {
    const document = {
      languageId: 'perl',
      lineAt: jest.fn(() => ({ text: '$obj-' })),
    } as unknown as vscode.TextDocument;
    testWindow.activeTextEditor = { document } as unknown as vscode.TextEditor;

    maybeNudgeArrowCompletion({
      document,
      contentChanges: [
        {
          text: '-',
          rangeLength: 0,
          range: { start: { line: 0, character: 4 } },
        },
      ],
    } as unknown as vscode.TextDocumentChangeEvent);

    expect(vscode.commands.executeCommand).toHaveBeenCalledWith('editor.action.triggerSuggest');
  });

  test('does not nudge completion for spaced subtraction-like dash', () => {
    const document = {
      languageId: 'perl',
      lineAt: jest.fn(() => ({ text: '$value -' })),
    } as unknown as vscode.TextDocument;
    testWindow.activeTextEditor = { document } as unknown as vscode.TextEditor;

    maybeNudgeArrowCompletion({
      document,
      contentChanges: [
        {
          text: '-',
          rangeLength: 0,
          range: { start: { line: 0, character: 7 } },
        },
      ],
    } as unknown as vscode.TextDocumentChangeEvent);

    expect(vscode.commands.executeCommand).not.toHaveBeenCalled();
  });

  test('does not treat double colon as arrow intent', () => {
    expect(shouldNudgeArrowCompletion('Foo:-')).toBe(false);
    expect(shouldNudgeArrowCompletion('Foo::')).toBe(false);
  });

  test('does not treat closing delimiters as arrow intent', () => {
    expect(shouldNudgeArrowCompletion('$arr[0]-')).toBe(false);
    expect(shouldNudgeArrowCompletion('(foo)-')).toBe(false);
    expect(shouldNudgeArrowCompletion('$h{key}-')).toBe(false);
  });
});
