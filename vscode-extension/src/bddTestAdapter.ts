/**
 * BDD Test Explorer integration for VS Code.
 *
 * This module implements VS Code Test Explorer integration for Test::BDD::Cucumber
 * .feature files, allowing developers to run BDD tests directly from VS Code with
 * results displayed in the Test Explorer panel.
 *
 * AC1: Feature file discovery
 * AC2: Scenario as individual tests
 * AC3: Scenario Outline expansion
 * AC4: Background step execution
 * AC5: Test execution via detected runner
 * AC6: Pass/fail status display
 * AC7: Failed test navigation
 * AC8: Configuration option for runner preference
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { spawn, execSync } from 'child_process';
import type { OutlineNode } from './gherkin/parser';
import {
    buildOutline,
    getBackgroundForScenario
} from './gherkin/parser';

/**
 * Supported BDD test runners in preference order.
 */
type BddRunner = 'prove' | 'yath' | 'pdc' | 'auto';

/**
 * TAP (Test Anything Protocol) parsing result.
 * @internal - Reserved for future TAP parsing implementation (AC6)
 */
interface _TapResult {
    total: number;
    passed: number;
    failed: number;
    bailOut: string | null;
}

/**
 * Parsed subtest result from TAP output.
 */
interface SubtestResult {
    ok: boolean;
    diagnostic: string;
    duration: number;
}

/**
 * Maps scenario names to their results for a single feature file.
 */
type ScenarioResults = Map<string, SubtestResult>;

/**
 * BDD Test Adapter for VS Code Test Explorer.
 *
 * Discovers .feature files in the workspace, parses them to extract scenarios,
 * and runs tests via the detected BDD runner (prove, yath, or pdc).
 */
export class BddTestAdapter implements vscode.Disposable {
    private testController: vscode.TestController;
    private disposables: vscode.Disposable[] = [];
    private featureItems = new Map<string, vscode.TestItem>();
    private fileContents = new Map<string, string>();

    constructor() {
        // Create the test controller with "BDD Tests" label
        this.testController = vscode.tests.createTestController(
            'bddTestController',
            'BDD Tests'
        );

        // Create a run profile for test execution
        this.testController.createRunProfile(
            'Run',
            vscode.TestRunProfileKind.Run,
            (request, token) => this.runHandler(request, token),
            true
        );

        // Initialize with empty test collection
        this.testController.items.replace([]);

        // File system watcher for .feature files
        const watcher = vscode.workspace.createFileSystemWatcher('**/*.feature');
        watcher.onDidCreate(uri => void this.discoverFeatureFile(uri));
        watcher.onDidChange(uri => void this.discoverFeatureFile(uri));
        watcher.onDidDelete(uri => this.removeFeatureFile(uri));
        this.disposables.push(watcher);

        // Initial discovery
        void this.discoverFeatureFiles();
    }

    // -- Discovery -----------------------------------------------------------

    /**
     * Discover all .feature files in the workspace.
     */
    async discoverFeatureFiles(): Promise<void> {
        this.testController.items.replace([]);
        this.featureItems.clear();
        this.fileContents.clear();

        const pattern = this.getFeaturePattern();
        const files = await vscode.workspace.findFiles(pattern, '{**/node_modules/**,**/.git/**}');

        for (const uri of files) {
            await this.discoverFeatureFile(uri);
        }
    }

    /**
     * Get the glob pattern for discovering feature files.
     */
    private getFeaturePattern(): string {
        const config = vscode.workspace.getConfiguration('perl');
        return config.get<string>('bddFeaturePattern', '**/*.feature');
    }

