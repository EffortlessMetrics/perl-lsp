# CI Cache Policy

Cache **save** is restricted to master pushes; cache **restore** runs on every PR. This
prevents PR cache write churn from displacing genuinely useful cache entries within
GitHub's 10 GB per-repo limit.

> Companion: [cost-and-verification-policy.md](cost-and-verification-policy.md).

---

## Rule

For every `Swatinem/rust-cache` invocation in a PR-capable workflow:

```yaml
- uses: Swatinem/rust-cache@c19371144df3bb44fab255c43d04cbc2ab54d1c4  # v2.9.1
  with:
    cache-on-failure: true
    cache-all-crates: true
    shared-key: <stable-key>-${{ hashFiles('Cargo.lock') }}
    save-if: ${{ github.ref == 'refs/heads/master' || github.ref == 'refs/heads/main' }}
```

Effects:

- **PR runs:** restore cache, run, **do not save**.
- **Master pushes:** restore cache, run, save canonical cache for the next PR's restore.
- **Matrix jobs:** keyed by matrix variant via `shared-key`; saving still gated on master.

---

## What this does not change

- Concurrency (`concurrency.cancel-in-progress`) for PR workflows is preserved.
- Release/deploy workflows (`release.yml`, `publish-*.yml`) are not modified by this
  policy — they are infrequent and need their own cache lifecycle.
- Nightly workflows that already have their own scheduling are unaffected.

---

## Workflows updated in PR 05

- `.github/workflows/ci.yml` (6 cache blocks: pr-smoke, merge-gate-shards × 2,
  ux-tests, check-all-targets, lsp-memory-smoke, windows-guardrails)
- `.github/workflows/ux-regression-gate.yml`
- `.github/workflows/ci-gate-self-tests.yml`
- `.github/workflows/publish-dry-run.yml`
- `.github/workflows/ci-security.yml`

---

## Verification

After this PR merges, the first master push saves the canonical cache. PRs from then on
restore-only. Expected impact:

- PR run wall time: ≈ unchanged (restore time is comparable).
- Cache write traffic: drops to one save per master push instead of one per PR push.
- Cache eviction churn: substantially reduced.

LEM impact appears in `target/ci/ci-actuals.json` after PR 08 lands.
