# Spec Update Checklist

Every product PR must be able to answer these questions before the builder publishes it.
Answer them in the PR body or a PR comment — failing to answer any item is a reviewable finding.

**Canonical spec structure**: [docs/reference/SPEC_TEMPLATE.md](../reference/SPEC_TEMPLATE.md) defines
the canonical `.spec/<issue#>-<slug>/` layout — checklist.md, acceptance.md (all six required sections:
§Behavior, §Hazards, §Contracts, §API-Shape, §Test-Grid, §Blast-Radius), and context.md.

**Spec-builder workflow**: [`.claude/workflows/spec-builder.js`](../../.claude/workflows/spec-builder.js)
runs six parallel haiku angles to populate acceptance.md §Hazards through §Blast-Radius for non-trivial issues.

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

## 8. Hazard-class invariants

Every product PR's `acceptance.md` **must explicitly address any applicable class** from the list below.
"Applicable" means the diff touches that surface — allocates an ID range, indexes into client-supplied data,
parses byte streams, modifies a coverage gate, etc. One line naming the invariant plus one adversarial test
per applicable class is the minimum.

| Class | Trigger surface | Required invariant + test |
|---|---|---|
| **ID/ref-space collision** | Any newly allocated numeric range (DAP `variablesReference`, scope IDs, frame IDs, etc.) | Enumerate ALL existing ranges in the area; acceptance row must assert the new range is provably disjoint. Add a test that constructs both ID types and asserts they never overlap. Motivating example: #1219 allocated base 50_000 colliding with scope refs (`frame_id*10+scope_type`). |
| **Bounds / overflow** | Indices or IDs that originate from a client or user (array subscript, thread index, depth counter) | All arithmetic is checked or saturating; out-of-range input produces a safe/empty result, never a panic or silent wrap. Test the out-of-range path explicitly. |
| **Protocol-safety** | Any handler for external protocol messages (LSP, DAP, stdin) | Unknown/malformed/empty input returns an honest empty or error response — never crashes the server. Test the invalid path. |
| **Scanner literal/comment blindness** | Any byte- or char-level scanner (delimiter matching, brace counting, LCOV range stripping, regex scanning) | Scanner must skip content inside string literals, char literals, comments, and raw strings. Test each of those four contexts as inputs containing the scanned delimiter. Motivating example: #1327 brace scanner stripped production LCOV lines inside string literals. |
| **Test-encodes-the-bug** | Any PR that modifies an existing test's expected output or assertion value | Confirm the old assertion characterized CORRECT behavior, not an existing defect. Document the reasoning in the PR body or acceptance row. Motivating example: #1337 had a test asserting the defect as expected output. |
| **Coverage / measurement integrity** | Any transform that filters, strips, or annotates source lines for coverage reporting | The transform must never drop or exclude production lines. Add a test that feeds a representative production-code line through the transform and asserts it survives. |

**If the change touches one of these surfaces, `acceptance.md` MUST list the invariant + the adversarial test that proves it.**
Absence of a required invariant row is a reviewable finding at architecture-reviewer (pre-build, cheap haiku pass)
and again at reviewer-deep (post-build confirmation net).

**Subsystem-specific defaults**: For DAP, Parser, LSP, and Coverage/CI changes, consult
[docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md)
for the pre-populated hazard rows that should appear by default in `acceptance.md` for
each subsystem. Each row includes the invariant, trigger condition, and required adversarial
test obligation — copy applicable rows verbatim and fill in the `Surface` field.

---

> **See also**: [docs/learnings/README.md](../learnings/README.md) for repo-specific
> incidents that motivated each hazard class in section 8 above:
> 2026-06-dap-ref-space-collision.md (Class 1 ID collision),
> 2026-06-coverage-gate-measurement.md (Class 4 scanner blindness),
> 2026-06-test-encodes-the-bug.md (Class 5 test-encodes-bug),
> 2026-06-ripr-output-schema-break.md and 2026-06-codecov-false-low.md (Class 6 coverage).
> Portable patterns: [docs/concepts/](../concepts/).

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
