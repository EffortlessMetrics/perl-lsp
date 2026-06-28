# Acceptance: Visual Regression Testing (TextMate grammar snapshots)

## Behavior

- `cd vscode-extension && npm run test:grammar` verifies every fixture's tokens
  against committed `.snap` files and exits **0** when the grammar is unchanged.
- `npm run test:grammar:update` regenerates the `.snap` files from the current
  grammar.
- Any change to `syntaxes/perl.tmLanguage.json` that alters a token's scope makes
  `npm run test:grammar` exit **non-zero** with a per-line diff (old scope vs new
  scope).

## Hazards

| Hazard | Mitigation / test |
| --- | --- |
| A regression test that can't detect regressions | Verified empirically: perturbing a scope name (`keyword.other.perl` → `keyword.other.REGRESSED.perl`) makes the suite exit 255 with a diff; reverting returns it to exit 0. |
| Non-determinism / flaky snapshots | Engine is deterministic; running verify twice with no change is green. No timestamps or paths embedded in `.snap` output. |
| Network/display dependency breaking CI | `vscode-oniguruma` WASM is bundled in the npm package; runs offline with no display. Confirmed locally. |
| Grammar/scope drift from what ships to users | Scope resolved from the extension's own `package.json` contributes (`--config package.json`), not a hardcoded path — tests the shipped mapping. |
| CI status falsely green when grammar fails | `ci/extension-jest` commit-status script updated to AND the `jest` and `grammar` step outcomes. |
| New non-Rust files rejected by file policy | `vscode-extension/**` is covered by the `non-rust-vscode-extension` allowlist entry. |

## Contracts

- Snapshot files live next to fixtures: `test/grammar/fixtures/<name>.pl.snap`.
- Fixtures are valid Perl and each targets a distinct grammar repository key.

## API-Shape

- `package.json` scripts: `test:grammar`, `test:grammar:update`.
- devDependency: `vscode-tmgrammar-test ^0.1.3` (in `package-lock.json`).

## Test-Grid

| Fixture | Grammar surface | Asserted scopes (examples) |
| --- | --- | --- |
| comments.pl | comments | `comment.line.number-sign.perl` |
| variables.pl | variables | `variable.other.scalar.perl`, array/hash/slice/special vars |
| strings.pl | strings, interpolation | `string.quoted.double.perl`, escapes, q/qq/qw |
| numbers.pl | numbers | int/float/hex/octal/binary/underscored/exponent |
| keywords_control.pl | keywords | `keyword.other.perl`, package/sub/control flow |
| operators.pl | operators | arithmetic/comparison/logical/ternary/range |
| functions.pl | functions | `support.function.builtin.perl` |
| regex.pl | regex | m// s/// tr// qr// + modifiers |
| pod.pl | pod | `=pod`…`=cut` documentation block |

## Blast-Radius

- Additive only: new fixtures, snapshots, two npm scripts, one devDependency,
  one CI step, doc updates. No production source or runtime behavior changes.
- Lint (`eslint src`) and typecheck (`tsc -p ./`) are unaffected — fixtures are
  `.pl`/`.snap`, outside `src/`.
