# Hybrid Control Plane Archaeology

## Question

How cleanly did the repository migrate from the older `.ops-perl-lsp/`
runtime to the current control plane built from `swarm-state`, skills, hooks,
and commands?

This note uses committed repo sources only. It does not rely on untracked local
runtime state or maintainer recollection.

## Short Answer

The committed repo does not show a hard cutover. It shows a layered migration.

- `swarm-state` becomes the committed durable ledger.
- skills become the preferred reusable procedure surface.
- hooks start enforcing behavior structurally instead of only by prompt.
- but live commands, live agent packs, and even one live hook still keep
  `.ops-perl-lsp` as the runtime location for handoffs, metrics, patches, and
  salvage.

That is why the current control plane is best described as hybrid rather than
fully post-`.ops-perl-lsp`.

## 1. The Split Becomes Explicit On 2026-03-15

The clearest explicit migration marker is commit `9cc2d3b9a` on
`2026-03-15`, `feat(swarm): continuous swarm infrastructure with agent teams
(#1553)`.

Its commit message says the repo split:

- tracked state into `.claude/swarm-state/`
- ephemeral runtime into `.ops-perl-lsp/`

The committed pack docs preserve that same distinction:

- [`docs/handoff/swarm-pack/README.md`](../../../docs/handoff/swarm-pack/README.md)
  lines 104-118
- [`docs/handoff/swarm-pack/SWARM_DESIGN.md`](../../../docs/handoff/swarm-pack/SWARM_DESIGN.md)
  lines 258-282

The strongest claim the repo supports is not "the repo replaced `.ops-perl-lsp`."
It is "the repo explicitly narrowed `.ops-perl-lsp` to runtime duties while
promoting `swarm-state` into committed durable state."

## 2. `swarm-state` Becomes The Durable Ledger, Not The Whole System

The current committed README for `swarm-state` is precise about scope:

- [`.claude/swarm-state/README.md`](../../../.claude/swarm-state/README.md)
  lines 3-13 say these files are committed and survive across sessions,
  worktrees, and operators.
- Lines 15-29 split jobs across:
  - `swarm-queue.json`
  - `completed-slices.md`
  - `discovered-issues.md`
  - `known-pitfalls.md`
  - `findings.json`

This is durable coordination memory, not total runtime replacement.

That distinction matters because the same repo still keeps:

- handoffs in `.ops-perl-lsp/handoffs/`
- metrics in `.ops-perl-lsp/swarm-metrics.jsonl`
- self-improvement patches in `.ops-perl-lsp/agent-patches/`
- salvage dumps in `.ops-perl-lsp/salvage/`

So the migration is state-splitting, not ops-directory deletion.

## 3. Skills And Findings Make The New Canonical Layer Explicit On 2026-03-17

Two March 17 commits make the new canonical layer much harder to miss:

1. `31f7854e8` on `2026-03-17`
   `feat(swarm): add core worker skills (#1737)`
2. `d9aab31bc` on `2026-03-17`
   `docs(swarm): track durable findings with schema (#1741)`

The skill-backed protocol already uses the new language:

- [`.claude/skills/swarm-protocol/SKILL.md`](../../../.claude/skills/swarm-protocol/SKILL.md)
  lines 30-43:
  - "Use GitHub and tracked swarm state as the durable ledger"
  - explicitly names `.claude/swarm-state/`
  - explicitly names `findings.json`
- Lines 57-69 tell agents to check open PRs, issues, `.claude/swarm-state/`,
  and existing handoffs/receipts before creating new work.

The findings ledger then records the migration as a control-plane conclusion:

- [`.claude/swarm-state/findings.json`](../../../.claude/swarm-state/findings.json)
  lines 130-153:
  `SWARM-FINDING-0005` says durable swarm findings need a machine-readable
  ledger instead of living only in chat transcripts and prose docs.
- Lines 156-179:
  `SWARM-FINDING-0006` says the legacy
  `.claude/commands/swarm-protocol.md` mirror still reflects older
  `.ops-perl-lsp` paths and workflow language.

That is unusually strong evidence because the repo is not merely drifting. It is
recording that drift as a tracked control-plane bug.

## 4. Hooks Move Behavior From Prompt Doctrine Into Enforcement, But Not Away From `.ops-perl-lsp`

The hook regime becomes explicit across March 16:

- `e4a089ef4` on `2026-03-16`
  `feat(hooks): add SubagentStart, Stop, PreToolUse, SessionStart hooks`
- `d17b84393` on `2026-03-16`
  `docs(swarm): codify worktree-first control plane (#1721)`

Current committed hook registration:

- [`.claude/settings.json`](../../../.claude/settings.json) lines 65-118
  registers `TaskCompleted`, `SubagentStart`, `SubagentStop`, `PreToolUse`,
  and `SessionStart`.

