'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');

const extensionRoot = path.resolve(__dirname, '..');
const repositoryRoot = path.resolve(extensionRoot, '..');
const extensionWorkflows = [
  'vscode-current-source-linux-smoke.yml',
  'vscode-managed-binary-smoke.yml',
  'vscode-published-extension-smoke.yml',
  'ux-regression-gate.yml',
  'publish-extension.yml',
];

function readWorkflow(name) {
  return fs.readFileSync(path.join(repositoryRoot, '.github', 'workflows', name), 'utf8');
}

void test('extension workflows use the repository-owned setup action', () => {
  for (const name of extensionWorkflows) {
    const source = readWorkflow(name);
    assert.match(source, /uses: \.\/\.github\/actions\/setup-vscode-toolchain/);
    assert.doesNotMatch(source, /actions\/setup-node@/);
    assert.doesNotMatch(source, /npm install --global npm@/);
    assert.doesNotMatch(source, /npm install -g (?:@vscode\/vsce|ovsx)/);
    assert.doesNotMatch(source, /npx ovsx/);
  }
});

void test('the setup action verifies the authority before npm ci', () => {
  const source = fs.readFileSync(
    path.join(repositoryRoot, '.github', 'actions', 'setup-vscode-toolchain', 'action.yml'),
    'utf8',
  );
  const verifyIndex = source.indexOf('run: npm run doctor');
  const installIndex = source.indexOf('run: npm ci');
  assert.notEqual(verifyIndex, -1);
  assert.notEqual(installIndex, -1);
  assert.ok(verifyIndex < installIndex);
  assert.match(source, /node-version: ['"]26\.5\.0['"]/);
  assert.match(source, /npm install --global npm@11\.18\.0/);
});

void test('publisher workflow invokes both CLIs offline through npm exec', () => {
  const source = readWorkflow('publish-extension.yml');
  assert.match(source, /npm exec --offline --no -- @vscode\/vsce publish/);
  assert.match(source, /npm exec --offline --no -- ovsx publish/);
  assert.doesNotMatch(source, /^\s+run: (?:vsce|ovsx) publish/m);
  assert.doesNotMatch(source, /^\s+run: ovsx --version/m);
});
