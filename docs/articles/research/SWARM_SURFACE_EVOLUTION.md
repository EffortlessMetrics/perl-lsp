# Swarm Surface Evolution
## How Commands, Skills, Hooks, and Swarm-State Became The Current Control Plane

This note focuses on one narrow historical question:

How did the repository move from prompt-heavy agent packs to the current swarm surfaces under `.claude/commands/`, `.claude/skills/`, `.claude/hooks/`, and `.claude/swarm-state/`?

The answer is visible in a compressed transition window between **January 11, 2026** and **March 19, 2026**.

---

## 1. Commands Arrive Before Skills

The oldest current-style swarm surface is not the skill layer. It is the command layer.

The earliest tracked history touching `.claude/commands/` appears on **2026-01-11**:

- `a95280802` — `feat: Parser audit infrastructure, CI baseline ratchets, and context-aware completions (#291)`
- `4d5861217` — `refactor(error-handling): enforce panic-safe production code (closes #143) (#292)`

That means the repo had already started encoding reusable slash-entry procedures in January 2026, well before the current skill layer appears in git.

By the time the current swarm is in motion, `.claude/commands/` already contains a broad operator surface, including:

- `swarm.md`
- `green-merge.md`
- `cleanup-worktrees.md`
- `ci-watch.md`
- `parser-scout.md`
- `worktree-pr.md`
- `swarm-wind-down.md`
- `swarm-stop.md`

The command layer is the first clear sign that procedure is being lifted out of one-off prompts and into reusable control-plane entrypoints.

---

## 2. March 15, 2026: Continuous Swarm Infrastructure Turns On

The next major inflection point is **2026-03-15**:

- `9cc2d3b9a` — `feat(swarm): continuous swarm infrastructure with agent teams (#1553)`

This commit family is the real "switch-on" moment for the current-era swarm.

It introduces or materially establishes:

- `.claude/hooks/`
- `.claude/swarm-state/`
- the command-based continuous swarm entrypoint
- the `agents5/6` persistent-teammate model

The earliest tracked hook and swarm-state history both point back to that same March 15 transition:

- `.claude/hooks/` first tracked at `9cc2d3b9a`
- `.claude/swarm-state/` first tracked at `9cc2d3b9a`
- `.claude/commands/swarm.md` first tracked at `9cc2d3b9a`

This matters because it shows the control plane becoming operational rather than merely descriptive.

Three new surfaces appear together:

1. **hooks** for deterministic enforcement
2. **swarm-state** for persistent coordination state
3. **continuous swarm command entrypoints** for operator control

---

## 3. March 16, 2026: Skills Extract The Durable Procedures

The skill layer arrives immediately after the March 15 swarm turn-on.

The earliest tracked `.claude/skills/` history lands on **2026-03-16**:

- `bf23f7904` — `refactor(swarm): restructure team from 12 to 5 coordinators`

That same day, several closely related commits reshape the control plane:

- `0fbdd8acb` — add launcher skills
- `d3706f695` — add `/cycle-transition` and `/agent-dashboard` skills
- `125089ac9` — add `/review-pr` skill
- `13e6a3ea1` — add `/pr-ready`
- `a7d7a7f14` — add `/verify-master-green`, `/rebase-pr`, and `/verify` governance skills
- `1fd8f7e36` — add invocation-control frontmatter to all skill definitions
- `69a6b9f16` — extract thin agent definitions into reusable skills

This is the architectural pivot:

- commands remain as operator entrypoints and broader procedures
- skills take over the durable worker/coordinator procedures

The repo even says this explicitly in the modern surfaces:

- command `/swarm` says the core worker procedures now ship from `.claude/skills/`
- skill `/swarm` keeps the control-plane contract, but in a leaner, more reusable form

---

## 4. The `swarm` and `swarm-protocol` Split Shows The Design Change

Two paired files make the transition unusually legible:

- `.claude/commands/swarm.md` versus `.claude/skills/swarm/SKILL.md`
- `.claude/commands/swarm-protocol.md` versus `.claude/skills/swarm-protocol/SKILL.md`

### `swarm`

The command version of `/swarm` is still a very full operator document:

- bootstrap
- label creation
- worktree cleanup
- CI queue discipline
- 5 coordinator spawn prompts
- lead periodic duties
- concrete command snippets

The skill version keeps the same architecture, but tightens the surface:

- same 5 coordinators
- same boundary model
- same orchestrator-vs-worker split
- but more emphasis on durable references, templates, and teammate contracts

The shift is not "new idea vs old idea." The shift is:

- command = broad operator procedure
- skill = reusable durable contract

### `swarm-protocol`

The command version of `/swarm-protocol` is expansive and highly operational:

- GitHub labels
- PR templates
- exact metrics JSONL shape
- issue creation shell snippets
- detailed worktree rules
- cost-efficiency rules
- lifecycle shutdown protocol

The skill version compresses that into a cleaner behavioral contract:

- autonomy
- direct communication
- GitHub-native state
- receipts
- dedup
- worktree/worker boundaries
- lifecycle
- review discipline
- research discipline

