#!/usr/bin/env node

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const root = path.resolve(__dirname, '..');
const serverPath = process.env.PERL_LSP_FIRST_HOUR_SERVER_PATH;
if (!serverPath || !fs.existsSync(serverPath)) {
  process.stderr.write(
    'PERL_LSP_FIRST_HOUR_SERVER_PATH must point to an existing server built from the current source revision.\n',
  );
  process.exit(2);
}
const serverSourceRevision = (process.env.PERL_LSP_SERVER_SOURCE_SHA || '').trim();
if (!serverSourceRevision) {
  process.stderr.write(
    'PERL_LSP_SERVER_SOURCE_SHA must identify the source revision used to build the server.\n',
  );
  process.exit(2);
}

const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';

function runNpm(args, env) {
  return spawnSync(npmCommand, args, {
    cwd: root,
    env,
    stdio: 'inherit',
    shell: process.platform === 'win32',
    windowsHide: true,
  });
}

function gitRevision() {
  const result = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(`Unable to determine source revision: ${result.stderr || result.error}`);
  }
  return result.stdout.trim();
}

function ensureCleanWorkingTree() {
  const result = spawnSync('git', ['status', '--porcelain'], {
    cwd: root,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    throw new Error(`Unable to inspect working tree: ${result.stderr || result.error}`);
  }
  if (result.stdout.trim()) {
    throw new Error(
      'Working tree has uncommitted changes; commit them before running the local VSIX smoke test.',
    );
  }
}

const revision = gitRevision();
ensureCleanWorkingTree();
if (serverSourceRevision !== revision) {
  throw new Error(
    `Server source revision ${serverSourceRevision} does not match extension source revision ${revision}`,
  );
}
const packageJson = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const vsixPath = path.join(root, `${packageJson.name}-${packageJson.version}.vsix`);

let smokeStatus = 1;
try {
  const packageResult = runNpm(['run', 'package'], process.env);
  if (packageResult.status !== 0) {
    smokeStatus = packageResult.status ?? 1;
  } else {
    if (!fs.existsSync(vsixPath)) {
      throw new Error(`vsce did not produce the expected VSIX: ${vsixPath}`);
    }

    const smokeEnv = {
      ...process.env,
      PERL_LSP_CURRENT_SOURCE_SHA: revision,
      PERL_LSP_CURRENT_SOURCE_SMOKE: '1',
      PERL_LSP_FIRST_HOUR_ONLY: '1',
      PERL_LSP_FIRST_HOUR_RECEIPT: '1',
      PERL_LSP_FIRST_HOUR_SERVER_PATH: path.resolve(serverPath),
      PERL_LSP_PUBLISHED_EXTENSION_SOURCE: 'vsix',
      PERL_LSP_PUBLISHED_VSIX_PATH: vsixPath,
      PERL_LSP_SERVER_SOURCE_SHA: serverSourceRevision,
      PERL_LSP_SMOKE_SOURCE_LABEL:
        process.env.PERL_LSP_SMOKE_SOURCE_LABEL || 'local-current-source',
      PERL_LSP_VSIX_SHA256: crypto
        .createHash('sha256')
        .update(fs.readFileSync(vsixPath))
        .digest('hex'),
    };

    const smokeResult = runNpm(['run', 'test:published'], smokeEnv);
    smokeStatus = smokeResult.status ?? 1;
  }
} finally {
  fs.rmSync(vsixPath, { force: true });
}
process.exit(smokeStatus);
