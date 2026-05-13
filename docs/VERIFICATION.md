# Verification

`perl-lsp` has three verification surfaces:

- README badges are public, repo-scoped trust markers.
- Pull request evidence is diff-scoped reviewer and agent feedback.
- Release evidence is shipped-truth proof for public version handoff.

Badges are the front panel. Generated evidence, CI receipts, and release artifacts remain the source of truth.

## README badges

### `ripr+`

`ripr+` is a repo-scoped static evidence badge. It counts unresolved static exposure gaps plus actionable test-efficiency findings under repository policy when `ripr` can produce the repo-scoped endpoint.

It is an inbox-zero signal, not coverage, runtime mutation proof, or correctness proof. Diff-scoped `ripr` artifacts belong in pull request summaries and CI artifacts, not public README badges.

If the badge endpoint reports `unavailable`, the endpoint generator could not obtain compatible repo-scoped `ripr+` output from the installed `ripr` binary and leaves the public marker neutral instead of publishing diff-scoped evidence as a repo badge.

### Release

The release badge shows the latest GitHub release. GitHub releases are the public version surface for this repository; crates.io downloads and docs.rs remain registry and documentation surfaces.

## Regeneration

Regenerate public badge endpoints:

```bash
cargo xtask badges
```

Check committed endpoint drift:

```bash
cargo xtask badges --check
```

Committed endpoint files live under `badges/`. Detailed reports stay under `target/` locally or in CI artifacts.

## Pull Request Evidence

Pull requests run advisory `ripr` evidence, impacted evidence, fast gates, docs-sync, publish preflight, example smoke checks, and targeted mutation when routing rules require it.

`ripr` may suggest focused tests or route targeted mutation. It does not edit code, generate tests, run mutation, or make merge decisions by default.

Pull request artifacts and summaries are diff-scoped. They must not be reused as repo-scope README badges.
