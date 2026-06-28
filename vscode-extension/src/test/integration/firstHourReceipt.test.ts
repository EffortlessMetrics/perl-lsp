import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

interface MomentResult {
  completion: Record<string, unknown>;
  definition: Record<string, unknown>;
  diagnostics: Record<string, unknown>;
  hover: Record<string, unknown>;
  references: Record<string, unknown>;
}

function delay(ms: number): Promise<void> {
  return new Promise(resolve => {
    setTimeout(resolve, ms);
  });
}

async function withTimeout<T>(label: string, operation: PromiseLike<T>, timeoutMs: number): Promise<T> {
  let timeout: NodeJS.Timeout | undefined;
  const timeoutPromise = new Promise<never>((_, reject) => {
    timeout = setTimeout(() => {
      reject(new Error(`${label} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  });

  try {
    return await Promise.race([Promise.resolve(operation), timeoutPromise]);
  } finally {
    if (timeout) {
      clearTimeout(timeout);
    }
  }
}

function platformLabel(): string {
  switch (process.platform) {
    case 'win32': return 'windows';
    case 'darwin': return 'macos';
    case 'linux': return 'linux';
    default: return process.platform;
  }
}

function receiptsDir(): string {
  const root = process.env.PERL_LSP_SMOKE_RECEIPTS_DIR
    ?? path.resolve(__dirname, '..', '..', '..', '..', 'target', 'receipts', 'vscode-smoke');
  const sourceLabel = process.env.PERL_LSP_SMOKE_SOURCE_LABEL ?? 'first-hour';
  const dir = path.join(root, sourceLabel, platformLabel());
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

function writeFirstHourReceipt(receipt: Record<string, unknown>): void {
  fs.writeFileSync(
    path.join(receiptsDir(), 'first_hour_vscode_receipt.json'),
    JSON.stringify(receipt, null, 2),
  );
}

function walkFiles(root: string, maxEntries: number): string[] {
  const results: string[] = [];
  const stack = [root];
  while (stack.length > 0 && results.length < maxEntries) {
    const current = stack.pop();
    if (!current) {
      break;
    }
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        if (!['.git', 'blib', 'node_modules', 'target'].includes(entry.name)) {
          stack.push(fullPath);
        }
      } else {
        results.push(fullPath);
      }
      if (results.length >= maxEntries) {
        break;
      }
    }
  }
  return results;
}

function findPerlFiles(root: string): string[] {
  return walkFiles(root, 10_000).filter(file => /\.(?:pl|pm|t|psgi)$/i.test(file));
}

function sampleLabels(items: readonly vscode.CompletionItem[]): string[] {
  return items.slice(0, 10).map(item => {
    if (typeof item.label === 'string') {
      return item.label;
    }
    return item.label.label;
  });
}

async function collectProviderMoment(
  label: string,
  document: vscode.TextDocument,
  completionPosition: vscode.Position,
  symbolPosition: vscode.Position,
): Promise<MomentResult> {
  const startedAt = Date.now();
  const completionStart = Date.now();
  let completion: Record<string, unknown>;
  try {
    const list = await withTimeout(
      `${label} completion`,
      vscode.commands.executeCommand<vscode.CompletionList>(
        'vscode.executeCompletionItemProvider',
        document.uri,
        completionPosition,
      ),
      10_000,
    );
    completion = {
      status: 'ok',
      duration_ms: Date.now() - completionStart,
      item_count: list.items.length,
      sample_labels: sampleLabels(list.items),
    };
  } catch (error: unknown) {
    completion = {
      status: 'error',
      duration_ms: Date.now() - completionStart,
      message: error instanceof Error ? error.message : String(error),
    };
  }

  const hoverStart = Date.now();
  let hover: Record<string, unknown>;
  try {
    const hovers = await withTimeout(
      `${label} hover`,
      vscode.commands.executeCommand<vscode.Hover[]>(
        'vscode.executeHoverProvider',
        document.uri,
        symbolPosition,
      ),
      10_000,
    );
    hover = {
      status: 'ok',
      duration_ms: Date.now() - hoverStart,
      item_count: hovers.length,
    };
  } catch (error: unknown) {
    hover = {
      status: 'error',
      duration_ms: Date.now() - hoverStart,
      message: error instanceof Error ? error.message : String(error),
    };
  }

  const definitionStart = Date.now();
  let definition: Record<string, unknown>;
  try {
    const definitions = await withTimeout(
      `${label} definition`,
      vscode.commands.executeCommand<readonly vscode.Location[]>(
        'vscode.executeDefinitionProvider',
        document.uri,
        symbolPosition,
      ),
      10_000,
    );
    definition = {
      status: 'ok',
      duration_ms: Date.now() - definitionStart,
      location_count: definitions.length,
    };
  } catch (error: unknown) {
    definition = {
      status: 'error',
      duration_ms: Date.now() - definitionStart,
      message: error instanceof Error ? error.message : String(error),
    };
  }

  const referencesStart = Date.now();
  let references: Record<string, unknown>;
  try {
    const refs = await withTimeout(
      `${label} references`,
      vscode.commands.executeCommand<readonly vscode.Location[]>(
        'vscode.executeReferenceProvider',
        document.uri,
        symbolPosition,
      ),
      10_000,
    );
    references = {
      status: 'ok',
      duration_ms: Date.now() - referencesStart,
      location_count: refs.length,
    };
  } catch (error: unknown) {
    references = {
      status: 'error',
      duration_ms: Date.now() - referencesStart,
      message: error instanceof Error ? error.message : String(error),
    };
  }

  const diagnostics = vscode.languages.getDiagnostics(document.uri);
  return {
    completion,
    definition,
    diagnostics: {
      status: 'ok',
      count: diagnostics.length,
      max_severity: diagnostics.reduce<number | null>((max, diagnostic) => {
        if (max === null) {
          return diagnostic.severity;
        }
        return Math.min(max, diagnostic.severity);
      }, null),
    },
    hover,
    references,
    elapsed_ms_since_moment_start: Date.now() - startedAt,
  } as MomentResult;
}

async function waitForDiagnostics(uri: vscode.Uri, timeoutMs: number): Promise<vscode.Diagnostic[]> {
  const existing = vscode.languages.getDiagnostics(uri);
  if (existing.length > 0) {
    return existing;
  }
  return new Promise(resolve => {
    const timeout = setTimeout(() => {
      subscription.dispose();
      resolve(vscode.languages.getDiagnostics(uri));
    }, timeoutMs);
    const subscription = vscode.languages.onDidChangeDiagnostics(event => {
      if (event.uris.some(changed => changed.toString() === uri.toString())) {
        clearTimeout(timeout);
        subscription.dispose();
        resolve(vscode.languages.getDiagnostics(uri));
      }
    });
  });
}

suite('First-hour VS Code receipt', function () {
  this.timeout(240_000);

  test('records real extension-host first-use behavior', async function () {
    this.timeout(240_000);

    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    assert.ok(workspaceFolder, 'first-hour receipt requires a workspace folder');
    const workspacePath = workspaceFolder.uri.fsPath;
    const serverPath = process.env.PERL_LSP_FIRST_HOUR_SERVER_PATH ?? '';
    assert.ok(serverPath, 'PERL_LSP_FIRST_HOUR_SERVER_PATH must point to the current-main LSP binary');
    assert.ok(fs.existsSync(serverPath), `server binary must exist: ${serverPath}`);

    const perlFiles = findPerlFiles(workspacePath);
    assert.ok(perlFiles.length > 0, `workspace must contain Perl files: ${workspacePath}`);

    const moduleName = process.env.PERL_LSP_FIRST_HOUR_MODULE ?? 'Perl::Critic';
    const probeText = [
      'use strict;',
      'use warnings;',
      `use ${moduleName};`,
      '',
      `my $object = ${moduleName}->new();`,
      '$object->',
      '',
    ].join('\n');
    const probePath = path.join(workspacePath, 'zz_first_hour_receipt_probe.pl');
    fs.writeFileSync(probePath, probeText);

    const badPath = path.join(workspacePath, 'zz_first_hour_receipt_bad.pl');
    fs.writeFileSync(badPath, "use strict;\nuse warnings;\nmy $x = ;\n");

    const extension = vscode.extensions.getExtension('EffortlessMetrics.perl-lsp-rs');
    assert.ok(extension, 'extension should be available in the extension host');
    const baseReceipt = {
      schema_version: 1,
      issue: 3102,
      generated_at: new Date().toISOString(),
      environment: {
        platform: process.platform,
        arch: process.arch,
        vscode_version: vscode.version,
        extension_id: 'EffortlessMetrics.perl-lsp-rs',
        extension_version: extension.packageJSON?.version ?? null,
        server_path: serverPath,
      },
      workspace: {
        path: workspacePath,
        file_count_sampled: walkFiles(workspacePath, 10_000).length,
        perl_file_count_sampled: perlFiles.length,
        module_under_probe: moduleName,
        probe_files: [
          path.basename(probePath),
          path.basename(badPath),
        ],
      },
      limitations: [
        'Automated extension-host run uses real VS Code and the real extension, but does not visually inspect the status bar.',
        'Indexing announcement is recorded as not observable because VS Code extension tests cannot read OutputChannel text through public API.',
      ],
    };

    const activationStart = Date.now();
    try {
      await withTimeout('extension activation', extension.activate(), 90_000);
    } catch (error: unknown) {
      writeFirstHourReceipt({
        ...baseReceipt,
        outcome: 'activation_timeout',
        startup: {
          extension_activation_status: 'timeout',
          extension_activation_ms: Date.now() - activationStart,
          extension_activated_within_30s: false,
          error: error instanceof Error ? error.message : String(error),
          indexing_announcement_observed: 'not_observable_activation_did_not_complete',
        },
        moments: null,
        diagnostics_probe: null,
        failures: [
          {
            moment: 'startup',
            kind: 'activation_timeout',
            message: error instanceof Error ? error.message : String(error),
          },
        ],
      });
      return;
    }
    const activationMs = Date.now() - activationStart;

    const commandWaitStart = Date.now();
    await withTimeout(
      'command registration',
      (async () => {
        for (;;) {
          const commands = await vscode.commands.getCommands(true);
          if (commands.includes('perl-lsp.runHealthCheck')) {
            return;
          }
          await delay(100);
        }
      })(),
      10_000,
    );
    const commandRegistrationMs = Date.now() - commandWaitStart;

    const healthStart = Date.now();
    let health: Record<string, unknown>;
    try {
      const result = await withTimeout(
        'health check command',
        vscode.commands.executeCommand('perl-lsp.runHealthCheck', serverPath),
        20_000,
      );
      health = {
        status: 'ok',
        duration_ms: Date.now() - healthStart,
        result,
      };
    } catch (error: unknown) {
      health = {
        status: 'error',
        duration_ms: Date.now() - healthStart,
        message: error instanceof Error ? error.message : String(error),
      };
    }

    const probeDocument = await vscode.workspace.openTextDocument(probePath);
    await vscode.window.showTextDocument(probeDocument);
    const completionPosition = probeDocument.positionAt(probeText.indexOf('$object->') + '$object->'.length);
    const symbolPosition = probeDocument.positionAt(probeText.indexOf(moduleName) + Math.floor(moduleName.length / 2));

    const immediate = await collectProviderMoment('immediate', probeDocument, completionPosition, symbolPosition);
    await delay(30_000);
    const afterThirtySeconds = await collectProviderMoment('after_30_seconds', probeDocument, completionPosition, symbolPosition);

    const badDocument = await vscode.workspace.openTextDocument(badPath);
    await vscode.window.showTextDocument(badDocument);
    const badDiagnostics = await waitForDiagnostics(badDocument.uri, 10_000);

    const receipt = {
      ...baseReceipt,
      outcome: 'completed',
      startup: {
        extension_activation_status: 'ok',
        extension_activation_ms: activationMs,
        extension_activated_within_30s: activationMs <= 30_000,
        command_registration_ms: commandRegistrationMs,
        health,
        indexing_announcement_observed: 'not_observable_from_extension_host_public_api',
      },
      moments: {
        immediate,
        after_30_seconds: afterThirtySeconds,
      },
      diagnostics_probe: {
        file: path.basename(badPath),
        count: badDiagnostics.length,
        messages: badDiagnostics.slice(0, 10).map(diagnostic => diagnostic.message),
      },
      failures: [],
    };

    writeFirstHourReceipt(receipt);

    assert.ok(afterThirtySeconds.completion.status, 'receipt should include completion result');
    assert.ok(afterThirtySeconds.hover.status, 'receipt should include hover result');
  });
});
