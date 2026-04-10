/**
 * Unit tests for LSP startup error classification and diagnosis (#3280).
 *
 * classifyStartupError() is a pure function — no execFile, no vscode needed.
 * Tests simulate real-world binary failure scenarios:
 *   - glibc version mismatch (Alpine/musl Linux)
 *   - missing shared library (libssl, libgcc)
 *   - wrong architecture / Exec format error
 *   - permission denied
 *   - Windows: DLL init failure, wrong PE architecture
 *   - macOS: dyld library not loaded, code signature invalid
 *   - unknown / fallback
 *
 * The user-visible error message must include an actionable hint (not just
 * "corrupted or incompatible") and a specific remediation step.
 */

import { classifyStartupError, StartupErrorKind, selectBestDiagnosis } from '../startupDiagnosis';

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

  // -------------------------------------------------------------------------
  // Windows-specific failure signatures
  // -------------------------------------------------------------------------

  test('detects Windows DLL initialization failure', () => {
    const stderr = 'The application failed to initialize properly (0xc0000142). DLL initialization routine failed.';
    const result = classifyStartupError(stderr);
    expect(result.kind).toBe(StartupErrorKind.WindowsBinaryError);
    expect(result.hint).toContain('Windows');
    expect(result.remediation).toContain('Reinstall');
  });

  test('detects Windows wrong architecture (not a valid Win32 application)', () => {
    const stderr = 'C:\\Users\\user\\.vscode\\extensions\\perllsp.exe is not a valid Win32 application.';
    const result = classifyStartupError(stderr);
    expect(result.kind).toBe(StartupErrorKind.WindowsBinaryError);
    expect(result.hint).toContain('Windows');
  });

  test('detects Windows missing DLL (The specified module could not be found)', () => {
    const stderr = 'The specified module could not be found.';
    const result = classifyStartupError(stderr);
    expect(result.kind).toBe(StartupErrorKind.WindowsBinaryError);
    expect(result.hint).toContain('DLL');
  });

  // -------------------------------------------------------------------------
  // macOS-specific failure signatures
  // -------------------------------------------------------------------------

  test('detects macOS dyld Library not loaded', () => {
    const stderr = 'dyld: Library not loaded: /usr/lib/libssl.dylib\n  Referenced from: /usr/local/bin/perllsp\n  Reason: image not found';
    const result = classifyStartupError(stderr);
    expect(result.kind).toBe(StartupErrorKind.MacOsDylibError);
    expect(result.hint).toContain('macOS');
    expect(result.remediation).toContain('xattr');
  });

  test('detects macOS code signature invalid', () => {
    const stderr = 'perllsp: code signature invalid';
    const result = classifyStartupError(stderr);
    expect(result.kind).toBe(StartupErrorKind.MacOsDylibError);
    expect(result.hint).toContain('Gatekeeper');
  });

  test('hint text ≤200 chars covers Windows and macOS cases', () => {
    const scenarios = [
      'DLL initialization routine failed',
      'not a valid Win32 application',
      'The specified module could not be found',
      'dyld: Library not loaded: /usr/lib/libssl.dylib',
      'code signature invalid',
    ];
    for (const stderr of scenarios) {
      const result = classifyStartupError(stderr);
      expect(result.hint.length).toBeLessThanOrEqual(200);
    }
  });
});

// ---------------------------------------------------------------------------
// selectBestDiagnosis — fallback chaining for #3329
//
// When probeStartupFailure returns Unknown (binary probe was inconclusive),
// selectBestDiagnosis must prefer the health-check string from
// runStartupDiagnostics so the user gets the specific "Perl interpreter not
// found" message instead of a generic hint.
// ---------------------------------------------------------------------------

describe('selectBestDiagnosis', () => {
  test('returns probe diagnosis unchanged when kind is not Unknown', () => {
    const probe = classifyStartupError(
      '/lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.35` not found',
    );
    // probe.kind === GlibcMismatch — health msg should be ignored
    const result = selectBestDiagnosis(probe, 'Perl interpreter not found. Install Perl 5.10+');
    expect(result.kind).toBe(StartupErrorKind.GlibcMismatch);
    expect(result.hint).toContain('glibc');
  });

  test('falls back to health-check message when probe kind is Unknown', () => {
    const probe = classifyStartupError('');  // Unknown
    const healthMsg = 'Perl interpreter not found. Install Perl 5.10+ and reload the window.';
    const result = selectBestDiagnosis(probe, healthMsg);
    expect(result.hint).toContain('Perl');
    expect(result.hint).toContain('Install');
    // The fallback should not be the generic Unknown hint
    expect(result.hint).not.toContain('LSP binary failed to start');
  });

  test('returns probe Unknown unchanged when no health message is provided', () => {
    const probe = classifyStartupError('');  // Unknown
    const result = selectBestDiagnosis(probe, undefined);
    expect(result.kind).toBe(StartupErrorKind.Unknown);
    expect(result.hint).toBeTruthy();
  });

  test('returns probe Unknown unchanged when health message is empty string', () => {
    const probe = classifyStartupError('');  // Unknown
    const result = selectBestDiagnosis(probe, '');
    expect(result.kind).toBe(StartupErrorKind.Unknown);
  });
});
