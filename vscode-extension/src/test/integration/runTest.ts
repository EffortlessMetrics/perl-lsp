import { execSync } from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { downloadAndUnzipVSCode, runTests } from '@vscode/test-electron';
import { resolveVSCodeTestVersion } from '../vscodeHostVersion';
import { runWithoutForcedWorkspaceTrust } from '../runVsCodeTests';
import { workspaceSmokeLaunchArgs, workspaceSmokeTrustMode } from '../workspaceSmokeOptions';

function toolchainNpmVersion(): string {
  const npmUserAgent = process.env.npm_config_user_agent ?? '';
  const configuredVersion = /(?:^|\s)npm\/([^\s]+)/.exec(npmUserAgent)?.[1];
  if (configuredVersion) {
    return configuredVersion;
  }
  return execSync('npm --version', { encoding: 'utf8', windowsHide: true }).trim();
}

function getGrepArg(args: string[]): string | undefined {
  const grepIndex = args.findIndex((arg) => arg === '--grep' || arg === '-g');
  if (grepIndex >= 0) {
    const parts: string[] = [];
    for (const arg of args.slice(grepIndex + 1)) {
      if (arg.startsWith('-')) {
        break;
      }
      parts.push(arg);
    }
    return parts.length > 0 ? parts.join(' ') : undefined;
  }

  const grepPrefix = '--grep=';
  const prefixed = args.find((arg) => arg.startsWith(grepPrefix));
  return prefixed ? prefixed.slice(grepPrefix.length) : undefined;
}

async function main(): Promise<void> {
  const extensionDevelopmentPath = path.resolve(__dirname, '../../..');
  const repoRoot = path.resolve(extensionDevelopmentPath, '..');
  const extensionTestsPath = path.resolve(__dirname, './suite');
  const vscodeVersion = resolveVSCodeTestVersion(process.env.PERL_LSP_VSCODE_VERSION);
  const toolchainNodeVersion = process.version;
  const toolchainNpmVersionValue = toolchainNpmVersion();
  const configuredWorkspace = process.env.PERL_LSP_SMOKE_WORKSPACE?.trim();
  const workspaceTrustMode = workspaceSmokeTrustMode();
  const generatedWorkspacePath = configuredWorkspace
    ? undefined
    : fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-vscode-smoke-workspace-'));
  const workspacePath = configuredWorkspace
    ? path.resolve(configuredWorkspace)
    : generatedWorkspacePath!;
  if (!fs.existsSync(workspacePath)) {
    throw new Error(`Configured smoke workspace does not exist: ${workspacePath}`);
  }
  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-vscode-smoke-user-'));
  const extensionsDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-vscode-smoke-extensions-'));
  const grep = getGrepArg(process.argv.slice(2));
  const receiptsRoot =
    process.env.PERL_LSP_SMOKE_RECEIPTS_DIR ||
    path.join(repoRoot, 'target', 'receipts', 'vscode-smoke');
  fs.mkdirSync(receiptsRoot, { recursive: true });

  try {
    if (!configuredWorkspace) {
      fs.writeFileSync(
        path.join(workspacePath, 'smoke.pl'),
        'use strict;\nuse warnings;\nprint "ok\\n";\n',
      );
    }

    const configuredServerPath = process.env.PERL_LSP_FIRST_HOUR_SERVER_PATH;
    if (configuredServerPath) {
      const settingsDir = path.join(userDataDir, 'User');
      fs.mkdirSync(settingsDir, { recursive: true });
      const settings: Record<string, unknown> = {
        'perl-lsp.autoDownload': false,
        'perl-lsp.perlcritic.enabled': false,
        'perl-lsp.serverPath': path.resolve(configuredServerPath),
      };
      if (workspaceTrustMode === 'untrusted') {
        settings['security.workspace.trust.enabled'] = true;
        settings['security.workspace.trust.startupPrompt'] = 'never';
      }
      fs.writeFileSync(path.join(settingsDir, 'settings.json'), JSON.stringify(settings, null, 2));
    }

    const testOptions = {
      version: vscodeVersion,
      extensionDevelopmentPath,
      extensionTestsPath,
      extensionTestsEnv: {
        ...process.env,
        PERL_LSP_EXTENSION_TEST_SKIP_STARTUP:
          process.env.PERL_LSP_EXTENSION_TEST_SKIP_STARTUP ?? '1',
        PERL_LSP_SMOKE_RECEIPTS_DIR: receiptsRoot,
        PERL_LSP_SMOKE_SOURCE_LABEL: process.env.PERL_LSP_SMOKE_SOURCE_LABEL || 'integration',
        PERL_LSP_TOOLCHAIN_NODE_VERSION: toolchainNodeVersion,
        PERL_LSP_TOOLCHAIN_NPM_VERSION: toolchainNpmVersionValue,
        PERL_LSP_VSCODE_VERSION: vscodeVersion,
        VSCODE_TEST_GREP: grep ?? '',
      },
      launchArgs: [
        ...workspaceSmokeLaunchArgs(workspacePath),
        `--user-data-dir=${userDataDir}`,
        `--extensions-dir=${extensionsDir}`,
      ],
    };
    if (workspaceTrustMode === 'untrusted') {
      const vscodeExecutablePath = await downloadAndUnzipVSCode({ version: vscodeVersion });
      await runWithoutForcedWorkspaceTrust({
        vscodeExecutablePath,
        extensionDevelopmentPath,
        extensionTestsPath,
        extensionTestsEnv: testOptions.extensionTestsEnv,
        launchArgs: testOptions.launchArgs,
      });
    } else {
      await runTests(testOptions);
    }
  } finally {
    for (const directory of [generatedWorkspacePath, userDataDir, extensionsDir]) {
      if (!directory) {
        continue;
      }
      try {
        fs.rmSync(directory, { recursive: true, force: true });
      } catch (error: unknown) {
        const message = error instanceof Error ? error.message : String(error);
        process.stderr.write(`[integration-smoke] cleanup failed for ${directory}: ${message}\n`);
      }
    }
  }
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`${message}\n`);
  process.exit(1);
});