    /**
     * Discover or update a single feature file.
     */
    private async discoverFeatureFile(uri: vscode.Uri): Promise<void> {
        const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
        const relativePath = workspaceFolder
            ? path.relative(workspaceFolder.uri.fsPath, uri.fsPath)
            : path.basename(uri.fsPath);

        const fileId = uri.toString();
        let featureItem = this.featureItems.get(fileId);

        if (!featureItem) {
            featureItem = this.testController.createTestItem(fileId, relativePath, uri);
            this.testController.items.add(featureItem);
            this.featureItems.set(fileId, featureItem);
        } else {
            featureItem.children.replace([]);
        }

        // Parse the feature file
        const outline = await this.parseFeatureFile(uri);
        if (!outline) {
            return;
        }

        // Create test items for scenarios
        this.createScenarioTestItems(featureItem, outline, uri);
    }

    /**
     * Parse a feature file into an outline tree.
     */
    private async parseFeatureFile(uri: vscode.Uri): Promise<OutlineNode[] | null> {
        try {
            const doc = await vscode.workspace.openTextDocument(uri);
            const text = doc.getText();
            this.fileContents.set(uri.toString(), text);
            return buildOutline(text);
        } catch {
            return null;
        }
    }

    /**
     * Create test items for scenarios in a feature file.
     * Handles regular scenarios and expands Scenario Outlines.
     */
    private createScenarioTestItems(
        featureItem: vscode.TestItem,
        outline: OutlineNode[],
        uri: vscode.Uri
    ): void {
        for (const node of outline) {
            // The outline contains Feature nodes as roots
            // We need to iterate over the Feature's children to find scenarios
            if (node.kind === 'feature') {
                for (const child of node.children) {
                    if (child.kind === 'background') {
                        // Background nodes are tracked but not added as test items directly
                        continue;
                    }

                    if (child.kind === 'scenario') {
                        this.createScenarioTestItem(featureItem, child, uri);
                    }
                }
            }
        }
    }

    /**
     * Create a test item for a single scenario.
     */
    private createScenarioTestItem(
        featureItem: vscode.TestItem,
        scenarioNode: OutlineNode,
        uri: vscode.Uri
    ): void {
        // Find the applicable background for this scenario
        const background = getBackgroundForScenario(scenarioNode, buildOutline(
            this.fileContents.get(uri.toString()) || ''
        ));

        // Build the scenario ID - include feature path and scenario line
        const scenarioId = `${uri.toString()}::${scenarioNode.name}::${scenarioNode.line}`;

        const scenarioItem = this.testController.createTestItem(
            scenarioId,
            scenarioNode.name,
            uri
        );

        // Set the range to the scenario line
        scenarioItem.range = new vscode.Range(
            scenarioNode.line,
            0,
            scenarioNode.line,
            0
        );

        // Add background info as description if present
        if (background) {
            scenarioItem.description = `Background: ${background.name}`;
        }

        featureItem.children.add(scenarioItem);
    }

    /**
     * Remove a feature file from the test collection.
     */
    private removeFeatureFile(uri: vscode.Uri): void {
        const fileId = uri.toString();
        this.testController.items.delete(fileId);
        this.featureItems.delete(fileId);
        this.fileContents.delete(fileId);
    }

    // -- Run handler -------------------------------------------------------

    /**
     * Handle a test run request.
     */
    private async runHandler(
        request: vscode.TestRunRequest,
        token: vscode.CancellationToken
    ): Promise<void> {
        const run = this.testController.createTestRun(request);

        // Collect tests to run - if none specified, run all
        const testsToRun = request.include ?? this.gatherAllTestItems();

        // Group by file
        const byFile = new Map<string, vscode.TestItem[]>();

        for (const item of testsToRun) {
            if (item.uri) {
                const fileId = item.uri.toString();
                const existing = byFile.get(fileId) ?? [];
                existing.push(item);
                byFile.set(fileId, existing);
            }
        }

        // Run tests for each file
        for (const [fileId, items] of byFile) {
            if (token.isCancellationRequested) {
                break;
            }

            const fileItem = this.featureItems.get(fileId);
            if (fileItem) {
                run.started(fileItem);
            }

            for (const item of items) {
                run.started(item);
            }

            await this.executeTests(fileId, items, run, token);
        }

        run.end();
    }

