// Rolldown production bundle config for the VS Code extension (#3662, final
// step of the TS7 migration train). Rolldown replaces TypeScript EMISSION as
// the production artifact builder — it does NOT type-check. TypeScript 7
// (`tsc --noEmit`, the `typecheck` npm script) remains the sole type-check
// authority; this config only turns already-valid TypeScript into the single
// runtime artifact VS Code loads.
//
// Single CJS entry: src/extension.ts -> out/extension.js. This exact path is
// load-bearing: package.json's "main" and the debugger's "program" both
// point at ./out/extension.js, and this config preserves it byte-for-byte.
//
// Rolldown is ESM-only (no CJS export), so this config file is itself ESM
// (.mjs) even though the rest of the extension's tooling is CommonJS.
import { builtinModules } from 'node:module';
import { defineConfig } from 'rolldown';

// Node built-ins must never be bundled — Node resolves `require('fs')` etc.
// natively at runtime. Cover both the bare form (`fs`) and the explicit
// `node:` prefix form (`node:fs`), since source may use either.
const nodeBuiltins = new Set([...builtinModules, ...builtinModules.map((m) => `node:${m}`)]);

// Runtime dependency classification (package.json "dependencies"):
//   - adm-zip: pure JS, no native bindings, no dynamic environment-based
//     require. Safe to bundle.
//   - tar: pure JS (no native/optional bindings of its own), heavy internal
//     module graph but no dynamic `require(computedPath)` patterns. Safe to
//     bundle — verified via the parity-proof integration tests, which
//     exercise real archive extraction end-to-end against the bundled
//     artifact, not just a grep.
//   - vscode-languageclient: pure JS/TS, itself already depends on `vscode`
//     as an external. Safe to bundle.
// `vscode` itself is supplied by the VS Code extension host at runtime, not
// resolvable as a real package — it MUST stay external regardless of the
// above.
const external = (id) => id === 'vscode' || nodeBuiltins.has(id);

export default defineConfig({
  input: 'src/extension.ts',
  tsconfig: './tsconfig.json',
  platform: 'node',
  external,
  output: {
    file: 'out/extension.js',
    format: 'cjs',
    sourcemap: true,
    // No minification in this first PR — the migration charter is explicit
    // that this pass proves parity of the bundling step alone. Minification
    // is a separate, later decision.
    minify: false,
    // Strict single-file output. Without this, Rolldown split a facade
    // chunk (out/commandResults.js) for a type-only `import type {...}
    // from './commandResults'` in extension.ts even though there is no
    // runtime dynamic import() anywhere in the source (verified). CJS has
    // no native async chunk-loading anyway, so a split there would just be
    // a synchronous `require()` of a sibling file — codeSplitting: false
    // inlines everything into the one artifact the "single CJS entry" spec
    // requires, and matches out/extension.js being the sole path referenced
    // by package.json's "main" and the debugger's "program".
    codeSplitting: false,
  },
});

// NOTE on out/ hygiene: `output.cleanDir` was tried here but does not apply
// in single-file (`output.file`) mode — verified empirically: a stray
// out/commandResults.js + .map left behind by an unrelated
// `tsc -p tsconfig.integration.json` run (out/ is a shared build directory;
// that command also emits the integration test harness into out/test/**)
// survived cleanDir:true across a subsequent `npm run compile`. The actual
// fix is the "clean:out" npm script (removes everything under out/ except
// out/test/**, which the test-harness tsc builds manage separately) that
// runs before this config is invoked — see package.json's "compile" script.
