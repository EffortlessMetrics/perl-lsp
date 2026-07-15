'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');
const assert = require('node:assert/strict');
const { SourceMapGenerator } = require('source-map');
const { symbolicateStackFrame } = require('./check-source-map');

void test('symbolicates Windows and POSIX-shaped bundled frames through a source map', async () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'perl-lsp-source-map-'));
  const mapPath = path.join(directory, 'extension.js.map');
  try {
    const generator = new SourceMapGenerator({ file: 'extension.js' });
    generator.addMapping({
      generated: { line: 3, column: 4 },
      original: { line: 12, column: 6 },
      source: 'src/knownFixture.ts',
    });
    generator.setSourceContent('src/knownFixture.ts', 'throw new Error("known fixture");\n');
    fs.writeFileSync(mapPath, generator.toString());

    const posix = await symbolicateStackFrame(
      '    at fixture (/tmp/out/extension.js:3:5)',
      mapPath,
    );
    const windows = await symbolicateStackFrame(
      '    at fixture (C:\\build\\out\\extension.js:3:5)',
      mapPath,
    );

    assert.deepEqual(posix.original, {
      source: 'src/knownFixture.ts',
      line: 12,
      column: 6,
      name: null,
    });
    assert.deepEqual(windows.original, posix.original);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
