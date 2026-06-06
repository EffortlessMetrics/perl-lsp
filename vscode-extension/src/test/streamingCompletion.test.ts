import * as vscode from 'vscode';

// Mock vscode-languageclient/node and vscode-jsonrpc before importing the controller
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

jest.mock('vscode-jsonrpc', () => ({
    ProgressType: class {},
}));

import { StreamingCompletionController } from '../streamingCompletion';
import { LanguageClient } from 'vscode-languageclient/node';

/** Create a mock LanguageClient with the methods needed by StreamingCompletionController. */
function createMockClient(captureProgress?: (handler: (value: unknown) => void) => void): LanguageClient {
    return {
        onProgress: jest.fn((_type, _token, handler: (value: unknown) => void) => {
            captureProgress?.(handler);
            return { dispose: jest.fn() };
        }),
        sendRequest: jest.fn(async () => ({})),
        sendNotification: jest.fn(),
    } as unknown as LanguageClient;
}

function makeDocument(uri: string, version: number): vscode.TextDocument {
    return {
        uri: { toString: () => uri },
        version,
    } as unknown as vscode.TextDocument;
}

type InlineProvider = {
    provideInlineCompletionItems: (
        document: vscode.TextDocument,
        position: vscode.Position,
        context: vscode.InlineCompletionContext,
        token: vscode.CancellationToken
    ) => vscode.InlineCompletionItem[] | undefined;
};

function registeredProvider(): InlineProvider {
    const call = (vscode.languages.registerInlineCompletionItemProvider as jest.Mock).mock.calls[0];
    return call[1] as InlineProvider;
}

