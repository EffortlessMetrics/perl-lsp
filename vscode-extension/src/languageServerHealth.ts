import { execFile } from 'child_process';
import { stripVTControlCharacters } from 'util';

export const MANAGED_BINARY_HEALTH_TIMEOUT_MS = 30_000;

export interface HealthCheckLog {
  appendLine(message: string): void;
}

export interface HealthCheckProcessOptions {
  execFile?: typeof execFile;
  timeoutMs?: number;
}

type HealthCheckChild = { kill?: () => boolean } | undefined;

function safeAppendLine(log: HealthCheckLog, message: string): void {
  try {
    log.appendLine(message);
  } catch {
    // Logging is diagnostic only; it must not prevent health-check settlement.
  }
}

function normalizeHealthOutput(output: string): string {
  return stripVTControlCharacters(output).trim();
}

function isHealthyOutput(stdout: string): boolean {
  // The server emits `ok <version>`.
  return /^ok(?:\s|$)/.test(stdout);
}

/**
 * Run `perllsp --health` without coupling binary probing to activation state.
 * The returned promise settles once, even if a process callback races the
 * module-owned timeout or a diagnostic logger throws.
 */
export async function runLanguageServerHealthCheck(
  serverPath: string,
  log: HealthCheckLog,
  options: HealthCheckProcessOptions = {},
): Promise<boolean> {
  const runProcess = options.execFile ?? execFile;
  const timeoutMs = options.timeoutMs ?? MANAGED_BINARY_HEALTH_TIMEOUT_MS;

  return new Promise((resolve) => {
    let settled = false;
    let child: HealthCheckChild;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const finish = (healthy: boolean): void => {
      if (settled) {
        return;
      }
      settled = true;
      if (timer !== undefined) {
        clearTimeout(timer);
        timer = undefined;
      }
      resolve(healthy);
    };

    const onTimeout = (): void => {
      if (settled) {
        return;
      }
      try {
        child?.kill?.();
      } catch {
        // The callback still owns settlement if a child exits during timeout.
      }
      safeAppendLine(log, `[health-check] Timed out after ${timeoutMs}ms`);
      finish(false);
    };

    if (timeoutMs > 0) {
      timer = setTimeout(onTimeout, timeoutMs);
    }

    try {
      child = runProcess(
        serverPath,
        ['--health'],
        { timeout: timeoutMs },
        (error: Error | null, stdout: string, stderr: string) => {
          if (settled) {
            return;
          }

          if (error) {
            safeAppendLine(log, `[health-check] Failed: ${error.message}`);
            const stderrText = normalizeHealthOutput(stderr);
            if (stderrText) {
              safeAppendLine(log, `[health-check] stderr: ${stderrText}`);
            }
            const stdoutText = normalizeHealthOutput(stdout);
            if (stdoutText) {
              safeAppendLine(log, `[health-check] stdout: ${stdoutText}`);
            }
            finish(false);
            return;
          }

          const stdoutText = normalizeHealthOutput(stdout);
          const stderrText = normalizeHealthOutput(stderr);
          const healthy = isHealthyOutput(stdoutText);
          if (!healthy) {
            safeAppendLine(log, `[health-check] Unexpected output: ${stdoutText}`);
            if (stderrText) {
              safeAppendLine(log, `[health-check] stderr: ${stderrText}`);
            }
          }
          finish(healthy);
        },
      );
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      safeAppendLine(log, `[health-check] Failed: ${message}`);
      finish(false);
    }
  });
}
