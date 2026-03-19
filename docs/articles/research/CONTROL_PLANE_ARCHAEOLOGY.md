# Control Plane Archaeology
## How The Repo Encoded Its Own Swarm History

This note traces the swarm's evolution through tracked `.claude/` and `.jules/` artifacts rather than commit volume alone.

The point is not that prompts existed. The point is that the repository kept promoting more of the development method into version-controlled surfaces: first orchestration guides, then three-phase role packs, then named persona lanes, then persistent teammates, and finally the current control plane split across agents, commands, skills, hooks, and swarm-state.

---

## 1. Early Orchestration Seed

The earliest clear control-plane artifact in the tracked history is `.claude/ORCHESTRATION_GUIDE.md`, first added on **2025-08-28** in commit `3341bebdb` (`feat: update agent documentation and add orchestration guide`).

That file already describes a flow-oriented system:

- `pr-initial-reviewer`
- `test-runner-analyzer`
- `context-scout`
- `pr-cleanup-agent`
- `pr-finalize-agent`
- `pr-merger`
- `pr-doc-finalize`

And it already thinks in loops, not chats:

- entry point
- iterative validation loop
- final quality gate
- merge
- post-merge documentation

This matters because it predates the current swarm stack. The repo was already trying to encode a software-delivery pipeline as cooperating roles in August 2025.

---

## 2. Q3 Canonical Swarm: `agents4`

The tracked `.claude/agents4/` directory is the clearest preserved form of the Q3 2025 Claude Code swarm.

Its earliest tracked commit is **2025-09-23** (`104bdc17e`, `feat: add spec-fixer agent for synchronizing documentation with codebase`), and its structure is explicit:

- `review/`
- `integration/`
- generation, stored on disk as `generative/`
- top-level flow files: `issue-to-draft.md`, `draft-to-pr.md`, `pr-to-merge.md`

The counts in that retained tree are revealing:

- `generative/`: 32 files
- `integration/`: 27 files
- `review/`: 52 files

This is not "a few helper prompts." It is a full three-phase operating model.

The important nuance is that the directory names and the top-level flow files
are two naming schemes for the same Q3 swarm:

- role-pack view: `generative/`, `integration/`, `review/`
- flow view: `issue-to-draft`, `draft-to-pr`, `pr-to-merge`

The explicit mapping the maintainer calls out is:

- `generative/` = `issue-to-draft`
- `review/` = `draft-to-pr`
- `integration/` = `pr-to-merge`

Those are not separate layers. They are two ways the same canonical swarm was
encoded on disk.

The repo still preserves nearby variants in `.claude/agents2/` and `.claude/agents3/`, but `agents4` reads as the canonical Q3 form because it cleanly presents the three lanes the maintainer still points back to: `review`, `integration`, and generation.

Operationally, this era is still prompt-pack heavy:

- large file-defined role catalogs
- explicit flow handoffs
- GitHub check-run and ledger language inside prompts
- serial or staged work more than persistent teammate behavior

It is already a swarm. It is just a more ceremonial, pack-defined swarm than the current one.

---

## 3. January 2026 Jules Bridge: Persona Lanes

The `.jules/` directory shows another important evolutionary step: the repo starts preserving lane-specific knowledge as named personas.

Three tracked persona journals matter most:

- `.jules/bolt.md`
- `.jules/sentinel.md`
- `.jules/palette.md`

These map cleanly to recurring work lanes:

- `Bolt`: performance and hot-path reasoning
- `Sentinel`: security and attack-surface hardening
- `Palette`: UX and editor ergonomics

The history around these files clusters in **2026-01-19 through 2026-01-29** and lines up with matching commits:

- `b1474bdf8` `fix(security): complete command injection hardening in executeCommand (#332)`
- `8115acba4` `feat(vscode): improve context menu visibility and add inline variable command (#335)`
- `3ba17d563` `perf(semantic): optimize is_builtin_global to reduce allocations (#465)`
- `85252c524` `🛡️ Sentinel: [HIGH] Fix command injection in version check (#514)`
- `92d0951e9` `feat(vscode): add keyboard shortcuts for running tests and restart (#522)`
- `d8dd0df5c` `🛡️ security: fix command injection in VS Code downloader (#530)`
- `ea8c56346` `🎨 Palette: Add keybinding hints to Status Menu (#602)`

The journals themselves preserve the lane logic:

- `Bolt` records hot-path lessons about allocation churn, iterator design, and bareword checks in `ScopeAnalyzer`
- `Sentinel` records concrete vulnerability classes: command injection, safe-eval bypasses, path traversal, checksum enforcement, HTTPS downgrade risks
- `Palette` records UX principles such as "broken promise" commands, startup noise, keybinding discovery, and status-bar feedback

This is a bridge, not yet the current swarm:

- the repo has specialized identities
- those identities carry reusable knowledge
- the knowledge is stored in committed journals and findings
- but the control plane is still not unified into the later skills/hooks/state model

What changed is subtle but important: the codebase stopped treating specialized work as only a branch or PR outcome and started treating it as a persistent lane with memory.

---

## 4. Transition Layer: `agents5` and `agents6`

