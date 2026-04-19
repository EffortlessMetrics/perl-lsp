/**
 * BddTestAdapter - BDD Test Explorer integration
 *
 * This is a STUB implementation for RED testing.
 * The actual implementation will be added by the code builder.
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { spawn } from 'child_process';

// Re-export types from parser for convenience
export type { OutlineNode } from './gherkin/parser';
import { buildOutline, expandScenarioOutline, getBackgroundForScenario, OutlineNode } from './gherkin/parser';

export interface TAPResult {
    scenario: string;
    ok: boolean;
    duration?: number;
    message?: string;
    line?: number;
}

/**
 * BDD Test Adapter for Test::BDD::Cucumber .feature files.
 *
 * Discovers .feature files in the workspace, parses Scenario/Scenario Outline
 * structures, and runs them via prove/yath/pdc, mapping TAP output to
 * VS Code test results.
 */
export class BddTestAdapter implements vscode.Disposable {
    private controller: vscode.TestController;
    private runProfile: vscode.TestRunProfile;
    private featureWatcher: vscode.FileSystemWatcher;
    private disposables: vscode.Disposable[] = [];
    private fileItems = new Map<string, vscode.TestItem>();

    constructor() {
        // Create test controller
        this.controller = vscode.tests.createTestController(
            'bddTestController',
            'BDD Tests'
        );

        // Create run profile
        this.runProfile = this.controller.createRunProfile(
            'Run',
            vscode.TestRunProfileKind.Run,
            (request, token) => this.runHandler(request, token),
            true
        );

        // Set up refresh handler
        this.controller.refreshHandler = () => this.discoverFeatureFiles();

        // Create file watcher for .feature files
        const config = vscode.workspace.getConfiguration('perl');
        const pattern = config.get<string>('bddFeaturePattern', '**/*.feature');
        this.featureWatcher = vscode.workspace.createFileSystemWatcher(pattern);

        this.featureWatcher.onDidCreate(uri => this.onFeatureCreated(uri));
        this.featureWatcher.onDidChange(uri => this.onFeatureChanged(uri));
        this.featureWatcher.onDidDelete(uri => this.onFeatureDeleted(uri));

        this.disposables.push(this.featureWatcher);

        // Initial discovery
        void this.discoverFeatureFiles();
    }

    // -- Discovery -----------------------------------------------------------

    async discoverFeatureFiles(): Promise<void> {
        this.controller.items.replace([]);
        this.fileItems.clear();

        const config = vscode.workspace.getConfiguration('perl');
        const pattern = config.get<string>('bddFeaturePattern', '**/*.feature');

        const files = await vscode.workspace.findFiles(pattern, '{**/node_modules/**,**/blib/**}');
        for (const uri of files) {
            await this.discoverFileTests(uri);
        }
    }

    private async discoverFileTests(uri: vscode.Uri): Promise<void> {
        const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
        const relativePath = workspaceFolder
            ? path.relative(workspaceFolder.uri.fsPath, uri.fsPath)
            : path.basename(uri.fsPath);

        const fileId = uri.toString();
        let fileItem = this.fileItems.get(fileId);

        if (!fileItem) {
            fileItem = this.controller.createTestItem(fileId, relativePath, uri);
            this.controller.items.add(fileItem);
            this.fileItems.set(fileId, fileItem);
        } else {
            fileItem.children.replace([]);
        }

        // Parse feature file
        const outline = await this.parseFeatureFile(uri);
        this.createTestItems(outline, fileItem);
    }

    async parseFeatureFile(uri: vscode.Uri): Promise<OutlineNode[]> {
        try {
            const doc = await vscode.workspace.openTextDocument(uri);
            return buildOutline(doc.getText());
        } catch {
            return [];
        }
    }

