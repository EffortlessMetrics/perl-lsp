import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import { describeWorkspaceTopology } from '../../workspaceTopology';

interface MomentResult {
  classification: 'cold' | 'warm' | 'post_restart';
  completion: Record<string, unknown>;
  definition: Record<string, unknown>;
  diagnostics: Record<string, unknown>;
  hover: Record<string, unknown>;
  references: Record<string, unknown>;
}

function monotonicNow(): number {
  return performance.now();
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

async function withTimeout<T>(
  label: string,
  operation: PromiseLike<T>,
  timeoutMs: number,
): Promise<T> {
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
    case 'win32':
      return 'windows';
    case 'darwin':
      return 'macos';
    case 'linux':
      return 'linux';
    default:
      return process.platform;
  }
}

function receiptsDir(): string {
  const root =
    process.env.PERL_LSP_SMOKE_RECEIPTS_DIR ??
    path.resolve(__dirname, '..', '..', '..', '..', 'target', 'receipts', 'vscode-smoke');
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

function currentSourceSmokeEnabled(): boolean {
  return process.env.PERL_LSP_CURRENT_SOURCE_SMOKE === '1';
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
  return walkFiles(root, 10_000).filter((file) => /\.(?:pl|pm|t|psgi)$/i.test(file));
}

function sampleLabels(items: readonly vscode.CompletionItem[]): string[] {
  return items.slice(0, 10).map((item) => {
    if (typeof item.label === 'string') {
      return item.label;
    }
    return item.label.label;
  });
}

function assertSuccessfulStartupMetrics(metrics: Record<string, unknown>, label: string): void {
  assert.equal(metrics.binary_resolution_status, 'ok', `${label} binary resolution should succeed`);
  assert.equal(metrics.server_start_status, 'ok', `${label} server start should succeed`);
  assert.equal(metrics.initialize_status, 'ok', `${label} initialize should succeed`);
  assert.equal(metrics.lifecycle_state, 'running', `${label} lifecycle should be running`);

  const milestones = metrics.milestones;
  assert.ok(milestones && typeof milestones === 'object', `${label} milestones should be present`);
  const values = milestones as Record<string, unknown>;
  const activationOrdered = [
    'extension_load',
    'activate_entered',
    'commands_registered',
    'activate_returned',
  ] as const;
  const startupOrdered = [
    'binary_resolution_started',
    'binary_resolution_completed',
    'process_started',
    'initialize_completed',
    'workspace_ready',
    'first_useful_request',
  ] as const;
  for (const ordered of [activationOrdered, startupOrdered]) {
    let previous = -1;
    for (const milestone of ordered) {
      const value = values[milestone];
      assert.equal(typeof value, 'number', `${label} must record ${milestone}`);
      assert.ok((value as number) >= previous, `${label} milestone ${milestone} must be monotonic`);
      previous = value as number;
    }
  }
}

async function collectProviderMoment(
  label: string,
  classification: MomentResult['classification'],
  document: vscode.TextDocument,
  completionPosition: vscode.Position,
  symbolPosition: vscode.Position,
): Promise<MomentResult> {
  const startedAt = monotonicNow();
  const completionStart = monotonicNow();
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
      duration_ms: Math.round(monotonicNow() - completionStart),
      item_count: list.items.length,
      sample_labels: sampleLabels(list.items),
    };
  } catch (error: unknown) {
    completion = {
      status: 'error',
      duration_ms: Math.round(monotonicNow() - completionStart),
      message: error instanceof Error ? error.message : String(error),
    };
  }

  const hoverStart = monotonicNow();
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
      duration_ms: Math.round(monotonicNow() - hoverStart),
      item_count: hovers.length,
    };
  } catch (error: unknown) {
    hover = {
      status: 'error',
      duration_ms: Math.round(monotonicNow() - hoverStart),
      message: error instanceof Error ? error.message : String(error),
    };
  }

  const definitionStart = monotonicNow();
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
      duration_ms: Math.round(monotonicNow() - definitionStart),
      location_count: definitions.length,
    };
  } catch (error: unknown) {
    definition = {
      status: 'error',
      duration_ms: Math.round(monotonicNow() - definitionStart),
      message: error instanceof Error ? error.message : String(error),
    };
  }

  const referencesStart = monotonicNow();
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
      duration_ms: Math.round(monotonicNow() - referencesStart),
      location_count: refs.length,
    };
  } catch (error: unknown) {
    references = {
      status: 'error',
      duration_ms: Math.round(monotonicNow() - referencesStart),
      message: error instanceof Error ? error.message : String(error),
    };
  }

  const diagnostics = vscode.languages.getDiagnostics(document.uri);
  return {
    classification,
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
    elapsed_ms_since_moment_start: Math.round(monotonicNow() - startedAt),
  } as MomentResult;
}

