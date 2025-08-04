import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

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
            });

            if (!tests || !Array.isArray(tests)) return;

            // Get or create file test item
            const fileId = uri.toString();
            let fileItem = this.fileTestData.get(fileId);
            
            if (tests.length > 0) {
                if (!fileItem) {
                    fileItem = this.testController.createTestItem(
                        fileId,
                        uri.path.split('/').pop() || 'test',
                        uri
                    );
                    this.testController.items.add(fileItem);
                    this.fileTestData.set(fileId, fileItem);
                }

                // Clear existing children
                fileItem.children.replace([]);

                // Add test items
                for (const test of tests) {
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

    private addTestItem(parent: vscode.TestItem, testData: any) {
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
        
        // Add icon based on test kind
        if (testData.kind === 'file') {
            testItem.description = 'Test File';
        } else if (testData.kind === 'suite') {
            testItem.description = 'Test Suite';
        } else {
            testItem.description = 'Test';
        }

        parent.children.add(testItem);

        // Recursively add children
        if (testData.children && testData.children.length > 0) {
            for (const child of testData.children) {
                this.addTestItem(testItem, child);
            }
        }
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

    dispose() {
        this.testController.dispose();
    }
}