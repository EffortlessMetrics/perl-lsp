'use strict';

const assert = require('node:assert/strict');
const { test } = require('node:test');
const { verifyPublisherTools } = require('./check-publisher-tools');

void test('publisher CLIs resolve from exact local dependencies without network access', () => {
  const tools = verifyPublisherTools();

  assert.deepEqual(
    tools.map(({ packageName, executable, expectedVersion, reportedVersion }) => ({
      packageName,
      executable,
      expectedVersion,
      reportedVersion,
    })),
    [
      {
        packageName: '@vscode/vsce',
        executable: '@vscode/vsce',
        expectedVersion: '3.9.2',
        reportedVersion: '3.9.2',
      },
      {
        packageName: 'ovsx',
        executable: 'ovsx',
        expectedVersion: '1.0.2',
        reportedVersion: '1.0.2',
      },
    ],
  );
  for (const tool of tools) assert.match(tool.packagePath, /node_modules/);
});
