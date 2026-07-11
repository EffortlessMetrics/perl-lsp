import * as fs from 'fs';
import * as https from 'https';
import * as os from 'os';
import * as path from 'path';
import { spawnSync } from 'child_process';
import {
  downloadAndUnzipVSCode,
  resolveCliArgsFromVSCodeExecutablePath,
  runTests,
} from '@vscode/test-electron';

const EXTENSION_ID = 'EffortlessMetrics.perl-lsp-rs';

type ExtensionSource = 'marketplace' | 'open-vsx' | 'vsix';

function envValue(name: string): string {
  return process.env[name]?.trim() ?? '';
}

function publishedSource(): ExtensionSource {
  const source = envValue('PERL_LSP_PUBLISHED_EXTENSION_SOURCE') || 'marketplace';
  if (source === 'marketplace' || source === 'open-vsx' || source === 'vsix') {
    return source;
  }

  throw new Error(`Unsupported PERL_LSP_PUBLISHED_EXTENSION_SOURCE=${source}`);
}

function validateExtensionId(extensionId: string): void {
  if (!/^[A-Za-z0-9][A-Za-z0-9-]*\.[A-Za-z0-9][A-Za-z0-9-]*$/.test(extensionId)) {
    throw new Error(`Extension id must be publisher.name, got ${extensionId}`);
  }
}

function validatePublishedVersion(version: string): void {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(`Published extension version must be a SemVer package version, got ${version}`);
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function downloadFile(url: string, destination: string, redirects = 0): Promise<void> {
  if (redirects > 5) {
    return Promise.reject(new Error(`Too many redirects while downloading ${url}`));
  }

  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      const statusCode = response.statusCode ?? 0;
      const location = response.headers.location;
      if (statusCode >= 300 && statusCode < 400 && location) {
        response.resume();
        const redirectedUrl = new URL(location, url).toString();
        downloadFile(redirectedUrl, destination, redirects + 1).then(resolve, reject);
        return;
      }

      if (statusCode !== 200) {
        response.resume();
        reject(new Error(`GET ${url} failed with HTTP ${statusCode}`));
        return;
      }

      const output = fs.createWriteStream(destination);
      response.pipe(output);
      output.on('finish', () => {
        output.close();
        resolve();
      });
      output.on('error', reject);
    });

    request.on('error', reject);
    request.setTimeout(60_000, () => {
      request.destroy(new Error(`Timed out downloading ${url}`));
    });
  });
}

async function downloadFileWithRetry(url: string, destination: string): Promise<void> {
  let lastFailure = '';

  for (let attempt = 1; attempt <= 12; attempt += 1) {
    try {
      await downloadFile(url, destination);
      return;
    } catch (error: unknown) {
      lastFailure = error instanceof Error ? error.message : String(error);
      if (fs.existsSync(destination)) {
        fs.rmSync(destination, { force: true });
      }
      if (attempt < 12) {
        await sleep(20_000);
      }
    }
  }

  throw new Error(`Failed to download published extension from ${url}\n${lastFailure}`);
}

async function resolveInstallTarget(source: ExtensionSource, tempDir: string): Promise<string> {
  const version = envValue('PERL_LSP_PUBLISHED_EXTENSION_VERSION');
  const extensionId = envValue('PERL_LSP_PUBLISHED_EXTENSION_ID') || EXTENSION_ID;
  validateExtensionId(extensionId);

  if (version) {
    validatePublishedVersion(version);
  }

  if (source === 'marketplace') {
    return version ? `${extensionId}@${version}` : extensionId;
  }

  if (source === 'vsix') {
    const vsixPath = envValue('PERL_LSP_PUBLISHED_VSIX_PATH');
    if (!vsixPath) {
      throw new Error('PERL_LSP_PUBLISHED_VSIX_PATH is required when source is vsix');
    }
    return path.resolve(vsixPath);
  }

  if (!version) {
    throw new Error('PERL_LSP_PUBLISHED_EXTENSION_VERSION is required when source is open-vsx');
  }

  const [namespace, extensionName] = extensionId.split('.');

  const fileName = `${namespace}.${extensionName}-${version}.vsix`;
  const url = `https://open-vsx.org/api/${namespace}/${extensionName}/${version}/file/${fileName}`;
  const destination = path.join(tempDir, fileName);
  await downloadFileWithRetry(url, destination);
  return destination;
}