Current committed runtime hook:

- [`.claude/hooks/subagent-stop.sh`](../../../.claude/hooks/subagent-stop.sh)
  lines 4-23 still defaults `OPS_DIR` to `.ops-perl-lsp` and appends JSON
  teardown events to `.ops-perl-lsp/swarm-metrics.jsonl`.

This is the sharpest single proof that the modern hook layer did not replace the
old runtime directory. It automated it.

One nuance matters:

- [`docs/adr/0032-skill-scoping-and-hook-enforcement.md`](../../../docs/adr/0032-skill-scoping-and-hook-enforcement.md)
  lines 11-20 and 44-55 describe the move from prompt-only discipline to hook
  enforcement.
- But the current committed
  [`.claude/hooks/task-completed.sh`](../../../.claude/hooks/task-completed.sh)
  lines 6-12 only gate `cargo fmt --all -- --check`.

So the committed repo proves hook hardening happened, but it also proves the
hook regime evolved: the live implementation is narrower than the ADR's
stronger metrics-enforcement discussion.

## 5. The Hybrid Residue Lives In Current Skills, Commands, And Agent Packs

### Current skills

- [`.claude/skills/swarm/SKILL.md`](../../../.claude/skills/swarm/SKILL.md)
  lines 21-26 say worker procedures now ship from skills while broader operator
  procedures still live under `.claude/commands/`.
- The same file still reaches back into `.ops-perl-lsp`:
  - line 96: agent patches
  - line 148: handoff file path
- [`.claude/skills/swarm-protocol/SKILL.md`](../../../.claude/skills/swarm-protocol/SKILL.md)
  line 54 still says "When the lane uses `.ops-perl-lsp/swarm-metrics.jsonl`,
  append an entry instead of rewriting history."

That is a hybrid policy surface: new durable state plus old runtime metrics.

### Current commands

- [`.claude/commands/swarm-status.md`](../../../.claude/commands/swarm-status.md)
  lines 26-32 mix `swarm-state` counts with `.ops-perl-lsp/handoffs` and
  `.ops-perl-lsp/agent-patches`.
- Lines 40-46 mix `.ops-perl-lsp/swarm-metrics.jsonl` with
  `.claude/swarm-state/known-pitfalls.md` and `findings.json`.
- [`.claude/commands/swarm-report.md`](../../../.claude/commands/swarm-report.md)
  lines 28-33 still read `.ops-perl-lsp/agent-patches/*.md` and
  `.ops-perl-lsp/swarm-metrics.jsonl`.
- [`.claude/commands/swarm-wind-down.md`](../../../.claude/commands/swarm-wind-down.md)
  lines 63-66 preserve `swarm-state` knowledge and `.ops-perl-lsp` performance
  history side by side.
- [`.claude/commands/swarm-protocol.md`](../../../.claude/commands/swarm-protocol.md)
  lines 42, 81, 105, 117, 130-132, and 250-252 still teach the older
  `.ops-perl-lsp` discovery, knowledge, and runtime model.

This is exactly what `SWARM-FINDING-0006` is complaining about.

### Current agent packs

`agents6` is where the split is visibly incomplete:

- [`.claude/agents6/swarm-scout.md`](../../../.claude/agents6/swarm-scout.md)
  lines 24, 75, 81-92, and 128 still use `.ops-perl-lsp` for discovered issues,
  completed slices, known pitfalls, and handoffs.
- [`.claude/agents6/swarm-builder.md`](../../../.claude/agents6/swarm-builder.md)
  lines 36-42, 59-60, 108, 153, and 166 still center work on
  `.ops-perl-lsp/handoffs` and `.ops-perl-lsp/swarm-metrics.jsonl`.
- [`.claude/agents6/swarm-merger.md`](../../../.claude/agents6/swarm-merger.md)
  lines 12 and 70-73 still use `.ops-perl-lsp/completed-slices.md` and
  `.ops-perl-lsp/swarm-metrics.jsonl`.

`agents-compat` makes the donor status explicit, but it also preserves mixed
pathing on purpose:

- [`.claude/agents-compat/README.md`](../../../.claude/agents-compat/README.md)
  lines 3-9 says this directory is tracked compatibility and donor material,
  not the active swarm roster.
- [`.claude/agents-compat/swarm-builder.md`](../../../.claude/agents-compat/swarm-builder.md)
  lines 27-33 and 50-52 mix `.ops-perl-lsp/handoffs` with
  `.claude/swarm-state/known-pitfalls.md`.

That means the compatibility layer is not merely old. It is intentionally
preserved mixed-path donor material.

`agents5` is weaker evidence for the live hybrid because it mostly predates the
tracked `swarm-state` split rather than negotiating it. The main concrete
residue there is janitor salvage:

