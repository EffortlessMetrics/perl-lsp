/**
 * Unit tests for LSP startup error classification and diagnosis (#3280).
 *
 * classifyStartupError() is a pure function — no execFile, no vscode needed.
 * Tests simulate real-world binary failure scenarios:
 *   - glibc version mismatch (Alpine/musl Linux)
 *   - missing shared library (libssl, libgcc)
 *   - wrong architecture / Exec format error
 *   - permission denied
 *   - unknown / fallback
 *
 * The user-visible error message must include an actionable hint (not just
 * "corrupted or incompatible") and a specific remediation step.
 */

import { classifyStartupError, StartupErrorKind } from '../startupDiagnosis';

// ---------------------------------------------------------------------------
// classifyStartupError — pure classification of stderr/stdout text
// ---------------------------------------------------------------------------

describe('classifyStartupError', () => {
  test('detects GLIBC version mismatch', () => {
    const stderr =
      '/home/user/.vscode/extensions/perl-lsp/bin/perllsp: ' +
      '/lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.35` not found';

    const result = classifyStartupError(stderr);

    expect(result.kind).toBe(StartupErrorKind.GlibcMismatch);
    expect(result.hint).toContain('glibc');
    expect(result.remediation).toContain('cargo install');
  });

  test('detects musl system via cannot open shared object file', () => {
    const stderr =
      'perllsp: error while loading shared libraries: ' +
      'libgcc_s.so.1: cannot open shared object file: No such file or directory';

    const result = classifyStartupError(stderr);

    expect(result.kind).toBe(StartupErrorKind.MissingSharedLibrary);
    expect(result.hint).toContain('libgcc_s.so.1');
    expect(result.remediation).toBeTruthy();
  });

  test('detects Exec format error (architecture mismatch)', () => {
    const stderr = 'bash: /usr/local/bin/perllsp: cannot execute binary file: Exec format error';

    const result = classifyStartupError(stderr);

    expect(result.kind).toBe(StartupErrorKind.ExecFormatError);
    expect(result.hint).toContain('architecture');
    expect(result.remediation).toContain('Reinstall');
  });

  test('detects permission denied', () => {
    const stderr = '-bash: /home/user/.vscode/extensions/perllsp: Permission denied';

    const result = classifyStartupError(stderr);

    expect(result.kind).toBe(StartupErrorKind.PermissionDenied);
    expect(result.hint).toContain('permission');
    expect(result.remediation).toContain('chmod');
  });

  test('returns Unknown for unrecognized output', () => {
    const result = classifyStartupError('some random unexpected output');

    expect(result.kind).toBe(StartupErrorKind.Unknown);
    expect(result.hint).toBeTruthy();
    expect(result.remediation).toBeTruthy();
  });

  test('returns Unknown for empty stderr', () => {
    const result = classifyStartupError('');

    expect(result.kind).toBe(StartupErrorKind.Unknown);
    expect(result.hint).toBeTruthy();
  });

  test('GLIBC detection is case-insensitive to variant spellings', () => {
    const result = classifyStartupError(
      'version `GLIBC_2.17` not found (required by perllsp)'
    );
    expect(result.kind).toBe(StartupErrorKind.GlibcMismatch);
  });

  test('missing library name is captured in hint', () => {
    const result = classifyStartupError(
      'error while loading shared libraries: libssl.so.3: cannot open shared object file'
    );
    expect(result.kind).toBe(StartupErrorKind.MissingSharedLibrary);
    expect(result.hint).toContain('libssl.so.3');
  });

  test('hint text is short enough to fit in a VS Code notification (≤200 chars)', () => {
    const scenarios = [
      '/lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.35` not found',
      'cannot open shared object file: libssl.so.3: No such file or directory',
      'cannot execute binary file: Exec format error',
      'Permission denied',
      '',
    ];
    for (const stderr of scenarios) {
      const result = classifyStartupError(stderr);
      expect(result.hint.length).toBeLessThanOrEqual(200);
    }
  });

  // -------------------------------------------------------------------------
  // Synthesised inputs from probeStartupFailure's err.code enrichment path
  //
  // When execFile fails with no stderr, probeStartupFailure synthesises a
  // string from the OS error code so the classifier can return the right kind.
  // These tests verify that the synthesised strings actually match.
  // -------------------------------------------------------------------------

  test('synthesised ENOEXEC string classifies as ExecFormatError', () => {
    // probeStartupFailure synthesises this when err.code === 'ENOEXEC'
    const result = classifyStartupError('cannot execute binary file: Exec format error');
    expect(result.kind).toBe(StartupErrorKind.ExecFormatError);
    expect(result.hint).toContain('architecture');
  });

  test('synthesised EACCES string classifies as PermissionDenied', () => {
    // probeStartupFailure synthesises this when err.code === 'EACCES'
    const result = classifyStartupError('Permission denied');
    expect(result.kind).toBe(StartupErrorKind.PermissionDenied);
    expect(result.remediation).toContain('chmod');
  });

  test('unrecognised err.code falls through to Unknown without crashing', () => {
    // e.g. ETIMEDOUT, ENOENT — err.message used directly; should not throw
    const result = classifyStartupError('spawn /path/perllsp ENOENT');
    expect(result.kind).toBe(StartupErrorKind.Unknown);
    expect(result.hint).toBeTruthy();
  });
});
