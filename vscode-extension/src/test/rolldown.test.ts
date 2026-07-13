/**
 * Contract tests for the Rolldown production bundle configuration.
 *
 * The final step of the TS7 migration (#3662): Rolldown replaces TypeScript
 * EMISSION as the production artifact builder (src/extension.ts ->
 * out/extension.js) — it does NOT type-check. TypeScript 7 (`tsc --noEmit`,
 * the `typecheck` npm script) remains the sole type-check authority.
 *
 * These tests verify that:
 *   - rolldown.config.mjs exists and is loadable
 *   - the config produces a single CJS file at the exact path package.json's
 *     "main" and the debugger's "program" both expect (out/extension.js)
 *   - `vscode` is externalized (never bundled — it's supplied by the
 *     extension host, not resolvable as a real package)
 *   - code splitting is disabled (strict single-file output; this exact
 *     regression was caught during development: a type-only `import type`
 *     produced a stray out/commandResults.js facade chunk until
 *     `codeSplitting: false` was added)
 *   - minification is off (deliberately out of scope for this first PR)
 *   - source maps are enabled
 *   - package.json's compile/typecheck scripts are correctly wired
 *   - rolldown is exactly pinned (no ^/~)
 *   - the packaged VSIX ships no node_modules/** at all (every runtime
 *     dependency is now bundled) and no dev-tooling native binaries
 */

import { execFileSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

const EXT_ROOT = path.resolve(__dirname, '..', '..');

describe('Rolldown bundle configuration', () => {
  test('rolldown.config.mjs exists at extension root', () => {
    const configPath = path.join(EXT_ROOT, 'rolldown.config.mjs');
    expect(fs.existsSync(configPath)).toBe(true);
  });

  test('rolldown.config.mjs targets the exact main/debugger entry path (out/extension.js)', () => {
    const configPath = path.join(EXT_ROOT, 'rolldown.config.mjs');
    const source = fs.readFileSync(configPath, 'utf8');
    expect(source).toContain("input: 'src/extension.ts'");
    expect(source).toContain("file: 'out/extension.js'");

    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.main).toBe('./out/extension.js');
    const debugger_ = pkg.contributes?.debuggers?.find((d: { type?: string }) => d.type === 'perl');
    expect(debugger_?.program).toBe('./out/extension.js');
  });

  test('rolldown.config.mjs externalizes vscode and disables code splitting/minification', () => {
    const configPath = path.join(EXT_ROOT, 'rolldown.config.mjs');
    const source = fs.readFileSync(configPath, 'utf8');
    expect(source).toContain("id === 'vscode'");
    expect(source).toContain('codeSplitting: false');
    expect(source).toContain('minify: false');
    expect(source).toContain('sourcemap: true');
    expect(source).toContain("format: 'cjs'");
    expect(source).toContain("platform: 'node'");
  });

  test('package.json prepublish verifies the toolchain, typechecks all surfaces, then compiles', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    // "clean:out" first: out/ is a shared build directory (the integration/
    // published-smoke test harnesses also emit into out/test/** via a
    // separate tsc step) — without cleaning stray top-level files first, a
    // leftover from an earlier build (verified: tsc -p tsconfig.integration.json
    // emitting out/commandResults.js as a type-only-import byproduct) can
    // survive a subsequent `npm run compile` and leak into a packaged VSIX.
    expect(pkg.scripts.compile).toBe('npm run clean:out && rolldown -c rolldown.config.mjs');
    expect(pkg.scripts['clean:out']).toContain("f!=='test'");
    expect(pkg.scripts.typecheck).toContain('tsc');
    expect(pkg.scripts.typecheck).toContain('--noEmit');
    // The real release/packaging path must typecheck before bundling — a
    // bundler alone cannot catch a type error.
    expect(pkg.scripts['vscode:prepublish']).toBe(
      'npm run doctor && npm run typecheck:all && npm run compile',
    );
  });

  test('package.json has rolldown devDependency, exactly pinned (no ^/~)', () => {
    const pkg = JSON.parse(fs.readFileSync(path.join(EXT_ROOT, 'package.json'), 'utf8'));
    expect(pkg.devDependencies).toHaveProperty('rolldown');
    expect(pkg.devDependencies.rolldown).toMatch(/^\d+\.\d+\.\d+$/);
  });
});