async function installExtension(
  vscodeExecutablePath: string,
  installTarget: string,
  userDataDir: string,
  extensionsDir: string,
): Promise<void> {
  const [cliPath, ...cliArgs] = resolveCliArgsFromVSCodeExecutablePath(vscodeExecutablePath);
  const args = [
    ...cliArgs,
    `--user-data-dir=${userDataDir}`,
    `--extensions-dir=${extensionsDir}`,
    '--install-extension',
    installTarget,
    '--force',
  ];
  const spawnTarget =
    process.platform === 'win32'
      ? {
          command: process.env.ComSpec || 'cmd.exe',
          args: ['/d', '/s', '/c', cliPath, ...args],
        }
      : {
          command: cliPath,
          args,
        };
  let lastFailure = '';

  for (let attempt = 1; attempt <= 12; attempt += 1) {
    const result = spawnSync(spawnTarget.command, spawnTarget.args, {
      encoding: 'utf8',
      windowsHide: true,
    });

    if (result.status === 0) {
      return;
    }

    lastFailure = [
      `attempt ${attempt}`,
      `exit ${result.status ?? 'unknown'}`,
      result.error instanceof Error ? result.error.message : '',
      result.stdout,
      result.stderr,
    ]
      .filter(Boolean)
      .join('\n');

    if (attempt < 12) {
      await sleep(20_000);
    }
  }

  throw new Error(`Failed to install published extension ${installTarget}\n${lastFailure}`);
}

async function main(): Promise<void> {
  const source = publishedSource();
  const workspacePath = fs.mkdtempSync(
    path.join(os.tmpdir(), 'perl-lsp-published-smoke-workspace-'),
  );
  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-published-smoke-user-'));
  const extensionsDir = fs.mkdtempSync(
    path.join(os.tmpdir(), 'perl-lsp-published-smoke-extensions-'),
  );
  const downloadDir = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-published-smoke-download-'));
  const harnessExtensionPath = path.resolve(process.cwd(), 'src/test/published/harness');
  const extensionTestsPath = path.resolve(__dirname, './suite');
  const repoRoot = path.resolve(__dirname, '../../../..');
  const vscodeExecutablePath = await downloadAndUnzipVSCode();
  const installTarget = await resolveInstallTarget(source, downloadDir);
  const receiptsRoot =
    process.env.PERL_LSP_SMOKE_RECEIPTS_DIR ||
    path.join(repoRoot, 'target', 'receipts', 'vscode-smoke');
  fs.mkdirSync(receiptsRoot, { recursive: true });

  fs.writeFileSync(
    path.join(workspacePath, 'smoke.pl'),
    'use strict;\nuse warnings;\nprint "ok\\n";\n',
  );

  await installExtension(vscodeExecutablePath, installTarget, userDataDir, extensionsDir);

  await runTests({
    vscodeExecutablePath,
    extensionDevelopmentPath: harnessExtensionPath,
    extensionTestsPath,
    extensionTestsEnv: {
      ...process.env,
      PERL_LSP_EXTENSION_TEST_SKIP_STARTUP: '1',
      PERL_LSP_PUBLISHED_EXTENSION_ID: envValue('PERL_LSP_PUBLISHED_EXTENSION_ID') || EXTENSION_ID,
      PERL_LSP_PUBLISHED_EXTENSION_SOURCE: source,
      PERL_LSP_SMOKE_RECEIPTS_DIR: receiptsRoot,
      PERL_LSP_SMOKE_SOURCE_LABEL: process.env.PERL_LSP_SMOKE_SOURCE_LABEL || source,
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
