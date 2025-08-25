## Description

Brief description of what this PR does.

## Related Issue

Fixes #(issue number)

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring

## How Has This Been Tested?

Describe the tests that you ran to verify your changes.

- [ ] Unit tests
- [ ] Integration tests
- [ ] Corpus tests
- [ ] Manual testing

## Checklist

### Required for ALL PRs
- [ ] My code follows the style guidelines of this project
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] I have run `cargo xtask fmt` to format my code
- [ ] I have run `cargo xtask check --clippy` and addressed any warnings
- [ ] I have run `cargo xtask test` and all tests pass

### LSP Features Merge-Gate (if modifying LSP features)
- [ ] Updated `features.toml` with any new/changed features
- [ ] Regenerated snapshots: `LC_ALL=C.UTF-8 INSTA_UPDATE=auto cargo test -p perl-parser --test lsp_features_snapshot_test`
- [ ] Verified catalog: `cargo xtask features verify` (no errors; no % drift)
- [ ] Synced docs: `cargo xtask features sync-docs && git diff --exit-code`
- [ ] Gating tests pass: `cargo test -p perl-parser --test lsp_feature_gating_test`

### Optional but Recommended
- [ ] Feature matrix: `cargo hack check -p perl-parser --feature-powerset --ignore-private`
- [ ] Stress tests: `PERL_LSP_STRESS_ITERS=100 cargo test -p perl-parser -- --ignored --test-threads=1`
- [ ] JSON export: `./target/debug/perl-lsp --features-json | jq '.advertised | length'`

## Additional Notes

Any additional information that reviewers should know.