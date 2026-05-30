/**
 * Unit tests for BinaryDownloader security and platform detection logic.
 *
 * These tests exercise pure functions and validation logic without network
 * access. We rely on the vscode mock in __mocks__/vscode.ts.
 */

import * as crypto from 'crypto';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { afterEach, beforeEach, describe, expect, jest, test } from '@jest/globals';
import {
  BinaryDownloader,
  buildBinaryAssetCandidateNames,
  compareVersions,
  copyManagedFileWithRetry,
  findReleaseAssetName,
  isTransientManagedInstallError,
  parseLocalVersion,
  __resetManagedInstallSingleflightForTesting,
} from '../downloader';

// ---------------------------------------------------------------------------
// Helpers: build a minimal mock ExtensionContext
// ---------------------------------------------------------------------------
function makeContext(storagePath?: string): any {
  const dir = storagePath ?? fs.mkdtempSync(path.join(os.tmpdir(), 'dl-test-'));
  return {
    globalStorageUri: { fsPath: dir },
    extensionPath: dir,
    subscriptions: [],
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
// Platform target detection
// ---------------------------------------------------------------------------
describe('BinaryDownloader.getPlatformTarget', () => {
  let downloader: BinaryDownloader;
  let androidRootBackup: string | undefined;
  let androidDataBackup: string | undefined;
  let termuxVersionBackup: string | undefined;

  beforeEach(() => {
    downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
    androidRootBackup = process.env.ANDROID_ROOT;
    androidDataBackup = process.env.ANDROID_DATA;
    termuxVersionBackup = process.env.TERMUX_VERSION;
    // Isolate the target-triple unit from ambient platform detection so these
    // tests are deterministic regardless of host or sibling-test state. The
    // Termux test overrides isTermuxEnvironment explicitly below; libc tests
    // exercise the non-Android Linux path and must not be shadowed by leaked
    // detector state.
    jest.spyOn(downloader as any, 'isTermuxEnvironment').mockReturnValue(false);
    jest.spyOn(downloader as any, 'isAndroidEnvironment').mockReturnValue(false);
  });

  afterEach(() => {
    process.env.ANDROID_ROOT = androidRootBackup;
    process.env.ANDROID_DATA = androidDataBackup;
    process.env.TERMUX_VERSION = termuxVersionBackup;
    jest.restoreAllMocks();
  });

  function getPlatformTarget(dl: any): string {
    return dl.getPlatformTarget();
  }

  function mockConfig(overrides: Record<string, unknown>): void {
    const vscode = require('vscode');
    vscode.workspace.getConfiguration.mockImplementationOnce(() => ({
      get: jest.fn((key: string, defaultValue?: unknown) => {
        if (key in overrides) { return overrides[key]; }
        return defaultValue;
      }),
      update: jest.fn(),
    }));
  }

  test('returns a non-empty target triple', () => {
    const target = getPlatformTarget(downloader);
    expect(target).toBeTruthy();
    expect(typeof target).toBe('string');
    expect(target.length).toBeGreaterThan(0);
  });

  test('target contains architecture component', () => {
    const target = getPlatformTarget(downloader);
    expect(target).toMatch(/^(x86_64|aarch64|arm64)/);
  });

  test('target contains platform component', () => {
    const target = getPlatformTarget(downloader);
    expect(target).toMatch(/(apple-darwin|unknown-linux|pc-windows)/);
  });

  test('uses linux-android target in Termux environments', () => {
    if (process.platform !== 'linux') {
      return;
    }

    jest.spyOn(downloader as any, 'isTermuxEnvironment').mockReturnValue(true);
    const target = getPlatformTarget(downloader);
    expect(target).toMatch(/-linux-android$/);
  });

  test('linuxLibc=glibc selects GNU Linux release target', () => {
    if (process.platform !== 'linux') {
      return;
    }

    mockConfig({ linuxLibc: 'glibc' });
    jest.spyOn(downloader as any, 'detectMusl').mockReturnValue(true);

    expect(getPlatformTarget(downloader)).toMatch(/-unknown-linux-gnu$/);
  });

  test('linuxLibc=musl selects musl Linux release target', () => {
    if (process.platform !== 'linux') {
      return;
    }

    mockConfig({ linuxLibc: 'musl' });
    jest.spyOn(downloader as any, 'detectMusl').mockReturnValue(false);

    expect(getPlatformTarget(downloader)).toMatch(/-unknown-linux-musl$/);
  });

  test('linuxLibc=auto follows musl detection', () => {
    if (process.platform !== 'linux') {
      return;
    }

    mockConfig({ linuxLibc: 'auto' });
    jest.spyOn(downloader as any, 'detectMusl').mockReturnValue(true);

    expect(getPlatformTarget(downloader)).toMatch(/-unknown-linux-musl$/);
  });
});

// ---------------------------------------------------------------------------
// Local binary path construction
// ---------------------------------------------------------------------------
describe('BinaryDownloader.getLocalBinaryPath', () => {
  test('binary path includes platform and arch subdirectory', () => {
    const ctx = makeContext('/tmp/test-storage');
    const downloader = new BinaryDownloader(ctx, makeOutputChannel()) as any;
    const binaryPath: string = downloader.getLocalBinaryPath();

    expect(binaryPath).toContain(process.platform);
    expect(binaryPath).toContain(process.arch);
  });

  test('binary name is perllsp (or perllsp.exe on win32)', () => {
    const ctx = makeContext('/tmp/test-storage');
    const downloader = new BinaryDownloader(ctx, makeOutputChannel()) as any;
    const binaryPath: string = downloader.getLocalBinaryPath();
    const basename = path.basename(binaryPath);

    if (process.platform === 'win32') {
      expect(basename).toBe('perllsp.exe');
    } else {
      expect(basename).toBe('perllsp');
    }
  });
});

// ---------------------------------------------------------------------------
// Static DAP path helper
// ---------------------------------------------------------------------------
describe('BinaryDownloader.getLocalDapPath', () => {
  test('returns path containing perl-dap binary name', () => {
    const ctx = makeContext('/tmp/dap-storage');
    const dapPath = BinaryDownloader.getLocalDapPath(ctx);
    const basename = path.basename(dapPath);

    if (process.platform === 'win32') {
      expect(basename).toBe('perl-dap.exe');
    } else {
      expect(basename).toBe('perl-dap');
    }
  });

  test('DAP path is in same directory as LSP binary', () => {
    const ctx = makeContext('/tmp/dap-storage');
    const dapPath = BinaryDownloader.getLocalDapPath(ctx);
    const downloader = new BinaryDownloader(ctx, makeOutputChannel()) as any;
    const lspPath: string = downloader.getLocalBinaryPath();

    expect(path.dirname(dapPath)).toBe(path.dirname(lspPath));
  });
});

// ---------------------------------------------------------------------------
// Managed binary installation
// ---------------------------------------------------------------------------
describe('BinaryDownloader managed file install', () => {
  test('classifies transient file-lock errors as retryable', () => {
    expect(isTransientManagedInstallError(Object.assign(new Error('locked'), { code: 'EBUSY' }))).toBe(true);
    expect(isTransientManagedInstallError(Object.assign(new Error('denied'), { code: 'EPERM' }))).toBe(true);
    expect(isTransientManagedInstallError(Object.assign(new Error('missing'), { code: 'ENOENT' }))).toBe(false);
  });

  test('retries transient file-lock errors while installing managed binaries', async () => {
    const logs: string[] = [];
    let attempts = 0;
    const transientError = Object.assign(new Error('locked'), { code: 'EBUSY' });
    const copyFile = jest.fn(() => {
      attempts += 1;
      if (attempts === 1) {
        throw transientError;
      }
    });

    await copyManagedFileWithRetry('source', 'destination', 'perllsp', message => logs.push(message), [0], copyFile);

    expect(attempts).toBe(2);
    expect(copyFile).toHaveBeenCalledTimes(2);
    expect(logs).toEqual([expect.stringContaining('Retrying perllsp install')]);
  });

  test('does not retry non-transient managed install errors', async () => {
    const logs: string[] = [];
    const missingError = Object.assign(new Error('missing'), { code: 'ENOENT' });
    const copyFile = jest.fn(() => {
      throw missingError;
    });

    await expect(
      copyManagedFileWithRetry('source', 'destination', 'perllsp', message => logs.push(message), [0], copyFile),
    ).rejects.toThrow('missing');

    expect(copyFile).toHaveBeenCalledTimes(1);
    expect(logs).toEqual([]);
  });

  test('long-tail retry budget allows up to 8 transient failures before succeeding', async () => {
    // Locks the post-0.13.3 retry budget (covers Defender first-time signature
    // scans on freshly extracted release artifacts). The default budget has 8
    // delay slots; 9 attempts total (initial + 8 retries).
    const logs: string[] = [];
    let attempts = 0;
    const transient = Object.assign(new Error('locked'), { code: 'EBUSY' });
    const copyFile = jest.fn(() => {
      attempts += 1;
      if (attempts <= 8) {
        throw transient;
      }
    });

    await copyManagedFileWithRetry(
      'src',
      'dst',
      'perllsp',
      message => logs.push(message),
      [0, 0, 0, 0, 0, 0, 0, 0],
      copyFile,
    );

    expect(attempts).toBe(9);
    expect(copyFile).toHaveBeenCalledTimes(9);
    expect(logs.length).toBe(8);
  });

  test('budget exhaustion eventually surfaces the transient error', async () => {
    const logs: string[] = [];
    const transient = Object.assign(new Error('locked'), { code: 'EBUSY' });
    const copyFile = jest.fn(() => {
      throw transient;
    });

    await expect(
      copyManagedFileWithRetry(
        'src',
        'dst',
        'perllsp',
        message => logs.push(message),
        [0, 0, 0],
        copyFile,
      ),
    ).rejects.toMatchObject({ code: 'EBUSY' });

    expect(copyFile).toHaveBeenCalledTimes(4);
  });

  test('all four transient codes are retried (EBUSY/EPERM/EACCES/ETXTBSY)', async () => {
    const codes = ['EBUSY', 'EPERM', 'EACCES', 'ETXTBSY'];
    for (const code of codes) {
      let attempts = 0;
      const transient = Object.assign(new Error(`locked:${code}`), { code });
      const copyFile = jest.fn(() => {
        attempts += 1;
        if (attempts === 1) {
          throw transient;
        }
      });

      await copyManagedFileWithRetry(
        'src',
        'dst',
        'perllsp',
        () => { /* drop */ },
        [0],
        copyFile,
      );

      expect(attempts).toBe(2);
    }
  });
});

// ---------------------------------------------------------------------------
// Versioned managed install layout
// ---------------------------------------------------------------------------
describe('Versioned managed install layout', () => {
  let storageRoot: string;
  let baseDir: string;
  let downloader: any;

  const lspBinaryName = process.platform === 'win32' ? 'perllsp.exe' : 'perllsp';
  const dapBinaryName = process.platform === 'win32' ? 'perl-dap.exe' : 'perl-dap';

  beforeEach(() => {
    storageRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'managed-install-test-'));
    const ctx = makeContext(storageRoot);
    downloader = new BinaryDownloader(ctx, makeOutputChannel());
    baseDir = path.join(storageRoot, 'bin', `${process.platform}-${process.arch}`);
    fs.mkdirSync(baseDir, { recursive: true });
  });

  afterEach(() => {
    if (storageRoot && fs.existsSync(storageRoot)) {
      fs.rmSync(storageRoot, { recursive: true, force: true });
    }
  });

  test('getLocalBinaryPath falls back to flat layout when no pointer exists (legacy users)', () => {
    const flat = path.join(baseDir, lspBinaryName);
    fs.writeFileSync(flat, 'fake binary');

    expect(downloader.getLocalBinaryPath()).toBe(flat);
  });

  test('getLocalBinaryPath returns versioned path when pointer points at existing dir', () => {
    const versionedName = 'v0.13.3-2026-05-03T00-00-00-000Z';
    const versionedDir = path.join(baseDir, versionedName);
    fs.mkdirSync(versionedDir);
    fs.writeFileSync(path.join(baseDir, 'current'), versionedName + '\n');

    expect(downloader.getLocalBinaryPath()).toBe(path.join(versionedDir, lspBinaryName));
  });

  test('pointer with .. is rejected and falls back to flat layout', () => {
    fs.writeFileSync(path.join(baseDir, 'current'), '..\n');

    expect(downloader.getLocalBinaryPath()).toBe(path.join(baseDir, lspBinaryName));
  });

  test('pointer with path separator is rejected', () => {
    fs.writeFileSync(path.join(baseDir, 'current'), 'evil/sub\n');

    expect(downloader.getLocalBinaryPath()).toBe(path.join(baseDir, lspBinaryName));
  });

  test('pointer to nonexistent dir is rejected', () => {
    fs.writeFileSync(path.join(baseDir, 'current'), 'absent-dir\n');

    expect(downloader.getLocalBinaryPath()).toBe(path.join(baseDir, lspBinaryName));
  });

  test('empty pointer falls back to flat layout', () => {
    fs.writeFileSync(path.join(baseDir, 'current'), '\n');

    expect(downloader.getLocalBinaryPath()).toBe(path.join(baseDir, lspBinaryName));
  });

  test('static getLocalDapPath honors the pointer when present', () => {
    const versionedName = 'v0.13.3-stamp';
    fs.mkdirSync(path.join(baseDir, versionedName));
    fs.writeFileSync(path.join(baseDir, 'current'), versionedName);

    const ctx = makeContext(storageRoot);
    expect(BinaryDownloader.getLocalDapPath(ctx)).toBe(
      path.join(baseDir, versionedName, dapBinaryName),
    );
  });

  test('static getLocalDapPath falls back to flat layout when pointer absent', () => {
    const ctx = makeContext(storageRoot);
    expect(BinaryDownloader.getLocalDapPath(ctx)).toBe(path.join(baseDir, dapBinaryName));
  });

  test('buildVersionedInstallDirName produces a unique name per call (force-reinstall safety)', () => {
    const a = downloader.buildVersionedInstallDirName('v0.13.3');
    // Spin until the OS clock advances at least 1ms so the ISO stamp differs.
    const start = Date.now();
    while (Date.now() === start) { /* spin */ }
    const b = downloader.buildVersionedInstallDirName('v0.13.3');

    expect(a).not.toBe(b);
    expect(a.startsWith('v0.13.3-')).toBe(true);
    expect(b.startsWith('v0.13.3-')).toBe(true);
  });

  test('buildVersionedInstallDirName sanitizes unsafe tag characters', () => {
    const name = downloader.buildVersionedInstallDirName('weird/tag with spaces');
    expect(name).toMatch(/^[A-Za-z0-9._-]+$/);
    expect(name.includes('/')).toBe(false);
    expect(name.includes(' ')).toBe(false);
  });

  test('commitVersionedInstall writes pointer atomically and cleans up tmp', () => {
    const installDirName = 'v0.13.3-stamp';
    fs.mkdirSync(path.join(baseDir, installDirName));

    downloader.commitVersionedInstall(installDirName);

    const pointer = path.join(baseDir, 'current');
    expect(fs.existsSync(pointer)).toBe(true);
    expect(fs.readFileSync(pointer, 'utf8').trim()).toBe(installDirName);
    expect(fs.existsSync(`${pointer}.tmp`)).toBe(false);
  });

  test('commitVersionedInstall overwrites a stale pointer', () => {
    fs.writeFileSync(path.join(baseDir, 'current'), 'stale-name\n');
    fs.mkdirSync(path.join(baseDir, 'v0.13.4-stamp'));

    downloader.commitVersionedInstall('v0.13.4-stamp');

    expect(fs.readFileSync(path.join(baseDir, 'current'), 'utf8').trim()).toBe('v0.13.4-stamp');
  });

  test('pruneOldVersionedInstalls keeps current plus exactly one prior install', () => {
    const names = ['v0.13.0-a', 'v0.13.1-b', 'v0.13.2-c', 'v0.13.3-d'];
    names.forEach((name, idx) => {
      const dir = path.join(baseDir, name);
      fs.mkdirSync(dir);
      // Older index → older mtime
      const t = Date.now() / 1000 - (names.length - idx) * 60;
      fs.utimesSync(dir, t, t);
    });

    downloader.pruneOldVersionedInstalls(baseDir, 'v0.13.3-d');

    expect(fs.existsSync(path.join(baseDir, 'v0.13.3-d'))).toBe(true);  // current
    expect(fs.existsSync(path.join(baseDir, 'v0.13.2-c'))).toBe(true);  // most recent prior
    expect(fs.existsSync(path.join(baseDir, 'v0.13.1-b'))).toBe(false); // pruned
    expect(fs.existsSync(path.join(baseDir, 'v0.13.0-a'))).toBe(false); // pruned
  });

  test('pruneOldVersionedInstalls preserves current when it is the only versioned dir', () => {
    fs.mkdirSync(path.join(baseDir, 'v0.13.3-d'));

    downloader.pruneOldVersionedInstalls(baseDir, 'v0.13.3-d');

    expect(fs.existsSync(path.join(baseDir, 'v0.13.3-d'))).toBe(true);
  });

  test('pruneOldVersionedInstalls ignores files at base dir (legacy flat binaries survive)', () => {
    const flatBin = path.join(baseDir, lspBinaryName);
    fs.writeFileSync(flatBin, 'legacy');
    fs.mkdirSync(path.join(baseDir, 'v0.13.3-d'));

    downloader.pruneOldVersionedInstalls(baseDir, 'v0.13.3-d');

    expect(fs.existsSync(flatBin)).toBe(true);
  });

  test('legacy migration: install side-by-side with flat layout, pointer activates versioned', () => {
    // Seed a legacy 0.13.2-style flat binary that a long-running user would have.
    const flatBin = path.join(baseDir, lspBinaryName);
    fs.writeFileSync(flatBin, 'legacy 0.13.2 bytes');

    // Pre-migration, getLocalBinaryPath returns the flat path.
    expect(downloader.getLocalBinaryPath()).toBe(flatBin);

    // Simulate a successful 0.13.3 reinstall that lands in a versioned dir
    // and atomically promotes the pointer.
    const versionedName = 'v0.13.3-stamp';
    fs.mkdirSync(path.join(baseDir, versionedName));
    fs.writeFileSync(path.join(baseDir, versionedName, lspBinaryName), 'fresh 0.13.3 bytes');
    downloader.commitVersionedInstall(versionedName);

    // Post-migration, getLocalBinaryPath returns the versioned path.
    expect(downloader.getLocalBinaryPath()).toBe(
      path.join(baseDir, versionedName, lspBinaryName),
    );

    // Legacy flat binary is preserved on disk (does not get auto-cleaned).
    expect(fs.existsSync(flatBin)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Singleflight managed install
// ---------------------------------------------------------------------------
describe('Singleflight managed install', () => {
  beforeEach(() => {
    __resetManagedInstallSingleflightForTesting();
  });

  afterEach(() => {
    __resetManagedInstallSingleflightForTesting();
    jest.restoreAllMocks();
  });

  function makeDeferred<T>(): { promise: Promise<T>; resolve: (v: T) => void } {
    let resolve!: (v: T) => void;
    const promise = new Promise<T>(r => { resolve = r; });
    return { promise, resolve };
  }

  test('two concurrent ensure calls share one runEnsureBinary invocation', async () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
    const deferred = makeDeferred<string | null>();
    const runSpy = jest.spyOn(downloader as any, 'runEnsureBinary')
      .mockReturnValue(deferred.promise);

    const p1 = downloader.ensureBinary(false);
    const p2 = downloader.ensureBinary(false);

    // Yield once so the second call observes the active install set by the first.
    await new Promise<void>(r => setImmediate(r));

    deferred.resolve('/path/from/first');
    const [r1, r2] = await Promise.all([p1, p2]);

    expect(runSpy).toHaveBeenCalledTimes(1);
    expect(r1).toBe('/path/from/first');
    expect(r2).toBe('/path/from/first');
  });

  test('two concurrent force calls share one runEnsureBinary invocation', async () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
    const deferred = makeDeferred<string | null>();
    const runSpy = jest.spyOn(downloader as any, 'runEnsureBinary')
      .mockReturnValue(deferred.promise);

    const p1 = downloader.ensureBinary(true);
    const p2 = downloader.ensureBinary(true);

    await new Promise<void>(r => setImmediate(r));

    deferred.resolve('/path/from/force');
    const [r1, r2] = await Promise.all([p1, p2]);

    expect(runSpy).toHaveBeenCalledTimes(1);
    expect(r1).toBe('/path/from/force');
    expect(r2).toBe('/path/from/force');
  });

  test('force during ensure waits for ensure to finish then runs its own install', async () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
    const ensureDeferred = makeDeferred<string | null>();
    const forceDeferred = makeDeferred<string | null>();
    const runSpy = jest.spyOn(downloader as any, 'runEnsureBinary')
      .mockReturnValueOnce(ensureDeferred.promise)
      .mockReturnValueOnce(forceDeferred.promise);

    const ensureCall = downloader.ensureBinary(false);
    await new Promise<void>(r => setImmediate(r));
    const forceCall = downloader.ensureBinary(true);

    // Force has not yet started a new install — it is waiting on the active ensure.
    await new Promise<void>(r => setImmediate(r));
    expect(runSpy).toHaveBeenCalledTimes(1);

    ensureDeferred.resolve('/path/from/ensure');
    expect(await ensureCall).toBe('/path/from/ensure');

    // Now the force call runs its own install.
    await new Promise<void>(r => setImmediate(r));
    expect(runSpy).toHaveBeenCalledTimes(2);

    forceDeferred.resolve('/path/from/force');
    expect(await forceCall).toBe('/path/from/force');
  });

  test('singleflight state is cleared after each install settles', async () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
    const runSpy = jest.spyOn(downloader as any, 'runEnsureBinary')
      .mockResolvedValueOnce('/path/first')
      .mockResolvedValueOnce('/path/second');

    expect(await downloader.ensureBinary(false)).toBe('/path/first');
    expect(await downloader.ensureBinary(false)).toBe('/path/second');

    expect(runSpy).toHaveBeenCalledTimes(2);
  });

  test('failure of an in-flight install does not poison subsequent calls', async () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
    const runSpy = jest.spyOn(downloader as any, 'runEnsureBinary')
      .mockRejectedValueOnce(new Error('install boom'))
      .mockResolvedValueOnce('/path/recovered');

    await expect(downloader.ensureBinary(false)).rejects.toThrow('install boom');
    expect(await downloader.ensureBinary(false)).toBe('/path/recovered');

    expect(runSpy).toHaveBeenCalledTimes(2);
  });

  test('force after force settles cleanly with two separate installs', async () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
    const runSpy = jest.spyOn(downloader as any, 'runEnsureBinary')
      .mockResolvedValueOnce('/path/force-1')
      .mockResolvedValueOnce('/path/force-2');

    expect(await downloader.ensureBinary(true)).toBe('/path/force-1');
    expect(await downloader.ensureBinary(true)).toBe('/path/force-2');

    expect(runSpy).toHaveBeenCalledTimes(2);
  });
});

