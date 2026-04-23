# Signal Intake Archaeology

## Question

How did the repo evolve from generic issue and PR entry points into a typed
signal-intake system that swarm agents could route and reuse?

## Short Answer

The committed history shows three successive intake models:

1. generic human contributor checklists
2. gate-aware PR intake and typed issue forms
3. swarm-native discovery intake and handoff-oriented issue creation

This matters because the Q3 talk framed the SDLC as `Signal -> Plan -> Build ->
Review -> Gate`. The repo eventually industrialized not only build and review,
but signal intake itself.

## 1. The Earliest PR Intake Surface Is A Human Checklist

The lower-case
[`.github/pull_request_template.md`](../../../.github/pull_request_template.md)
first appears on `2025-07-17` in `63c9f4245`, then changes again on
`2025-08-25` in `0c969dcd3`.

Its structure is familiar and contributor-centric:

- description
- related issue
- type of change
- testing checkboxes
- checklist items

This is not yet swarm-native. It is a conventional PR form aimed at a human
author proving they did the expected hygiene work.

## 2. January 2026 Makes PR Intake More Gate-Aware

The upper-case
[`.github/PULL_REQUEST_TEMPLATE.md`](../../../.github/PULL_REQUEST_TEMPLATE.md)
lands on `2026-01-07` in `982727ac8`,
`ci: add PR template with local gate requirement and label guidance (#274)`.

That template is already more repo-specific than the earlier one:

- it foregrounds verification
- it calls out local gate expectations
- it carries explicit label guidance

This is an important bridge stage. Intake is no longer just "tell us what you
changed." It is starting to ask for proof and routing hints at the moment the
change enters review.

## 3. February 2026 Types Issue Intake Explicitly

The strongest single intake-normalization marker is
[`.github/ISSUE_TEMPLATE/config.yml`](../../../.github/ISSUE_TEMPLATE/config.yml),
introduced in `7f5b5290d` on `2026-02-20`.

It sets `blank_issues_enabled: false`.

That is a stronger claim than any prose guideline because it means the repo
preferred structured issue intake over freeform backlog text.

The same issue-form set introduces typed forms such as:

- [bug_report.yml](../../../.github/ISSUE_TEMPLATE/bug_report.yml)
- [parser_bug.yml](../../../.github/ISSUE_TEMPLATE/parser_bug.yml)
- [performance_issue.yml](../../../.github/ISSUE_TEMPLATE/performance_issue.yml)

Those forms collect different kinds of evidence:

- component, reproduction, expected/actual behavior, logs, editor, and OS for
  bugs
- minimal Perl snippet, issue type, expected/actual parse result, and Perl
  version for parser bugs
- expected vs actual performance, reproduction, workspace size, and profiling
  data for performance issues

That is typed intake by problem class, not one undifferentiated issue funnel.

## 4. March 15, 2026 Adds Swarm-Native Discovery Intake

The current swarm-native issue form is
[`.github/ISSUE_TEMPLATE/swarm_discovered.yml`](../../../.github/ISSUE_TEMPLATE/swarm_discovered.yml),
introduced in `9cc2d3b9a` on `2026-03-15`.

Its fields are not generic bug-report fields. They are handoff fields:

- discovering agent
- context
- relevant files with line numbers
- suggested approach
- category

The most revealing line is the context description: include enough detail that
a fresh agent can act without re-investigating.

That is not just intake normalization. It is a direct design for delegated,
resumable swarm work.

The same commit also rewrites
[`.github/PULL_REQUEST_TEMPLATE.md`](../../../.github/PULL_REQUEST_TEMPLATE.md)
into a more swarm-shaped review entry:

- `Summary`
- `Changes`
- `Verification`
- `Agent`

The `Agent` section is especially important because it asks for `agent type,
branch, handoff path`. That is change intake as control-plane metadata.

## 5. The GitHub Ledger Shows These Surfaces In Use

The archaeology is stronger than "the templates existed." The issue and PR
ledger shows the repo using these typed shapes in practice.

### Swarm-discovered issues become a real intake lane

In the GitHub issue ledger snapshot queried on `2026-03-19`:

- `347` total issues exist
- `189` carry the `swarm-discovered` label

That is much stronger than a handful of examples. It means the label is not
decorative metadata. It is one of the dominant intake lanes in the issue
archive.

The March 16 to March 19 wave contains multiple issue families with the
`swarm-discovered` label:

- parser bucket issues such as `#2184`, `#2186`, `#2188`, `#2189`
- learning issues such as `#2190`, `#2191`, `#2192`
- article issues such as `#2193` through `#2197`
- improvement issues such as `#2213` through `#2218`

Those are not one class of backlog item. They are a shared discovery lane for
implementation slices, learning reports, article generation, and follow-on
quality work.

### Bodies look handoff-oriented, not conversational

The bodies also show a clear adoption asymmetry between structured markdown and
literal form-field wording.

In the same `2026-03-19` issue snapshot:

- `87` issue bodies contain a `## Problem` heading
- `15` contain a `## Context` heading
- `9` contain `Suggested Approach`
- `0` contain literal `Discovering Agent`
- `0` contain literal `Relevant Files`

That is useful evidence. The repo clearly adopted structured issue bodies, but
it did not preserve the raw GitHub-form labels very often in the final text.

