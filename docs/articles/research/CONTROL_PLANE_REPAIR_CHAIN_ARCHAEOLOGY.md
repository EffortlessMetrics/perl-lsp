# Control Plane Repair Chain Archaeology

## Question

When the swarm discovered flaws in its own operating system, did those findings
actually turn into concrete repair PRs, or were they just recorded as process
notes?

## Short Answer

The repo shows both patterns, and the distinction is historically useful.

- some swarm-infra findings turn into immediate control-plane PRs in the same
  cycle
- others become explicitly banked operating-system debt for later work

That means the repo was not only good at discovering its own process flaws. It
was also willing to leave a visible debt ledger when repair throughput lagged
behind discovery throughput.

## 1. March 16 Already Shows Clean Repair Chains

The clearest early examples come from the cycle-2 self-audit issues:

- issue `#1667`
  `audit(swarm): cycle 2 improvements & protocol gaps`
- issue `#1678`
  `friction: cycle 2 operational friction log — 14 items`

These are not vague retrospectives. They name concrete missing surfaces and
workflow pain, and several of those items map cleanly into nearby PRs.

### Friction -> dashboard and cycle-transition

Issue `#1678` identifies two missing operator surfaces:

- no agent status dashboard
- no end-of-cycle skill

PR `#1687`,
`feat(skills): add /cycle-transition and /agent-dashboard skills`,
is a direct repair response to exactly those gaps.

Its body says:

- `/cycle-transition` automates the cycle boundary that previously required
  `10+` minutes of manual orchestration
- `/agent-dashboard` scans active agent worktrees and associated PRs to display
  progress

That is a clean problem-to-surface repair chain.

### Friction -> worktree cleanup

Issue `#1678` also identifies worktree accumulation as a recurring operational
cost.

PR `#1689`,
`feat(janitor): add mid-cycle worktree cleanup for completed agents`,
is another clean response. It adds:

- `scripts/cleanup-completed-worktrees.sh`
- `/cleanup-worktrees`
- janitor cadence in `/swarm-protocol`

Again, the operating system is not only being criticized. It is being patched.

## 2. Audit Findings Can Route Through Partial PRs Before Landing Properly

One of the more revealing chains is the skill-scope correction on March 16.

Issue `#1667` criticizes two relevant gaps:

- orchestrator-vs-agent skill confusion
- metrics-compliance failure

PR `#1698`,
`fix(skills): remove orchestrator-level skill invocations from /swarm`,
addresses part of that problem.

But the PR comment ledger then captures a distinctly maintainer-shaped
intervention. In a comment on `2026-03-16`, `EffortlessSteven` says:

- "Superseded by #1699 which includes this fix plus task tool integration and
  metrics mandate additions to all teammate prompts."

That makes PR `#1699`,
`fix(skills): separate orchestrator vs agent skill scopes in /swarm`,
the stronger final repair for that chain.

This is a useful historical pattern:

1. an audit surfaces a control-plane flaw
2. a partial repair PR appears
3. the maintainer curates the shape of the fix
4. a broader follow-up PR becomes the preferred landing surface

That is more than ordinary review. It is operating-system curation.

## 3. Some Audit Themes Fan Out Into Multiple Repairs

Issue `#1667` is not answered by only one PR.

Two nearby PRs respond to different parts of the same critique family:

- `#1699` clarifies orchestrator-vs-agent skill scope and adds metrics mandate
- `#1707` adds invocation-control frontmatter to skill definitions and lands
  ADR-0032 plus the skill/agent design reference

That means one audit issue can split into:

- immediate procedural fix
- broader architectural normalization

The repo is therefore not using issues only as one-to-one fix tickets. It also
uses them as parent diagnoses that can generate multiple control-plane changes.

## 4. March 19 Preserves A Different Pattern: Explicitly Banked Debt

The March 19 `swarm-infra` wave behaves differently.

Issues such as:

- `#2151` missing `/scout-report` and `/pr-create` skills
- `#2154` incomplete hook enforcement
- `#2156` inconsistent issue/PR templates across scout commands
- `#2157` incomplete worktree cleanup automation
- `#2158` underused skill frontmatter controls
- `#2159` missing receipts directory for `/verify-build`
- `#2161` unclear skill invocation context
- `#2162` missing composite skills

do not show the same clean immediate PR closure pattern in the current ledger.

That does not make them weaker evidence. It shows a different control-plane
state:

- discovery throughput has outpaced repair throughput
- the repo now records operating-system debt explicitly instead of letting it
  disappear into chat

This is one of the most distinctive current-era behaviors. The swarm discovers
more infrastructure work than the current cycle necessarily closes.

## 5. Some Later Chains Are Clean Again

The repo also shows clean repair chains within the March 19 wave.

Issue `#2116`,
`chore(swarm): encode cycle 5 learnings into skills and agent templates`,
is explicitly closed by PR `#2123`:

- PR `#2123` begins with `Closes #2116`
- it updates `.claude/skills/verify-build/SKILL.md`
- it adds agent prompt templates
- it adds cycle-5 pitfalls to `known-pitfalls.md`

That is an especially good example because it turns execution learnings into:

- templates
- verification rules
- durable pitfalls

The learning system and the repair system are tightly coupled here.

## 6. What Makes These Chains Historically Interesting

The repo is not merely self-documenting. It is self-routing.

Its control-plane repair chains show at least three distinct modes:

1. **direct repair**
   issue `#1678` -> PR `#1687` / `#1689`
2. **partial repair plus maintainer supersession**
   issue `#1667` -> PR `#1698` -> maintainer supersedes to `#1699`
3. **banked debt**
   March 19 `swarm-infra` issues remain explicit future work rather than being
   silently dropped

That is a more mature pattern than either of the simpler alternatives:

- pretending every issue gets closed immediately
- or letting process problems remain undocumented

## 7. Strongest Evidence-Backed Claims

1. March 16 self-audit issues produce real same-cycle control-plane repairs,
   not just retrospective notes.
2. Issue `#1678` maps cleanly to PR `#1687` and PR `#1689`, covering dashboard,
   cycle-transition, and worktree-cleanup surfaces.
3. Issue `#1667` maps less cleanly but more interestingly: it generates a
   partial repair in `#1698`, then a maintainer-curated supersession to `#1699`,
   with additional normalization in `#1707`.
4. The March 19 `swarm-infra` wave mostly functions as explicit banked
   operating-system debt rather than immediate closure, which is itself an
   important operating-model fact.
5. Issue `#2116` -> PR `#2123` is a clean later example of learnings being
   turned into templates, verification rules, and durable pitfalls.

## See Also

- [CONTROL_PLANE_SELF_REPAIR_ARCHAEOLOGY.md](CONTROL_PLANE_SELF_REPAIR_ARCHAEOLOGY.md)
- [LEARNING_LOOP_ARCHAEOLOGY.md](LEARNING_LOOP_ARCHAEOLOGY.md)
- [SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md](SWARM_MEMORY_TAXONOMY_ARCHAEOLOGY.md)
- [MAINTAINER_VISION_ARCHAEOLOGY.md](MAINTAINER_VISION_ARCHAEOLOGY.md)
- [HOOK_CONTROL_ARCHAEOLOGY.md](HOOK_CONTROL_ARCHAEOLOGY.md)
