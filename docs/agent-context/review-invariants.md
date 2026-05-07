# Review Invariants

These invariants apply to all reviews in perl-lsp, whether human or Droid.

## Correctness Rules

1. **No silent failures.** All error paths must be audited. `?` operator is preferred; avoid bare `.unwrap()` outside tests and static initializers.

2. **Async safety.** Cannot hold a lock across `.await`. Clippy lint `await_holding_lock` is denied.

3. **Type safety.** Generic bounds must be sufficient. If adding a bound seems burdensome, the type design likely needs rethinking.

4. **Module boundaries.** Public APIs from leaf crates should not expose internal implementation details. Use `#[non_exhaustive]` on public enums and structs.

5. **Cross-file consistency.** Similar operations in different files should use the same patterns. No "special cases" unless there's a comment explaining why.

## Performance Rules

1. **Regex allocation.** Declare all regex patterns as `static LazyLock<Regex>` — never compile per invocation.

2. **Unnecessary clones.** Copy types should not be cloned. Avoid `.clone()` on primitives and small structs.

3. **Iterator allocation.** Prefer lazy iterators over collecting into Vec when possible.

4. **Path operations.** Cache `PathBuf` results when used multiple times in a scope.

## Testing Rules

1. **Test names describe behavior.** Test names should be `test_<behavior>_when_<condition>`, not `test_1`, `test_2`, etc.

2. **No test orphans.** Every test should belong to a logical group. Use module nesting when appropriate.

3. **Property-based tests.** Use quickcheck/proptest for input validation and parsing invariants.

4. **Snapshot tests.** Parser test corpus uses `insta` snapshots. Update snapshots only when the change is intentional.

## Documentation Rules

1. **No doctests that ignore errors.** Tests in doc comments must be runnable as written.

2. **One-line comments only.** Implementation comments should explain the WHY when non-obvious. Multi-line blocks indicate over-engineering.

3. **No hardcoded metrics.** Documentation should link to truth sources, not copy numbers from README.

4. **Function scope.** Public function documentation should explain invariants and failure modes, not just restate the signature.

## Scope Rules

1. **One concern per PR.** One fix, one feature, one refactor. No "while I'm here" cleanups.

2. **No dead code.** If removing something seems necessary, remove it. Do not comment it out or leave `// unused: ...` markers.

3. **No feature flags.** If the old behavior is not needed, delete it. Backwards-compatibility shims make code harder to maintain.

4. **Bundle related tests.** If a feature needs 10 test cases, keep them together. Do not scatter them across multiple PRs.

## Naming Rules

1. **Verb-based functions.** Functions that perform actions should start with a verb: `parse_`, `collect_`, `validate_`.

2. **Adjective-based booleans.** Boolean properties should be `is_<property>`, `has_<property>`, not `check_` or `get_`.

3. **Collection naming.** Plural for collections: `errors`, `ranges`, `tokens`. Singular for the iterator or single item.

4. **Private is underscore-prefixed.** Internal functions and fields use `_` prefix only when truly internal. Otherwise, just use `pub(crate)`.
