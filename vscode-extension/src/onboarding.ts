/**
 * OnboardingManager — first-run setup experience and environment health check.
 *
 * Responsibilities:
 * - Detect first run via `context.globalState` key `perl-lsp.welcomed`.
 * - Run a multi-step health check: Perl version, perltidy, perlcritic, LSP binary.
 * - Expose individual check methods so tests can exercise them in isolation.
 */

import * as fs from 'fs';
import { execFile } from 'child_process';
import * as vscode from 'vscode';

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/** Severity level for a single health-check item. */
export enum HealthCheckStatus {
  Ok = 'ok',
  Warning = 'warning',
  Error = 'error',
}

/** Result of one health-check step. */
export interface HealthCheckResult {
  /** Short display name shown in the notification/output. */
  label: string;
  /** True when the check passed without errors or warnings. */
  ok: boolean;
  /** Severity: Ok / Warning / Error. */
  status: HealthCheckStatus;
  /** Human-readable detail (version string, error message, etc.). */
  detail: string;
}

// ---------------------------------------------------------------------------
// Startup failure classifier
// ---------------------------------------------------------------------------

/**
 * User-facing message surfaced when Perl interpreter is not found.
 * Shown instead of the generic "Restart the server" message so the user
 * immediately knows the root cause and what to do.
 */
export const PERL_MISSING_MESSAGE =
  'Perl interpreter not found. ' +
  'Install Perl 5.10+ (e.g. https://www.perl.org/get.html or Strawberry Perl for Windows) ' +
  'and reload the window. ' +
  'Alternatively, set the `perl-lsp.perl.path` setting to an existing Perl executable.';

/**
 * Given the results of a health check, return a specific user-facing error
 * string that names the root cause of an LSP startup failure.
 *
 * Priority: Perl missing > binary missing > unknown crash.
 */
export function classifyStartupFailure(results: HealthCheckResult[]): string {
  const perlResult = results.find(r => r.label === 'Perl interpreter');
  const binaryResult = results.find(r => r.label === 'LSP binary');

  // Perl not found — highest priority, most actionable for the user.
  if (!perlResult || (perlResult.ok === false && perlResult.status === HealthCheckStatus.Error)) {
    return PERL_MISSING_MESSAGE;
  }

  // LSP binary not found — Perl is present but the server binary is missing.
  if (binaryResult && binaryResult.ok === false && binaryResult.status === HealthCheckStatus.Error) {
    const detail = binaryResult.detail.trimEnd();
    const detailWithPeriod = detail.endsWith('.') ? detail : `${detail}.`;
    return (
      'Perl Language Server binary (perllsp) not found. ' +
      detailWithPeriod +
      ' Check the Output panel for download details or reinstall the extension.'
    );
  }

  // All checks passed — unknown crash (e.g. version mismatch, system ABI issue).
  return (
    'Perl Language Server failed to start. ' +
    'Check the Output panel for details. ' +
    'You can also try reinstalling the extension or running the Health Check from the command palette.'
  );
}

// ---------------------------------------------------------------------------
// Internal exec helper type
// ---------------------------------------------------------------------------

type ExecCheckFn = (
  cmd: string,
  args: string[],
) => Promise<{ stdout: string; stderr: string }>;

interface ExecInvocation {
  command: string;
  args: string[];
}

// ---------------------------------------------------------------------------
// OnboardingManager
// ---------------------------------------------------------------------------

export class OnboardingManager {
  private readonly context: vscode.ExtensionContext;
  private readonly outputChannel: vscode.OutputChannel;

  /**
   * Replaceable exec function.  In production this wraps `child_process.execFile`;
   * in tests it is replaced with a jest mock via `mgr._execCheck = jest.fn(...)`.
   */
  _execCheck: ExecCheckFn;

  constructor(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel,
  ) {
    this.context = context;
    this.outputChannel = outputChannel;
    this._execCheck = defaultExecCheck;
  }

  // ---------------------------------------------------------------------------
  // First-run state
  // ---------------------------------------------------------------------------

  /**
   * Returns `true` on the very first activation (welcome flag not yet stored).
   */
  shouldShowWelcome(): boolean {
    return !this.context.globalState.get<boolean>('perl-lsp.welcomed', false);
  }

  /**
   * Persist the welcomed flag so the welcome view only appears once per install.
   */
  async markWelcomed(): Promise<void> {
    await this.context.globalState.update('perl-lsp.welcomed', true);
  }

  // ---------------------------------------------------------------------------
  // Individual checks
  // ---------------------------------------------------------------------------

