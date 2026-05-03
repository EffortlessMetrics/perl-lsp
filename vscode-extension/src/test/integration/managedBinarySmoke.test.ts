import * as assert from 'assert';
import * as fs from 'fs';
import * as path from 'path';
import { spawn, type ChildProcessWithoutNullStreams } from 'child_process';
import * as vscode from 'vscode';
import type { HealthCheckCommandResult, ReinstallCommandResult } from '../../commandResults';

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

function delay(ms: number): Promise<void> {
  return new Promise(resolve => {
    setTimeout(resolve, ms);
  });
}

async function waitForCommand(command: string, timeoutMs: number): Promise<void> {
  await withTimeout(
    `${command} registration`,
    (async () => {
      for (;;) {
        const commands = await vscode.commands.getCommands(true);
        if (commands.includes(command)) {
          return;
        }
        await delay(100);
      }
    })(),
    timeoutMs,
  );
}

function platformLabel(): string {
  switch (process.platform) {
    case 'win32': return 'windows';
    case 'darwin': return 'macos';
    case 'linux': return 'linux';
    default: return process.platform;
  }
}

function smokeReceiptsDir(sourceLabel: string): string {
  // __dirname after compile is <repo>/vscode-extension/out/test/integration → 4x .. reaches repo root.
  const root = process.env.PERL_LSP_SMOKE_RECEIPTS_DIR
    ?? path.resolve(__dirname, '..', '..', '..', '..', 'target', 'receipts', 'vscode-smoke');
  const dir = path.join(root, sourceLabel, platformLabel());
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

interface SmokeArtifacts {
  state: Record<string, unknown>;
  log: string[];
}

function appendLog(artifacts: SmokeArtifacts, line: string): void {
  artifacts.log.push(`[${new Date().toISOString()}] ${line}`);
}

function writeArtifacts(
  receiptsDir: string,
  artifacts: SmokeArtifacts,
  results: Record<string, unknown>,
): void {
  try {
    fs.writeFileSync(
      path.join(receiptsDir, 'extension-output.log'),
      artifacts.log.join('\n') + '\n',
    );
    fs.writeFileSync(
      path.join(receiptsDir, 'command-results.json'),
      JSON.stringify(results, null, 2),
    );
    fs.writeFileSync(
      path.join(receiptsDir, 'managed-binary-state.json'),
      JSON.stringify(artifacts.state, null, 2),
    );
  } catch (err: unknown) {
    // Receipt-writing failures must not mask test failures — log to stderr.
    const msg = err instanceof Error ? err.message : String(err);
    process.stderr.write(`[smoke] failed to write receipts to ${receiptsDir}: ${msg}\n`);
  }
}

function killLockingProcess(child: ChildProcessWithoutNullStreams | undefined): void {
  if (!child || child.killed || child.exitCode !== null) {
    return;
  }
  try {
    child.kill();
  } catch {
    // best-effort
  }
}

suite('Managed binary smoke', function () {
  this.timeout(240_000);

  test('Managed binary smoke reinstalls perllsp twice and survives running-binary lock', async function () {
    this.timeout(240_000);

    const sourceLabel = process.env.PERL_LSP_SMOKE_SOURCE_LABEL ?? 'integration';
    const receiptsDir = smokeReceiptsDir(sourceLabel);
    const artifacts: SmokeArtifacts = {
      log: [],
      state: {
        platform: process.platform,
        arch: process.arch,
        sourceLabel,
        startedAt: new Date().toISOString(),
        firstReinstallOk: false,
        secondReinstallOk: false,
        firstHealthOk: false,
        secondHealthOk: false,
        binaryRewrittenSecondPass: false,
        lockingProcessSpawned: false,
        lastError: null,
      },
    };
    const results: Record<string, unknown> = {};
    let lockingProcess: ChildProcessWithoutNullStreams | undefined;

    try {
      appendLog(artifacts, 'configuring perl-lsp settings for managed-download mode');
      const config = vscode.workspace.getConfiguration('perl-lsp');
      await config.update('autoDownload', false, vscode.ConfigurationTarget.Global);
      await config.update('serverPath', '', vscode.ConfigurationTarget.Global);
      await config.update('channel', 'tag', vscode.ConfigurationTarget.Global);
      await config.update('versionTag', 'v0.13.1', vscode.ConfigurationTarget.Global);
      await config.update('downloadBaseUrl', '', vscode.ConfigurationTarget.Global);
      await config.update('updateCheckInterval', 0, vscode.ConfigurationTarget.Global);
      await config.update('perlcritic.enabled', false, vscode.ConfigurationTarget.Global);

      if (process.platform === 'linux') {
        await config.update('linuxLibc', 'gnu', vscode.ConfigurationTarget.Global);
      }

      const extension = vscode.extensions.getExtension('EffortlessMetrics.perl-lsp-rs');
      assert.ok(extension, 'extension should be available in the extension host');
      await withTimeout('extension activation', extension.activate(), 30_000);
      await waitForCommand('perl-lsp.reinstall', 10_000);
      await config.update('autoDownload', true, vscode.ConfigurationTarget.Global);

      appendLog(artifacts, 'starting first reinstall');
      const reinstall1 = await withTimeout(
        'managed binary reinstall command (first)',
        vscode.commands.executeCommand<ReinstallCommandResult>('perl-lsp.reinstall'),
        120_000,
      );
      results.reinstall1 = reinstall1;
      assert.ok(reinstall1, 'first reinstall command should return a result');
      assert.equal(reinstall1.ok, true, JSON.stringify(reinstall1, null, 2));
      assert.ok(reinstall1.serverPath, 'first reinstall should include the managed binary path');
      assert.ok(
        fs.existsSync(reinstall1.serverPath),
        `managed binary should exist after first reinstall: ${reinstall1.serverPath}`,
      );
      assert.ok(reinstall1.target, 'first reinstall should include the release target triple');
      assert.equal(
        reinstall1.checksumVerified,
        true,
        'first reinstall should verify SHA256SUMS',
      );

      if (process.platform === 'linux') {
        assert.match(reinstall1.target, /-unknown-linux-gnu$/);
      }

      artifacts.state.firstReinstallOk = true;
      artifacts.state.serverPath = reinstall1.serverPath;
      artifacts.state.target = reinstall1.target;
      artifacts.state.binaryVersion = reinstall1.version ?? null;
      artifacts.state.managedSource = reinstall1.source;

      const mtimeBefore = fs.statSync(reinstall1.serverPath).mtimeMs;
      artifacts.state.binaryMtimeBefore = mtimeBefore;

      appendLog(artifacts, 'running first health check');
      const health1 = await withTimeout(
        'managed binary health check command (first)',
        vscode.commands.executeCommand<HealthCheckCommandResult>(
          'perl-lsp.runHealthCheck',
          reinstall1.serverPath,
        ),
        45_000,
      );
      results.health1 = health1;
      assert.ok(health1, 'first health check should return a result');
      assert.equal(health1.ok, true, JSON.stringify(health1.checks, null, 2));
      assert.ok(
        health1.checks.some(check => check.label === 'LSP binary' && check.status === 'ok'),
        JSON.stringify(health1.checks, null, 2),
      );
      artifacts.state.firstHealthOk = true;

      // Force the binary to be held by a running process so the second reinstall
      // exercises the same lock condition users hit on Windows when the language
      // server is running. The server invocation will idle waiting for an LSP
      // handshake on stdin; we kill it once the second reinstall completes.
      appendLog(artifacts, 'spawning perllsp to hold the binary across second reinstall');
      lockingProcess = spawn(reinstall1.serverPath, [], {
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true,
      });
      lockingProcess.stdout.on('data', () => { /* drain */ });
      lockingProcess.stderr.on('data', () => { /* drain */ });
      lockingProcess.on('error', err => {
        appendLog(artifacts, `locking process error: ${err.message}`);
      });
      // Give the OS a moment to actually hold the executable file handle.
      await delay(750);
      const lockingExited = lockingProcess.exitCode !== null;
      artifacts.state.lockingProcessSpawned = !lockingExited;
      artifacts.state.lockingProcessPid = lockingProcess.pid ?? null;
      if (lockingExited) {
        appendLog(
          artifacts,
          `locking process exited early with code=${lockingProcess.exitCode}; second reinstall will run without lock`,
        );
      }

      appendLog(artifacts, 'starting second reinstall while binary is held');
      const reinstall2 = await withTimeout(
        'managed binary reinstall command (second, locked)',
        vscode.commands.executeCommand<ReinstallCommandResult>('perl-lsp.reinstall'),
        120_000,
      );
      results.reinstall2 = reinstall2;
      assert.ok(reinstall2, 'second reinstall command should return a result');
      assert.equal(reinstall2.ok, true, JSON.stringify(reinstall2, null, 2));
      assert.ok(
        fs.existsSync(reinstall2.serverPath),
        `managed binary should exist after second reinstall: ${reinstall2.serverPath}`,
      );
      assert.equal(
        reinstall2.checksumVerified,
        true,
        'second reinstall should verify SHA256SUMS',
      );

      const mtimeAfter = fs.statSync(reinstall2.serverPath).mtimeMs;
      artifacts.state.binaryMtimeAfter = mtimeAfter;
      artifacts.state.binaryRewrittenSecondPass = mtimeAfter >= mtimeBefore;
      artifacts.state.secondReinstallOk = true;

      appendLog(artifacts, 'running second health check');
      const health2 = await withTimeout(
        'managed binary health check command (second)',
        vscode.commands.executeCommand<HealthCheckCommandResult>(
          'perl-lsp.runHealthCheck',
          reinstall2.serverPath,
        ),
        45_000,
      );
      results.health2 = health2;
      assert.ok(health2, 'second health check should return a result');
      assert.equal(health2.ok, true, JSON.stringify(health2.checks, null, 2));
      assert.ok(
        health2.checks.some(check => check.label === 'LSP binary' && check.status === 'ok'),
        JSON.stringify(health2.checks, null, 2),
      );
      artifacts.state.secondHealthOk = true;
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      artifacts.state.lastError = msg;
      appendLog(artifacts, `smoke failed: ${msg}`);
      throw err;
    } finally {
      killLockingProcess(lockingProcess);
      artifacts.state.completedAt = new Date().toISOString();
      writeArtifacts(receiptsDir, artifacts, results);
    }
  });
});
