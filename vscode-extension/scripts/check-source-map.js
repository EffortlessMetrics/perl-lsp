'use strict';

const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { SourceMapConsumer } = require('source-map');

const extensionRoot = path.resolve(__dirname, '..');
const bundlePath = path.join(extensionRoot, 'out', 'extension.js');
const sourceMapPath = path.join(extensionRoot, 'out', 'extension.js.map');
const receiptDirectory = path.join(extensionRoot, '..', 'target', 'receipts', 'vscode-source-map');

function sha256(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

function gitRevision() {
  try {
    return execFileSync('git', ['rev-parse', 'HEAD'], {
      cwd: extensionRoot,
      encoding: 'utf8',
      windowsHide: true,
    }).trim();
  } catch {
    return process.env.GITHUB_SHA || null;
  }
}

function findPackagedVsix() {
  const candidates = fs
    .readdirSync(extensionRoot)
    .filter((file) => file.endsWith('.vsix'))
    .map((file) => path.join(extensionRoot, file));
  return candidates.length === 1 ? candidates[0] : undefined;
}

function stackFrame(bundleFile, line, column) {
  return `    at known bundled fixture (${bundleFile}:${line}:${column + 1})`;
}

async function symbolicateStackFrame(frame, mapFile) {
  const match = /\((.*):([0-9]+):([0-9]+)\)$/.exec(frame.trim());
  if (!match) {
    throw new Error(`Unsupported bundled stack frame: ${frame}`);
  }
  const generatedLine = Number(match[2]);
  const generatedColumn = Number(match[3]) - 1;
  const map = JSON.parse(fs.readFileSync(mapFile, 'utf8'));

  return SourceMapConsumer.with(map, null, (consumer) => {
    const original = consumer.originalPositionFor({
      line: generatedLine,
      column: generatedColumn,
    });
    if (!original.source || original.line === null || original.column === null) {
      throw new Error(`No source-map position for ${frame}`);
    }
    return {
      generated: {
        file: match[1],
        line: generatedLine,
        column: generatedColumn,
      },
      original: {
        source: original.source,
        line: original.line,
        column: original.column,
        name: original.name,
      },
    };
  });
}

async function createReceipt({
  bundle = bundlePath,
  sourceMap = sourceMapPath,
  vsix = findPackagedVsix(),
} = {}) {
  if (!fs.existsSync(bundle)) {
    throw new Error(`Bundle does not exist: ${bundle}`);
  }
  if (!fs.existsSync(sourceMap)) {
    throw new Error(`Source map does not exist: ${sourceMap}`);
  }

  const map = JSON.parse(fs.readFileSync(sourceMap, 'utf8'));
  if (map.version !== 3 || map.file !== 'extension.js') {
    throw new Error('Expected a version-3 source map for extension.js');
  }
  const source = map.sources.find((entry) => entry.endsWith('/src/workspaceTopology.ts'));
  if (!source) {
    throw new Error('Source map does not retain the known workspaceTopology.ts source');
  }
  const sourceIndex = map.sources.indexOf(source);
  if (!map.sourcesContent?.[sourceIndex]) {
    throw new Error(`Source map has no embedded content for ${source}`);
  }

  const mapped = await SourceMapConsumer.with(map, null, (consumer) => {
    const sourceLineCount = map.sourcesContent[sourceIndex].split(/\r?\n/).length;
    for (let sourceLine = 1; sourceLine <= sourceLineCount; sourceLine += 1) {
      const positions = consumer.allGeneratedPositionsFor({ source, line: sourceLine, column: 0 });
      const position = positions.find(
        (candidate) => candidate.line !== null && candidate.column !== null,
      );
      if (position && position.line !== null && position.column !== null) {
        return { line: position.line, column: position.column, sourceLine };
      }
    }
    throw new Error(`Source map has no generated position for ${source}`);
  });
  const frame = stackFrame(path.basename(bundle), mapped.line, mapped.column);
  const symbolicated = await symbolicateStackFrame(frame, sourceMap);
  if (!symbolicated.original.source.endsWith('/src/workspaceTopology.ts')) {
    throw new Error(
      `Known fixture mapped to an unexpected source: ${symbolicated.original.source}`,
    );
  }

  const packageJson = JSON.parse(fs.readFileSync(path.join(extensionRoot, 'package.json'), 'utf8'));
  const receipt = {
    schema_version: 1,
    kind: 'vscode-bundle-source-map',
    extension_version: packageJson.version,
    source_revision: gitRevision(),
    bundle_sha256: sha256(bundle),
    source_map_sha256: sha256(sourceMap),
    vsix_sha256: vsix && fs.existsSync(vsix) ? sha256(vsix) : null,
    rolldown_version: packageJson.devDependencies.rolldown,
    source_map: {
      file: path.basename(sourceMap),
      source_count: map.sources.length,
      sources_content: true,
      known_fixture_source: source,
      known_fixture_source_line: mapped.sourceLine,
      generated_frame: symbolicated.generated,
      original_frame: symbolicated.original,
    },
    package_policy: {
      source_maps_in_development_bundle: true,
      source_maps_in_vsix: false,
      archive_required_for_diagnostics: true,
    },
    claim_boundary:
      'This receipt proves that the exact development bundle and retained map resolve a known generated frame to the embedded source path. It does not prove a production crash occurred or authorize publishing the map as a public VSIX asset.',
  };
  fs.mkdirSync(receiptDirectory, { recursive: true });
  fs.copyFileSync(bundle, path.join(receiptDirectory, 'extension.js'));
  fs.copyFileSync(sourceMap, path.join(receiptDirectory, 'extension.js.map'));
  fs.writeFileSync(
    path.join(receiptDirectory, 'source-map-receipt.json'),
    `${JSON.stringify(receipt, null, 2)}\n`,
  );
  return receipt;
}

if (require.main === module) {
  createReceipt()
    .then((receipt) => {
      process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
    })
    .catch((error) => {
      process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
      process.exitCode = 1;
    });
}

module.exports = { createReceipt, symbolicateStackFrame };
