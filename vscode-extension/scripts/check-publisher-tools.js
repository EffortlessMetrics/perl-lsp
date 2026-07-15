'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const extensionRoot = path.resolve(__dirname, '..');
const packageJson = JSON.parse(fs.readFileSync(path.join(extensionRoot, 'package.json'), 'utf8'));

const publisherTools = [
  { packageName: '@vscode/vsce', executable: '@vscode/vsce' },
  { packageName: 'ovsx', executable: 'ovsx' },
];

function npmInvocation(args) {
  if (process.env.npm_execpath) {
    return { command: process.execPath, args: [process.env.npm_execpath, ...args], shell: false };
  }
  if (process.platform === 'win32') {
    const npmCliPath = path.join(
      path.dirname(process.execPath),
      'node_modules',
      'npm',
      'bin',
      'npm-cli.js',
    );
    if (fs.existsSync(npmCliPath)) {
      return { command: process.execPath, args: [npmCliPath, ...args], shell: false };
    }
  }
  return {
    command: process.platform === 'win32' ? 'npm.cmd' : 'npm',
    args,
    shell: process.platform === 'win32',
  };
}

function exactDependencyVersion(packageName) {
  const version = packageJson.devDependencies?.[packageName];
  if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(
      `${packageName} must use an exact semver devDependency, received ${String(version)}`,
    );
  }
  return version;
}

function verifyPublisherTools() {
  return publisherTools.map(({ packageName, executable }) => {
    const expectedVersion = exactDependencyVersion(packageName);
    const packagePath = require.resolve(`${packageName}/package.json`, { paths: [extensionRoot] });
    const installedVersion = JSON.parse(fs.readFileSync(packagePath, 'utf8')).version;
    if (installedVersion !== expectedVersion) {
      throw new Error(
        `${packageName} resolved to ${installedVersion}, expected lockfile authority ${expectedVersion}`,
      );
    }

    const invocation = npmInvocation(['exec', '--offline', '--no', '--', executable, '--version']);
    const result = spawnSync(invocation.command, invocation.args, {
      cwd: extensionRoot,
      encoding: 'utf8',
      shell: invocation.shell,
      windowsHide: true,
    });
    if (result.error) throw result.error;
    if (result.status !== 0) {
      throw new Error(
        `${executable} --version failed with ${result.status}: ${(result.stderr || result.stdout).trim()}`,
      );
    }
    const reportedVersion = (result.stdout || '').trim().split(/\r?\n/).filter(Boolean).at(-1);
    if (reportedVersion !== expectedVersion) {
      throw new Error(
        `${executable} reported ${String(reportedVersion)}, expected ${expectedVersion}`,
      );
    }

    return { packageName, expectedVersion, packagePath, executable, reportedVersion };
  });
}

function main() {
  for (const tool of verifyPublisherTools()) {
    process.stdout.write(
      `[publisher-tools] ${tool.packageName}@${tool.reportedVersion} via npm exec --offline (${tool.packagePath})\n`,
    );
  }
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    process.stderr.write(
      `[publisher-tools] ${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}

module.exports = { exactDependencyVersion, verifyPublisherTools };
