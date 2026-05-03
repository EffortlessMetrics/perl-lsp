# 2026-05-03 — `target/` Gitignore vs. Receipt PRs

**Lens**: `target/` is gitignored, so anything written there cannot be PR'd or shared via git. A wakeup prompt over-engineered around the assumption that release receipts could be committed.

## What I assumed

The release-receipt template (codified now in `docs/reference/RELEASE_PROOF_PROTOCOL.md`) says to write receipts at:

```
target/receipts/release-install-surface-v0.13.3.md
```

The convention from prior releases (`release-install-surface-v0.13.2.md` etc.) lives in the same directory. I assumed that meant the receipts were committed and that "open a PR for the receipt" would work.

## What's actually true

`target/` is gitignored at the repo root:

```
$ grep '/target/' /h/Code/Rust/perl-lsp/.gitignore
/target/
```

Receipts written to `target/receipts/` are **local-only forensic artifacts**. They survive on the operator's machine but they cannot be pushed to GitHub, PR'd, or read by another agent on a different machine.

Verifying with git history:

```bash
$ git log --all --oneline -- 'target/receipts/release-install-surface-*'
# (no commits — confirms the receipts have never been part of the repo)
```

The 0.13.2 release receipt was purely local. The wakeup prompt I wrote that said "git add + commit on a new branch + push + open PR for the receipt" was over-engineered relative to the actual convention.

## What I did

Aborted the receipt-PR plan. Wrote the v0.13.3 receipt to `target/receipts/release-install-surface-v0.13.3.md` as a local artifact only, matching the prior-release convention.

## Implications

Receipts are tier-1 evidence (ephemeral, rich, free-form) per `docs/articles/EVIDENCE_DURABILITY_TIERS.md`. They serve the *current* operator and the *immediate-next-cycle* research partner — not future contributors.

When something in a receipt is durably valuable (a structural lesson, a workflow gotcha, an architectural insight), it gets *promoted* to a forensic note (`docs/forensics/<date>-<topic>.md`, tier 2) or codified as a test (tier 3). Those promotions are durable.

Conflating "receipt" with "doc" is the error. They serve different purposes:

- **Receipt** (`target/receipts/`): operator's view at decision time. Lost on `cargo clean`, on a fresh checkout, or on a different machine. That's fine — the operator already used the information.
- **Forensic note** (`docs/forensics/`): durable record of a lesson worth keeping. Survives forever in git history.

## Detection signal

If you find yourself writing `git add target/...` or `gh pr create` for a receipt, stop. Either the file shouldn't exist (because the convention is local-only) or it should be a forensic note (in which case it lives in `docs/forensics/`, not `target/receipts/`).

## Lessons

1. **Read the convention before writing it.** The prior-release receipts existed locally; that should have been enough signal that they're not committed.
2. **Ephemeral artifacts have value even though they're not durable.** Don't try to elevate every artifact to a permanent doc. Let the tiers do their job.
3. **When a wakeup prompt prescribes an unusual action, sanity-check the precondition.** "Commit and PR a file in `target/`" should have triggered "wait, isn't `target/` gitignored?"

## Related

- Articles: `../articles/EVIDENCE_DURABILITY_TIERS.md` (the three-tier design)
- Reference: `../reference/RELEASE_PROOF_PROTOCOL.md` (receipt template — now explicitly documents the local-only convention)
