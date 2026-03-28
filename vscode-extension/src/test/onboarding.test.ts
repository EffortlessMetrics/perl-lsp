/**
 * Unit tests for OnboardingManager.
 *
 * Tests cover:
 * - First-run detection via globalState
 * - checkPerlInstalled: detects Perl presence and version
 * - checkPerltidyInstalled: detects perltidy on PATH
 * - checkBinaryDownloaded: confirms server path present
 * - runSetupHealthCheck: full check sequence returning HealthCheckResult
 * - shouldShowWelcome: returns false after welcome flag is set
 */

import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import {
  OnboardingManager,
  HealthCheckResult,
  HealthCheckStatus,
  selectWindowsCommandCandidate,
} from '../onboarding';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeContext(opts?: { welcomed?: boolean; storagePath?: string }): any {
  const store = new Map<string, any>();
  if (opts?.welcomed) {
    store.set('perl-lsp.welcomed', true);
  }
  const dir =
    opts?.storagePath ??
    fs.mkdtempSync(path.join(os.tmpdir(), 'onboarding-test-'));
  return {
    globalStorageUri: { fsPath: dir },
    extensionPath: dir,
    subscriptions: [],
    globalState: {
      get: jest.fn((key: string, defaultValue?: any) => {
        if (store.has(key)) return store.get(key);
        return defaultValue;
      }),
      update: jest.fn(async (key: string, value: any) => {
        store.set(key, value);
      }),
    },
  };
}

function makeOutputChannel(): any {
  return {
    appendLine: jest.fn(),
    show: jest.fn(),
    dispose: jest.fn(),
  };
}

// ---------------------------------------------------------------------------
// shouldShowWelcome
// ---------------------------------------------------------------------------

