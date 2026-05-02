import * as assert from 'assert';
import * as fs from 'fs';
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

suite('Managed binary smoke', function () {
  this.timeout(180_000);

  test('Managed binary smoke reinstalls managed perllsp and passes health check', async function () {
    this.timeout(180_000);

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

    const reinstall = await withTimeout(
      'managed binary reinstall command',
      vscode.commands.executeCommand<ReinstallCommandResult>('perl-lsp.reinstall'),
      120_000,
    );
    assert.ok(reinstall, 'reinstall command should return a result');
    assert.equal(reinstall.ok, true, JSON.stringify(reinstall, null, 2));
    assert.ok(reinstall.serverPath, 'reinstall result should include the managed binary path');
    assert.ok(fs.existsSync(reinstall.serverPath), `managed binary should exist: ${reinstall.serverPath}`);
    assert.ok(reinstall.target, 'reinstall result should include the release target triple');
    assert.equal(reinstall.checksumVerified, true, 'managed binary download should verify SHA256SUMS');

    if (process.platform === 'linux') {
      assert.match(reinstall.target, /-unknown-linux-gnu$/);
    }

    const health = await withTimeout(
      'managed binary health check command',
      vscode.commands.executeCommand<HealthCheckCommandResult>(
        'perl-lsp.runHealthCheck',
        reinstall.serverPath,
      ),
      30_000,
    );

    assert.ok(health, 'health check command should return a result');
    assert.equal(health.ok, true, JSON.stringify(health.checks, null, 2));
    assert.ok(
      health.checks.some(check => check.label === 'LSP binary' && check.status === 'ok'),
      JSON.stringify(health.checks, null, 2),
    );
  });
});
