import * as path from 'path';
import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

interface DiscoveredTestItem {
    id: string;
    label: string;
    uri: string;
    kind: 'file' | 'suite' | 'test';
    range: {
        start: { line: number; character: number };
        end: { line: number; character: number };
    };
    children?: DiscoveredTestItem[];
}

export class PerlTestAdapter {
    private testController: vscode.TestController;
    private client: LanguageClient;
    private fileTestData = new Map<string, vscode.TestItem>();

    constructor(client: LanguageClient) {
        this.client = client;
        this.testController = vscode.tests.createTestController(
            'perlTestController',
            'Perl Tests'
        );

        // Set up test discovery
        this.testController.createRunProfile(
            'Run Tests',
            vscode.TestRunProfileKind.Run,
            (request, token) => this.runTests(request, token),
            true
        );

        // Watch for document changes
        vscode.workspace.onDidOpenTextDocument(doc => this.parseDocument(doc));
        vscode.workspace.onDidChangeTextDocument(e => this.parseDocument(e.document));

        // Discover tests in all open documents
        vscode.workspace.textDocuments.forEach(doc => this.parseDocument(doc));

        // Watch for new files
        const watcher = vscode.workspace.createFileSystemWatcher('**/*.{t,pl,pm}');
        watcher.onDidCreate(uri => this.discoverTests(uri));
        watcher.onDidChange(uri => this.discoverTests(uri));
        watcher.onDidDelete(uri => this.deleteTest(uri));

        // Refresh handler
        this.testController.refreshHandler = async () => {
            await this.discoverAllTests();
        };
    }

    private async parseDocument(document: vscode.TextDocument) {
        if (document.languageId !== 'perl') return;

        await this.discoverTests(document.uri);
    }

    private async discoverTests(uri: vscode.Uri) {
        try {
            // Request test discovery from LSP server
            const tests = await this.client.sendRequest('experimental/testDiscovery', {
                textDocument: {
                    uri: uri.toString()
                }
            }) as DiscoveredTestItem[] | undefined;

            if (!tests || !Array.isArray(tests)) return;

            // Get or create file test item
            const fileId = uri.toString();
            let fileItem = this.fileTestData.get(fileId);

            if (tests.length > 0) {
                if (!fileItem) {
                    fileItem = this.testController.createTestItem(
                        fileId,
                        path.basename(uri.fsPath || uri.path) || 'test',
                        uri
                    );
                    this.testController.items.add(fileItem);
                    this.fileTestData.set(fileId, fileItem);
                }

                this.decorateTestItem(fileItem, {
                    id: fileId,
                    label: path.basename(uri.fsPath || uri.path) || 'test',
                    uri: uri.toString(),
                    kind: 'file',
                    range: tests[0]?.range ?? {
                        start: { line: 0, character: 0 },
                        end: { line: 0, character: 0 }
                    },
                    children: tests
                });

                // Clear existing children
                fileItem.children.replace([]);

                // Add test items in source order for a stable visual tree
                for (const test of this.sortTestsByLocation(tests)) {
                    this.addTestItem(fileItem, test);
                }
            } else if (fileItem) {
                // No tests found, remove the file item
                this.testController.items.delete(fileId);
                this.fileTestData.delete(fileId);
            }
        } catch (error) {
            console.error('Failed to discover tests:', error);
        }
    }

    private addTestItem(parent: vscode.TestItem, testData: DiscoveredTestItem) {
        const range = new vscode.Range(
            testData.range.start.line,
            testData.range.start.character,
            testData.range.end.line,
            testData.range.end.character
        );

        const testItem = this.testController.createTestItem(
            testData.id,
            testData.label,
            vscode.Uri.parse(testData.uri)
        );

        testItem.range = range;
        testItem.sortText = this.sortKeyFor(testData);
        this.decorateTestItem(testItem, testData);

        parent.children.add(testItem);

        // Recursively add children in source order
        const children = this.sortTestsByLocation(testData.children);
        for (const child of children) {
            this.addTestItem(testItem, child);
        }
    }

    private decorateTestItem(testItem: vscode.TestItem, testData: DiscoveredTestItem) {
        const uri = vscode.Uri.parse(testData.uri);
        const relativePath = this.relativePathFor(uri);
        const startLine = testData.range.start.line + 1;
        const endLine = testData.range.end.line + 1;
        const lineLabel = startLine === endLine ? `line ${startLine}` : `lines ${startLine}-${endLine}`;

        switch (testData.kind) {
            case 'file':
                testItem.description = relativePath;
                break;
            case 'suite':
                testItem.description = `suite • ${lineLabel}`;
                break;
            default:
                testItem.description = lineLabel;
                break;
        }
    }

