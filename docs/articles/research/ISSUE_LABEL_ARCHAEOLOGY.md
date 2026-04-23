# Issue Label Archaeology
## How GitHub Issues Became Swarm Routing And Control Memory

This note tracks a narrow but important change in the repository's operating
model: issues stopped behaving like plain backlog entries and started acting as
typed memory nodes for swarm discovery, routing, and follow-up work.

The strongest evidence is not a single label. It is the combination of:

- a large `swarm-discovered` backlog
- smaller `swarm-improve-*` routing lanes
- priority labels that still carry release pressure
- issue-title prefixes such as `learning:`, `article:`, `friction:`, and
  `audit:` that preserve operational context in the ledger itself

All counts and examples in this note were verified from the GitHub issue
archive on `2026-03-19`.

---

## 1. The Repo Already Used Issues As Operational Metadata

The current label catalog is strongly typed. It includes:

- `swarm-discovered`
- `swarm-improve-devex`
- `swarm-improve-infra`
- `swarm-improve-tests`
- `priority:high`
- `priority:critical`
- `priority-high`
- `P1-high`
- `P0-critical`

That is not a generic backlog vocabulary. It is a routing vocabulary.

Verified issue-label counts on `2026-03-19`:

- `swarm-discovered`: `189`
- `swarm-improve-devex`: `13`
- `swarm-improve-infra`: `18`
- `swarm-improve-tests`: `11`
- `swarm-improve-docs`: `0`
- `swarm-architectural`: `0`
- `priority:high`: `29`
- `priority:critical`: `8`
- `priority-high`: `1`
- `P1-high`: `6`
- `P0-critical`: `2`

The important historical detail is that the taxonomy is intentionally uneven.
Some lanes are heavily used, some are reserved, and some exist before the repo
needs them at scale. That is how a control plane looks when it is designed for
future routing rather than only present-day triage.

---

## 2. `swarm-discovered` Turned The Issue Tracker Into Overflow Memory

The biggest signal is the `swarm-discovered` label itself.

By `2026-03-19`, the repo had `189` issues carrying that label. A few examples
show the pattern clearly:

- `#2184` through `#2189` capture parser discoveries while working on CPAN
  corpus failures
- `#2178` routes a keyboard-shortcut compatibility audit into the swarm
- `#2156` records inconsistent issue and PR templates in the scout commands
- `#2132` turns a shortcut-conflict audit into a follow-up task

That label does not mean "important bug." It means "an agent found something
outside the current slice, and the repository wants that discovery preserved."

In other words, the issue queue became a spillover ledger:

- it captured adjacent work that did not fit the current branch
- it preserved discoveries instead of leaving them in chat
- it let later sessions recover the finding without rediscovering it

That is why `swarm-discovered` is a control surface, not a tag.

---

## 3. Priority Labels Still Encode Release Pressure

The older priority taxonomy was not replaced by swarm routing. It still exists
alongside it.

Examples from the archive include:

- `#154` `priority-high` on a parser regression report
- `#213` `priority:high` on a large LSP polish meta-issue
- `#211` `priority:critical` and `P0-critical` on CI pipeline cleanup
- `#446` `priority:high` on node-kind coverage gaps
- `#343` `priority:high` on an LSP execution hang

This matters historically because the repo did not collapse all issue meaning
into swarm labels. Priority labels remained a separate channel for urgency,
release risk, and blocking work.

So the ledger is doing at least two jobs at once:

- `priority:*` says how urgent or blocking the item is
- `swarm-discovered` and `swarm-improve-*` say how the swarm should route it

That separation is one of the reasons the issue tracker works as a durable
control plane rather than as a flat todo list.

---

## 4. `Learning`, `Article`, `Friction`, And `Audit` Are Mostly Title Taxonomy

One nuance is easy to miss: the repo does not currently expose dedicated
`learning`, `article`, `friction`, or `audit` labels in the label catalog.
Those terms show up primarily as issue-title prefixes and issue-class patterns.

That is not a weakness. It is part of the memory model.

Concrete examples:

- `#2190` and `#2191` are `learning:` experience reports for parser-fix agents
- `#2192` is a `learning:` report with `auto-import` in the title
- `#2193` through `#2197` are `article:` issues for launch-story and
  documentation drafts
- `#1678` is a `friction:` log with a numbered list of operational pain points
- `#1667` is an `audit(swarm):` issue that reports protocol gaps and missing
  coverage
- `#1670` is an `audit:` issue that records missing governance skills and
  outdated patterns

Those titles carry more than topic. They preserve the nature of the discovery:

- `learning:` means the issue is a session-derived lesson
- `article:` means the issue is an article draft or narrative slice that should
  survive beyond the current session
- `friction:` means the issue is a run log of real operational pain
- `audit:` means the issue is a bounded inspection with named findings

This is why the issue tracker became control memory. The title itself encodes
what kind of artifact the swarm is preserving.

---

## 5. The Audit And Friction Issues Show The Handoff Mechanism

The `audit` and `friction` issues are especially revealing because they are not
just observations. They are observations with follow-up intent.

Issue `#1678`, `friction: cycle 2 operational friction log`, documents real
pain encountered during cleanup, PR drain, and swarm execution. The body turns
those pain points into concrete fixes, such as handling checkout blockers and
untracked-file interference.

Issue `#1667`, `audit(swarm): cycle 2 improvements & protocol gaps`, reports
multiple protocol gaps, operational gaps, and a skills-coverage gap. It is not
recorded as commentary; it is recorded as a backlog of corrections.

Issue `#1670`, `audit: skill definitions have documentation gaps, missing
governance skills, and outdated patterns`, does the same thing for skill
metadata. It turns a review of 26 skills into a tracked set of corrective work.

That is the historical pattern worth preserving:

- the swarm finds friction or audit findings while working
- the finding is captured as an issue
- the issue gets a typed title and a routing label
- later work can consume the issue as a memory node instead of rediscovering it

This is exactly the behavior you want from a swarm control plane.

---

## 6. Historical Meaning

The issue tracker evolved through three roles:

1. classic backlog storage
2. priority and release-pressure tracking
3. typed swarm routing and persistent memory

The third role is the distinctive one.

By March 2026, the repo is using issues to preserve:

- discoveries that do not fit the current slice
- self-improvement work for the swarm itself
- article and learning artifacts that would otherwise disappear into chat
- audit and friction records that explain why follow-up work exists

That is a stronger model than a normal backlog. It is a durable ledger for
scout findings, operational lessons, and work routing.
