# Spec Update Checklist

Every product PR must be able to answer these questions before the builder publishes it.
Answer them in the PR body or a PR comment — failing to answer any item is a reviewable finding.

---

## 1. Spec / contract alignment

- Which spec, contract, or feature entry does this change touch?
  (e.g. `features.toml` capability, LSP spec section, DAP protocol message, CI required-check name)
- Does the diff implement exactly what the spec describes — no more, no less?

## 2. Test evidence

- Which test(s) prove the change is correct?
  Name the test file and function (e.g. `crates/perl-parser/tests/foo.rs::test_bar_empty`).
- Do those tests exist in the diff, or were they pre-existing?
- If pre-existing, did you verify they still pass against this diff?

## 3. PR body vs diff

- Does the PR body's "what this does" match the actual diff?
  (Read the diff with `git diff origin/main..HEAD` — not just from memory.)
- Are there any claims in the body ("adds X", "fixes Y") that are NOT in the diff? Remove or correct them.

## 4. Exceptions added or retired

- Does this change add a new `#[allow(...)]` annotation, `LCOV_EXCL_LINE`, or other suppression?
  If yes: cite the specific reason and link the upstream issue (e.g. ripr#1429).
- Does this change retire a previously documented exception? Update the exception list if so.

## 5. Agent / workflow behavior

- Does this change affect how an agent operates, what a CI check validates, or how the pipeline routes work?
  If yes: update the relevant agent def in `.claude/agents/` or the workflow file.

## 6. Release / status claims

- Does the PR body or a linked doc claim something about release readiness, version bumps, or status metrics?
  If yes: the claim must cite a receipt (CI run URL, `cargo semver-checks` output, `Cargo.toml` version diff).
  "Looks ready" is not a receipt.

## 7. Docs update

- Does this change invalidate or extend any doc in `docs/`?
  Either update the doc in this PR, or file a follow-up issue and reference it in the PR body.

---

## Quick reference: required CI checks

Branch-protection required checks (three only — everything else is advisory):

| Check name | "Skipping" = satisfied? |
|------------|------------------------|
| `Perl LSP Rust Small Result` | Yes |
| `ripr+ New Gap Gate` | Yes |
| `Codecov / Patch 95` | Yes |

RIPR pin: `RIPR_VERSION=0.5.0` (`.github/workflows/ripr.yml`). Local ripr installs may differ — verify from the `ripr+ New Gap Gate` CI receipt, not local output.

Codecov false-low: patch coverage counts `--lib` profdata only. Integration tests in `tests/` do not count. Fix with inline `#[cfg(test)]` lib tests, not `LCOV_EXCL_*` padding.