    private relativePathFor(uri: vscode.Uri): string {
        const relative = vscode.workspace.asRelativePath(uri, false);
        return relative || path.basename(uri.fsPath || uri.path);
    }

    private sortTestsByLocation(tests: DiscoveredTestItem[] | undefined): DiscoveredTestItem[] {
        if (!tests) {
            return [];
        }

        return [...tests].sort((left, right) => {
            const lineDelta = left.range.start.line - right.range.start.line;
            if (lineDelta !== 0) {
                return lineDelta;
            }

            const charDelta = left.range.start.character - right.range.start.character;
            if (charDelta !== 0) {
                return charDelta;
            }

            return left.label.localeCompare(right.label);
        });
    }

    private sortKeyFor(testData: DiscoveredTestItem): string {
        const line = testData.range.start.line.toString().padStart(6, '0');
        const character = testData.range.start.character.toString().padStart(4, '0');
        return `${line}:${character}:${testData.label}`;
    }

    private async discoverAllTests() {
        // Clear all tests
        this.testController.items.replace([]);
        this.fileTestData.clear();

        // Discover tests in all workspace files
        const files = await vscode.workspace.findFiles('**/*.{t,pl,pm}', '**/node_modules/**');

        for (const file of files) {
            await this.discoverTests(file);
        }
    }

    private deleteTest(uri: vscode.Uri) {
        const fileId = uri.toString();
        const fileItem = this.fileTestData.get(fileId);

        if (fileItem) {
            this.testController.items.delete(fileId);
            this.fileTestData.delete(fileId);
        }
    }

    private async runTests(request: vscode.TestRunRequest, token: vscode.CancellationToken) {
        const run = this.testController.createTestRun(request);
        const tests = request.include || [];

        for (const test of tests) {
            if (token.isCancellationRequested) {
                break;
            }

            await this.runTest(test, run, token);
        }

        run.end();
    }

    private async runTest(
        test: vscode.TestItem,
        run: vscode.TestRun,
        token: vscode.CancellationToken
    ) {
        run.started(test);

        try {
            // Check if this is a file-level test or individual test
            const isFile = test.id.endsWith('.t') || test.id.endsWith('.pl');
            const command = isFile ? 'perl.runTestFile' : 'perl.runTest';

            // Execute test via LSP server
            const result = await this.client.sendRequest('workspace/executeCommand', {
                command: command,
                arguments: [test.id]
            });

            if (!result || typeof result !== 'object') {
                throw new Error('Invalid test result');
            }

            const testResult = result as any;

            if (testResult.status === 'error') {
                run.failed(test, new vscode.TestMessage(testResult.message || 'Test execution failed'));
            } else if (testResult.results && Array.isArray(testResult.results)) {
                // Process test results
                for (const r of testResult.results) {
                    // Find the specific test item if this is a sub-test
                    let targetTest = test;
                    if (r.testId !== test.id) {
                        targetTest = this.findTestById(test, r.testId) || test;
                    }

                    const duration = r.duration || 0;

                    switch (r.status) {
                        case 'passed':
                            run.passed(targetTest, duration);
                            break;
                        case 'failed':
                            run.failed(
                                targetTest,
                                new vscode.TestMessage(r.message || 'Test failed'),
                                duration
                            );
                            break;
                        case 'skipped':
                            run.skipped(targetTest);
                            break;
                        case 'errored':
                            run.errored(
                                targetTest,
                                new vscode.TestMessage(r.message || 'Test error'),
                                duration
                            );
                            break;
                    }
                }
            }
        } catch (error: any) {
            run.failed(test, new vscode.TestMessage(error.message || 'Unknown error'));
        }
    }

    private findTestById(parent: vscode.TestItem, id: string): vscode.TestItem | undefined {
        if (parent.id === id) return parent;

        for (const [, child] of parent.children) {
            const found = this.findTestById(child, id);
            if (found) return found;
        }

        return undefined;
    }

    public async runFileTests(uri: vscode.Uri) {
        const fileId = uri.toString();
        const fileItem = this.fileTestData.get(fileId);

        if (fileItem) {
            const request = new vscode.TestRunRequest([fileItem]);
            const tokenSource = new vscode.CancellationTokenSource();
            try {
                await this.runTests(request, tokenSource.token);
            } finally {
                tokenSource.dispose();
            }
        } else {
            vscode.window.showWarningMessage('No tests found in this file');
        }
    }

    dispose() {
        this.testController.dispose();
    }
}