  /** Check whether `perl` is on PATH and return its version. */
  async checkPerlInstalled(): Promise<HealthCheckResult> {
    const label = 'Perl interpreter';
    try {
      const { stdout } = await this._execCheck('perl', [
        '-e',
        'print $]',
      ]);
      const version = stdout.trim() || '(unknown)';
      this.outputChannel.appendLine(`[onboarding] Perl version: ${version}`);
      return {
        label,
        ok: true,
        status: HealthCheckStatus.Ok,
        detail: `Perl ${version} found`,
      };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      this.outputChannel.appendLine(`[onboarding] Perl not found: ${msg}`);
      return {
        label,
        ok: false,
        status: HealthCheckStatus.Error,
        detail: `Perl not found on PATH. Install Perl and reload. (${msg})`,
      };
    }
  }

  /**
   * Check whether `perltidy` is on PATH.
   *
   * Perltidy absence is a *warning*, not an error — the LSP works without it
   * but formatting will be unavailable.
   */
  async checkPerltidyInstalled(): Promise<HealthCheckResult> {
    const label = 'perltidy';
    try {
      const { stdout } = await this._execCheck('perltidy', ['--version']);
      const version = stdout.trim() || '(unknown)';
      this.outputChannel.appendLine(`[onboarding] perltidy: ${version}`);
      return {
        label,
        ok: true,
        status: HealthCheckStatus.Ok,
        detail: `perltidy found (${version})`,
      };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      this.outputChannel.appendLine(
        `[onboarding] perltidy not found: ${msg}`,
      );
      return {
        label,
        ok: false,
        status: HealthCheckStatus.Warning,
        detail:
          'perltidy not found — document formatting will be unavailable. ' +
          'Install via: cpanm Perl::Tidy',
      };
    }
  }


  /**
   * Check whether `perlcritic` is on PATH.
   *
   * Perl::Critic absence is a warning — core LSP features still work.
   */
  async checkPerlcriticInstalled(): Promise<HealthCheckResult> {
    const label = 'perlcritic';
    try {
      const { stdout } = await this._execCheck('perlcritic', ['--version']);
      const version = stdout.trim() || '(unknown)';
      this.outputChannel.appendLine(`[onboarding] perlcritic: ${version}`);
      return {
        label,
        ok: true,
        status: HealthCheckStatus.Ok,
        detail: `perlcritic found (${version})`,
      };
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      this.outputChannel.appendLine(`[onboarding] perlcritic not found: ${msg}`);
      return {
        label,
        ok: false,
        status: HealthCheckStatus.Warning,
        detail:
          'perlcritic not found — Perl::Critic diagnostics will be unavailable. Install via: cpanm Perl::Critic',
      };
    }
  }

  /**
   * Check whether the LSP binary has been downloaded / located.
   *
   * @param serverPath  Path returned by `getServerPath`, or `null` if not found.
   */
  checkBinaryDownloaded(serverPath: string | null): HealthCheckResult {
    const label = 'LSP binary';
    if (!serverPath) {
      return {
        label,
        ok: false,
        status: HealthCheckStatus.Error,
        detail: 'perl-lsp binary not found. Check Output for download details.',
      };
    }
    if (!fs.existsSync(serverPath)) {
      return {
        label,
        ok: false,
        status: HealthCheckStatus.Error,
        detail: `perl-lsp binary path does not exist on disk: ${serverPath}`,
      };
    }
    return {
      label,
      ok: true,
      status: HealthCheckStatus.Ok,
      detail: `Binary found: ${serverPath}`,
    };
  }

  // ---------------------------------------------------------------------------
  // Composite health check
  // ---------------------------------------------------------------------------

  /**
   * Run all setup health checks and return an ordered list of results.
   *
   * @param serverPath  Path to the LSP binary, or `null` if unavailable.
   */
  async runSetupHealthCheck(
    serverPath: string | null,
  ): Promise<HealthCheckResult[]> {
    this.outputChannel.appendLine('[onboarding] Running setup health check...');

    const [perlResult, perltidyResult, perlcriticResult] = await Promise.all([
      this.checkPerlInstalled(),
      this.checkPerltidyInstalled(),
      this.checkPerlcriticInstalled(),
    ]);

    const binaryResult = this.checkBinaryDownloaded(serverPath);

    const results: HealthCheckResult[] = [
      perlResult,
      perltidyResult,
      perlcriticResult,
      binaryResult,
    ];

    for (const r of results) {
      const icon =
        r.status === HealthCheckStatus.Ok
          ? '[OK]'
          : r.status === HealthCheckStatus.Warning
            ? '[WARN]'
            : '[ERROR]';
      this.outputChannel.appendLine(
        `[onboarding] ${icon} ${r.label}: ${r.detail}`,
      );
    }

    return results;
  }

  // ---------------------------------------------------------------------------
  // Startup diagnostics
  // ---------------------------------------------------------------------------

