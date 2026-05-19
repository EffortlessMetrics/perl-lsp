# PLSP-SPEC-0004: Corpus receipt freshness

Status: proposed
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0001](PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md)
Linked ADRs: planned `PLSP-ADR-0001`
Linked plan: planned `plans/real-perl-editor-trust/implementation-plan.md`
Status impact: parser status, parser raw buckets, support claims

## Contract

Corpus receipts are point-in-time evidence. Raw parser bucket counts may route
work only within the freshness boundary recorded by generated parser status.

The generated [parser status](../project/status/parser.md) owns the current
receipt snapshot, raw bucket rows, and any freshness note. Agents and PRs must
not treat stale corpus data as current proof of bucket movement.

Before a parser lane claims bucket-count movement, it must either:

- refresh the relevant Linux corpus receipt and regenerate parser status, or
- avoid the bucket-count claim and limit the PR to fixture discovery or a
  narrow fixture-backed parser behavior claim.

Generated status sections remain owned by xtask commands. This spec describes
how to interpret corpus receipts; it does not authorize hand-editing generated
parser status.

## Receipt States

| Receipt state | Definition | Allowed use | Claim boundary |
|---|---|---|---|
| Fresh receipt | The corpus sweep was rerun for the PR or current lane and parser status was regenerated from that receipt | Route bucket closeout, update counts, close cluster rows when counts prove it | May claim measured bucket movement for the refreshed corpus |
| Stale receipt | The corpus sweep predates current parser work or was generated on an older commit | Discover source-backed fixture shapes when current generated status still lists a nonzero bucket or a current fixture fails | Must not claim current bucket movement |
| Fixture-only PR | The PR adds focused source-backed coverage without changing parser runtime behavior | Lock a real-Perl shape and prevent regression | Must not claim corpus improvement or bucket reduction |
| Parser-fix PR without fresh corpus | The PR changes parser behavior for a narrow fixture-backed failure but cannot refresh the corpus | Claim the fixture-backed behavior fix | Must not claim broad corpus movement |
| Refreshed corpus PR | The PR reruns the corpus receipt and updates generated parser status only through tooling | Update raw bucket counts and close rows when the generated output proves it | Must avoid unrelated parser/runtime changes |

## Lane Rules

When [parser accuracy next](../project/status/parser_accuracy_next.md) has no
active failure packets and points to raw buckets, agents must first confirm
that generated parser status lists a nonzero raw bucket or that a current
source-backed fixture fails against the parser. If generated status lists
`none` and there is no current failing fixture, agents must not start
raw-bucket work from stale context.

1. Read the receipt snapshot and freshness note in
   [parser raw failure buckets](../project/status/parser.md#raw-failure-buckets).
2. Decide whether the next PR is a corpus-refresh PR, fixture-only PR, or
   narrow parser-fix PR.
3. State the receipt state in the PR body.
4. Keep fixture-only work separate from parser runtime changes.
5. Run targeted parser checks and parser status checks appropriate to the PR.
6. Avoid bucket-count language unless the PR includes a fresh generated corpus
   receipt.

If the required Linux roots are unavailable, the PR may continue as
fixture-only discovery only from current failing evidence. The PR must say that
Linux receipt refresh was deferred and that bucket-count movement remains
unproven.

## Valid Claims

Fixture-only PRs may say:

```text
Locks a source-backed parser shape from current failing raw-bucket evidence.
Does not claim raw bucket reduction without a refreshed Linux corpus receipt.
```

Parser-fix PRs without a fresh corpus receipt may say:

```text
Fixes the covered fixture shape and passes targeted parser checks.
Corpus movement remains unclaimed until the next refreshed receipt.
```

Refreshed corpus PRs may say:

```text
Refreshes the parser corpus receipt and updates generated parser status.
Bucket-count changes are limited to the regenerated receipt.
```

## Invalid Claims

The following claims are invalid without a fresh generated corpus receipt:

- raw bucket count reduced
- cluster closed
- system Perl compatibility improved
- support tier promoted because of corpus movement
- stale bucket disappeared
- fixture-only coverage proves current corpus cleanliness

## Acceptance

A parser PR satisfies this spec when:

- the PR identifies the corpus receipt state
- stale receipt work is limited to fixture discovery or fixture-backed behavior
  proof
- bucket-count claims appear only in refreshed corpus PRs
- generated parser status edits, when present, come from xtask output
- unavailable Linux receipt refresh is explicitly deferred
- the PR body states allowed claims and unproven claims

## Proof Commands

Fresh corpus receipt proof when Linux roots are available:

```bash
cargo xtask parser-corpus-sweep --baseline .ci/parser-corpus-baseline.json --enforce --receipt
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
git diff --check
```

Fixture-only or narrow parser-fix proof:

```bash
cargo test -p perl-parser-core --test <bucket-test> --profile agent --locked -- --nocapture
cargo xtask metrics parser-accuracy --check
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
cargo xtask fmt --check
git diff --check
```

## Non-goals

- no new corpus sweep implementation
- no generated parser status hand edits
- no parser runtime behavior change in freshness-only PRs
- no public full-CPAN or all-system-Perl compatibility claim
- no replacement for parser bucket closeout rules in `PLSP-SPEC-0001`
- no provider confidence or real-workspace baseline requirements

## Claim Boundaries

Stale corpus receipts may route work only when current generated parser status
still lists a nonzero bucket or the PR identifies a current source-backed
fixture failure. They are not proof of the current parser state.

Fixture-only PRs preserve discovered real-Perl shapes. They do not prove that a
raw bucket shrank.

Fresh corpus-refresh PRs may update bucket counts only for the refreshed corpus
and commit. They should stay measurement-only so reviewers can trust the count
movement independently from parser behavior changes.

## Status Links

Canonical generated status:

- [Parser accuracy next](../project/status/parser_accuracy_next.md)
- [Parser raw failure buckets](../project/status/parser.md#raw-failure-buckets)

Related specs:

- [Parser compatibility bucket closeout](PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md)
