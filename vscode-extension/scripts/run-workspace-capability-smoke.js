#!/usr/bin/env node

const fs = require('fs');
const crypto = require('crypto');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const extensionRoot = path.resolve(__dirname, '..');
const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const serverPath = process.env.PERL_LSP_FIRST_HOUR_SERVER_PATH;
const serverSha = process.env.PERL_LSP_SERVER_SOURCE_SHA;

if (!serverPath || !fs.existsSync(serverPath) || !serverSha) {
  throw new Error(
    'PERL_LSP_FIRST_HOUR_SERVER_PATH and PERL_LSP_SERVER_SOURCE_SHA must identify the matching server build.',
  );
}
const serverArtifactSha256 = crypto
  .createHash('sha256')
  .update(fs.readFileSync(serverPath))
  .digest('hex');

function writePerlFixture(directory, name) {
  const fixturePath = path.join(directory, name);
  fs.writeFileSync(fixturePath, 'use strict;\nuse warnings;\nprint "ok\\n";\n');
}

function runSmoke(label, workspace, expectedTrust, expectedMode, expectedFolderCount, trustMode) {
  const result = spawnSync(npmCommand, ['run', 'test:published:local'], {
    cwd: extensionRoot,
    env: {
      ...process.env,
      PERL_LSP_SMOKE_WORKSPACE: workspace,
      PERL_LSP_SMOKE_SOURCE_LABEL: label,
      PERL_LSP_SERVER_ARTIFACT_SHA256: serverArtifactSha256,
      PERL_LSP_SMOKE_WORKSPACE_TRUST: trustMode,
      PERL_LSP_EXPECTED_WORKSPACE_TRUST: expectedTrust,
      PERL_LSP_EXPECTED_WORKSPACE_MODE: expectedMode,
      PERL_LSP_EXPECTED_FOLDER_COUNT: String(expectedFolderCount),
    },
    stdio: 'inherit',
    shell: process.platform === 'win32',
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(`${label} workspace capability smoke failed with exit ${result.status}`);
  }
}

const fixtureRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-workspace-capabilities-'));
try {
  const multiRoot = path.join(fixtureRoot, 'multi-root');
  const multiRootA = path.join(multiRoot, 'workspace-a');
  const multiRootB = path.join(multiRoot, 'workspace-b');
  fs.mkdirSync(multiRootA, { recursive: true });
  fs.mkdirSync(multiRootB, { recursive: true });
  writePerlFixture(multiRootA, 'smoke.pl');
  writePerlFixture(multiRootB, 'smoke.pl');
  const descriptorPath = path.join(multiRoot, 'multi-root.code-workspace');
  fs.writeFileSync(
    descriptorPath,
    `${JSON.stringify({ folders: [{ path: 'workspace-a' }, { path: 'workspace-b' }] }, null, 2)}\n`,
  );

  runSmoke('workspace-multi-root', descriptorPath, 'trusted', 'multi-root', 2, 'disabled');

  const untrustedRoot = path.join(fixtureRoot, 'untrusted');
  fs.mkdirSync(untrustedRoot, { recursive: true });
  writePerlFixture(untrustedRoot, 'smoke.pl');
  runSmoke('workspace-untrusted', untrustedRoot, 'untrusted', 'single-root', 1, 'untrusted');
} finally {
  fs.rmSync(fixtureRoot, { recursive: true, force: true });
}
