import * as vscode from 'vscode';
import * as path from 'path';
import { spawn } from 'child_process';

/**
 * Perl Test Explorer integration.
 *
 * Discovers `.t` test files in the workspace, parses `subtest` blocks,
 * and runs them via `prove -v`, mapping TAP output to VSCode test results.
 */

interface SubtestInfo {
    name: string;
    line: number;
}

// Matches: subtest 'name' => sub {   or   subtest "name" => sub {
// Also matches: subtest 'name', sub {
const SUBTEST_RE = /^\s*subtest\s+(['"])(.*?)\1\s*(?:=>|,)\s*sub\s*\{/;

export class PerlTestAdapter implements vscode.Disposable {
    private testController: vscode.TestController;
    private disposables: vscode.Disposable[] = [];
    private fileItems = new Map<string, vscode.TestItem>();

    constructor() {
        this.testController = vscode.tests.createTestController(
            'perlTestController',
            'Perl Tests'
        );

        this.testController.createRunProfile(
            'Run',
            vscode.TestRunProfileKind.Run,
            (request, token) => this.runHandler(request, token),
            true
        );

        this.testController.refreshHandler = () => this.discoverAllTests();

        // File system watcher for .t files
        const watcher = vscode.workspace.createFileSystemWatcher('**/*.t');
        watcher.onDidCreate(uri => this.discoverFileTests(uri));
        watcher.onDidChange(uri => this.discoverFileTests(uri));
        watcher.onDidDelete(uri => this.removeFile(uri));
        this.disposables.push(watcher);

        // Re-parse on document save (picks up new subtests)
        const saveListener = vscode.workspace.onDidSaveTextDocument(doc => {
            if (doc.uri.fsPath.endsWith('.t')) {
                void this.discoverFileTests(doc.uri);
            }
        });
        this.disposables.push(saveListener);

        // Initial discovery
        void this.discoverAllTests();
    }

    // -- Discovery -------------------------------------------------------

    private async discoverAllTests(): Promise<void> {
        this.testController.items.replace([]);
        this.fileItems.clear();

        const files = await vscode.workspace.findFiles('**/*.t', '{**/node_modules/**,**/blib/**}');
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
            fileItem = this.testController.createTestItem(fileId, relativePath, uri);
            this.testController.items.add(fileItem);
            this.fileItems.set(fileId, fileItem);
        } else {
            fileItem.children.replace([]);
        }

        // Parse subtests from file content
        const subtests = await this.parseSubtests(uri);
        for (const st of subtests) {
            const child = this.testController.createTestItem(
                `${fileId}::${st.name}`,
                st.name,
                uri
            );
            child.range = new vscode.Range(st.line, 0, st.line, 0);
            fileItem.children.add(child);
        }
    }

    private async parseSubtests(uri: vscode.Uri): Promise<SubtestInfo[]> {
        try {
            const doc = await vscode.workspace.openTextDocument(uri);
            const subtests: SubtestInfo[] = [];

            for (let i = 0; i < doc.lineCount; i++) {
                const line = doc.lineAt(i).text;
                const match = SUBTEST_RE.exec(line);
                if (match) {
                    subtests.push({ name: match[2], line: i });
                }
            }

            return subtests;
        } catch {
            return [];
        }
    }

    private removeFile(uri: vscode.Uri): void {
        const fileId = uri.toString();
        this.testController.items.delete(fileId);
        this.fileItems.delete(fileId);
    }

    // -- Run handler -----------------------------------------------------

    private async runHandler(
        request: vscode.TestRunRequest,
        token: vscode.CancellationToken
    ): Promise<void> {
        const run = this.testController.createTestRun(request);

        // Collect files to run. If no specific tests requested, run all.
        const testsToRun = request.include ?? this.gatherAllItems();

        // Group by file so we run prove once per file
        const byFile = new Map<string, { fileItem: vscode.TestItem; subtests: vscode.TestItem[] }>();

        for (const item of testsToRun) {
            if (token.isCancellationRequested) { break; }

            if (item.uri && item.children.size > 0) {
                // This is a file-level item
                const children: vscode.TestItem[] = [];
                item.children.forEach(c => children.push(c));
                byFile.set(item.uri.fsPath, { fileItem: item, subtests: children });
            } else if (item.uri) {
                // This is a subtest -- find parent file
                const fsPath = item.uri.fsPath;
                const entry = byFile.get(fsPath);
                if (entry) {
                    entry.subtests.push(item);
                } else {
                    // Find the file item for this subtest
                    const fileId = item.uri.toString();
                    const fileItem = this.fileItems.get(fileId);
                    if (fileItem) {
                        byFile.set(fsPath, { fileItem, subtests: [item] });
                    }
                }
            }
        }

        for (const [filePath, { fileItem, subtests }] of byFile) {
            if (token.isCancellationRequested) { break; }

            run.started(fileItem);
            for (const st of subtests) { run.started(st); }

            await this.runProve(filePath, fileItem, subtests, run, token);
        }

        run.end();
    }

    private gatherAllItems(): vscode.TestItem[] {
        const items: vscode.TestItem[] = [];
        this.testController.items.forEach(item => items.push(item));
        return items;
    }

    // -- prove execution & TAP parsing -----------------------------------

    private async runProve(
        filePath: string,
        fileItem: vscode.TestItem,
        subtests: vscode.TestItem[],
        run: vscode.TestRun,
        token: vscode.CancellationToken
    ): Promise<void> {
        const workspaceFolder = vscode.workspace.getWorkspaceFolder(fileItem.uri!);
        const cwd = workspaceFolder?.uri.fsPath ?? path.dirname(filePath);

        return new Promise<void>(resolve => {
            const startTime = Date.now();
            const proc = spawn('prove', ['-v', '--nocolor', filePath], {
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

                const tapResults = this.parseTapOutput(stdout);
                const subtestResults = this.parseSubtestResults(stdout);

                // Map subtest results to test items
                for (const st of subtests) {
                    const stName = st.label;
                    const result = subtestResults.get(stName);

                    if (result !== undefined) {
                        if (result.ok) {
                            run.passed(st, result.duration);
                        } else {
                            run.failed(st, new vscode.TestMessage(
                                result.diagnostic || `Subtest "${stName}" failed`
                            ), result.duration);
                        }
                    } else {
                        // Subtest was not in output -- mark skipped
                        run.skipped(st);
                    }
                }

                // File-level result
                if (code === 0 && tapResults.failed === 0) {
                    run.passed(fileItem, duration);
                } else {
                    const message = new vscode.TestMessage(
                        stderr.trim() ||
                        `${tapResults.failed} of ${tapResults.total} tests failed` +
                        (tapResults.bailOut ? ` (Bail out! ${tapResults.bailOut})` : '')
                    );
                    if (fileItem.uri) {
                        message.location = new vscode.Location(fileItem.uri, new vscode.Position(0, 0));
                    }
                    run.failed(fileItem, message, duration);
                }

                resolve();
            });

            proc.on('error', (err: Error) => {
                killOnCancel.dispose();
                run.errored(fileItem, new vscode.TestMessage(
                    `Failed to run prove: ${err.message}. Is prove installed?`
                ));
                for (const st of subtests) {
                    run.errored(st, new vscode.TestMessage('prove not available'));
                }
                resolve();
            });
        });
    }

    // -- TAP output parsing ----------------------------------------------

    /**
     * Parse top-level TAP summary from prove output.
     */
    private parseTapOutput(output: string): { total: number; passed: number; failed: number; bailOut: string | null } {
        const lines = output.split('\n');
        let total = 0;
        let passed = 0;
        let failed = 0;
        let bailOut: string | null = null;

        for (const line of lines) {
            // Only match top-level TAP lines (not indented subtest TAP)
            if (/^ok \d+/.test(line)) {
                total++;
                passed++;
            } else if (/^not ok \d+/.test(line)) {
                total++;
                failed++;
            } else if (/^Bail out!\s*(.*)/.test(line)) {
                const m = /^Bail out!\s*(.*)/.exec(line);
                bailOut = m ? m[1] : '';
            } else if (/^1\.\.(\d+)/.test(line)) {
                const m = /^1\.\.(\d+)/.exec(line);
                if (m) { total = Math.max(total, parseInt(m[1], 10)); }
            }
        }

        return { total, passed, failed, bailOut };
    }

    /**
     * Parse subtest results from verbose prove TAP output.
     *
     * prove -v outputs subtests like:
     *   # Subtest: constructor
     *       ok 1 - new() returns object
     *       1..1
     *   ok 1 - constructor
     *
     * or on failure:
     *   # Subtest: methods
     *       not ok 1 - method works
     *       #   Failed test 'method works'
     *       1..1
     *   not ok 2 - methods
     */
    private parseSubtestResults(output: string): Map<string, { ok: boolean; diagnostic: string; duration: number }> {
        const results = new Map<string, { ok: boolean; diagnostic: string; duration: number }>();
        const lines = output.split('\n');

        let currentSubtest: string | null = null;
        let diagnosticLines: string[] = [];

        for (const line of lines) {
            // Detect start of subtest
            const subtestStart = /^\s*#\s*Subtest:\s*(.+)/.exec(line);
            if (subtestStart) {
                currentSubtest = subtestStart[1].trim();
                diagnosticLines = [];
                continue;
            }

            // Collect indented diagnostic lines within a subtest
            if (currentSubtest && /^\s{4,}#/.test(line)) {
                diagnosticLines.push(line.trim());
                continue;
            }

            // Detect subtest result line (top-level ok/not ok with subtest name)
            if (currentSubtest) {
                const okMatch = /^ok \d+\s*-\s*(.*)/.exec(line);
                const notOkMatch = /^not ok \d+\s*-\s*(.*)/.exec(line);

                if (okMatch && okMatch[1].trim() === currentSubtest) {
                    results.set(currentSubtest, {
                        ok: true,
                        diagnostic: diagnosticLines.join('\n'),
                        duration: 0
                    });
                    currentSubtest = null;
                    diagnosticLines = [];
                } else if (notOkMatch && notOkMatch[1].trim() === currentSubtest) {
                    results.set(currentSubtest, {
                        ok: false,
                        diagnostic: diagnosticLines.join('\n') || `Subtest "${currentSubtest}" failed`,
                        duration: 0
                    });
                    currentSubtest = null;
                    diagnosticLines = [];
                }
            }
        }

        return results;
    }

    // -- Public API -------------------------------------------------------

    public async runFileTests(uri: vscode.Uri): Promise<void> {
        const fileId = uri.toString();
        const fileItem = this.fileItems.get(fileId);

        if (fileItem) {
            const request = new vscode.TestRunRequest([fileItem]);
            const tokenSource = new vscode.CancellationTokenSource();
            try {
                await this.runHandler(request, tokenSource.token);
            } finally {
                tokenSource.dispose();
            }
        } else {
            vscode.window.showWarningMessage('No tests found in this file. Try refreshing the test explorer.');
        }
    }

    dispose(): void {
        this.testController.dispose();
        for (const d of this.disposables) {
            d.dispose();
        }
    }
}
