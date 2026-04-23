# Control Plane Self-Repair Archaeology

## Question

When did the repo start treating the swarm operating system itself as an active
improvement target rather than as fixed scaffolding around product work?

## Short Answer

The strongest committed evidence clusters around `2026-03-16` to `2026-03-19`.

In that window, the repo does not just use the swarm. It repeatedly audits,
criticizes, routes, and patches the swarm's own skills, hooks, agent roster,
verification procedures, and templates.

That is a distinct operating-model shift:

- product issues remain product work
- swarm-infra issues become operating-system work
- follow-up PRs modify the control plane in response

This is the clearest evidence that the development method itself had become a
maintained subsystem inside the repository.

## 1. Cycle 2 Already Produces Typed Self-Critique

Two March 16 issues show the self-repair loop clearly:

- issue `#1667`
  `audit(swarm): cycle 2 improvements & protocol gaps`
- issue `#1678`
  `friction: cycle 2 operational friction log — 14 items`

These are not ordinary bug reports.

They are structured operating-system diagnostics:

- protocol gaps
- CI discipline gaps
- scout deliverable problems
- handoff boundary problems
- modularization guidance
- worktree friction
- stale branch tracking
- missing dashboarding
- cleanup automation gaps

That matters because the swarm is already being treated as something that can
have operational debt and design defects separate from product bugs.

## 2. The Immediate Repairs Land As Control-Plane PRs

The repo then turns those findings into direct control-plane changes.

Representative PRs from March 16:

- PR `#1698`
  `fix(skills): remove orchestrator-level skill invocations from /swarm`
- PR `#1707`
  `feat(skills): add invocation-control frontmatter to all skill definitions`
- PR `#1723`
  `refactor(swarm): canonicalize agent roster and hook surfaces`

These are not feature PRs for the Perl parser, LSP, or DAP. They are
architecture repairs to the swarm itself.

Their bodies are revealing:

- `#1698` removes the wrong skill-loading behavior from the orchestrator layer
- `#1707` formalizes which skills are orchestrator-only, agent-only, or dual-use
- `#1723` makes the runtime roster canonical and clarifies which hooks the repo
  does and does not own

The operating system is being debugged in public, in the same repo, through the
same PR machinery as product work.

## 3. March 19 Shows A Second Self-Repair Wave

By March 19, the repo has a much more explicit swarm-infrastructure issue lane.

Representative issues include:

- `#2116`
  `chore(swarm): encode cycle 5 learnings into skills and agent templates`
- `#2151`
  `swarm-infra: Missing /scout-report and /pr-create skills`
- `#2153`
  `swarm-infra: swarm-metrics.jsonl data not aggregated into dashboards`
- `#2154`
  `swarm-infra: Hook enforcement incomplete — verify-build and other skills can't gate completion`
- `#2156`
  `swarm-infra: Inconsistent issue/PR templates across scout commands`
- `#2157`
  `swarm-infra: Worktree cleanup automation incomplete — no smart reuse or lifecycle tracking`
- `#2158`
  `swarm-infra: Skill frontmatter controls underused — only 2/8 skills define allowed contexts`
- `#2159`
  `swarm-infra: .ops-perl-lsp/receipts/ directory missing — /verify-build has nowhere to write`
- `#2161`
  `swarm-infra: Skill descriptions missing context on orchestrator vs. agent invocation`
- `#2162`
  `swarm-infra: Create composite skills for common agent workflows`

This is especially notable because the swarm-discovered lane is no longer only
finding product work. It is generating backlog for improving the swarm's own
operating surfaces.

## 4. Verification And Reporting Procedures Also Get Repaired

One of the clearest examples is PR `#1920`,
`fix(swarm): add status update step to verify-build skill`.

That PR does not widen product capability. It changes the finish-line ritual:

- add `just status-update`
- add `just status-check`
- prevent policy/status drift from making the verification story stale

This is the operating system learning from execution pain and then rewriting
its own verification skill to account for that pain.

That is exactly the kind of change that distinguishes a maintained control
plane from a static prompt pack.

## 5. Learning Artifacts Feed The Next Operating-System Pass

The loop is not only issue -> PR. It is broader:

- friction issues such as `#1678`
- audit issues such as `#1667`
- learning reports such as `#2190`, `#2191`, and `#2192`
- article issue `#2197`
  `The Self-Improving Swarm — How Our Development System Learns From Every Session`

That means the repo is preserving multiple kinds of self-repair evidence:

- immediate pain
- design diagnosis
- specific builder lessons
- narrative synthesis

The swarm is not only being improved. It is being analyzed as a system.

## 6. What Makes This Distinctive

Many repos have meta-docs about process. This repo goes further:

1. it files operating-system problems as issues
2. it labels and routes them through the same governance machinery as product work
3. it fixes them with PRs against skills, hooks, templates, commands, and agent packs
4. it preserves the lessons in logs, findings, issues, and archaeology notes

That is why "self-improving swarm" is not just branding language. The GitHub
ledger shows a real maintenance loop pointed at the control plane itself.

## 7. Strongest Evidence-Backed Claims

1. By March 16, 2026, the repo is already emitting typed self-critique about
   swarm process, not only product bugs.
2. The follow-up PRs from March 16 repair the swarm's actual operating surfaces:
   skill scope, skill invocation policy, agent roster, and hook ownership.
3. March 19 broadens this into a dedicated swarm-infra discovery wave, proving
   the control plane itself had become a normal backlog target.
4. Verification procedures are part of that self-repair loop, not an external
   concern, as shown by `#1920`.
5. The repo preserves the feedback loop across issues, PRs, learning reports,
   and article/story artifacts, which makes the swarm auditable as a changing
   system.

## See Also

- [LEARNING_LOOP_ARCHAEOLOGY.md](LEARNING_LOOP_ARCHAEOLOGY.md)
- [SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md](SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md)
- [OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md](OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md)
- [HOOK_CONTROL_ARCHAEOLOGY.md](HOOK_CONTROL_ARCHAEOLOGY.md)
- [SIGNAL_INTAKE_ARCHAEOLOGY.md](SIGNAL_INTAKE_ARCHAEOLOGY.md)
