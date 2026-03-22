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
import { BinaryDownloader, parseLocalVersion, compareVersions } from '../downloader';

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

  beforeEach(() => {
    downloader = new BinaryDownloader(makeContext(), makeOutputChannel());
  });

  function getPlatformTarget(dl: any): string {
    return dl.getPlatformTarget();
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

  test('binary name is perl-lsp (or perl-lsp.exe on win32)', () => {
    const ctx = makeContext('/tmp/test-storage');
    const downloader = new BinaryDownloader(ctx, makeOutputChannel()) as any;
    const binaryPath: string = downloader.getLocalBinaryPath();
    const basename = path.basename(binaryPath);

    if (process.platform === 'win32') {
      expect(basename).toBe('perl-lsp.exe');
    } else {
      expect(basename).toBe('perl-lsp');
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
    fs.writeFileSync(path.join(tmpDir, 'perl-lsp'), 'binary');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.findBinary(tmpDir, 'perl-lsp');

    expect(result).toBe(path.join(tmpDir, 'perl-lsp'));
  });

  test('finds binary in nested directory', () => {
    const nested = path.join(tmpDir, 'subdir', 'bin');
    fs.mkdirSync(nested, { recursive: true });
    fs.writeFileSync(path.join(nested, 'perl-lsp'), 'binary');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.findBinary(tmpDir, 'perl-lsp');

    expect(result).toBe(path.join(nested, 'perl-lsp'));
  });

  test('returns null when binary is not found', () => {
    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.findBinary(tmpDir, 'nonexistent');

    expect(result).toBeNull();
  });

  test('ignores files with different names', () => {
    fs.writeFileSync(path.join(tmpDir, 'not-perl-lsp'), 'wrong');
    fs.writeFileSync(path.join(tmpDir, 'perl-lsp.old'), 'wrong');

    const downloader = new BinaryDownloader(makeContext(), makeOutputChannel()) as any;
    const result = downloader.findBinary(tmpDir, 'perl-lsp');

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
      'perl-lsp-v0.12.0-x86_64-unknown-linux-gnu.tar.gz',
      'perl-lsp-v0.12.0-aarch64-apple-darwin.tar.gz',
      'perl-lsp-v0.12.0-x86_64-pc-windows-msvc.zip',
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
      'perl-lsp/../../../etc/shadow',
      'perl-lsp; rm -rf /',
      'perl-lsp\x00.tar.gz',
      'perl-lsp$(whoami).tar.gz',
    ];
    const pattern = /^[a-zA-Z0-9_.-]+$/;
    for (const name of maliciousNames) {
      expect(pattern.test(name) && !name.includes('..')).toBe(false);
    }
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
    const out = 'perl-lsp 0.12.0\nGit tag: v0.12.0\nPerl Language Server using perl-parser v3\n';
    expect(parseLocalVersion(out)).toBe('0.12.0');
  });

  test('returns null on unexpected format', () => {
    expect(parseLocalVersion('something else entirely')).toBeNull();
  });

  test('handles trailing whitespace on first line', () => {
    expect(parseLocalVersion('perl-lsp 0.12.1  \n')).toBe('0.12.1');
  });

  test('returns null on empty string', () => {
    expect(parseLocalVersion('')).toBeNull();
  });

  test('handles single-line output with no newline', () => {
    expect(parseLocalVersion('perl-lsp 0.13.0')).toBe('0.13.0');
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
});
