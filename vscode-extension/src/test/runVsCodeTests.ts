import { spawn } from 'child_process';

export interface UntrustedHostTestOptions {
  vscodeExecutablePath: string;
  extensionDevelopmentPath: string;
  extensionTestsPath: string;
  extensionTestsEnv: NodeJS.ProcessEnv;
  launchArgs: string[];
}

/**
 * Runs the extension host without the test-electron package's forced
 * `--disable-workspace-trust` argument. This is intentionally test-only: the
 * normal runner remains authoritative for every other smoke mode.
 */
export function runWithoutForcedWorkspaceTrust(options: UntrustedHostTestOptions): Promise<void> {
  const args = [
    ...options.launchArgs,
    '--no-sandbox',
    '--disable-gpu-sandbox',
    '--disable-updates',
    '--skip-welcome',
    '--skip-release-notes',
    `--extensionTestsPath=${options.extensionTestsPath}`,
    `--extensionDevelopmentPath=${options.extensionDevelopmentPath}`,
  ];
  return new Promise((resolve, reject) => {
    const child = spawn(options.vscodeExecutablePath, args, {
      env: { ...process.env, ...options.extensionTestsEnv },
      stdio: 'inherit',
      windowsHide: true,
    });
    child.on('error', reject);
    child.on('exit', (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`VS Code extension host exited with ${code ?? signal ?? 'unknown'}`));
    });
  });
}