async function waitForDiagnostics(
  uri: vscode.Uri,
  timeoutMs: number,
): Promise<vscode.Diagnostic[]> {
  const existing = vscode.languages.getDiagnostics(uri);
  if (existing.length > 0) {
    return existing;
  }
  return new Promise((resolve) => {
    const timeout = setTimeout(() => {
      subscription.dispose();
      resolve(vscode.languages.getDiagnostics(uri));
    }, timeoutMs);
    const subscription = vscode.languages.onDidChangeDiagnostics((event) => {
      if (event.uris.some((changed) => changed.toString() === uri.toString())) {
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
    assert.ok(
      serverPath,
      'PERL_LSP_FIRST_HOUR_SERVER_PATH must point to the current-main LSP binary',
    );
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
    fs.writeFileSync(badPath, 'use strict;\nuse warnings;\nmy $x = ;\n');

    const extension = vscode.extensions.getExtension('EffortlessMetrics.perl-lsp-rs');
    assert.ok(extension, 'extension should be available in the extension host');
    const currentSourceSmoke = currentSourceSmokeEnabled();
    if (currentSourceSmoke) {
      const expectedExtensionsDir = process.env.PERL_LSP_PUBLISHED_EXTENSIONS_DIR ?? '';
      assert.ok(
        expectedExtensionsDir,
        'current-source smoke requires the clean extensions directory',
      );
      let extensionPath = path.resolve(extension.extensionPath);
      let extensionsDir = path.resolve(expectedExtensionsDir);
      if (process.platform === 'win32') {
        extensionPath = extensionPath.toLowerCase();
        extensionsDir = extensionsDir.toLowerCase();
      }
      assert.ok(
        extensionPath === extensionsDir || extensionPath.startsWith(`${extensionsDir}${path.sep}`),
        `extension must be loaded from the clean installed profile: ${extensionPath}`,
      );
    }
    const baseReceipt = {
      schema_version: 1,
      sample_count: 1,
      issue: 3102,
      generated_at: new Date().toISOString(),
      environment: {
        platform: process.platform,
        arch: process.arch,
        toolchain_node_version: process.env.PERL_LSP_TOOLCHAIN_NODE_VERSION ?? null,
        toolchain_npm_version: process.env.PERL_LSP_TOOLCHAIN_NPM_VERSION ?? null,
        extension_host_node_version: process.version,
        node_version: process.version,
        requested_vscode_version: process.env.PERL_LSP_VSCODE_VERSION ?? 'stable',
        vscode_version: vscode.version,
        extension_id: 'EffortlessMetrics.perl-lsp-rs',
        extension_version: extension.packageJSON?.version ?? null,
        extension_path: extension.extensionPath,
        server_path: serverPath,
        source_revision: process.env.PERL_LSP_CURRENT_SOURCE_SHA ?? null,
        server_source_revision: process.env.PERL_LSP_SERVER_SOURCE_SHA ?? null,
        vsix_sha256: process.env.PERL_LSP_VSIX_SHA256 ?? null,
      },
      workspace: {
        path: workspacePath,
        topology: describeWorkspaceTopology({
          folders: vscode.workspace.workspaceFolders ?? [],
          documents: vscode.workspace.textDocuments,
          isTrusted: vscode.workspace.isTrusted,
          remoteName: vscode.env.remoteName,
        }),
        file_count_sampled: walkFiles(workspacePath, 10_000).length,
        perl_file_count_sampled: perlFiles.length,
        module_under_probe: moduleName,
        probe_files: [path.basename(probePath), path.basename(badPath)],
      },
      performance: {
        run_classification: currentSourceSmoke
          ? 'cold_start_with_restart'
          : 'cold_start_with_warm_request',
      },
      limitations: [
        'Automated extension-host run uses real VS Code and the real extension, but does not visually inspect the status bar.',
        'Indexing announcement is recorded as not observable because VS Code extension tests cannot read OutputChannel text through public API.',
      ],
    };

    const activationStart = monotonicNow();
    let activationExports: unknown;
    try {
      activationExports = await withTimeout('extension activation', extension.activate(), 90_000);
    } catch (error: unknown) {
      writeFirstHourReceipt({
        ...baseReceipt,
        outcome: 'activation_timeout',
        startup: {
          extension_activation_status: 'timeout',
          extension_activation_ms: Math.round(monotonicNow() - activationStart),
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
    const activationMs = Math.round(monotonicNow() - activationStart);
    const extensionApi = (activationExports ?? extension.exports) as
      | {
          getLanguageClientStartupMetrics?: () => Record<string, unknown>;
          markLanguageClientStartupMilestone?: (milestone: string) => void;
          stop?: () => Promise<void>;
        }
      | undefined;

    const commandWaitStart = monotonicNow();
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
    const commandRegistrationMs = Math.round(monotonicNow() - commandWaitStart);

    const healthStart = monotonicNow();
    let health: Record<string, unknown>;
    try {
      const result = await withTimeout(
        'health check command',
        vscode.commands.executeCommand('perl-lsp.runHealthCheck', serverPath),
        20_000,
      );
      health = {
        status: 'ok',
        duration_ms: Math.round(monotonicNow() - healthStart),
        result,
      };
    } catch (error: unknown) {
      health = {
        status: 'error',
        duration_ms: Math.round(monotonicNow() - healthStart),
        message: error instanceof Error ? error.message : String(error),
      };
    }

    let failureGuidance: Record<string, unknown> | null = null;
    if (currentSourceSmoke) {
      const failureGuidanceStart = monotonicNow();
      const missingServerPath = path.join(workspacePath, 'zz_missing_perllsp');
      const failureResult = (await withTimeout(
        'failure guidance health check',
        vscode.commands.executeCommand('perl-lsp.runHealthCheck', missingServerPath),
        20_000,
      )) as {
        ok?: boolean;
        checks?: Array<{ label?: unknown; status?: unknown }>;
      };
      assert.equal(
        failureResult.ok,
        false,
        'invalid server path should produce a failed health result',
      );
      assert.ok(
        failureResult.checks?.some(
          (check) => check.label === 'LSP binary' && check.status === 'error',
        ),
        'failure guidance should identify the invalid LSP binary',
      );
      failureGuidance = {
        status: 'ok',
        duration_ms: Math.round(monotonicNow() - failureGuidanceStart),
        result: failureResult,
      };
    }

    const probeDocument = await vscode.workspace.openTextDocument(probePath);
    await vscode.window.showTextDocument(probeDocument);
    const completionPosition = probeDocument.positionAt(
      probeText.indexOf('$object->') + '$object->'.length,
    );
    const symbolPosition = probeDocument.positionAt(
      probeText.indexOf(moduleName) + Math.floor(moduleName.length / 2),
    );

    const immediate = await collectProviderMoment(
      'immediate',
      'cold',
      probeDocument,
      completionPosition,
      symbolPosition,
    );
    if (currentSourceSmoke) {
      assert.equal(
        immediate.completion.status,
        'ok',
        'current-source smoke requires a successful completion provider response',
      );
    }
    extensionApi?.markLanguageClientStartupMilestone?.('first_useful_request');
    const initialLanguageClientMetrics = extensionApi?.getLanguageClientStartupMetrics?.() ?? {
      status: 'unavailable',
      limitation: 'extension activation API did not expose startup metrics',
    };
    if (currentSourceSmoke) {
      assertSuccessfulStartupMetrics(initialLanguageClientMetrics, 'initial startup');
    }

    let lifecycle: Record<string, unknown> | undefined;
    let restartedMoment: MomentResult | undefined;
    if (currentSourceSmoke) {
      const restartStart = monotonicNow();
      await withTimeout(
        'language client restart',
        vscode.commands.executeCommand('perl-lsp.restart'),
        90_000,
      );
      const restartMetrics = extensionApi?.getLanguageClientStartupMetrics?.();
      assert.ok(restartMetrics, 'current-source smoke must expose restart metrics');
      assertSuccessfulStartupMetrics(restartMetrics, 'restart startup');
      const restartMilestones = restartMetrics.milestones;
      assert.ok(
        restartMilestones &&
          typeof restartMilestones === 'object' &&
          typeof (restartMilestones as Record<string, unknown>).restart === 'number',
        'restart startup should record the restart milestone',
      );
      const restarted = await collectProviderMoment(
        'after_restart',
        'post_restart',
        probeDocument,
        completionPosition,
        symbolPosition,
      );
      const restartStatus =
        restartMetrics === undefined
          ? 'unavailable'
          : restartMetrics.binary_resolution_status === 'ok' &&
              restartMetrics.server_start_status === 'ok' &&
              restartMetrics.initialize_status === 'ok'
            ? 'ok'
            : 'error';
      assert.equal(
        restarted.completion.status,
        'ok',
        'current-source smoke requires a successful completion after restart',
      );
      restartedMoment = restarted;

      const deactivateStart = monotonicNow();
      let shutdownMetrics: Record<string, unknown> | undefined;
      if (extensionApi?.stop) {
        await withTimeout('language client shutdown', extensionApi.stop(), 30_000);
        shutdownMetrics = extensionApi.getLanguageClientStartupMetrics?.();
      } else {
        const mainScript = extension.packageJSON?.main;
        assert.ok(mainScript, 'extension package.json must define a main script');
        const extensionMain = require(path.join(extension.extensionPath, mainScript)) as {
          deactivate?: () => Promise<void>;
          getLanguageClientStartupMetrics?: () => Record<string, unknown>;
        };
        assert.equal(
          typeof extensionMain.deactivate,
          'function',
          'extension must export deactivate',
        );
        await withTimeout('language client shutdown', extensionMain.deactivate!(), 30_000);
        shutdownMetrics =
          extensionMain.getLanguageClientStartupMetrics?.() ??
          extensionApi?.getLanguageClientStartupMetrics?.();
      }
      assert.ok(shutdownMetrics, 'current-source smoke must expose shutdown metrics');
      assert.equal(
        shutdownMetrics.lifecycle_state,
        'stopped',
        'shutdown should stop the lifecycle',
      );
      const shutdownMilestones = shutdownMetrics.milestones;
      assert.ok(
        shutdownMilestones &&
          typeof shutdownMilestones === 'object' &&
          typeof (shutdownMilestones as Record<string, unknown>).shutdown === 'number',
        'shutdown should record the shutdown milestone',
      );
      lifecycle = {
        restart: {
          status: restartStatus,
          duration_ms: Math.round(monotonicNow() - restartStart),
          language_client: restartMetrics ?? {
            status: 'unavailable',
            limitation: 'extension activation API did not expose startup metrics',
          },
          provider: restarted,
        },
        shutdown: {
          status: 'ok',
          duration_ms: Math.round(monotonicNow() - deactivateStart),
          language_client: shutdownMetrics ?? {
            status: 'unavailable',
            limitation: 'extension shutdown API did not expose startup metrics',
          },
        },
      };
    }

    const afterThirtySeconds = currentSourceSmoke
      ? restartedMoment!
      : await (async () => {
          await delay(30_000);
          return collectProviderMoment(
            'after_30_seconds',
            'warm',
            probeDocument,
            completionPosition,
            symbolPosition,
          );
        })();
    if (!currentSourceSmoke) {
      extensionApi?.markLanguageClientStartupMilestone?.('warm_request');
    }
    const receiptLanguageClientMetrics = currentSourceSmoke
      ? initialLanguageClientMetrics
      : (extensionApi?.getLanguageClientStartupMetrics?.() ?? {
          status: 'unavailable',
          limitation: 'extension activation API did not expose startup metrics',
        });

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
        language_client: receiptLanguageClientMetrics,
        health,
        indexing_announcement_observed: 'not_observable_from_extension_host_public_api',
        failure_guidance: failureGuidance,
      },
      moments: {
        immediate,
        after_30_seconds: afterThirtySeconds,
      },
      lifecycle,
      diagnostics_probe: {
        file: path.basename(badPath),
        count: badDiagnostics.length,
        messages: badDiagnostics.slice(0, 10).map((diagnostic) => diagnostic.message),
      },
      failures: [],
    };

    writeFirstHourReceipt(receipt);

    assert.ok(afterThirtySeconds.completion.status, 'receipt should include completion result');
    assert.ok(afterThirtySeconds.hover.status, 'receipt should include hover result');
  });
});