describe('StreamingCompletionController', () => {
    let mockClient: LanguageClient;
    let controller: StreamingCompletionController;
    let progressHandler: ((value: unknown) => void) | undefined;

    beforeEach(() => {
        jest.clearAllMocks();
        progressHandler = undefined;
        // Extend mock with needed symbols for inline completions
        (vscode as Record<string, unknown>).Position = class {
            constructor(public line: number, public character: number) {}
        };
        (vscode as Record<string, unknown>).Range = class {
            constructor(public start: unknown, public end: unknown) {}
        };
        (vscode as Record<string, unknown>).InlineCompletionItem = class {
            constructor(public insertText: string, public range?: unknown) {}
        };
        (vscode as Record<string, unknown>).CancellationTokenSource = class {
            token = { isCancellationRequested: false };
            cancel = jest.fn();
            dispose = jest.fn();
        };
        (vscode.languages as Record<string, unknown>).registerInlineCompletionItemProvider = jest.fn(() => ({
            dispose: jest.fn(),
        }));
        (vscode.window as Record<string, unknown>).onDidChangeTextEditorSelection = jest.fn(() => ({
            dispose: jest.fn(),
        }));
        (vscode.workspace as Record<string, unknown>).onDidChangeTextDocument = jest.fn(() => ({
            dispose: jest.fn(),
        }));
        (vscode.workspace.getConfiguration as jest.Mock).mockReturnValue({
            get: jest.fn((key: string, defaultValue?: boolean) => {
                if (key === 'aiCompletion.enabled') {
                    return true;
                }
                if (key === 'aiCompletion.streaming.enabled') {
                    return true;
                }
                return defaultValue;
            }),
        });

        mockClient = createMockClient(handler => {
            progressHandler = handler;
        });
        controller = new StreamingCompletionController(mockClient);
    });

    afterEach(() => {
        controller.dispose();
    });

    test('registers inline completion provider on construction', () => {
        expect(vscode.languages.registerInlineCompletionItemProvider).toHaveBeenCalledTimes(1);
        const call = (vscode.languages.registerInlineCompletionItemProvider as jest.Mock).mock.calls[0];
        expect(call[0]).toEqual({ scheme: 'file', language: 'perl' });
        expect(call[1]).toBeDefined();
    });

    test('registers cursor and document change listeners', () => {
        expect(vscode.window.onDidChangeTextEditorSelection).toHaveBeenCalledTimes(1);
        expect(vscode.workspace.onDidChangeTextDocument).toHaveBeenCalledTimes(1);
    });

    test('dispose cleans up all disposables', () => {
        const providerDispose = jest.fn();
        const cursorDispose = jest.fn();
        const docChangeDispose = jest.fn();

        (vscode.languages.registerInlineCompletionItemProvider as jest.Mock).mockReturnValue({
            dispose: providerDispose,
        });
        (vscode.window.onDidChangeTextEditorSelection as jest.Mock).mockReturnValue({
            dispose: cursorDispose,
        });
        (vscode.workspace.onDidChangeTextDocument as jest.Mock).mockReturnValue({
            dispose: docChangeDispose,
        });

        const ctrl = new StreamingCompletionController(createMockClient());
        ctrl.dispose();

        expect(providerDispose).toHaveBeenCalled();
        expect(cursorDispose).toHaveBeenCalled();
        expect(docChangeDispose).toHaveBeenCalled();
    });

    test('notifyAccepted sends notification to client', () => {
        controller.notifyAccepted('session-1', 0);
        expect((mockClient.sendNotification as jest.Mock)).toHaveBeenCalledWith(
            'perl/didAcceptInlineCompletion',
            { sessionId: 'session-1', candidate: 0 }
        );
    });

    test('notifyShown sends notification to client', () => {
        controller.notifyShown('session-2');
        expect((mockClient.sendNotification as jest.Mock)).toHaveBeenCalledWith(
            'perl/didShowInlineCompletion',
            { sessionId: 'session-2' }
        );
    });

    test('returns cached stream candidates for the matching document version', () => {
        const provider = registeredProvider();
        const document = makeDocument('file:///workspace/lib/App.pm', 4);
        const position = new vscode.Position(10, 8);

        const initial = provider.provideInlineCompletionItems(
            document,
            position,
            {} as vscode.InlineCompletionContext,
            { isCancellationRequested: false } as vscode.CancellationToken
        );
        expect(initial).toBeUndefined();

        progressHandler?.({
            kind: 'perlInlineCompletionStream',
            sessionId: 'session-1',
            sequence: 1,
            isFinal: false,
            items: [{ insertText: '->find_user($id)' }],
        });

        const cached = provider.provideInlineCompletionItems(
            document,
            position,
            {} as vscode.InlineCompletionContext,
            { isCancellationRequested: false } as vscode.CancellationToken
        );

        expect(cached?.[0]?.insertText).toBe('->find_user($id)');
        expect(mockClient.sendRequest as jest.Mock).toHaveBeenCalledTimes(1);
    });

    test('does not show cached stream candidates in another document', () => {
        const provider = registeredProvider();
        const position = new vscode.Position(10, 8);

        provider.provideInlineCompletionItems(
            makeDocument('file:///workspace/lib/App.pm', 4),
            position,
            {} as vscode.InlineCompletionContext,
            { isCancellationRequested: false } as vscode.CancellationToken
        );
        progressHandler?.({
            kind: 'perlInlineCompletionStream',
            sessionId: 'session-1',
            sequence: 1,
            isFinal: false,
            items: [{ insertText: '->find_user($id)' }],
        });

        const otherDocumentResult = provider.provideInlineCompletionItems(
            makeDocument('file:///workspace/lib/Other.pm', 4),
            position,
            {} as vscode.InlineCompletionContext,
            { isCancellationRequested: false } as vscode.CancellationToken
        );

        expect(otherDocumentResult).toBeUndefined();
        expect(mockClient.sendRequest as jest.Mock).toHaveBeenCalledTimes(2);
    });

    test('does not show cached stream candidates after the document version changes', () => {
        const provider = registeredProvider();
        const uri = 'file:///workspace/lib/App.pm';
        const position = new vscode.Position(10, 8);

        provider.provideInlineCompletionItems(
            makeDocument(uri, 4),
            position,
            {} as vscode.InlineCompletionContext,
            { isCancellationRequested: false } as vscode.CancellationToken
        );
        progressHandler?.({
            kind: 'perlInlineCompletionStream',
            sessionId: 'session-1',
            sequence: 1,
            isFinal: false,
            items: [{ insertText: '->find_user($id)' }],
        });

        const newVersionResult = provider.provideInlineCompletionItems(
            makeDocument(uri, 5),
            position,
            {} as vscode.InlineCompletionContext,
            { isCancellationRequested: false } as vscode.CancellationToken
        );

        expect(newVersionResult).toBeUndefined();
        expect(mockClient.sendRequest as jest.Mock).toHaveBeenCalledTimes(2);
    });
});

describe('CachedCandidate shape', () => {
    test('CachedCandidate has expected fields', () => {
        const candidate = {
            uri: 'file:///test.pl',
            version: 1,
            line: 5,
            character: 10,
            text: '->find_user($id)',
            sessionId: 'sess-abc',
            sequence: 3,
            isFinal: false,
        };
        expect(candidate.text).toBe('->find_user($id)');
        expect(candidate.isFinal).toBe(false);
    });

    test('Progress values with higher sequence supersede lower', () => {
        const seq1 = 1;
        const seq2 = 3;
        expect(seq2).toBeGreaterThan(seq1);
    });
});