// ---------------------------------------------------------------------------
// SHA-256 checksum calculation
// ---------------------------------------------------------------------------
describe('BinaryDownloader.calculateSHA256', () => {
  let tmpFile: string;

  afterEach(() => {
    if (tmpFile && fs.existsSync(tmpFile)) {
      fs.unlinkSync(tmpFile);
    }
  });

  test('computes correct SHA256 for known content', async () => {
    tmpFile = path.join(os.tmpdir(), `sha-test-${Date.now()}`);
    const content = 'hello world\n';
    fs.writeFileSync(tmpFile, content);

    const expected = crypto.createHash('sha256').update(content).digest('hex');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const actual = await downloader.calculateSHA256(tmpFile);

    expect(actual).toBe(expected);
  });

  test('computes correct SHA256 for empty file', async () => {
    tmpFile = path.join(os.tmpdir(), `sha-empty-${Date.now()}`);
    fs.writeFileSync(tmpFile, '');

    const expected = crypto.createHash('sha256').update('').digest('hex');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const actual = await downloader.calculateSHA256(tmpFile);

    expect(actual).toBe(expected);
  });

  test('computes correct SHA256 for binary content', async () => {
    tmpFile = path.join(os.tmpdir(), `sha-bin-${Date.now()}`);
    const buf = Buffer.from([0x00, 0xff, 0xde, 0xad, 0xbe, 0xef]);
    fs.writeFileSync(tmpFile, buf);

    const expected = crypto.createHash('sha256').update(buf).digest('hex');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const actual = await downloader.calculateSHA256(tmpFile);

    expect(actual).toBe(expected);
  });
});

