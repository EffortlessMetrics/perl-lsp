import * as vscode from 'vscode';
import type { LanguageClient } from 'vscode-languageclient/node';
import { ProgressType } from 'vscode-jsonrpc';

/** Shape of each candidate item inside a stream progress value. */
interface StreamCandidateItem {
    insertText: string;
    filterText?: string;
    range?: {
        start: { line: number; character: number };
        end: { line: number; character: number };
    };
}

/** Payload sent by the server via $/progress for streaming inline completions. */
interface StreamProgressValue {
    kind: string;
    sessionId: string;
    sequence: number;
    isFinal: boolean;
    items: StreamCandidateItem[];
}

/** Snapshot of the most recent candidate received from the server. */
interface CachedCandidate {
    uri: string;
    version: number;
    line: number;
    character: number;
    text: string;
    sessionId: string;
    sequence: number;
    isFinal: boolean;
}

/**
 * Progress type marker used with `LanguageClient.onProgress`.
 *
 * `ProgressType<T>` is purely a type-level marker in vscode-jsonrpc —
 * the constructor takes no arguments and the generic parameter is
 * only used for TypeScript inference.
 */
const streamProgressType = new ProgressType<StreamProgressValue>();

/**
 * Streaming inline completion controller.
 *
 * Manages the lifecycle of progressive AI inline completions:
 * 1. Triggers custom stream requests to the server
 * 2. Caches cumulative candidates from $/progress
 * 3. Feeds cached candidates through the inline completion provider
 * 4. Cancels streams on cursor movement or document changes
 */
export class StreamingCompletionController implements vscode.Disposable {
    private client: LanguageClient;
    private cachedCandidate: CachedCandidate | null = null;
    private activeTokenSource: vscode.CancellationTokenSource | null = null;
    private activeProgressToken: string | null = null;
    private activeProgressDisposable: vscode.Disposable | null = null;
    private disposables: vscode.Disposable[] = [];

    constructor(client: LanguageClient) {
        this.client = client;

        // Register inline completion provider
        const provider = vscode.languages.registerInlineCompletionItemProvider(
            { scheme: 'file', language: 'perl' },
            {
                provideInlineCompletionItems: (
                    document: vscode.TextDocument,
                    position: vscode.Position,
                    context: vscode.InlineCompletionContext,
                    token: vscode.CancellationToken
                ) => {
                    return this.provideInlineCompletionItems(document, position, context, token);
                }
            }
        );
        this.disposables.push(provider);

        // Cancel on cursor movement
        const cursorDisposable = vscode.window.onDidChangeTextEditorSelection(() => {
            this.cancelActiveStream();
        });
        this.disposables.push(cursorDisposable);

        // Cancel on document change
        const changeDisposable = vscode.workspace.onDidChangeTextDocument(() => {
            this.cancelActiveStream();
        });
        this.disposables.push(changeDisposable);
    }

    /**
     * Check whether a progress value matches our streaming completion protocol.
     */
    private isStreamProgressValue(value: unknown): value is StreamProgressValue {
        if (typeof value !== 'object' || value === null) {
            return false;
        }
        const obj = value as Record<string, unknown>;
        return (
            obj.kind === 'perlInlineCompletionStream' &&
            typeof obj.sessionId === 'string' &&
            typeof obj.sequence === 'number' &&
            typeof obj.isFinal === 'boolean' &&
            Array.isArray(obj.items)
        );
    }

    private handleProgress(value: unknown): void {
        if (!this.isStreamProgressValue(value)) {
            return;
        }

        if (value.items.length === 0) {
            return;
        }

        const item = value.items[0];
        if (!item || typeof item.insertText !== 'string') {
            return;
        }

        // Update cached candidate if it's newer
        if (
            this.cachedCandidate &&
            this.cachedCandidate.sessionId === value.sessionId &&
            value.sequence <= this.cachedCandidate.sequence
        ) {
            return; // Stale update
        }

        this.cachedCandidate = {
            uri: '',
            version: 0,
            line: item.range?.start.line ?? 0,
            character: item.range?.start.character ?? 0,
            text: item.insertText,
            sessionId: value.sessionId,
            sequence: value.sequence,
            isFinal: value.isFinal,
        };

        // Trigger re-evaluation of inline completions
        void vscode.commands.executeCommand('editor.action.inlineSuggest.trigger');
    }

