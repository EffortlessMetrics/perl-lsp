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
 *
 * Also covers the diagnosis cache introduced in #4193:
 *   - serverNotRunningMessage() returns the cached diagnosis hint when set
 *   - serverNotRunningMessage() returns the generic fallback when no cache
 *   - Cache can be cleared (simulates successful server restart)
 */

jest.mock('vscode-languageclient/node', () => ({
  LanguageClient: class {},
  Trace: { Off: 'off', Messages: 'messages', Verbose: 'verbose' },
  TransportKind: { stdio: 0 },
}));

import {
  classifyStartupError,
  formatStartupFailureDialog,
  StartupErrorKind,
  selectBestDiagnosis,
} from '../startupDiagnosis';
import {
  serverNotRunningMessage,
  _setLastStartupDiagnosisForTest,
} from '../extension';

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

// ---------------------------------------------------------------------------
// formatStartupFailureDialog — exact dialog text surfaced on startup failure
// ---------------------------------------------------------------------------

describe('formatStartupFailureDialog', () => {
  test('surfaces onboarding guidance verbatim when probe is Unknown and health message exists', () => {
    const probe = classifyStartupError('');
    const healthMsg =
      'Perl interpreter not found. Install Perl 5.10+ and reload the window. ' +
      'Alternatively, set the `perl-lsp.perl.path` setting to an existing Perl executable.';

    expect(formatStartupFailureDialog(probe, healthMsg)).toBe(healthMsg);
  });

  test('keeps the generic startup wrapper for classified probe failures', () => {
    const probe = classifyStartupError(
      '/lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.35` not found',
    );

    const message = formatStartupFailureDialog(probe, 'Perl interpreter not found...');

    expect(message).toContain('Perl Language Server failed to start.');
    expect(message).toContain('glibc');
    expect(message).not.toContain('Perl interpreter not found...');
  });
});

// ---------------------------------------------------------------------------
// serverNotRunningMessage — diagnosis cache integration (#4193)
//
// When a StartupErrorDiagnosis is cached via lastStartupDiagnosis,
// serverNotRunningMessage() must format and return it so users see the
// specific root cause (e.g. "glibc mismatch") rather than a generic hint.
// When no diagnosis is cached, the generic fallback must be returned.
// ---------------------------------------------------------------------------

describe('serverNotRunningMessage diagnosis cache (#4193)', () => {
  afterEach(() => {
    // Reset cache after each test to prevent state leak
    _setLastStartupDiagnosisForTest(undefined);
  });

  test('returns generic fallback when no diagnosis is cached', () => {
    _setLastStartupDiagnosisForTest(undefined);
    const msg = serverNotRunningMessage();
    expect(msg).toContain('Perl Language Server is not running');
    expect(msg).toContain('Health Check');
  });

  test('returns formatted diagnosis when glibc mismatch is cached', () => {
    const diagnosis = classifyStartupError(
      "error while loading shared libraries: libc.so.6: version `GLIBC_2.32' not found",
    );
    _setLastStartupDiagnosisForTest(diagnosis);

    const msg = serverNotRunningMessage();
    expect(msg).toContain('glibc');
    expect(msg).toContain('cargo install');
    // Must not show generic fallback when diagnosis is present
    expect(msg).not.toBe('Perl Language Server is not running. Run the Health Check (Command Palette: "Perl: Run Health Check") to diagnose the issue.');
  });

  test('returns formatted diagnosis when permission denied is cached', () => {
    const diagnosis = classifyStartupError('Permission denied');
    _setLastStartupDiagnosisForTest(diagnosis);

    const msg = serverNotRunningMessage();
    expect(msg).toContain('permission');
    expect(msg).toContain('chmod');
  });

  test('returns generic fallback after cache is cleared (simulates successful restart)', () => {
    // Set a diagnosis, then clear it (as initializeLanguageClient does on success)
    _setLastStartupDiagnosisForTest(classifyStartupError('Permission denied'));
    _setLastStartupDiagnosisForTest(undefined);

    const msg = serverNotRunningMessage();
    expect(msg).toContain('Perl Language Server is not running');
  });

  test('mid-session crash diagnosis is surfaced via serverNotRunningMessage', () => {
    // Simulate what bindClientState sets when server stops unexpectedly
    _setLastStartupDiagnosisForTest({
      kind: StartupErrorKind.Unknown,
      hint: 'The Perl Language Server stopped unexpectedly. Check the Output panel for details.',
      remediation: 'Try restarting the server (Command Palette: "Perl: Restart Server") or run the Health Check.',
    });

    const msg = serverNotRunningMessage();
    expect(msg).toContain('stopped unexpectedly');
    expect(msg).toContain('Restart Server');
  });
});