// ---------------------------------------------------------------------------
// findBinary (recursive directory search)
// ---------------------------------------------------------------------------
describe('BinaryDownloader.findBinary', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'find-bin-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('finds binary in top-level directory', () => {
    fs.writeFileSync(path.join(tmpDir, 'perllsp'), 'binary');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.findBinary(tmpDir, 'perllsp');

    expect(result).toBe(path.join(tmpDir, 'perllsp'));
  });

  test('finds binary in nested directory', () => {
    const nested = path.join(tmpDir, 'subdir', 'bin');
    fs.mkdirSync(nested, { recursive: true });
    fs.writeFileSync(path.join(nested, 'perllsp'), 'binary');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.findBinary(tmpDir, 'perllsp');

    expect(result).toBe(path.join(nested, 'perllsp'));
  });

  test('returns null when binary is not found', () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.findBinary(tmpDir, 'nonexistent');

    expect(result).toBeNull();
  });

  test('ignores files with different names', () => {
    fs.writeFileSync(path.join(tmpDir, 'not-perllsp'), 'wrong');
    fs.writeFileSync(path.join(tmpDir, 'perllsp.old'), 'wrong');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.findBinary(tmpDir, 'perllsp');

    expect(result).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Download URL security validation (downloadFile method)
// ---------------------------------------------------------------------------
describe('BinaryDownloader download URL security', () => {
  let downloader: any;
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dl-sec-'));
    downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  const dest = () => path.join(tmpDir, 'test-download');

  test('rejects FTP protocol', async () => {
    await expect(
      downloader.downloadFile('ftp://example.com/file', dest(), 1000)
    ).rejects.toThrow(/Unsupported protocol/);
  });

  test('rejects file:// protocol', async () => {
    await expect(
      downloader.downloadFile('file:///etc/passwd', dest(), 1000)
    ).rejects.toThrow(/Unsupported protocol/);
  });

  test('rejects data: protocol', async () => {
    await expect(
      downloader.downloadFile('data:text/plain,hello', dest(), 1000)
    ).rejects.toThrow(/Unsupported protocol/);
  });

  test('rejects HTTP for remote hosts', async () => {
    await expect(
      downloader.downloadFile('http://evil.example.com/malware', dest(), 1000)
    ).rejects.toThrow(/Security violation.*Insecure HTTP/);
  });

  test('rejects HTTP for remote IP addresses', async () => {
    await expect(
      downloader.downloadFile('http://192.168.1.1/file', dest(), 1000)
    ).rejects.toThrow(/Security violation.*Insecure HTTP/);
  });

  test('allows HTTP for localhost (fails on connection, not security)', async () => {
    await expect(
      downloader.downloadFile('http://localhost:9999/file', dest(), 500)
    ).rejects.not.toThrow(/Security violation/);
  });

  test('allows HTTP for 127.0.0.1', async () => {
    await expect(
      downloader.downloadFile('http://127.0.0.1:9999/file', dest(), 500)
    ).rejects.not.toThrow(/Security violation/);
  });

  test('allows HTTP for 127.x.y.z loopback range', async () => {
    await expect(
      downloader.downloadFile('http://127.0.0.2:9999/file', dest(), 500)
    ).rejects.not.toThrow(/Security violation/);
  });

  // NOTE: Node 24 URL.hostname includes brackets for IPv6 (e.g. "[::1]"),
  // so the loopback check for '::1' does not match. This documents the
  // current behavior; fixing it is tracked separately.
  test('rejects HTTP for IPv6 loopback due to bracket mismatch (known limitation)', async () => {
    await expect(
      downloader.downloadFile('http://[::1]:9999/file', dest(), 500)
    ).rejects.toThrow(/Security violation/);
  });

  test('allows HTTP for subdomain of localhost', async () => {
    await expect(
      downloader.downloadFile('http://foo.localhost:9999/file', dest(), 500)
    ).rejects.not.toThrow(/Security violation/);
  });

  test('rejects invalid URL format', async () => {
    await expect(
      downloader.downloadFile('not-a-url', dest(), 1000)
    ).rejects.toThrow(/Invalid URL/);
  });
});

// ---------------------------------------------------------------------------
// Asset name validation (path traversal prevention)
// ---------------------------------------------------------------------------
describe('asset name validation', () => {
  test('valid asset names pass the regex', () => {
    const validNames = [
      'perllsp-v0.12.0-x86_64-unknown-linux-gnu.tar.gz',
      'perllsp-v0.12.0-aarch64-apple-darwin.tar.gz',
      'perllsp-v0.12.0-x86_64-pc-windows-msvc.zip',
      'SHA256SUMS',
    ];
    const pattern = /^[a-zA-Z0-9_.-]+$/;
    for (const name of validNames) {
      expect(pattern.test(name) && !name.includes('..')).toBe(true);
    }
  });

  test('malicious asset names are rejected', () => {
    const maliciousNames = [
      '../../../etc/passwd',
      'perllsp/../../../etc/shadow',
      'perllsp; rm -rf /',
      'perllsp\x00.tar.gz',
      'perllsp$(whoami).tar.gz',
    ];
    const pattern = /^[a-zA-Z0-9_.-]+$/;
    for (const name of maliciousNames) {
      expect(pattern.test(name) && !name.includes('..')).toBe(false);
    }
  });
});

// ---------------------------------------------------------------------------
// Release asset candidate selection
// ---------------------------------------------------------------------------
describe('release asset candidate selection', () => {
  test('finds non-v Windows asset for v-prefixed release tag', () => {
    const assetName = 'perllsp-0.13.1-x86_64-pc-windows-msvc.zip';
    const found = findReleaseAssetName(
      [{ name: assetName }, { name: 'SHA256SUMS' }],
      'v0.13.1',
      'x86_64-pc-windows-msvc',
      '.zip'
    );

    expect(found).toBe(assetName);
  });

  test('finds non-v Linux asset for v-prefixed release tag', () => {
    const assetName = 'perllsp-0.13.1-x86_64-unknown-linux-gnu.tar.gz';
    const found = findReleaseAssetName(
      [{ name: assetName }, { name: 'SHA256SUMS' }],
      'v0.13.1',
      'x86_64-unknown-linux-gnu',
      '.tar.gz'
    );

    expect(found).toBe(assetName);
  });

  test('prefers release workflow non-v asset before v-prefixed alias', () => {
    const candidates = buildBinaryAssetCandidateNames(
      'v0.13.1',
      'x86_64-pc-windows-msvc',
      '.zip'
    );

    expect(candidates[0]).toBe('perllsp-0.13.1-x86_64-pc-windows-msvc.zip');
    expect(candidates).toContain('perllsp-v0.13.1-x86_64-pc-windows-msvc.zip');
  });
});

// ---------------------------------------------------------------------------
// musl detection
// ---------------------------------------------------------------------------
describe('BinaryDownloader.detectMusl', () => {
  test('detectMusl returns a boolean', () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.detectMusl();
    expect(typeof result).toBe('boolean');
  });
});

// ---------------------------------------------------------------------------
// parseLocalVersion — pure function
// ---------------------------------------------------------------------------
describe('parseLocalVersion', () => {
  test('extracts version from standard --version output', () => {
    const out = 'perllsp 0.12.0\nGit tag: v0.12.0\nPerl Language Server using perl-parser v3\n';
    expect(parseLocalVersion(out)).toBe('0.12.0');
  });

  test('returns null on unexpected format', () => {
    expect(parseLocalVersion('something else entirely')).toBeNull();
  });

  test('handles trailing whitespace on first line', () => {
    expect(parseLocalVersion('perllsp 0.12.1  \n')).toBe('0.12.1');
  });

  test('returns null on empty string', () => {
    expect(parseLocalVersion('')).toBeNull();
  });

  test('handles single-line output with no newline', () => {
    expect(parseLocalVersion('perllsp 0.13.0')).toBe('0.13.0');
  });

  test('handles Windows CRLF line endings', () => {
    // On Windows, execFile stdout may use \r\n; trim() strips the \r from the first line.
    const out = 'perllsp 0.12.0\r\nGit tag: v0.12.0\r\n';
    expect(parseLocalVersion(out)).toBe('0.12.0');
  });
});

// ---------------------------------------------------------------------------
// compareVersions — pure function
// ---------------------------------------------------------------------------
describe('compareVersions', () => {
  test('equal versions return 0', () => {
    expect(compareVersions('0.12.0', '0.12.0')).toBe(0);
  });

  test('v-prefix on remote is stripped', () => {
    expect(compareVersions('0.12.0', 'v0.12.0')).toBe(0);
  });

  test('v-prefix on local is stripped', () => {
    expect(compareVersions('v0.12.0', '0.12.0')).toBe(0);
  });

  test('patch bump: local older returns -1', () => {
    expect(compareVersions('0.12.0', '0.12.1')).toBe(-1);
  });

  test('minor bump with larger number not fooled by lexicographic compare', () => {
    // lexicographic "9" > "10" — numeric must return -1
    expect(compareVersions('0.9.0', '0.10.0')).toBe(-1);
  });

  test('local ahead of remote returns 1', () => {
    expect(compareVersions('0.13.0', '0.12.0')).toBe(1);
  });

  test('major bump detected correctly', () => {
    expect(compareVersions('0.12.0', '1.0.0')).toBe(-1);
  });

  test('patch downgrade returns 1', () => {
    expect(compareVersions('0.12.1', '0.12.0')).toBe(1);
  });

  test('malformed local version (NaN components) returns 0 — no spurious notification', () => {
    // If parseInt produces NaN, NaN < x and NaN > x are both false,
    // so the loop exits unchanged and the function returns 0 (treat as equal).
    // This prevents a malformed binary stdout from triggering a spurious update prompt.
    expect(compareVersions('not-a-version', '0.12.0')).toBe(0);
  });

  test('malformed remote version (NaN components) returns 0 — no spurious notification', () => {
    expect(compareVersions('0.12.0', 'not-a-version')).toBe(0);
  });

  test('pre-release suffix is ignored by parseInt — treats 0.12.0-rc1 same as 0.12.0', () => {
    // parseInt('0-rc1', 10) === 0, so patch component is 0 for both sides.
    // Document this known limitation: rc builds are treated as equal to their release.
    expect(compareVersions('0.12.0-rc1', '0.12.0')).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// checkForUpdateSilent — integration of version check + notification
// ---------------------------------------------------------------------------
describe('checkForUpdateSilent', () => {
  // Build a full context mock that includes globalState storage.
  function makeFullContext(storagePath?: string): any {
    const dir = storagePath ?? fs.mkdtempSync(path.join(os.tmpdir(), 'dl-full-'));
    const store = new Map<string, unknown>();
    return {
      globalStorageUri: { fsPath: dir },
      extensionPath: dir,
      subscriptions: [],
      globalState: {
        get: jest.fn((key: string, defaultValue?: unknown) =>
          store.has(key) ? store.get(key) : defaultValue
        ),
        update: jest.fn((key: string, value: unknown) => {
          store.set(key, value);
          return Promise.resolve();
        }),
        _store: store,
      },
    };
  }

  let ctx: any;
  let outputChannel: any;
  let downloader: BinaryDownloader;
  let tmpBinary: string;

  beforeEach(() => {
    // Reset all mocks so call counts don't bleed between tests.
    jest.clearAllMocks();

    ctx = makeFullContext();
    outputChannel = {
      appendLine: jest.fn(),
      show: jest.fn(),
      dispose: jest.fn(),
    };
    downloader = new BinaryDownloader(ctx, outputChannel);

    // Place a stub binary in the expected auto-download location so
    // fs.existsSync passes.
    const binaryName = process.platform === 'win32' ? 'perllsp.exe' : 'perllsp';
    const binDir = path.join(ctx.globalStorageUri.fsPath, 'bin', `${process.platform}-${process.arch}`);
    fs.mkdirSync(binDir, { recursive: true });
    tmpBinary = path.join(binDir, binaryName);
    fs.writeFileSync(tmpBinary, '#!/bin/sh\necho "perllsp 0.12.0"');
  });

  afterEach(() => {
    // Clean up temp storage directory
    try {
      fs.rmSync(ctx.globalStorageUri.fsPath, { recursive: true, force: true });
    } catch (_e) {
      // ignore
    }
    jest.restoreAllMocks();
  });

  // Helper: configure getConfiguration mock to return specific values.
  function mockConfig(overrides: Record<string, unknown>): void {
    const vscode = require('vscode');
    vscode.workspace.getConfiguration.mockReturnValue({
      get: jest.fn((key: string, defaultValue?: unknown) => {
        if (key in overrides) return overrides[key];
        return defaultValue;
      }),
      update: jest.fn(),
    });
  }

  test('no-ops when channel is "tag" (user pinned a version)', async () => {
    mockConfig({ channel: 'tag' });
    const getLatestSpy = jest.spyOn(downloader as any, 'getLatestRelease');

    await downloader.checkForUpdateSilent();

    expect(getLatestSpy).not.toHaveBeenCalled();
  });

  test('no-ops when serverPath is user-configured', async () => {
    mockConfig({ channel: 'latest', serverPath: '/custom/perllsp' });
    const getLatestSpy = jest.spyOn(downloader as any, 'getLatestRelease');

    await downloader.checkForUpdateSilent();

    expect(getLatestSpy).not.toHaveBeenCalled();
  });

  test('no-ops when updateCheckInterval is 0 (disabled)', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 0 });
    const getLatestSpy = jest.spyOn(downloader as any, 'getLatestRelease');

    await downloader.checkForUpdateSilent();

    expect(getLatestSpy).not.toHaveBeenCalled();
  });

  test('no-ops when updateCheckInterval is negative (treated as disabled)', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: -1 });
    const getLatestSpy = jest.spyOn(downloader as any, 'getLatestRelease');

    await downloader.checkForUpdateSilent();

    expect(getLatestSpy).not.toHaveBeenCalled();
  });

  test('no-ops when the interval has not elapsed', async () => {
    // Set lastUpdateCheck to "just now" so elapsed < interval
    ctx.globalState._store.set('perl-lsp.lastUpdateCheck', Date.now());
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24 });
    const getLatestSpy = jest.spyOn(downloader as any, 'getLatestRelease');

    await downloader.checkForUpdateSilent();

    expect(getLatestSpy).not.toHaveBeenCalled();
  });

  test('no-ops when versions are equal — no notification shown', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24 });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.12.0',
      assets: [],
    });
    const vscode = require('vscode');
    vscode.window.showInformationMessage.mockResolvedValue(undefined);

    await downloader.checkForUpdateSilent();

    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
  });

  test('no-ops when local version is ahead — no notification shown', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24 });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.13.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.12.0',
      assets: [],
    });
    const vscode = require('vscode');
    vscode.window.showInformationMessage.mockResolvedValue(undefined);

    await downloader.checkForUpdateSilent();

    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
  });

  test('shows notification when remote version is newer', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24, autoUpdate: false });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.13.0',
      assets: [],
    });
    const vscode = require('vscode');
    vscode.window.showInformationMessage.mockResolvedValue(undefined);

    await downloader.checkForUpdateSilent();

    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      expect.stringContaining('0.13.0'),
      'Update',
      'Dismiss',
      "Don't ask again"
    );
  });

  test('notification message contains installed version', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24, autoUpdate: false });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.13.0',
      assets: [],
    });
    const vscode = require('vscode');
    vscode.window.showInformationMessage.mockResolvedValue(undefined);

    await downloader.checkForUpdateSilent();

    expect(vscode.window.showInformationMessage).toHaveBeenCalledWith(
      expect.stringContaining('0.12.0'),
      expect.anything(),
      expect.anything(),
      expect.anything()
    );
  });

  test('"Don\'t ask again" sets updateCheckInterval to 0', async () => {
    const updateFn = jest.fn();
    const vscode = require('vscode');
    vscode.workspace.getConfiguration.mockReturnValue({
      get: jest.fn((key: string, defaultValue?: unknown) => {
        const cfg: Record<string, unknown> = {
          channel: 'latest',
          serverPath: '',
          updateCheckInterval: 24,
          autoUpdate: false,
        };
        return key in cfg ? cfg[key] : defaultValue;
      }),
      update: updateFn,
    });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.13.0',
      assets: [],
    });
    vscode.window.showInformationMessage.mockResolvedValue("Don't ask again");

    await downloader.checkForUpdateSilent();

    // ConfigurationTarget.Global === 1 in the vscode mock
    expect(updateFn).toHaveBeenCalledWith('updateCheckInterval', 0, 1);
  });

  test('silent failure — logs error but shows no notification on network error', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24 });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockRejectedValue(
      new Error('ETIMEDOUT')
    );
    const vscode = require('vscode');
    vscode.window.showInformationMessage.mockResolvedValue(undefined);

    await downloader.checkForUpdateSilent();

    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
    expect(outputChannel.appendLine).toHaveBeenCalledWith(
      expect.stringContaining('[update-check]')
    );
  });

  test('silent failure — logs error but shows no notification when getLocalVersion returns null', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24 });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue(null);
    const getLatestSpy = jest.spyOn(downloader as any, 'getLatestRelease');
    const vscode = require('vscode');
    vscode.window.showInformationMessage.mockResolvedValue(undefined);

    await downloader.checkForUpdateSilent();

    // getLatestRelease should NOT be called if local version cannot be read
    expect(getLatestSpy).not.toHaveBeenCalled();
    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
  });

  test('records lastUpdateCheck timestamp when check runs', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24 });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.12.0',
      assets: [],
    });

    const before = Date.now();
    await downloader.checkForUpdateSilent();
    const after = Date.now();

    expect(ctx.globalState.update).toHaveBeenCalledWith(
      'perl-lsp.lastUpdateCheck',
      expect.any(Number)
    );
    const recorded = ctx.globalState._store.get('perl-lsp.lastUpdateCheck') as number;
    expect(recorded).toBeGreaterThanOrEqual(before);
    expect(recorded).toBeLessThanOrEqual(after);
  });

  test('strips "v" prefix from remote tag_name before comparison', async () => {
    // Remote tag is "v0.12.0"; local is "0.12.0" — should be treated as equal.
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24 });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.12.0',
      assets: [],
    });
    const vscode = require('vscode');
    vscode.window.showInformationMessage.mockResolvedValue(undefined);

    await downloader.checkForUpdateSilent();

    // Must NOT show notification — they are equal after normalization
    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
  });

  test('autoUpdate=true triggers ensureBinary without showing a notification', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24, autoUpdate: true });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.13.0',
      assets: [],
    });
    const ensureSpy = jest.spyOn(downloader as any, 'ensureBinary').mockResolvedValue('/path/to/perllsp');
    const vscode = require('vscode');

    await downloader.checkForUpdateSilent();

    // No prompt — downloads silently
    expect(vscode.window.showInformationMessage).not.toHaveBeenCalled();
    expect(ensureSpy).toHaveBeenCalledWith(true);
  });

  test('"Update" button click triggers ensureBinary', async () => {
    mockConfig({ channel: 'latest', serverPath: '', updateCheckInterval: 24, autoUpdate: false });
    jest.spyOn(downloader as any, 'getLocalVersion').mockResolvedValue('0.12.0');
    jest.spyOn(downloader as any, 'getLatestRelease').mockResolvedValue({
      tag_name: 'v0.13.0',
      assets: [],
    });
    const ensureSpy = jest.spyOn(downloader as any, 'ensureBinary').mockResolvedValue('/path/to/perllsp');
    const vscode = require('vscode');
    vscode.window.showInformationMessage.mockResolvedValue('Update');

    await downloader.checkForUpdateSilent();

    expect(ensureSpy).toHaveBeenCalledWith(true);
  });
});

