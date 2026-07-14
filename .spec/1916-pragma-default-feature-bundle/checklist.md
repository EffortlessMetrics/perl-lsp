# Checklist — #1916 pragma `:default` feature bundle baseline

- [x] Verify `:default` bundle membership against perldoc.perl.org/feature
- [x] Verify `DEFAULT_FEATURES` already equals the `:default` set
- [x] Custom `Default for PragmaState` seeds `features = DEFAULT_FEATURES`
- [x] `all_strict()` spreads `..Self::default()` (inherits the bundle)
- [x] Strengthen `comprehensive_unit_tests` baseline-contract tests
- [x] Add `tests/default_feature_bundle_tests.rs` (baseline + version bundles + no-feature)
- [x] `cargo test -p perl-pragma` green
- [x] `cargo check --all-targets -p perl-pragma` green
- [x] Consumers green: perl-parser-core, perl-lsp-rs-core, tree-sitter-perl-rs
- [x] `cargo clippy -p perl-pragma --all-targets` clean
- [x] `cargo xtask fmt`
