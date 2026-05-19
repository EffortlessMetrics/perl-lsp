# PLSP-SPEC-0001: Parser compatibility bucket closeout

Status: proposed
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: planned `PLSP-ADR-0001`
Linked plan: planned `plans/real-perl-editor-trust/implementation-plan.md`
Status impact: parser accuracy next, parser status, parser receipts

## Contract

Generated parser status routes parser work.

When [parser accuracy next](../project/status/parser_accuracy_next.md) has active
failure packets, agents must address those measurement failures before starting
capability work.

When [parser accuracy next](../project/status/parser_accuracy_next.md) has no
active failure packets and no measurement gaps, agents must follow
[parser raw failure buckets](../project/status/parser.md#raw-failure-buckets)
for capability work only when generated parser status lists a nonzero raw
bucket. The next parser capability lane is the largest raw bucket inside the
largest nonzero parser failure cluster unless the implementation plan
explicitly parks it with a reason. If generated status lists `none`, agents
must not start bucket work from stale context; they must refresh the corpus
receipt, identify a current failing source-backed fixture, or move to the next
provider or real-workspace trust lane.

Generated sections are not hand-edited. Status changes must come from xtask
generators and be covered by the relevant parser status checks.

## Freshness Rule

Raw bucket counts are point-in-time compatibility data. They can drive
discovery, but they do not prove current bucket movement unless refreshed.

| Receipt state | Allowed use | Claim boundary |
|---|---|---|
| Fresh corpus receipt | Route parser-fix lanes and update bucket counts | May claim bucket count movement when generated status updates prove it |
| Stale corpus receipt | Discover source-backed fixture shapes when current generated status still lists a nonzero bucket or a current fixture fails | Must not claim bucket count movement |
| Fixture-only PR | Lock one source-backed real-Perl shape | Must not claim corpus improvement or bucket reduction |
| Parser-fix PR | Change parser behavior for one narrow failure shape | May claim behavior fix only for covered fixtures until corpus receipt refresh |

Before starting a bucket-count closeout claim, refresh the Linux corpus receipt
with the corpus sweep command when the required roots are available. When the
roots are not available, continue with fixture extraction only when current
generated status lists a nonzero bucket or a current source-backed fixture
fails against the parser. State that fresh corpus movement is deferred.

## Bucket Lane Shape

Each raw bucket lane must stay PR-sized.

1. Verify current status pointers in `parser_accuracy_next.md` and `parser.md`.
2. Check receipt freshness in `parser.md#raw-failure-buckets`.
3. Stop if generated status lists `none` and no current source-backed fixture
   fails against the parser.
4. Refresh the Linux corpus receipt when available.
5. If the receipt cannot be refreshed, extract one focused source-backed
   fixture from current failing evidence.
6. Add no parser runtime change unless the new fixture fails.
7. Keep fixture-only PRs separate from parser runtime fixes.
8. Run focused parser tests and parser status checks.
9. Regenerate generated status only through xtask commands.
10. State the claim boundary in the PR body.

## Current Example

Historical example bucket routing is owned by
[parser raw failure buckets](../project/status/parser.md#raw-failure-buckets).
At spec creation time, the largest listed raw bucket is
`unclosed_paren_identifier` under the `heredoc / delimiter handling` cluster.
This example is illustrative; agents must re-read generated status before
starting work and must not use this stale bucket name when generated status
lists `none`.

Valid PR shapes:

- `test(parser): lock Unicode::Collate map fixture`
- `test(parser): lock Regexp::Common map fixture`
- `test(parser): lock ExtUtils map-list fixture`
- `test(parser): lock Carp local caller fixture`

Invalid PR shapes:

- broad parser rewrite because a bucket exists
- bucket-count reduction claim without a fresh generated corpus receipt
- fixture-only coverage mixed with parser runtime behavior change
- hand-editing generated parser status
- combining measurement wiring, corpus refresh, and parser behavior change in
  one PR

## Acceptance

A parser bucket PR satisfies this spec when:

- the PR names the source status pointer it follows
- the PR starts from a current nonzero raw bucket or a current failing
  source-backed fixture
- the PR states whether the corpus receipt is fresh or stale
- fixture-only PRs add one focused source-backed fixture and no parser runtime
  behavior change
- parser-fix PRs include a failing fixture or receipt-backed failure shape
  before changing parser behavior
- generated status updates, when present, come from xtask output
- the PR body states what can and cannot be claimed after the change
- proof commands pass or any unavailable receipt is explicitly deferred

## Proof Commands

Focused fixture or parser-fix proof:

```bash
cargo test -p perl-parser-core --test <bucket-test> --profile agent --locked -- --nocapture
```

Parser status proof:

```bash
cargo xtask metrics parser-accuracy --check
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
cargo xtask fmt --check
git diff --check
```

Fresh corpus receipt proof when Linux roots are available:

```bash
cargo xtask parser-corpus-sweep --baseline .ci/parser-corpus-baseline.json --enforce --receipt
cargo xtask update-status --only parser --check
```

## Non-goals

- no full CPAN-clean claim
- no broad parser rewrite
- no provider confidence or cutover behavior
- no real-workspace baseline requirements
- no generated status hand edits
- no claim that stale raw buckets prove current failures

## Claim Boundaries

Fixture-only PRs may claim that `perl-lsp` locks a source-backed real-Perl
shape. They may not claim that corpus compatibility improved or that the raw
bucket shrank.

Parser-fix PRs may claim the fixed behavior covered by their fixtures and
targeted tests. They may not claim broader corpus movement until generated
status from a fresh receipt proves it.

Corpus-refresh PRs may update generated status and bucket counts. They should
avoid unrelated parser runtime changes so reviewers can separate measurement
movement from behavior changes.
