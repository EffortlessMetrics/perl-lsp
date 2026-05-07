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
    shared-key: <stable-key>
    save-if: ${{ github.ref == 'refs/heads/master' || github.ref == 'refs/heads/main' }}
```

The example uses a stable `shared-key` without `${{ hashFiles('Cargo.lock') }}` because
`Swatinem/rust-cache` already incorporates the lockfile hash into its internal keying;
adding it to `shared-key` prevents restore-fallback when `Cargo.lock` changes (the action
uses `shared-key` as a restore prefix). Existing workflows that include the hash for
historical reasons are not changed by this rollout.

Effects:

- **PR runs:** restore cache, run, **do not save**.
- **Master pushes:** restore cache, run, save canonical cache for the next PR's restore.
- **Matrix jobs:** keyed by matrix variant via `shared-key`; saving still gated on master.

---

## What this does not change

- Concurrency (`concurrency.cancel-in-progress`) for PR workflows is preserved.
- Release/deploy workflows are not modified by this policy — they are infrequent and
  need their own cache lifecycle.
- Nightly workflows that already have their own scheduling are unaffected.

---

## Scope

This policy applies to every `Swatinem/rust-cache` invocation in any workflow that runs
on `pull_request`. The exhaustive list is not maintained here to avoid drift; verify by
grepping `Swatinem/rust-cache` against `pull_request`-triggered workflows under
`.github/workflows/`.

---

## Verification

After this policy lands, the first master push saves the canonical cache. PRs from then
on restore-only. Expected impact:

- PR run wall time: ≈ unchanged (restore time is comparable).
- Cache write traffic: drops to one save per master push instead of one per PR push.
- Cache eviction churn: substantially reduced.

LEM impact appears in `target/ci/ci-actuals.json` once the CI actuals receipt is wired up.