// ---------------------------------------------------------------------------
// ensureBinary error classification — actionable messages (#3274)
// ---------------------------------------------------------------------------
describe('ensureBinary error classification', () => {
  let ctx: any;
  let outputChannel: any;
  let downloader: BinaryDownloader;

  function makeFullContext(storagePath?: string): any {
    const dir = storagePath ?? fs.mkdtempSync(path.join(os.tmpdir(), 'dl-err-'));
    const store = new Map<string, unknown>();
    return {
      globalStorageUri: { fsPath: dir },
      extensionPath: dir,
      subscriptions: [],
      globalState: {
        get: jest.fn((key: string, defaultValue?: unknown) =>
          store.has(key) ? store.get(key) : defaultValue
        ),
        update: jest.fn(() => Promise.resolve()),
        _store: store,
      },
    };
  }

  beforeEach(() => {
    jest.clearAllMocks();
    ctx = makeFullContext();
    outputChannel = { appendLine: jest.fn(), show: jest.fn(), dispose: jest.fn() };
    downloader = new BinaryDownloader(ctx, outputChannel);

    // Prevent actual download attempts
    jest.spyOn(downloader as any, 'downloadWithProgress').mockRejectedValue(
      new Error('placeholder')
    );
  });

  afterEach(() => {
    try {
      fs.rmSync(ctx.globalStorageUri.fsPath, { recursive: true, force: true });
    } catch (_e) { /* ignore */ }
    jest.restoreAllMocks();
  });

  function setupDownloadError(errorMessage: string) {
    jest.spyOn(downloader as any, 'downloadWithProgress').mockRejectedValue(
      new Error(errorMessage)
    );
  }

  test('network timeout shows message containing proxy/VPN guidance and manual install path', async () => {
    setupDownloadError('Download timeout after 30 seconds');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    expect(vscode.window.showErrorMessage).toHaveBeenCalledWith(
      expect.stringMatching(/proxy|VPN|network/i),
      expect.anything(),
      expect.anything()
    );
    // Must mention the manual install setting
    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('ETIMEDOUT shows message containing proxy/VPN guidance and manual install path', async () => {
    setupDownloadError('connect ETIMEDOUT 140.82.121.3:443');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/proxy|VPN|network/i);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('ECONNREFUSED shows message containing proxy/VPN guidance and manual install path', async () => {
    setupDownloadError('connect ECONNREFUSED 127.0.0.1:443');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('arch mismatch shows message naming the attempted target', async () => {
    setupDownloadError('No binary found for platform: arm64-unknown-linux-gnu. Available assets: perllsp-x86_64-unknown-linux-gnu.tar.gz');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    // Must include the platform string and the manual install setting
    expect(call[0]).toMatch(/arm64-unknown-linux-gnu/);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('termux platform mismatch shows Termux-specific source-build guidance', async () => {
    setupDownloadError('No binary found for platform: aarch64-linux-android. Available assets: perllsp-x86_64-unknown-linux-gnu.tar.gz');
    jest.spyOn(downloader as any, 'isTermuxEnvironment').mockReturnValue(true);
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/Termux|Android/i);
    expect(call[0]).toMatch(/pkg install rust/i);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('HTTP 403 rate limit shows GitHub rate limit guidance', async () => {
    setupDownloadError('Failed to download: HTTP 403');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/403|rate.?limit|GITHUB_TOKEN/i);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('HTTP 404 shows not-found guidance with download URL', async () => {
    setupDownloadError('Failed to download: HTTP 404');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/404|not found/i);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('checksum failure shows corruption message and retry guidance', async () => {
    setupDownloadError('Security check failed: Checksum verification failed (file may be corrupted or tampered with).');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/checksum|corrupt|retry/i);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('checksum-not-found in SHA256SUMS shows corruption message (case-insensitive match)', async () => {
    // This error has capital-C "Checksum" — verifies the classifier uses case-insensitive matching
    setupDownloadError('Security check failed: Checksum for perllsp-x86_64-unknown-linux-gnu.tar.gz not found in SHA256SUMS file.');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/checksum|corrupt|retry/i);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('missing SHA256SUMS file routes to checksum guidance not generic error', async () => {
    // 'Security check failed: No SHA256SUMS file found in release assets.' contains 'SHA256SUMS'
    // but not 'checksum' (any case) — the classifier must catch it explicitly
    setupDownloadError('Security check failed: No SHA256SUMS file found in release assets.');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/checksum|corrupt|retry/i);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('archive extraction failure shows tar/zip guidance', async () => {
    setupDownloadError('Failed to extract archive: tar exited with code 1');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/extract|tar|zip/i);
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('unknown error still mentions manual install path', async () => {
    setupDownloadError('some unexpected error occurred');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    expect(call[0]).toMatch(/perl-lsp\.serverPath/);
  });

  test('error message always offers "Install Manually" button', async () => {
    setupDownloadError('some unexpected error occurred');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue(undefined);

    await downloader.ensureBinary();

    const call = vscode.window.showErrorMessage.mock.calls[0];
    const buttons: string[] = call.slice(1);
    expect(buttons).toContain('Install Manually');
  });

  test('"Install Manually" button opens the manual install URL', async () => {
    setupDownloadError('some unexpected error occurred');
    const vscode = require('vscode');
    vscode.window.showErrorMessage.mockResolvedValue('Install Manually');

    await downloader.ensureBinary();

    expect(vscode.env.openExternal).toHaveBeenCalledWith(
      expect.objectContaining({ toString: expect.any(Function) })
    );
    const uriArg = vscode.env.openExternal.mock.calls[0][0];
    expect(uriArg.toString()).toMatch(/github\.com.*perl-lsp/i);
  });
});