describe('VSIX packaging ships a single bundled artifact and no raw node_modules', () => {
  // `vsce ls` is the packaging tool's own dry-run manifest of every file that
  // would be included in the published .vsix. Oxfmt/Oxlint/Rolldown all ship
  // native platform bindings via optionalDependencies under devDependencies
  // — vsce is expected to exclude devDependencies-only packages from the
  // package, but that exclusion is vsce's dependency-analysis behavior, not
  // this repo's own code. A future change that moves one of them to
  // `dependencies` by mistake, or an .vscodeignore edit that accidentally
  // widens node_modules inclusion, would silently bundle a multi-megabyte
  // native binary into the published extension with no other signal. This
  // test asserts against the real packaging manifest, not just config
  // intent, so that regression is caught here instead of in a published
  // release.
  //
  // Since the Rolldown production bundle inlines every runtime dependency
  // (adm-zip, tar, vscode-languageclient — verified pure JS, no
  // __dirname-relative asset loading, no native .node bindings anywhere in
  // their transitive trees) into the single out/extension.js artifact,
  // node_modules/** is excluded from the VSIX entirely (see
  // .vscodeignore). These assertions are stricter than the pre-Rolldown
  // version (which only checked for dev-tooling names): no node_modules/**
  // entries of ANY kind are expected now.
  //
  // `vsce ls` packages whatever is currently on disk under out/ — unlike
  // `vsce package`, it does NOT run the "vscode:prepublish" script itself.
  // Nothing else in the jest pipeline (compile:test only rebuilds
  // out-test/, a separate directory) guarantees out/extension.js exists.
  // Build it explicitly here so this suite is self-sufficient regardless
  // of what ran before it (a bare `npm test`, a CI job ordering change,
  // etc.) and reflects the current source, not a stale leftover build.
  beforeAll(() => {
    // Replicate the "compile" npm script (clean:out + rolldown) via direct
    // `node` invocations rather than `npm.cmd`/`npx` — npm's own Windows
    // .cmd shim EINVALs under spawnSync without shell:true, and shell:true
    // string-concatenates args instead of escaping them (the same class of
    // issue already fixed in scripts/lint-canary.js during PREP-2).
    const outDir = path.join(EXT_ROOT, 'out');
    if (fs.existsSync(outDir)) {
      for (const entry of fs.readdirSync(outDir)) {
        if (entry !== 'test') {
          fs.rmSync(path.join(outDir, entry), { recursive: true, force: true });
        }
      }
    }
    const rolldownEntry = path.join(EXT_ROOT, 'node_modules', 'rolldown', 'bin', 'cli.mjs');
    execFileSync(process.execPath, [rolldownEntry, '-c', 'rolldown.config.mjs'], {
      cwd: EXT_ROOT,
    });
  }, 60000);

  function listVsixFiles(): string[] {
    // Invoke vsce's own JS entry point directly via `node`, not `npx` (which
    // needs a shell on Windows to resolve — shell:true string-concatenates
    // args instead of escaping them, a real risk once paths contain spaces).
    const vsceEntry = path.join(EXT_ROOT, 'node_modules', '@vscode', 'vsce', 'vsce');
    const output = execFileSync(process.execPath, [vsceEntry, 'ls'], {
      cwd: EXT_ROOT,
      encoding: 'utf8',
    });
    return output.split(/\r?\n/).filter((line) => line.trim().length > 0);
  }

  test('vsce ls manifest contains no node_modules entries at all', () => {
    const files = listVsixFiles();
    expect(files.length).toBeGreaterThan(0);

    const nodeModulesEntries = files.filter((f) => /(^|[\\/])node_modules[\\/]/.test(f));
    expect(nodeModulesEntries).toEqual([]);

    // The two tiny rc config files are expected and harmless (not code, not
    // a binary) — assert their presence so this test also documents that
    // deliberate, known inclusion rather than silently tolerating it.
    expect(files).toEqual(expect.arrayContaining(['.oxfmtrc.json', '.oxlintrc.json']));
  }, 30000);

  test('vsce ls manifest ships exactly one bundled artifact at out/extension.js', () => {
    const files = listVsixFiles();
    const outFiles = files.filter((f) => f.replace(/\\/g, '/').startsWith('out/'));

    // Strict single-file bundle: no facade/split chunks (e.g. a stray
    // out/commandResults.js from an under-elided type-only import — this
    // exact regression was caught and fixed via output.codeSplitting: false
    // in rolldown.config.mjs), no leftover .map (excluded via
    // .vscodeignore's **/*.map), no rolldown.config.mjs itself (also
    // excluded via .vscodeignore).
    expect(outFiles).toEqual(['out/extension.js']);
  }, 30000);

  test('vsce ls manifest contains no rolldown config or dev-tooling rc leaks beyond the known two', () => {
    const files = listVsixFiles();
    expect(files).not.toContain('rolldown.config.mjs');
    const rcFiles = files.filter((f) => f.endsWith('rc.json'));
    expect(rcFiles.sort()).toEqual(['.oxfmtrc.json', '.oxlintrc.json']);
  }, 30000);
});
