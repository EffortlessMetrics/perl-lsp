# Acceptance — #1916 pragma `:default` feature bundle baseline

## Behavior

- `PragmaState::default()` enables the `:default` bundle and nothing else.
- `all_strict()` enables the three strict categories AND keeps `:default`.
- A plain file (no pragmas) reports the `:default` features at every offset.
- `use vX.Y` correctly disables default-on features per the bundle table.
- `no feature 'X'` removes one default-on feature; bare `no feature` resets.

## Hazards

- `Default` is consumed as the universal "no pragmas" baseline (build init,
  before-first-entry / empty-map fallbacks). All must reflect `:default`.
- HIR `PragmaStateFact.features` now lists the baseline for real directives.
  No golden full-list assertions exist (only `has_feature` queries) — verified.
- `say`/`state`/`signatures` are NOT in `:default`; existing `has_feature("say")`
  consumer checks must stay correct.

## Contracts

- `:default` set == `version::DEFAULT_FEATURES` — single source of truth.
- `use vX.Y` replaces (not merges) the feature set; bare `no feature` resets to
  `DEFAULT_FEATURES`. (Pre-existing, unchanged.)

## Test-Grid

| Case | File | Assertion |
|------|------|-----------|
| default has `:default` | default_feature_bundle_tests | has indirect/multidimensional/bareword/apostrophe/smartmatch; not say/state |
| default flags cleared | default_feature_bundle_tests | strict/warnings/utf8/... false |
| plain file → `:default` | default_feature_bundle_tests | state_for_offset reports bundle |
| `all_strict` keeps bundle | default_feature_bundle_tests | strict on + bundle |
| `use strict` == all_strict | default_feature_bundle_tests | final_state eq all_strict() |
| v5.36 disables indirect/multi | default_feature_bundle_tests | none indirect/multi/switch; keeps bareword/apos/smartmatch |
| v5.38 disables bareword | default_feature_bundle_tests | none bareword; module_true present |
| v5.42 disables apos/smartmatch | default_feature_bundle_tests | none of the 5; try/module_true present |
| `no feature 'multi'` | default_feature_bundle_tests | multi off, rest on |
| bare `no feature` reset | default_feature_bundle_tests | say off, bundle restored |
| strengthened contract | comprehensive_unit_tests | default carries bundle; use strict keeps it |

## Blast-Radius

`perl-pragma` + direct consumers `perl-parser-core`, `perl-lsp-rs-core`,
`tree-sitter-perl-rs`. All test suites pass unchanged.