    private provideInlineCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        _context: vscode.InlineCompletionContext,
        _token: vscode.CancellationToken
    ): vscode.InlineCompletionItem[] | undefined {
        // Check if AI completion is enabled
        const config = vscode.workspace.getConfiguration('perl-lsp');
        const aiEnabled = config.get<boolean>('aiCompletion.enabled', false);
        const streamingEnabled = config.get<boolean>('aiCompletion.streaming.enabled', true);

        if (!aiEnabled || !streamingEnabled) {
            return undefined; // Let the server handle via standard path
        }

        // If we have a cached candidate for this position, return it
        if (
            this.cachedCandidate &&
            this.cachedCandidate.line === position.line &&
            this.cachedCandidate.character === position.character
        ) {
            const insertPosition = new vscode.Position(
                this.cachedCandidate.line,
                this.cachedCandidate.character
            );
            return [
                new vscode.InlineCompletionItem(
                    this.cachedCandidate.text,
                    new vscode.Range(insertPosition, insertPosition)
                ),
            ];
        }

        // Start a new stream request
        this.startStreamRequest(document, position);

        return undefined; // No immediate result -- will come via progress
    }

    private startStreamRequest(
        document: vscode.TextDocument,
        position: vscode.Position
    ): void {
        // Cancel any existing stream
        this.cancelActiveStream();

        // Create cancellation token
        this.activeTokenSource = new vscode.CancellationTokenSource();

        const partialResultToken = `stream-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
        this.activeProgressToken = partialResultToken;

        // Register progress handler for this specific token
        this.activeProgressDisposable = this.client.onProgress(
            streamProgressType,
            partialResultToken,
            (value: unknown) => {
                this.handleProgress(value);
            }
        );

        // Send custom request
        const params = {
            textDocument: {
                uri: document.uri.toString(),
                version: document.version,
            },
            position: {
                line: position.line,
                character: position.character,
            },
            context: {
                triggerKind: 2, // Automatic
            },
            partialResultToken,
        };

        this.client
            .sendRequest(
                'textDocument/perlInlineCompletionStream',
                params,
                this.activeTokenSource.token
            )
            .catch((err: unknown) => {
                // Silently ignore cancellation errors (expected on cursor movement)
                if (err instanceof Error && err.message.includes('cancelled')) {
                    return;
                }
                // Non-cancellation errors are suppressed — the stream will be
                // retried on the next inline completion trigger.
            });
    }

    private cancelActiveStream(): void {
        this.cachedCandidate = null;
        if (this.activeProgressDisposable) {
            this.activeProgressDisposable.dispose();
            this.activeProgressDisposable = null;
        }
        this.activeProgressToken = null;
        if (this.activeTokenSource) {
            this.activeTokenSource.cancel();
            this.activeTokenSource.dispose();
            this.activeTokenSource = null;
        }
    }

    /**
     * Send telemetry notification when a completion is accepted.
     */
    public notifyAccepted(sessionId: string, candidateIndex: number): void {
        void this.client.sendNotification('perl/didAcceptInlineCompletion', {
            sessionId,
            candidate: candidateIndex,
        });
    }

    /**
     * Send telemetry notification when a completion is shown to the user.
     */
    public notifyShown(sessionId: string): void {
        void this.client.sendNotification('perl/didShowInlineCompletion', {
            sessionId,
        });
    }

    dispose(): void {
        this.cancelActiveStream();
        for (const d of this.disposables) {
            d.dispose();
        }
        this.disposables = [];
    }
}