    private createTestItems(nodes: OutlineNode[], parent: vscode.TestItem): void {
        for (const node of nodes) {
            if (node.kind === 'scenario') {
                // Check if this is a Scenario Outline that needs expansion
                if (node.name.startsWith('Scenario Outline:') || node.name.startsWith('Scenario Template:')) {
                    const expanded = expandScenarioOutline(node);
                    for (const expandedNode of expanded) {
                        const child = this.controller.createTestItem(
                            `${parent.id}::${expandedNode.name}`,
                            expandedNode.name,
                            parent.uri
                        );
                        child.range = new vscode.Range(expandedNode.line, 0, expandedNode.line, 0);
                        parent.children.add(child);
                    }
                } else {
                    // Regular Scenario
                    const child = this.controller.createTestItem(
                        `${parent.id}::${node.name}`,
                        node.name,
                        parent.uri
                    );
                    child.range = new vscode.Range(node.line, 0, node.line, 0);
                    parent.children.add(child);
                }
            }

            // Recurse into children
            if (node.children.length > 0) {
                this.createTestItems(node.children, parent);
            }
        }
    }

    private onFeatureCreated(uri: vscode.Uri): void {
        void this.discoverFileTests(uri);
    }

    private onFeatureChanged(uri: vscode.Uri): void {
        void this.discoverFileTests(uri);
    }

    private onFeatureDeleted(uri: vscode.Uri): void {
        const fileId = uri.toString();
        this.controller.items.delete(fileId);
        this.fileItems.delete(fileId);
    }

    // -- Run handler ---------------------------------------------------------

    async runHandler(
        request: vscode.TestRunRequest,
        token: vscode.CancellationToken
    ): Promise<void> {
        const run = this.controller.createTestRun(request);

        // Collect tests to run
        const testsToRun = request.include ?? this.gatherAllItems();

        // Group by file
        const byFile = new Map<string, vscode.TestItem[]>();
        for (const item of testsToRun) {
            if (token.isCancellationRequested) break;
            if (item.uri) {
                const existing = byFile.get(item.uri.fsPath) ?? [];
                existing.push(item);
                byFile.set(item.uri.fsPath, existing);
            }
        }

        // Run each file
        for (const [filePath, items] of byFile) {
            if (token.isCancellationRequested) break;

            for (const item of items) {
                run.started(item);
            }

            await this.executeTests(filePath, items, run, token);
        }

        run.end();
    }

    private gatherAllItems(): vscode.TestItem[] {
        const items: vscode.TestItem[] = [];
        this.controller.items.forEach(item => items.push(item));
        return items;
    }

    // -- Test execution -------------------------------------------------------

    private async executeTests(
        filePath: string,
        items: vscode.TestItem[],
        run: vscode.TestRun,
        token: vscode.CancellationToken
    ): Promise<void> {
        const workspaceFolder = vscode.workspace.getWorkspaceFolder(
            vscode.Uri.file(filePath)
        );
        const cwd = workspaceFolder?.uri.fsPath ?? path.dirname(filePath);

        const runner = await this.detectRunner();
        if (!runner) {
            for (const item of items) {
                run.errored(item, new vscode.TestMessage(
                    'No BDD test runner available. Install prove, yath, or pdc.'
                ));
            }
            return;
        }

        return new Promise<void>(resolve => {
            const startTime = Date.now();
            // detectRunner returns { command, args } already
            const { command, args } = runner;

            const proc = spawn(command, args, {
                cwd,
                env: { ...process.env, HARNESS_ACTIVE: '1' }
            });

            let stdout = '';
            let stderr = '';

            proc.stdout.on('data', (data: Buffer) => { stdout += data.toString(); });
            proc.stderr.on('data', (data: Buffer) => { stderr += data.toString(); });

            const killOnCancel = token.onCancellationRequested(() => {
                proc.kill('SIGTERM');
            });

            proc.on('close', (code) => {
                killOnCancel.dispose();
                const duration = Date.now() - startTime;

                const results = this.parseTAPOutput(stdout);

                // Map results to test items
                for (const item of items) {
                    const result = results.find(r =>
                        item.label.includes(r.scenario) || r.scenario.includes(item.label)
                    );

                    if (result) {
                        if (result.ok) {
                            run.passed(item, result.duration ?? duration);
                        } else {
                            const message = new vscode.TestMessage(result.message ?? 'Test failed');
                            if (result.line !== undefined && item.uri) {
                                message.location = new vscode.Location(item.uri, new vscode.Position(result.line, 0));
                            }
                            run.failed(item, message, result.duration ?? duration);
                        }
                    } else {
                        run.skipped(item);
                    }
                }

                resolve();
            });

            proc.on('error', (err) => {
                killOnCancel.dispose();
                for (const item of items) {
                    run.errored(item, new vscode.TestMessage(
                        `Failed to run BDD tests: ${err.message}`
                    ));
                }
                resolve();
            });
        });
    }