This is a classic maturation move: the protocol becomes more declarative and less noisy once other surfaces can carry the mechanics.

---

## 5. Hooks Turn Repeated Advice Into Enforcement

The hook layer is small, but historically important.

Two tracked hook files illustrate what changed:

- `.claude/hooks/task-completed.sh`
- `.claude/hooks/teammate-idle.sh`

`task-completed.sh` is blunt:

- run `cargo fmt --all -- --check`
- reject task completion if formatting is not clean

`teammate-idle.sh` is also modest but revealing:

- track idle transitions
- suppress repeated idle noise
- treat teammate lifecycle as something the control plane should manage systematically

Earlier swarm generations mostly asked agents to behave correctly. Hooks let the repo enforce parts of that behavior mechanically.

That is a real control-plane upgrade, even if the scripts themselves are small.

---

## 6. `swarm-state` Turns The Swarm Into A Self-Documenting System

The `swarm-state` layer appears with the March 15 turn-on, but its contract becomes especially clear on **2026-03-17**:

- `d9aab31bc` — `docs(swarm): track durable findings with schema (#1741)`
- `37ddcf56d` — `fix(swarm): validate empty findings ledgers (#1743)`
- `e5f8bcfab` — `chore(swarm): add active agent roster contract (#1744)`

The tracked `README.md` for `.claude/swarm-state/` shows exactly what the repo thinks should persist across sessions:

- `swarm-queue.json`
- `completed-slices.md`
- `discovered-issues.md`
- `known-pitfalls.md`
- `findings.json`
- `findings.schema.json`

This is the difference between "we ran a swarm" and "the swarm leaves durable organizational memory."

The most important file conceptually is `findings.json`:

- not a bug tracker
- not a temporary handoff
- a durable ledger for conclusions that should change how the swarm operates

That is a strong sign that the repository had started treating the development method itself as first-class versioned infrastructure.

---

## 7. March 16–18: The Current Control Plane Is Rationalized

After the new surfaces appear, the repo spends the next few days reducing ambiguity.

Important commits in that cleanup/rationalization burst:

- `bf23f7904` — restructure team from 12 to 5 coordinators
- `d17b84393` — codify worktree-first control plane
- `bf7407d24` — canonicalize agent roster and hook surfaces
- `5d8f96d0e` — mark legacy workflow surfaces as archived
- `00f80da5c` — clarify skill vs command surfaces
- `2e2624e14` — demote legacy wave entrypoints
- `dfdd3be72` — archive unused agent definitions
- `99d2b17f0` — revert attempted cleanup of historical lineage directories

This is the moment where the repo stops merely accumulating control-plane files and starts curating them:

- fewer coordinators
- clearer boundaries
- explicit archive status for legacy surfaces
- preserved historical lineage instead of destructive cleanup

The historical preservation point matters. The repo explicitly chose not to erase `agents2-6` and `agents-compat`, because those surfaces are part of the archaeology.

---

## 8. The Actual Evolution Pattern

The sequence looks like this:

1. **August 2025**: orchestration guide encodes a flow
2. **Q3 2025**: `agents4` encodes the canonical three-phase swarm pack
3. **January 2026**: commands exist as reusable slash procedures before the current skill layer
4. **January 2026**: `.jules` journals preserve lane-specific knowledge
5. **March 15, 2026**: continuous swarm infrastructure turns on with hooks, swarm-state, and agent teams
6. **March 16, 2026**: skills extract the durable reusable procedures
7. **March 17, 2026**: swarm-state gets schema and findings contracts
8. **March 16–19, 2026**: the repo rationalizes the surfaces and preserves the lineage

The key insight is simple:

The current control plane is not just "skills replaced prompts."

It is:

- commands first
- then continuous swarm infrastructure
- then skills
- then hooks and state contracts tightening behavior
- then curation of what stays active versus what becomes archival

---

## 9. Evidence Pointers

Files:

- `.claude/commands/swarm.md`
- `.claude/commands/swarm-protocol.md`
- `.claude/skills/swarm/SKILL.md`
- `.claude/skills/swarm-protocol/SKILL.md`
- `.claude/hooks/task-completed.sh`
- `.claude/hooks/teammate-idle.sh`
- `.claude/swarm-state/README.md`
- `.claude/README.md`

Key commits:

- `a95280802` — earliest tracked `.claude/commands/` history (`2026-01-11`)
- `9cc2d3b9a` — continuous swarm infrastructure with agent teams (`2026-03-15`)
- `bf23f7904` — restructure team from 12 to 5 coordinators (`2026-03-16`)
- `69a6b9f16` — extract thin agent definitions into reusable skills (`2026-03-16`)
- `d17b84393` — codify worktree-first control plane (`2026-03-16`)
- `d9aab31bc` — track durable findings with schema (`2026-03-17`)
- `dfdd3be72` — archive unused agent definitions (`2026-03-18`)
- `99d2b17f0` — preserve historical lineage by reverting removal (`2026-03-19`)