    /**
     * Gather all test items from the test controller.
     */
    private gatherAllTestItems(): vscode.TestItem[] {
        const items: vscode.TestItem[] = [];
        this.testController.items.forEach(item => items.push(item));
        return items;
    }

    // -- Test execution --------------------------------------------------

    /**
     * Execute BDD tests via the detected runner.
     */
    private async executeTests(
        fileId: string,
        testItems: vscode.TestItem[],
        run: vscode.TestRun,
        token: vscode.CancellationToken
    ): Promise<void> {
        const uri = this.featureItems.get(fileId)?.uri;
        if (!uri) {
            return;
        }

        const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
        const cwd = workspaceFolder?.uri.fsPath ?? path.dirname(uri.fsPath);

        // Detect or use configured runner
        const runner = await this.detectRunner();
        const runnerCommand = this.getRunnerCommand(runner);

        if (!runnerCommand) {
            for (const item of testItems) {
                run.errored(item, new vscode.TestMessage(
                    'No BDD test runner found. Install prove, yath, or pdc.'
                ));
            }
            return;
        }

        return new Promise<void>(resolve => {
            const startTime = Date.now();

            // Build the runner command and arguments
            // For prove: prove --nocolor -I lib t/features/*.feature
            // For yath: yath --nocolor t/features/*.feature
            // For pdc: pdc t/features/*.feature
            const runnerArgs = this.getRunnerArgs(runner, uri.fsPath);

            const proc = spawn(runnerCommand, runnerArgs, {
                cwd,
                env: { ...process.env, HARNESS_ACTIVE: '1' },
            });

            let stdout = '';
            let _stderr = '';

            proc.stdout.on('data', (data: Buffer) => {
                stdout += data.toString();
            });

            proc.stderr.on('data', (data: Buffer) => {
                _stderr += data.toString();
            });

            const killOnCancel = token.onCancellationRequested(() => {
                proc.kill('SIGTERM');
            });

            proc.on('close', (_code) => {
                killOnCancel.dispose();
                const duration = Date.now() - startTime;

                // Parse TAP output and map results to test items
                const scenarioResults = this.parseSubtestResults(stdout);
                this.mapResultsToTestItems(testItems, scenarioResults, run, duration);

                resolve();
            });

            proc.on('error', (err: Error) => {
                killOnCancel.dispose();
                for (const item of testItems) {
                    run.errored(item, new vscode.TestMessage(
                        `Failed to run ${runner}: ${err.message}`
                    ));
                }
                resolve();
            });
        });
    }

    /**
     * Detect the available BDD runner.
     */
    private async detectRunner(): Promise<BddRunner> {
        const config = vscode.workspace.getConfiguration('perl');
        const preferredRunner = config.get<BddRunner>('bddRunner', 'auto');

        if (preferredRunner !== 'auto') {
            // Verify the preferred runner is actually available
            if (this.isRunnerAvailable(preferredRunner)) {
                return preferredRunner;
            }
            // Fall through to auto-detection
        }

        // Auto-detect: check runners in order of preference
        const runners: BddRunner[] = ['prove', 'yath', 'pdc'];
        for (const runner of runners) {
            if (this.isRunnerAvailable(runner)) {
                return runner;
            }
        }

        return 'auto';
    }

    /**
     * Check if a runner command is available.
     */
    private isRunnerAvailable(runner: string): boolean {
        try {
            execSync(`which ${runner}`, { stdio: 'ignore' });
            return true;
        } catch {
            return false;
        }
    }

    /**
     * Get the command path for a runner.
     */
    private getRunnerCommand(runner: BddRunner): string | null {
        if (runner === 'auto') {
            return null; // Should have been resolved before calling this
        }
        try {
            return execSync(`which ${runner}`, { encoding: 'utf8' }).trim();
        } catch {
            return null;
        }
    }

