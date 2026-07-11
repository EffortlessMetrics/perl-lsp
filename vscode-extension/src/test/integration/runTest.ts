import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { runTests } from '@vscode/test-electron';

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
  const configuredWorkspace = process.env.PERL_LSP_SMOKE_WORKSPACE;
  const workspacePath = configuredWorkspace
    ? path.resolve(configuredWorkspace)
    : fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-vscode-smoke-workspace-'));
  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-vscode-smoke-user-'));
  const extensionsDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-vscode-smoke-extensions-'));
  const grep = getGrepArg(process.argv.slice(2));
  const receiptsRoot =
    process.env.PERL_LSP_SMOKE_RECEIPTS_DIR ||
    path.join(repoRoot, 'target', 'receipts', 'vscode-smoke');
  fs.mkdirSync(receiptsRoot, { recursive: true });

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
    fs.writeFileSync(
      path.join(settingsDir, 'settings.json'),
      JSON.stringify(
        {
          'perl-lsp.autoDownload': false,
          'perl-lsp.perlcritic.enabled': false,
          'perl-lsp.serverPath': path.resolve(configuredServerPath),
        },
        null,
        2,
      ),
    );
  }

  await runTests({
    extensionDevelopmentPath,
    extensionTestsPath,
    extensionTestsEnv: {
      ...process.env,
      PERL_LSP_EXTENSION_TEST_SKIP_STARTUP: process.env.PERL_LSP_EXTENSION_TEST_SKIP_STARTUP ?? '1',
      PERL_LSP_SMOKE_RECEIPTS_DIR: receiptsRoot,
      PERL_LSP_SMOKE_SOURCE_LABEL: process.env.PERL_LSP_SMOKE_SOURCE_LABEL || 'integration',
      VSCODE_TEST_GREP: grep ?? '',
    },
    launchArgs: [
      workspacePath,
      '--disable-workspace-trust',
      `--user-data-dir=${userDataDir}`,
      `--extensions-dir=${extensionsDir}`,
    ],
  });
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`${message}\n`);
  process.exit(1);
});
