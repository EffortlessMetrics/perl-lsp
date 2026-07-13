'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const extensionRoot = path.resolve(__dirname, '..');
const packageJsonPath = path.join(extensionRoot, 'package.json');

function readPackageJson() {
  return JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
}

function parseVersion(version) {
  const match = /^(\d+)\.(\d+)\.(\d+)/.exec(version.trim());
  if (!match) {
    throw new Error(`invalid semantic version: ${version}`);
  }
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function compareVersions(left, right) {
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) {
      return left[index] - right[index];
    }
  }
  return 0;
}

function readNodeFloor(range) {
  const match = /^\s*>=\s*(\d+)(?:\.(\d+))?(?:\.(\d+))?/.exec(range);
  if (!match) {
    throw new Error(`unsupported Node engine range: ${range}`);
  }
  return [Number(match[1]), Number(match[2] ?? 0), Number(match[3] ?? 0)];
}

function readNpmVersion() {
  const npmExecPath = process.env.npm_execpath;
  if (npmExecPath) {
    return execFileSync(process.execPath, [npmExecPath, '--version'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    }).trim();
  }

  if (process.platform === 'win32') {
    const npmCliPath = path.join(
      path.dirname(process.execPath),
      'node_modules',
      'npm',
      'bin',
      'npm-cli.js',
    );
    if (!fs.existsSync(npmCliPath)) {
      throw new Error(`could not locate npm CLI entry point at ${npmCliPath}`);
    }
    return execFileSync(process.execPath, [npmCliPath, '--version'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    }).trim();
  }

  return execFileSync('npm', ['--version'], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

function describeError(error) {
  if (error && typeof error === 'object' && 'status' in error && typeof error.status === 'number') {
    return `command exited with status ${error.status}`;
  }
  if (error && typeof error === 'object' && 'code' in error && typeof error.code === 'string') {
    return `command failed with ${error.code}`;
  }
  const message = error instanceof Error ? error.message : String(error);
  return message.split(/\r?\n/, 1)[0];
}

function fail(message) {
  console.error(`Toolchain doctor failed: ${message}`);
  process.exitCode = 1;
}

function main() {
  const packageJson = readPackageJson();
  const packageManager = packageJson.packageManager;
  const packageManagerMatch = /^npm@(.+)$/.exec(packageManager ?? '');
  if (!packageManagerMatch) {
    fail(`packageManager must be an npm version, received ${String(packageManager)}`);
    return;
  }

  const expectedNpmVersion = packageManagerMatch[1];
  if (packageJson.engines?.npm !== expectedNpmVersion) {
    fail(
      `engines.npm ${String(packageJson.engines?.npm)} does not match packageManager npm@${expectedNpmVersion}`,
    );
    return;
  }
  let nodeFloor;
  try {
    nodeFloor = readNodeFloor(packageJson.engines?.node ?? '');
  } catch (error) {
    fail(
      `could not read the Node engine floor: ${error instanceof Error ? error.message : String(error)}`,
    );
    return;
  }
  const nodeVersion = parseVersion(process.versions.node);
  if (compareVersions(nodeVersion, nodeFloor) < 0) {
    fail(`Node ${process.versions.node} is below the declared floor ${packageJson.engines.node}`);
    return;
  }

  let npmVersion;
  try {
    npmVersion = readNpmVersion();
  } catch (error) {
    fail(`could not determine npm version: ${describeError(error)}`);
    return;
  }
  if (npmVersion !== expectedNpmVersion) {
    fail(`npm ${npmVersion} does not match packageManager npm@${expectedNpmVersion}`);
    return;
  }

  console.log(
    `Toolchain doctor passed: Node ${process.versions.node}, npm ${npmVersion}, packageManager npm@${expectedNpmVersion}`,
  );
}

main();