    private async detectRunner(): Promise<{ command: string; args: string[] } | null> {
        const config = vscode.workspace.getConfiguration('perl');
        const preference = config.get<string>('bddRunner', 'auto');

        // Check in order of preference based on config
        const runners = ['prove', 'yath', 'pdc'];

        if (preference !== 'auto') {
            const cmd = await this.tryRunCommand(preference, ['--version']);
            if (cmd) {
                return { command: preference, args: this.getRunnerArgs(preference, 'features/') };
            }
            return null;
        }

        // Auto-detect: try each runner
        for (const runner of runners) {
            const cmd = await this.tryRunCommand(runner, ['--version']);
            if (cmd) {
                return { command: runner, args: this.getRunnerArgs(runner, 'features/') };
            }
        }

        return null;
    }

    private async tryRunCommand(command: string, args: string[]): Promise<string | null> {
        return new Promise<string | null>(resolve => {
            const proc = spawn(command, args, { timeout: 5000 });
            let output = '';
            proc.stdout.on('data', (data: Buffer) => { output += data.toString(); });
            proc.on('close', (code) => {
                resolve(code === 0 ? output : null);
            });
            proc.on('error', () => resolve(null));
            setTimeout(() => {
                proc.kill();
                resolve(null);
            }, 5000);
        });
    }

    private getRunnerArgs(runner: string, target: string): string[] {
        switch (runner) {
            case 'prove':
                return ['-lvr', target];
            case 'yath':
                return [target];
            case 'pdc':
                return ['run', target];
            default:
                return ['-lvr', target];
        }
    }

    // -- TAP parsing ----------------------------------------------------------

    parseTAPOutput(output: string): TAPResult[] {
        const results: TAPResult[] = [];
        const lines = output.split('\n');

        let currentSubtest: string | null = null;
        let currentOk: boolean = true;
        let diagnosticLines: string[] = [];

        for (const line of lines) {
            // Subtest start
            const subtestMatch = /^\s*#\s*Subtest:\s*(.+)/.exec(line);
            if (subtestMatch) {
                currentSubtest = subtestMatch[1].trim();
                currentOk = true;
                diagnosticLines = [];
                continue;
            }

            // Collect diagnostic lines
            if (currentSubtest && /^\s{4,}#/.test(line)) {
                diagnosticLines.push(line.trim());
                continue;
            }

            // Subtest result
            if (currentSubtest) {
                const okMatch = /^ok \d+\s*-\s*(.*)/.exec(line);
                const notOkMatch = /^not ok \d+\s*-\s*(.*)/.exec(line);

                if (okMatch && okMatch[1].trim() === currentSubtest) {
                    results.push({
                        scenario: currentSubtest,
                        ok: true,
                        message: diagnosticLines.join('\n'),
                    });
                    currentSubtest = null;
                    diagnosticLines = [];
                } else if (notOkMatch && notOkMatch[1].trim() === currentSubtest) {
                    results.push({
                        scenario: currentSubtest,
                        ok: false,
                        message: diagnosticLines.join('\n') || `Test "${currentSubtest}" failed`,
                    });
                    currentSubtest = null;
                    diagnosticLines = [];
                }
            }

            // Top-level results (non-verbose mode)
            const topOkMatch = /^ok \d+\s*-\s*(.+)/.exec(line);
            const topNotOkMatch = /^not ok \d+\s*-\s*(.+)/.exec(line);

            if (topOkMatch && !currentSubtest) {
                results.push({
                    scenario: topOkMatch[1].trim(),
                    ok: true,
                });
            } else if (topNotOkMatch && !currentSubtest) {
                results.push({
                    scenario: topNotOkMatch[1].trim(),
                    ok: false,
                    message: `Test "${topNotOkMatch[1].trim()}" failed`,
                });
            }
        }

        return results;
    }

    // -- Public API ----------------------------------------------------------

    dispose(): void {
        this.controller.dispose();
        for (const d of this.disposables) {
            d.dispose();
        }
    }
}
