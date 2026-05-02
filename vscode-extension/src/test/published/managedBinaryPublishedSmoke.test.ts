import * as assert from 'assert';
import * as fs from 'fs';
import * as vscode from 'vscode';

type CheckStatus = 'ok' | 'warning' | 'error';

interface ReinstallCommandResult {
  ok: boolean;
  serverPath: string;
  target: string;
  version?: string;
  checksumVerified?: boolean;
  source: 'github-release' | 'internal-base-url' | 'existing';
  error?: string;
}

interface HealthCheckCommandResult {
  ok: boolean;
  checks: Array<{
    label: string;
    status: CheckStatus;
    detail: string;
  }>;
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

function versionParts(version: string): [number, number, number] {
  const match = /^(\d+)\.(\d+)\.(\d+)/.exec(version);
  if (!match) {
    return [0, 0, 0];
  }

  return [
    Number.parseInt(match[1], 10),
    Number.parseInt(match[2], 10),
    Number.parseInt(match[3], 10),
  ];
}

function versionAtLeast(actual: string, minimum: string): boolean {
  const actualParts = versionParts(actual);
  const minimumParts = versionParts(minimum);

  for (let index = 0; index < actualParts.length; index += 1) {
    if (actualParts[index] > minimumParts[index]) {
      return true;
    }
    if (actualParts[index] < minimumParts[index]) {
      return false;
    }
  }

  return true;
}

function envFlag(name: string): boolean {
  return ['1', 'true', 'yes'].includes((process.env[name] ?? '').toLowerCase());
}

function releaseTag(version: string): string {
  return version.startsWith('v') ? version : `v${version}`;
}

suite('Published extension managed binary smoke', function () {
  this.timeout(180_000);

  test('Published extension installs and runs managed binary commands when supported', async function () {
    this.timeout(180_000);

    const extensionId = process.env.PERL_LSP_PUBLISHED_EXTENSION_ID ?? 'EffortlessMetrics.perl-lsp-rs';
    const expectedVersion = process.env.PERL_LSP_PUBLISHED_EXTENSION_VERSION ?? '';
    const requireStructuredCommands = envFlag('PERL_LSP_REQUIRE_STRUCTURED_COMMANDS');
    const extension = vscode.extensions.getExtension(extensionId);

    assert.ok(extension, `${extensionId} should be installed in the clean VS Code profile`);

    const packageVersion = String(extension.packageJSON?.version ?? '');
    assert.ok(packageVersion, 'published extension package should expose a version');
    if (expectedVersion) {
      assert.equal(packageVersion, expectedVersion);
    }

    const supportsStructuredCommands = versionAtLeast(packageVersion, '0.13.2');
    if (!supportsStructuredCommands) {
      assert.equal(
        requireStructuredCommands,
        false,
        `published extension ${packageVersion} predates structured command results`,
      );
      return;
    }

    const config = vscode.workspace.getConfiguration('perl-lsp');
    const binaryVersion = process.env.PERL_LSP_PUBLISHED_BINARY_VERSION || expectedVersion || packageVersion;
    await config.update('autoDownload', false, vscode.ConfigurationTarget.Global);
    await config.update('serverPath', '', vscode.ConfigurationTarget.Global);
    await config.update('channel', 'tag', vscode.ConfigurationTarget.Global);
    await config.update('versionTag', releaseTag(binaryVersion), vscode.ConfigurationTarget.Global);
    await config.update('downloadBaseUrl', '', vscode.ConfigurationTarget.Global);
    await config.update('updateCheckInterval', 0, vscode.ConfigurationTarget.Global);
    await config.update('perlcritic.enabled', false, vscode.ConfigurationTarget.Global);

    if (process.platform === 'linux') {
      await config.update('linuxLibc', 'gnu', vscode.ConfigurationTarget.Global);
    }

    await withTimeout('published extension activation', extension.activate(), 45_000);
    await waitForCommand('perl-lsp.reinstall', 15_000);
    await waitForCommand('perl-lsp.runHealthCheck', 15_000);

    await config.update('autoDownload', true, vscode.ConfigurationTarget.Global);

    const reinstall = await withTimeout(
      'published managed binary reinstall command',
      vscode.commands.executeCommand<ReinstallCommandResult>('perl-lsp.reinstall'),
      120_000,
    );
    assert.ok(reinstall, 'reinstall command should return a structured result');
    assert.equal(reinstall.ok, true, JSON.stringify(reinstall, null, 2));
    assert.ok(reinstall.serverPath, 'reinstall result should include the managed binary path');
    assert.ok(fs.existsSync(reinstall.serverPath), `managed binary should exist: ${reinstall.serverPath}`);
    assert.ok(reinstall.target, 'reinstall result should include the release target triple');
    assert.equal(reinstall.checksumVerified, true, 'managed binary download should verify SHA256SUMS');

    if (process.platform === 'linux') {
      assert.match(reinstall.target, /-unknown-linux-gnu$/);
    }

    const health = await withTimeout(
      'published managed binary health check command',
      vscode.commands.executeCommand<HealthCheckCommandResult>(
        'perl-lsp.runHealthCheck',
        reinstall.serverPath,
      ),
      45_000,
    );

    assert.ok(health, 'health check command should return a structured result');
    assert.equal(health.ok, true, JSON.stringify(health.checks, null, 2));
    assert.ok(
      health.checks.some(check => check.label === 'LSP binary' && check.status === 'ok'),
      JSON.stringify(health.checks, null, 2),
    );
  });
});