  /**
   * Run a targeted health check after an LSP startup failure and return a
   * user-facing error string that names the specific root cause.
   *
   * Call this instead of surfacing the generic "restart server" message so
   * users immediately know whether the problem is a missing Perl interpreter,
   * a missing LSP binary, or an unknown crash.
   *
   * @param serverPath  Path to the LSP binary, or `null` if unavailable.
   */
  async runStartupDiagnostics(serverPath: string | null): Promise<string> {
    try {
      const results = await this.runSetupHealthCheck(serverPath);
      return classifyStartupFailure(results);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      this.outputChannel.appendLine(`[onboarding] Startup diagnostics failed: ${msg}`);
      return PERL_MISSING_MESSAGE;
    }
  }

  // ---------------------------------------------------------------------------
  // Welcome notification
  // ---------------------------------------------------------------------------

  /**
   * Show a first-run welcome notification and offer to run the health check.
   *
   * @param serverPath  Path to the LSP binary for the health check.
   */
  async showWelcomeNotification(serverPath: string | null): Promise<void> {
    await this.markWelcomed();

    const selection = await vscode.window.showInformationMessage(
      'Welcome to Perl Language Server! Run a setup health check to verify your environment.',
      'Run Health Check',
      'Show Output',
      'Dismiss',
    );

    if (selection === 'Run Health Check') {
      await vscode.commands.executeCommand(
        'perl-lsp.runHealthCheck',
        serverPath,
      );
    } else if (selection === 'Show Output') {
      this.outputChannel.show();
    }
  }
}

// ---------------------------------------------------------------------------
// Default exec implementation (production)
// ---------------------------------------------------------------------------

function defaultExecCheck(
  cmd: string,
  args: string[],
): Promise<{ stdout: string; stderr: string }> {
  const initialInvocation = { command: cmd, args };
  return runExecInvocation(initialInvocation).catch(async (err: unknown) => {
    const fallbackInvocation = await resolveWindowsInvocationFallback(
      initialInvocation,
      err,
    );
    if (!fallbackInvocation) {
      throw err;
    }
    return runExecInvocation(fallbackInvocation);
  });
}

function runExecInvocation(
  invocation: ExecInvocation,
): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    execFile(
      invocation.command,
      invocation.args,
      { timeout: 5000 },
      (err, stdout, stderr) => {
        if (err) {
          reject(err);
        } else {
          resolve({ stdout, stderr });
        }
      },
    );
  });
}

async function resolveWindowsInvocationFallback(
  invocation: ExecInvocation,
  err: unknown,
): Promise<ExecInvocation | null> {
  if (
    process.platform !== 'win32' ||
    !isSpawnNotFound(err) ||
    invocation.command.includes('\\') ||
    invocation.command.includes('/')
  ) {
    return null;
  }

  const candidate = await resolveWindowsCommandCandidate(invocation.command);
  if (!candidate) {
    return null;
  }

  return buildWindowsExecInvocation(candidate, invocation.args);
}

function isSpawnNotFound(err: unknown): boolean {
  return Boolean(
    err &&
      typeof err === 'object' &&
      'code' in err &&
      (err as { code?: unknown }).code === 'ENOENT',
  );
}

function resolveWindowsCommandCandidate(command: string): Promise<string | null> {
  return new Promise(resolve => {
    execFile('where.exe', [command], { timeout: 5000 }, (err, stdout) => {
      if (err) {
        resolve(null);
        return;
      }
      resolve(selectWindowsCommandCandidate(stdout));
    });
  });
}

export function selectWindowsCommandCandidate(stdout: string): string | null {
  const candidates = stdout
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter(line => line.length > 0);

  if (candidates.length === 0) {
    return null;
  }

  return candidates.reduce((best, candidate) => {
    return windowsCommandPriority(candidate) > windowsCommandPriority(best)
      ? candidate
      : best;
  });
}

function buildWindowsExecInvocation(command: string, args: string[]): ExecInvocation {
  if (isWindowsBatchWrapper(command)) {
    return {
      command: 'cmd.exe',
      args: ['/d', '/s', '/c', command, ...args],
    };
  }

  return { command, args };
}

function isWindowsBatchWrapper(command: string): boolean {
  const lower = command.toLowerCase();
  return lower.endsWith('.bat') || lower.endsWith('.cmd');
}

function windowsCommandPriority(command: string): number {
  const lower = command.toLowerCase();
  if (lower.endsWith('.exe')) {
    return 5;
  }
  if (lower.endsWith('.com')) {
    return 4;
  }
  if (lower.endsWith('.cmd')) {
    return 3;
  }
  if (lower.endsWith('.bat')) {
    return 2;
  }
  if (/\.[^\\/]+$/.test(lower)) {
    return 1;
  }
  return 0;
}