describe('OnboardingManager.shouldShowWelcome', () => {
  test('returns true on first run (welcomed flag not set)', () => {
    const ctx = makeContext();
    const mgr = new OnboardingManager(ctx, makeOutputChannel());
    expect(mgr.shouldShowWelcome()).toBe(true);
  });

  test('returns false if welcomed flag is already set', () => {
    const ctx = makeContext({ welcomed: true });
    const mgr = new OnboardingManager(ctx, makeOutputChannel());
    expect(mgr.shouldShowWelcome()).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// markWelcomed
// ---------------------------------------------------------------------------

describe('OnboardingManager.markWelcomed', () => {
  test('sets the welcomed flag in globalState', async () => {
    const ctx = makeContext();
    const mgr = new OnboardingManager(ctx, makeOutputChannel());
    expect(mgr.shouldShowWelcome()).toBe(true);
    await mgr.markWelcomed();
    expect(ctx.globalState.update).toHaveBeenCalledWith(
      'perl-lsp.welcomed',
      true,
    );
  });
});

// ---------------------------------------------------------------------------
// checkPerlInstalled
// ---------------------------------------------------------------------------

describe('OnboardingManager.checkPerlInstalled', () => {
  test('returns ok status with version when perl is available', async () => {
    const mgr = new OnboardingManager(makeContext(), makeOutputChannel()) as any;
    // Inject a mock that simulates `perl -e 'print $]'` returning a version
    mgr._execCheck = jest.fn((_cmd: string, _args: string[]) =>
      Promise.resolve({ stdout: '5.036000', stderr: '' }),
    );
    const result = await mgr.checkPerlInstalled();
    expect(result.ok).toBe(true);
    expect(result.detail).toContain('5.036000');
  });

  test('returns error status when perl is not found', async () => {
    const mgr = new OnboardingManager(makeContext(), makeOutputChannel()) as any;
    mgr._execCheck = jest.fn(() =>
      Promise.reject(new Error('perl: command not found')),
    );
    const result = await mgr.checkPerlInstalled();
    expect(result.ok).toBe(false);
    expect(result.detail).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// checkPerltidyInstalled
// ---------------------------------------------------------------------------

describe('OnboardingManager.checkPerltidyInstalled', () => {
  test('returns ok status when perltidy is available', async () => {
    const mgr = new OnboardingManager(makeContext(), makeOutputChannel()) as any;
    mgr._execCheck = jest.fn((_cmd: string, _args: string[]) =>
      Promise.resolve({ stdout: 'perltidy, v20230309', stderr: '' }),
    );
    const result = await mgr.checkPerltidyInstalled();
    expect(result.ok).toBe(true);
    expect(result.detail).toContain('perltidy');
  });

  test('returns warning (not error) when perltidy is absent', async () => {
    const mgr = new OnboardingManager(makeContext(), makeOutputChannel()) as any;
    mgr._execCheck = jest.fn(() =>
      Promise.reject(new Error('perltidy: command not found')),
    );
    const result = await mgr.checkPerltidyInstalled();
    expect(result.ok).toBe(false);
    expect(result.status).toBe(HealthCheckStatus.Warning);
  });
});

describe('selectWindowsCommandCandidate', () => {
  test('prefers executable or wrapper paths over extensionless shims', () => {
    const selected = selectWindowsCommandCandidate(
      [
        'C:\\Strawberry\\perl\\bin\\perltidy',
        'C:\\Strawberry\\perl\\bin\\perltidy.bat',
        'C:\\tools\\perltidy.exe',
      ].join('\r\n'),
    );

    expect(selected).toBe('C:\\tools\\perltidy.exe');
  });

  test('returns null for empty where output', () => {
    expect(selectWindowsCommandCandidate(' \r\n \r\n')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// checkBinaryDownloaded
// ---------------------------------------------------------------------------

describe('OnboardingManager.checkBinaryDownloaded', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ob-bin-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('returns ok when server path exists', () => {
    const binPath = path.join(tmpDir, 'perl-lsp');
    fs.writeFileSync(binPath, '#!/bin/sh\necho ok');
    const mgr = new OnboardingManager(makeContext(), makeOutputChannel());
    const result = mgr.checkBinaryDownloaded(binPath);
    expect(result.ok).toBe(true);
  });

  test('returns error when server path is null', () => {
    const mgr = new OnboardingManager(makeContext(), makeOutputChannel());
    const result = mgr.checkBinaryDownloaded(null);
    expect(result.ok).toBe(false);
    expect(result.status).toBe(HealthCheckStatus.Error);
  });

  test('returns error when server path does not exist on disk', () => {
    const mgr = new OnboardingManager(makeContext(), makeOutputChannel());
    const result = mgr.checkBinaryDownloaded('/nonexistent/perl-lsp');
    expect(result.ok).toBe(false);
    expect(result.status).toBe(HealthCheckStatus.Error);
  });
});

// ---------------------------------------------------------------------------
// runSetupHealthCheck
// ---------------------------------------------------------------------------

describe('OnboardingManager.runSetupHealthCheck', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'ob-health-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('returns a HealthCheckResult array with one entry per check', async () => {
    const binPath = path.join(tmpDir, 'perl-lsp');
    fs.writeFileSync(binPath, '#!/bin/sh\necho ok');

    const mgr = new OnboardingManager(makeContext(), makeOutputChannel()) as any;
    mgr._execCheck = jest.fn((_cmd: string, _args: string[]) =>
      Promise.resolve({ stdout: '5.036000', stderr: '' }),
    );
    const results: HealthCheckResult[] = await mgr.runSetupHealthCheck(binPath);
    expect(Array.isArray(results)).toBe(true);
    expect(results.length).toBeGreaterThanOrEqual(3);
  });

  test('each result has label, ok, status, and detail properties', async () => {
    const binPath = path.join(tmpDir, 'perl-lsp');
    fs.writeFileSync(binPath, '#!/bin/sh\necho ok');

    const mgr = new OnboardingManager(makeContext(), makeOutputChannel()) as any;
    mgr._execCheck = jest.fn((_cmd: string, _args: string[]) =>
      Promise.resolve({ stdout: '5.036000', stderr: '' }),
    );
    const results: HealthCheckResult[] = await mgr.runSetupHealthCheck(binPath);
    for (const r of results) {
      expect(typeof r.label).toBe('string');
      expect(typeof r.ok).toBe('boolean');
      expect(Object.values(HealthCheckStatus)).toContain(r.status);
      expect(typeof r.detail).toBe('string');
    }
  });

  test('binary check fails when server path is null', async () => {
    const mgr = new OnboardingManager(makeContext(), makeOutputChannel()) as any;
    mgr._execCheck = jest.fn((_cmd: string, _args: string[]) =>
      Promise.resolve({ stdout: '5.036000', stderr: '' }),
    );
    const results: HealthCheckResult[] = await mgr.runSetupHealthCheck(null);
    const binCheck = results.find(r => r.label === 'LSP binary');
    expect(binCheck).toBeDefined();
    expect(binCheck!.ok).toBe(false);
  });

  test('all checks pass on a fully healthy environment', async () => {
    const binPath = path.join(tmpDir, 'perl-lsp');
    fs.writeFileSync(binPath, '#!/bin/sh\necho ok');

    const mgr = new OnboardingManager(makeContext(), makeOutputChannel()) as any;
    // Simulate both perl and perltidy available
    mgr._execCheck = jest.fn((cmd: string, _args: string[]) => {
      if (cmd === 'perl') return Promise.resolve({ stdout: '5.036000', stderr: '' });
      if (cmd === 'perltidy') return Promise.resolve({ stdout: 'perltidy, v20230309', stderr: '' });
      return Promise.reject(new Error('unknown'));
    });
    const results: HealthCheckResult[] = await mgr.runSetupHealthCheck(binPath);
    const errors = results.filter(r => r.status === HealthCheckStatus.Error);
    expect(errors).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// package.json contract: healthCheck command registered
// ---------------------------------------------------------------------------

describe('package.json health check command', () => {
  const EXT_ROOT = path.resolve(__dirname, '..', '..');
  let pkg: any;

  beforeAll(() => {
    pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
  });

  test('registers perl-lsp.runHealthCheck command', () => {
    const commandIds = pkg.contributes.commands.map((c: any) => c.command);
    expect(commandIds).toContain('perl-lsp.runHealthCheck');
  });

  test('health check command has Perl category', () => {
    const cmd = pkg.contributes.commands.find(
      (c: any) => c.command === 'perl-lsp.runHealthCheck',
    );
    expect(cmd).toBeDefined();
    expect(cmd.category).toBe('Perl');
  });

  test('health check command title is user-friendly', () => {
    const cmd = pkg.contributes.commands.find(
      (c: any) => c.command === 'perl-lsp.runHealthCheck',
    );
    expect(cmd.title).toBeTruthy();
    expect(cmd.title.toLowerCase()).toContain('health');
  });

  test('runHealthCheck has an activation event so it works without a Perl file open', () => {
    // runHealthCheck is palette-global (no when clause restricting to editorLangId == perl).
    // Without its own activation event, VSCode will not load the extension when the user
    // triggers the command from the command palette if no Perl file is active.
    expect(pkg.activationEvents).toContain('onCommand:perl-lsp.runHealthCheck');
  });

  test('runHealthCheck is listed in commandPalette without a language restriction', () => {
    const palette = pkg.contributes.menus.commandPalette;
    const entry = palette.find((e: any) => e.command === 'perl-lsp.runHealthCheck');
    expect(entry).toBeDefined();
    // No editorLangId restriction — the health check must be reachable from any context.
    expect(entry.when ?? '').not.toMatch(/editorLangId/);
  });
});