- [`.claude/agents5/swarm-janitor.md`](../../../.claude/agents5/swarm-janitor.md)
  lines 15 and 47-51 write salvage into `.ops-perl-lsp/salvage/`.

## 6. Some Current Docs Already Normalize The Split Better Than The Live Repo

The portable pack and handoff design docs are actually cleaner than several live
repo surfaces:

- [`docs/handoff/swarm-pack/README.md`](../../../docs/handoff/swarm-pack/README.md)
  lines 104-118 normalize the split as:
  - tracked `swarm-state`
  - ephemeral `.ops/`
- [`docs/handoff/swarm-pack/SWARM_DESIGN.md`](../../../docs/handoff/swarm-pack/SWARM_DESIGN.md)
  lines 260-282 do the same in the persistence-layer model.

But other current docs still carry live `.ops-perl-lsp` residue:

- [`docs/reference/SKILL_AND_AGENT_DESIGN.md`](../../../docs/reference/SKILL_AND_AGENT_DESIGN.md)
  lines 268-276 still use `.ops-perl-lsp/handoffs/fix-heredoc-queue.md` in the
  example spawn prompt.
- [`docs/adr/0032-skill-scoping-and-hook-enforcement.md`](../../../docs/adr/0032-skill-scoping-and-hook-enforcement.md)
  line 15 still anchors the metrics compliance story to
  `.ops-perl-lsp/swarm-metrics.jsonl`.

So the migration is uneven even inside committed documentation.

## 7. Date Clues

The clearest explicit migration window is March 15 to March 17, 2026:

1. `2026-03-15` `9cc2d3b9a`
   `feat(swarm): continuous swarm infrastructure with agent teams (#1553)`
   - introduces the tracked-vs-ephemeral split
   - adds `agents6`, command surfaces, pack docs, settings changes

2. `2026-03-16` `e4a089ef4`
   `feat(hooks): add SubagentStart, Stop, PreToolUse, SessionStart hooks`
   - behavioral enforcement starts moving into hooks

3. `2026-03-16` `d17b84393`
   `docs(swarm): codify worktree-first control plane (#1721)`
   - adds the current `subagent-stop.sh`
   - keeps metrics routed through `OPS_DIR`, defaulting to `.ops-perl-lsp`

4. `2026-03-17` `d870a3d5f`
   `chore(swarm): move donor agents out of active roster (#1736)`
   - `agents-compat/` becomes explicit donor/compatibility storage

5. `2026-03-17` `31f7854e8`
   `feat(swarm): add core worker skills (#1737)`
   - skills become the preferred reusable worker surface

6. `2026-03-17` `d9aab31bc`
   `docs(swarm): track durable findings with schema (#1741)`
   - `findings.json` lands
   - the repo starts recording control-plane drift as tracked findings

## 8. Strongest Evidence-Backed Claims

1. The repo explicitly split durable tracked state from ephemeral runtime on
   `2026-03-15`, but did not delete or retire `.ops-perl-lsp`.
2. `swarm-state` is the committed durable memory layer, not a total replacement
   for runtime handoffs, metrics, or salvage.
3. March 16 hook work externalized behavioral enforcement, but at least one live
   hook still writes directly to `.ops-perl-lsp`.
4. March 17 makes the migration self-aware: skills become canonical worker
   surfaces, `findings.json` lands, and the repo explicitly records command-path
   drift in `SWARM-FINDING-0006`.
5. `agents6` is the strongest live residue surface; `agents-compat` is the
   strongest intentional donor/compatibility residue surface; `agents5` is
   mostly pre-split donor material rather than the main migration battleground.

## 9. Suggested Outline For A Launch-Article Evidence Note

If this evidence gets compressed into a shorter launch-facing note, the cleanest
outline is:

1. What changed: tracked state split from ephemeral runtime
2. What became canonical: `swarm-state`, skills, findings
3. What became enforceable: hooks instead of prompt-only doctrine
4. What stayed hybrid: handoffs, metrics, patches, salvage
5. Where the residue still shows up: commands, agents6, agents-compat, docs
6. Why that matters: this was a layered migration, not a greenfield rewrite

## See Also

- [CONTROL_PLANE_ARCHAEOLOGY.md](CONTROL_PLANE_ARCHAEOLOGY.md)
- [SWARM_SURFACE_EVOLUTION.md](SWARM_SURFACE_EVOLUTION.md)
- [SWARM_STATE_ARCHAEOLOGY.md](SWARM_STATE_ARCHAEOLOGY.md)
- [KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md](KNOWLEDGE_COMPOUNDING_ARCHAEOLOGY.md)
- [OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md](OPERATING_SYSTEM_GAP_ARCHAEOLOGY.md)