The next major change lands on **2026-03-15** in commit `9cc2d3b9a` (`feat(swarm): continuous swarm infrastructure with agent teams (#1553)`).

This is where the retained agent directories stop looking like a three-phase pack and start looking like a team.

### `agents5`

`.claude/agents5/` still carries strong generative-flow DNA:

- `spec-creator`
- `impl-creator`
- `test-creator`
- `policy-gatekeeper`
- `pr-preparer`
- `pr-publisher`

But it also introduces the core persistent swarm roles:

- `swarm-scout`
- `swarm-builder`
- `swarm-reviewer`
- `swarm-merger`
- `swarm-fixer`
- `swarm-janitor`

This is a hybrid stage: old flow-finalizer machinery plus new teammate roles.

### `agents6`

`.claude/agents6/` is more recognizably the ancestor of the modern system.

It keeps the core swarm roles and adds more durable operational lanes:

- `swarm-strategist`
- `swarm-validator`
- `swarm-pr-responder`
- `swarm-improver-docs`
- `swarm-improver-tests`
- `swarm-improver-infra`
- `swarm-improver-devex`

It also expands domain-specific execution workers:

- `parser-fix-engine`
- `parser-corpus`
- `dap-feature`
- `workspace-index`
- `security-audit`
- `baseline-ratchet`
- `adr-writer`

This is the key transition:

- from document-processing flow
- to repository-operating team

The swarm is no longer only "generate, integrate, review." It is now "scout, build, review, merge, improve, steer."

---

## 5. The Current Swarm Surfaces

The repo's current control-plane readme in `.claude/README.md` defines the active surfaces more explicitly than the older generations did.

Today, the swarm lives across:

- `.claude/agents/`
- `.claude/commands/`
- `.claude/skills/`
- `.claude/hooks/`
- `.claude/swarm-state/`

### `.claude/agents/`

This is effectively the `agents7` layer.

It is not the old "load this file as the runtime prompt" surface. Instead, it is the canonical archived roster surface:

- `AGENT_CATALOG.md`
- `archive/agent-roster.json`
- 54 archived definitions for reference

The current orchestrator uses these as historical or roster material, not as the sole runtime contract.

### `.claude/commands/`

These are slash-entry procedures that still matter operationally:

- `swarm.md`
- `green-merge.md`
- `cleanup-worktrees.md`
- `swarm-wind-down.md`
- `agent-dashboard.md`
- `parser-scout.md`

This is where a lot of operator ceremony remains explicit.

### `.claude/skills/`

This is the biggest architectural shift from the Q3 pack model.

Skills now hold reusable procedures such as:

- `swarm`
- `swarm-protocol`
- `verify-build`
- `parser-fix`
- `triage-prs`
- `swarm-priorities`

That means stable behavior is encoded once and invoked repeatedly, instead of being duplicated across dozens of role prompts.

### `.claude/hooks/`

Hooks make enforcement deterministic:

- `task-completed.sh`
- `teammate-idle.sh`

This is an important difference from earlier swarm generations, where more behavior lived in exhortation inside prompts.

### `.claude/swarm-state/`

This directory is best understood as the current-ish state and documentation layer around the control plane.

Its tracked contract is explicit:

- `swarm-queue.json` — overlap/ownership tracking
- `completed-slices.md` — dedup ledger
- `discovered-issues.md` — opportunistic leads
- `known-pitfalls.md` — reusable failure lessons
- `findings.json` — durable swarm-control conclusions

This is where the repo stores what the swarm has learned about itself.

---

## 6. Why This Matters

The codebase's development history is not just visible in commit counts or PR volume. It is visible in the surfaces the repo chose to preserve.

The progression looks like this:

1. **Flow thinking**: orchestration guide and review loops
2. **Phase packs**: Q3 `agents4` three-phase swarm
3. **Persona lanes**: `Bolt`, `Sentinel`, `Palette` journals and findings
4. **Persistent teammates**: `agents5` and `agents6`
5. **Current control plane**: agents + commands + skills + hooks + swarm-state

That is the deeper story of this repository:

- not just "AI wrote code"
- but "the repo kept encoding more of its own operating method into tracked artifacts"

The swarm did not appear all at once in March 2026. It had been trying to exist for months.

---

## 7. Evidence Pointers

- `.claude/ORCHESTRATION_GUIDE.md`
- `.claude/README.md`
- `.claude/swarm-state/README.md`
- `.claude/agents4/issue-to-draft.md`
- `.claude/agents4/pr-to-merge.md`
- `.claude/agents5/swarm-builder.md`
- `.claude/agents6/swarm-strategist.md`
- `.jules/bolt.md`
- `.jules/sentinel.md`
- `.jules/palette.md`
- `.jules/findings/security/sentinel.md`
- `.jules/findings/ux/palette.md`

Key commits:

- `3341bebdb` — orchestration guide added (`2025-08-28`)
- `104bdc17e` — earliest tracked `agents4` addition (`2025-09-23`)
- `9cc2d3b9a` — `agents5`/`agents6` swarm-team transition (`2026-03-15`)
- `cb4251735` — archived lineage explicitly retained (`2026-03-19`)
- `99d2b17f0` — revert of attempted lineage cleanup (`2026-03-19`)