Issue `#2188` ("parser: fix unexpected_arrow_expr") carries a structured
builder-oriented body:

- bucket
- problem
- root cause
- impact
- related PR
- test template
- success criteria

Issue `#2190` ("learning: parser fix agent experience report") is structured as
a post-build learning artifact:

- context
- what was harder than expected
- parser patterns other builders should know
- worktree friction
- what would have made this easier
- built-but-not-wired patterns

Issue `#2218` ("test: increase coverage for ...") uses a scoped improvement
shape with `Problem`, `Gaps`, and `Acceptance`.

These are typed intake artifacts even when their wording is richer than the raw
GitHub form fields.

### PR bodies also show normalization, with a caveat

By March 15-19, 2026, PR bodies commonly use the new `Summary` section, but the
rest of the template lands unevenly.

In a `200`-PR slice created on or after `2026-03-15`:

- `183` contain `## Summary`
- `6` contain `## Changes`
- `10` contain `## Verification`
- `0` contain `## Agent`
- `0` contain all four canonical headings together

Representative examples include:

- PR `#2230`
- PR `#2229`
- PR `#2221`
- PR `#2171`

But the `## Agent` section is less consistently populated than the template
would suggest. So the strongest claim is not full PR-body conformity. It is
that the PR intake surface was normalized enough for summary-plus-proof to
become common, while swarm metadata remained partially adopted.

## 6. Typed Intake Also Has Pre-Swarm Precursors

The repo was already moving toward structured operational intake before
`swarm_discovered.yml`.

Issue `#1667` ("audit(swarm): cycle 2 improvements & protocol gaps") is a
typed ops audit artifact with enumerated gaps, impacts, and proposed fixes.

Issue `#1678` ("friction: cycle 2 operational friction log — 14 items") is a
typed friction ledger, not a casual discussion thread.

Those issues matter archaeologically because they show the repo already wanted
discoveries and operational pain to be captured as reusable artifacts. March
2026 then formalizes that desire into labeled issue lanes and dedicated forms.

## 7. The Generic Public Issue Forms Leave Much Weaker Visible Artifacts

The repo clearly defines typed public forms for parser bugs, generic bugs, and
performance issues.

But in the same `347`-issue GitHub snapshot queried on `2026-03-19`, there are:

- `0` issue titles preserving the literal `[Parser]` prefix
- `0` issue titles preserving the literal `[Bug]` prefix
- `0` issue titles preserving the literal `[Performance]` prefix

That does not prove those forms were never used. Titles can be edited after
creation, and internal issue creation flows may bypass browser-form entry.

But it does support a narrower and more defensible claim: the swarm-facing
typed intake lane leaves strong on-ledger artifacts, while the public-facing
typed forms leave much weaker visible artifacts in the current archive.

## 8. The Repo Also Self-Audits Intake Drift

The March 19, 2026 issue wave includes an especially revealing control-plane
artifact:

- issue `#2156`
  `swarm-infra: Inconsistent issue/PR templates across scout commands`

That title matters because it shows the repo is not merely using typed intake.
It is also discovering where its own issue and PR creation surfaces are still
inconsistent.

This is one of the more interesting current-era patterns:

1. a control-plane surface lands
2. the swarm starts using it at scale
3. the swarm then opens issues about the remaining inconsistency in that same
   surface

That makes signal intake part of the self-improving loop, not only a static
template choice.

## 9. Strongest Evidence-Backed Claims

1. The repo started with generic human PR checklist intake, not swarm-native
   intake.
2. January 2026 makes PR entry more gate-aware by foregrounding local
   verification and review guidance.
3. February 2026 makes issue intake explicitly typed by disabling blank issues
   and adding problem-specific forms.
4. March 15, 2026 adds a genuinely swarm-native intake surface:
   `swarm_discovered.yml` is designed for resumable handoffs between agents.
5. The GitHub ledger confirms strong adoption of the swarm-facing intake lane:
   `189` of `347` issues in the `2026-03-19` snapshot carry
   `swarm-discovered`.
6. Actual issue-body adoption happens through structured markdown sections like
   `Problem`, `Context`, and `Suggested Approach`, not through literal
   preservation of the raw GitHub-form labels.
7. PR-body conformity is real but partial: in a `200`-PR March 15+ slice,
   `Summary` becomes near-normal, while `Changes`, `Verification`, and
   especially `Agent` remain much less consistently populated.
8. The public-facing typed issue forms leave much weaker visible artifacts in
   the current ledger than the swarm-facing discovery lane.
9. By March 19, 2026, the repo is already opening `swarm-discovered`
   infrastructure issues about inconsistent issue/PR intake surfaces, meaning
   intake standardization is itself part of the self-improvement loop.

## See Also

- [ISSUE_LABEL_ARCHAEOLOGY.md](ISSUE_LABEL_ARCHAEOLOGY.md)
- [ISSUE_ROUTING_ARCHAEOLOGY.md](ISSUE_ROUTING_ARCHAEOLOGY.md)
- [ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md](ISSUE_PR_CROSSLINK_ARCHAEOLOGY.md)
- [Q3_SWARM_TALK_ARCHAEOLOGY.md](Q3_SWARM_TALK_ARCHAEOLOGY.md)
- [RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md](RECEIPT_SURFACE_EVOLUTION_ARCHAEOLOGY.md)