    /**
     * Get the arguments for a runner command.
     */
    private getRunnerArgs(runner: BddRunner, featurePath: string): string[] {
        switch (runner) {
            case 'prove':
                return ['--nocolor', featurePath];
            case 'yath':
                return ['--nocolor', featurePath];
            case 'pdc':
                return [featurePath];
            default:
                return [featurePath];
        }
    }

    // -- TAP output parsing -----------------------------------------------

    /**
     * Parse subtest results from TAP output.
     * TAP output from BDD runners typically looks like:
     *   # Subtest: scenario name
     *       ok 1 - step description
     *       1..1
     *   ok 1 - scenario name
     */
    private parseSubtestResults(output: string): ScenarioResults {
        const results = new Map<string, SubtestResult>();
        const lines = output.split('\n');

        let currentScenario: string | null = null;
        let diagnosticLines: string[] = [];

        for (const line of lines) {
            // Detect start of subtest/scenario
            const subtestMatch = /^\s*#\s*Subtest:\s*(.+)/.exec(line);
            if (subtestMatch) {
                currentScenario = subtestMatch[1]!.trim();
                diagnosticLines = [];
                continue;
            }

            // Collect diagnostic lines
            if (currentScenario && /^\s{4,}#/.test(line)) {
                diagnosticLines.push(line.trim());
                continue;
            }

            // Detect scenario result line
            if (currentScenario) {
                const okMatch = /^ok \d+\s*-\s*(.+)/.exec(line);
                const notOkMatch = /^not ok \d+\s*-\s*(.+)/.exec(line);

                if (okMatch && okMatch[1]!.trim() === currentScenario) {
                    results.set(currentScenario, {
                        ok: true,
                        diagnostic: diagnosticLines.join('\n'),
                        duration: 0,
                    });
                    currentScenario = null;
                    diagnosticLines = [];
                } else if (notOkMatch && notOkMatch[1]!.trim() === currentScenario) {
                    results.set(currentScenario, {
                        ok: false,
                        diagnostic: diagnosticLines.join('\n') || `Scenario "${currentScenario}" failed`,
                        duration: 0,
                    });
                    currentScenario = null;
                    diagnosticLines = [];
                }
            }
        }

        return results;
    }

    /**
     * Map parsed TAP results to test items and update their status.
     */
    private mapResultsToTestItems(
        testItems: vscode.TestItem[],
        results: ScenarioResults,
        run: vscode.TestRun,
        duration: number
    ): void {
        for (const item of testItems) {
            // Extract scenario name from test item label
            const scenarioName = item.label;
            const result = results.get(scenarioName);

            if (result !== undefined) {
                if (result.ok) {
                    run.passed(item, duration);
                } else {
                    // Create a test message with location pointing to the scenario
                    const message = new vscode.TestMessage(result.diagnostic);
                    if (item.uri && item.range) {
                        message.location = new vscode.Location(
                            item.uri,
                            new vscode.Range(
                                item.range.start.line,
                                0,
                                item.range.start.line,
                                0
                            )
                        );
                    }
                    run.failed(item, message, duration);
                }
            } else {
                // Scenario was not in output - mark skipped
                run.skipped(item);
            }
        }
    }

    // -- Public API -------------------------------------------------------

    /**
     * Run tests for a specific feature file.
     */
    public async runFeatureTests(uri: vscode.Uri): Promise<void> {
        const fileId = uri.toString();
        const fileItem = this.featureItems.get(fileId);

        if (fileItem) {
            const request = new vscode.TestRunRequest([fileItem]);
            const tokenSource = new vscode.CancellationTokenSource();
            try {
                await this.runHandler(request, tokenSource.token);
            } finally {
                tokenSource.dispose();
            }
        }
    }

    /**
     * Dispose of the test adapter and release all resources.
     * Removes the test controller and all registered file watchers.
     */
    dispose(): void {
        this.testController.dispose();
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
    }
}
