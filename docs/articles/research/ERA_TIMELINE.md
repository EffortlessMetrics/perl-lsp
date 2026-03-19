# perl-lsp Development Era Timeline
## Five Distinct AI Development Paradigms (July 2025 – March 2026)

_Deep archaeological analysis of git history across five distinct development eras, each with unique patterns, tooling, and velocity characteristics._

_Method note: this report uses two lenses intentionally. Mainline sections call out merged `master` history when integration pressure is the point; swarm sections also use `git log --all` and remote-branch evidence when branch proliferation and session volume are the point._

---

## Era 1: Opus Direct (July–August 2025)
### "Claude Opus as Direct Coding Partner"

**Date Range**: 2025-07-01 to 2025-09-01
**Duration**: 2 months
**Total Commits**: 947
**Average Velocity**: ~14-15 commits/day
**Peak Day**: 2025-07-16 (192 commits)

### Commit Patterns
- **Conventional commits**: 1,213/947 (128% — duplicate commits suggest merge artifacts)
- **Commit message style**: Verbose, descriptive, natural language
  - Example: "docs: update architecture and progress reports for Rope-based incremental parsing enhancements"
  - Example: "final: apply pr-cleanup-agent fixes for production-ready merge"
  - Example: "comprehensive PR cleanup: fix parser/scope analyzer issues"
- **No branching convention**: Direct commits to master or long-lived feature branches
- **Merge commits**: Heavy merge conflicts, manual conflict resolution in commit messages
  - "resolve merge conflicts in CLAUDE.md on master branch"
  - "resolve merge conflicts with master - integrate incremental parsing documentation"

### Contributors
- **Steven Zimmerman** (100% of commits)
- **No external contributions in this era**
- **No bot activity** (pre-automation)

### Key Work
1. **Incremental parsing**: Rope-based structure for efficient re-parsing
   - Major feature PR #65: "Support complex heredoc delimiter expressions"
   - Major feature PR #66: "Improve incremental parsing reuse and benchmarks"
   - Major feature PR #67: "Make LSP harness async and enable definition timeout test"

2. **Scope analyzer**: Hash key context detection
   - PR #50: "implement-is_in_hash_key_context-function"
   - Focus: Variable resolution in different syntactic contexts

3. **Parser foundation**: Early variable resolution work
   - "extend-parsing-logic-in-variable-resolution"
   - "compute-line-and-column-offsets-in-parser"

4. **File completion logic**: Initial LSP provider implementation
   - "implement-file-completion-logic"

5. **Error handling**: Converting panics to returns
   - "convert-panic-branches-to-return-errors"

### Distinctive Artifacts
- **Branch naming**: `codex/` prefix appeared (early Copilot signal)
- **PR references**: Manual PR numbers (#50, #65, #66, #67)
- **Documentation updates**: Heavy doc maintenance during development
- **Merge conflict discipline**: Explicit conflict resolution commits

### Architectural Milestones
- **Parser version**: v1 (C-based tree-sitter) being wrapped in Rust
- **Incremental parsing**: Rope-based Crdt introduced (later removed)
- **Scope analysis**: Initial variable context tracking implemented

### Verdict
**Exploratory phase**. Opus directly writing code with human review. Emphasis on architectural foundations rather than feature breadth. High-quality but slow: ~947 commits in 2 months suggests manual, careful development.

---

## Era 2: Early Swarms (August–October 2025)
### "First Multi-Agent Experiments"

**Date Range**: 2025-08-01 to 2025-10-31
**Duration**: 3 months
**Total Commits**: 840
**Average Velocity**: ~9-10 commits/day
**Peak Day**: 2025-08-26 (109 commits)

### Commit Patterns
- **Conventional commits**: Slight uptick but still inconsistent
  - "Improve pest postfix modifier parsing"
  - "Improve qualified calls in pest parser"
  - "Improve pest delimiter parsing"
- **Branch strategy**: Heavy `codex/` prefix usage intensifies
  - `codex/add-test-for-tree-sitter-perl-highlights`
  - `codex/extend-dead_code_detector-to-analyze-code`
  - `codex/add-lightweight-http-server-with-watch-feature`
  - `codex/replace-c-scanner-with-rust-implementation`
  - `codex/convert-panic-branches-to-return-errors`
- **Merge pattern**: Long-lived branches with frequent "Merge branch 'master' into codex/..." commits
  - Indicates parallel work with frequent rebasing

### Contributors
- **Steven Zimmerman** (100% human-attributed commits)
- **EffortlessSteven** (variant, same person)
- **No bot commits yet** (but infrastructure preparing for it)

### Key Work
1. **Parser v2 (Pest)**: Significant development
   - Multiple "Improve pest" commits for postfix modifiers, qualified calls, delimiters
   - Suggests v2 was being improved in parallel with v1/v3

2. **Tree-sitter highlights**: C scanner replacement
   - "replace-c-scanner-with-rust-implementation"
   - Moving from C dependencies to pure Rust

3. **Dead code detector**: Infrastructure tooling
   - "extend-dead_code_detector-to-analyze-code"

4. **Dev experience**: HTTP server for watching files
   - "add-lightweight-http-server-with-watch-feature"
   - Signals shift toward local development tooling

### Distinctive Artifacts
- **codex/ prefix dominates**: 90%+ of meaningful commits in `codex/` branches
- **Merge hell**: Multiple "Merge branch 'master' into codex/X" per day
  - Suggests 2-4 concurrent long-running feature branches
- **Test infrastructure**: Heavy test addition
  - "Add comprehensive test suite for Perl code parsing"
  - "Add comprehensive test suite for Perl code parsing"
- **Q3 control-plane strata**:
  - `.claude/agents4/` looks like the canonical Q3 three-phase swarm
  - That canonical form centers on `review/`, `integration/`, and generation (stored as `generative/`) packs plus `issue-to-draft` / `pr-to-merge` orchestration
  - Signal: the swarm is still prompt-file-driven, with heavier role-pack definitions than the later teammate model

### Velocity Pattern
Peak at **109 commits 2025-08-26**, drops to 75-78 commits 2025-08-27, suggesting single-day batch merges.

### Architectural Milestones
- **Parser v2 evolution**: Pest grammar improvements (postfix, qualified calls)
- **v1 deprecation signal**: C scanner replacement suggests moving away from C dependencies
- **Testing infrastructure**: Corpus tests, highlight tests, S-expression comparison

### Verdict
**Transition phase**. This is where the repo visibly becomes swarm-driven: large daily PR batches appear in Q3 2025, even though Steven still remains the final integrator and much of the merged history keeps single-human attribution. Era 1 was more direct; Era 2 makes the PR stream itself part of the story.

---

## Era 3: Architectural Sidechain (October 2025–February 2026)
### "Heavy Correctness Sprint + Browser-Chat Architecture Design"

**Date Range**: 2025-10-01 to 2026-02-28
**Duration**: 5 months
**Total Commits**: 423
**Average Velocity**: ~2.8 commits/day
**Peak Day**: 2026-02-28 (91 commits)

### Commit Patterns
- **Conventional commits**: Still lower adoption (not all commits follow format)
  - Mix of "fix:", "feat:", "chore:" with natural language
- **Branch naming**: `codex/` remains but less frequent
  - Some shift toward descriptive feature branches
- **Merge style**: Longer individual commits (more work per commit)
  - Suggests fewer, larger PRs rather than incremental commits
- **Documentation emphasis**: Heavy ADR (Architecture Decision Record) addition
  - "docs: ADR-008 swarm-v2 architecture + CHANGELOG"
  - "docs: add ADR-005 for skill scoping and hook enforcement design"

### Contributors
- **Steven Zimmerman** (nearly 100%)
- **Paul Evans** appears (Perl 5 core team joining)
- **Still no bot activity** (infrastructure not yet live)

### Key Work
1. **Parser v3 emergence**: Recursive descent parser development
   - Focus on error recovery, not just parsing
   - Commits like "restore architectural integrity for Issue #146"
   - "eliminate fragile unreachable!() macros (Issue #178)"

2. **Mutation testing**: Hardening phase
   - "Add comprehensive mutation tests and improve parser robustness"
   - "Implement comprehensive mutation testing hardening for quote parser"
   - Suggests rigorous testing before v0.10+ release

3. **Semantic analysis**: Variable scope and context
   - "Import Refactoring and Test Infrastructure Improvements (v2)"
   - "Dead Code Detection, Diagnostics, and Performance Improvements"

4. **LSP completeness**: Feature provider implementation
   - Heavy testing infrastructure for LSP integration
   - Flaky test handling: "mark all flaky LSP integration tests... as ignored"

### Distinctive Artifacts
- **ADR system introduced**: Architectural Decision Records for design trade-offs
  - ADR-005: skill scoping and hook enforcement
  - ADR-008: swarm-v2 architecture
  - Signals: design being formalized, swarm infrastructure being designed in prose first

- **Flaky test acknowledgment**: "mark all flaky LSP integration tests... as ignored"
  - Pragmatic acceptance of flakiness while fixing root causes

- **Correctness focus**: Mutation testing, error recovery, panic elimination
  - Quality-over-velocity mindset

- **January Jules bridge**:
  - `google-labs-jules[bot]` authored roughly 210 all-ref commits between 2026-01-16 and 2026-01-30
  - The same window also contains "superseded", revert, and maintainer-improvement commits plus Bolt/Sentinel/Palette follow-up work
  - Signal: draft PRs are entering the workflow, but acceptance and integration are still highly hands-on

### Velocity Pattern
Dramatic drop: 423 commits over 5 months (~2.8/day vs ~14/day in Era 1). Only 91 commits on peak day (2026-02-28), suggesting slower, more deliberate work. This is the **slowest era**.

### Architectural Milestones
- **Parser v3 (Recursive Descent)**: Born from error recovery requirements
- **Skill scoping system**: Designed via ADR (hook enforcement, agent responsibilities)
- **Swarm v2 architecture**: Designed in prose via ADR-008
- **Mutation testing**: Integrated into development workflow

### Verdict
**Architecture and correctness phase**. Velocity plummets (423 commits / 5mo vs 947 / 2mo in Era 1), but this phase is notably high-quality, stable, and consistent. The tradeoff is that it remains too hands-on: the human is still carrying a large share of the architecture, coordination, and integration burden directly. January's Jules draft-PR burst shows the system probing for more delegation, but the swarm infrastructure is still being *designed* here rather than allowed to operate at full scale.

---

## Era 4: Copilot CLI Fleet Mode (Late February – March 5, 2026)
### "GitHub Copilot CLI Fleet/Autopilot, 'Firehose of Tokens'"

**Date Range**: 2026-02-27 to 2026-03-05, with the first `Co-authored-by: Copilot` trailers appearing on 2026-02-28
**Duration**: 7 days
**Total Commits**: 255 merged commits on `master`
**Average Velocity**: ~36 commits/day
**Peak Day**: 2026-03-04 (152 commits)

### Commit Patterns
- **Conventional commits**: Dominant across the merged burst
  - Nearly every commit is "fix(X): ...", "feat(Y): ...", "chore(Z): ..."
  - Suggests a strongly tool-shaped workflow even when final merges land under Steven's identities

- **Branch naming**: Massive codex/ prefix proliferation
  - `codex/describe-architecture-decisions`
  - `codex/document-apparent-adrs-in-codebase`
  - Consistent naming suggests automated branching

- **Commit velocity**: **Firehose pattern**
  - 255 merged commits in 7 days = **~36 commits/day average**
  - Peak day: 152 commits on 2026-03-04
  - Suggests high-parallelism, short-lived PRs

- **Commit message uniqueness**: High variation, many similar but not identical
  - Suggests: code-gen + human-review cycles, not pure automation

### Contributors
- **Steven Zimmerman / Steven Zimmerman, CPA** (attributed for merges on `master`)
- **GitHub Copilot attribution via `Co-authored-by: Copilot` trailers** across the burst even when merged commits land under Steven's identities
- **Dependabot** continues in the background but is not the defining signal for this era

### Key Work
1. **ADR coverage expansion**: Massive documentation push
   - "document-apparent-adrs-in-codebase"
   - "catalog ADR coverage"
   - "improve ADR discoverability"
   - 10+ ADR-related commits in short period

2. **Diagnostic improvements**: Perl modernization suggestions
   - "add Perl version compatibility warnings"
   - "add helpful fix suggestions to error messages"
   - Error-to-action mapping

3. **Performance tuning**: Workspace-scale optimization
   - "tune for CPAN-scale workspaces (#1664)"
   - Global reference index for O(1) lookups
   - Progress reporting for workspace indexing

4. **Code cleanup**: Dead code elimination, formatting
   - "remove dead code before 0.12.0 release"
   - "auto-format with cargo fmt" (repeated many times)

### Distinctive Artifacts
- **Copilot signature**:
  - The first all-ref `Co-authored-by: Copilot` trailers appear on 2026-02-28 and then dominate the burst
  - High merged-commit volume in a one-week window (~36/day, with a 152-commit peak day)
  - Conventional commit format enforcement
  - Branch names start with `codex/`
  - Similar but varied commit messages (not template-based)
  - Short, intense burst rather than a long steady phase

- **Quality gate enforcement**:
  - "Merge pull request #XXX" pattern
  - Steven doing all final merges
  - No bot-direct merges

- **Documentation as artifact**:
  - ADR system being populated (not designed anymore, but catalogued)
  - Architecture becoming documented as-built rather than as-designed

### Velocity Pattern
Peak merged velocity reaches **255 commits / 7 days** (~36/day), with a single-day spike of **152 commits on 2026-03-04**. The defining shape is a compressed burst, not a long plateau.

### Architectural Milestones
- **v0.12.0 polish**: Features implemented and documented
- **Workspace indexing**: CPAN-scale performance achieved
- **Error-to-action**: LSP diagnostic improvements
- **ADR catalog**: Architecture decisions formalized and indexed

### Verdict
**Copilot CLI mass-production burst**. Clear firehose pattern: short-lived branches, high parallelism, consistent naming, and GitHub Copilot trailers beginning on 2026-02-28. Steven remains the merge bottleneck, but now the burden is selecting and integrating from a very dense PR stream rather than doing the work directly.

---

## Era 5: Claude Code Agent Teams (March 11–19, 2026)
### "Native Claude Agents in Worktree Isolation"

**Date Range**: 2026-03-11 to 2026-03-19
**Operating Mode**: A couple of short Claude Code swarm runs, well under 20 hours total, interleaved with Codex CLI PR waves
**Tool Mix**: Claude Code agent teams + Codex CLI batches (often in sets of 4 PRs)
**Usage Pattern**: Sometimes cleanup/merge Codex output, other times scout issues and build directly

### Commit Patterns
- **Conventional commits**: Near-universal on merged commits during the session bursts
  - High consistency, near-perfect format adoption
  - The exact author mix is less informative here than the branch/task artifacts around each session

- **Branch naming**: **Worktree pattern emerges**
  - `worktree-agent-ab615443`
  - `worktree-agent-a638b42e`
  - `worktree-agent-a90d7ded`
  - Hash-based naming suggests automated worktree naming from agent IDs

- **Session cadence**:
  - A couple of concentrated Claude swarm runs inside the March 11–19 window
  - Codex CLI PR waves continue in parallel, often in sets of four
  - Suggests: Era 5 is a mixed-tool coordination phase, not a continuously running single-tool swarm

- **Commit message style**:
  - Verb-first, conventional: "chore(swarm): add X", "fix(parser): handle Y", "test(perl-Z): add W"
  - Highly uniform suggests templated generation
  - Many reference skills: "add /scout-then-build skill", "encode cycle 5 learnings"

### Contributors
- **Steven Zimmerman / Steven Zimmerman, CPA** dominate the author field in this window
- **Author names are not the primary signal**: `git log --all` for 2026-03-11..2026-03-19 shows no `google-labs-jules[bot]` commits
- **The defining artifacts are structural instead**: 44 remote `worktree-agent-*` branches plus concurrent Codex CLI PR waves

### Key Work
1. **Swarm infrastructure maturation**:
   - "/scout-then-build skill" (#2118)
   - "/merge-queue skill" (#2121)
   - "cycle 5 learnings into skills and templates" (#2116, #2123)
   - "smart orchestrator model" (#2181)
   - "enhancement-builder prompt template" (#2183)

2. **Parser fixes** (constrained agents):
   - "handle complex expressions in parenthesized arguments (#1704)"
   - "recognize fat-arrow in function argument lists (#2147)"
   - "improve delimiter recovery to reduce cascading errors"

3. **Corpus ratcheting**:
   - Baseline improvements visible in commits
   - Error bucket analysis and fixes

4. **Distribution features**:
   - "add PowerShell completion generation"
   - "add man page generation"
   - "cargo-binstall configuration"

5. **Code quality**:
   - "exclude agent worktrees from rust-analyzer (#2031)"
   - Dead code cleanup (#2126)
   - Pre-push hook improvements (#2128)

### Distinctive Artifacts
- **Worktree isolation visible**:
  - Branch names like `worktree-agent-HASH` prove per-agent isolation
  - Enables 50-100 parallel agents without branch conflicts
  - Zero circular dependency issues (enforced by arch)

- **Skills as primitives**:
  - Agents invoke `/verify-build`, `/parser-fix`, `/scout-then-build`
  - Skills reusable across agents
  - Memory system persists learnings across cycles

- **Agent lineage still on disk**:
  - `.claude/agents5/` and `.claude/agents6/` capture the evolution toward persistent teammate roles
  - `.claude/agents/` is effectively the `agents7` layer
  - The current swarm lives across `.claude/agents/`, `.claude/commands/`, `.claude/skills/`, and `.claude/hooks/`
  - `.claude/swarm-state/` serves as the current-ish state/documentation layer around that control plane

- **Batch merging pattern**:
  - 321 all-ref commits (2026-03-18), 250 (2026-03-15), 238 (2026-03-19)
  - Suggests: session bursts create large artifact waves, then merge/triage passes process them in batches
  - "run /green-merge for Batch 0" visible in task names

- **100-agent session signal**:
  - Cycle 5 memory reports "~100 agents deployed"
  - The 100-agent scale belongs to specific session bursts, not the full March 11–19 window
  - Most agents complete 1-2 features per session

### Velocity Pattern
**Not directly comparable to Era 4 on a simple commits/day basis.** Era 5 mixes short Claude swarm sessions, Codex CLI PR waves, and merge-cleanup work in the same March 11–19 window. The key signal is not a daily average. It is the shift to session-based orchestration with cleaner routing and better downstream triage.

### Architectural Milestones
- **Swarm orchestration complete**: Coordinator + workers model encoded
- **Agent lifecycle documented**: Spawn focused, shutdown when done, respawn fresh
- **Skills library stabilized**: 8 reusable skills, 10 commands, 48 hooks
- **Memory persistence**: 30+ memory files surviving across sessions
- **Worktree isolation proven**: 100 agents, zero conflicts

### Verdict
**Mature swarm era**. This is a mixed-tool, session-based phase: a couple of short Claude Code swarm runs inside the March 11–19 window, plus ongoing Codex CLI PR waves. The important improvement is control. Every agent has its own worktree branch, the Claude side uses skills rather than raw prompts, and the human can alternate between cleanup/merge sessions and scout/build sessions. This is **purpose-built infrastructure**, not generic automation.

---

## Comparative Analysis Across Eras

### Velocity Trajectory
```
Era 1 (Opus):        947 commits / 2mo   = 14.2 commits/day
Era 2 (Early Swarms): 840 commits / 3mo   =  9.3 commits/day
Era 3 (Sidechain):   423 commits / 5mo   =  2.8 commits/day [SLOWEST]
Era 4 (Copilot):     255 commits / 7d   = 36.4 commits/day
Era 5 (Claude Code): session-based mixed-tool bursts (not directly comparable)
```

**Peak merged average among continuous phases**: Era 4 (Copilot CLI) at 36/day
**Peak all-ref artifact day**: Era 5 with 321 commits on 2026-03-18
**Most sustainable**: Era 5, because the swarm is used selectively instead of running as a constant firehose
**Most deliberate**: Era 3 at 2.8/day (architecture + mutation testing phase)

### Conventional Commit Adoption
```
Era 1: 1,213 / 947   = 128% (duplicates, early stage)
Era 2: ~50% (mixed, mostly natural language)
Era 3: ~40% (still exploring)
Era 4: dominant on merged commits (Copilot CLI burst)
Era 5: near-universal on merged commits
```

**Insight**: Tool-shaped workflows push Era 4 toward conventional commits. Mature agent teams (Era 5) achieve high adoption naturally while keeping the output easier to reason about.

### Branch Naming Evolution
```
Era 1: Direct commits + long-lived features
Era 2: codex/* prefix emerges (~80% of branches)
Era 3: codex/* dominates, some descriptive branches
Era 4: codex/* pervasive (~95% of branches, Copilot signal)
Era 5: worktree-agent-HASH (isolated, non-conflicting)
```

**Critical transition**: Era 5 abandons generic naming for **deterministic hash-based naming**, enabling N agents on same codebase.

### Merge Pattern Evolution
```
Era 1: Manual conflict resolution, long commits
Era 2: "Merge branch 'master' into codex/X" (frequent rebases)
Era 3: Fewer merges overall (architecture phase)
Era 4: "Merge pull request #XXX" (Steven doing final merges)
Era 5: "Merge pull request #YYYY from EffortlessMetrics/worktree-agent-Z"
```

**Insight**: Steven remains human bottleneck in all eras (does final merge), but infrastructure evolves to reduce conflicts (worktree isolation in Era 5).

### Test Infrastructure Evolution
```
Era 1: Heavy incremental parsing tests
Era 2: Corpus tests, highlight tests
Era 3: Mutation testing, flaky test isolation
Era 4: ADR coverage, diagnostic tests
Era 5: Test infrastructure mature, focus on pest vs v3 comparison
```

### Contributor Count
```
Era 1: 1 (Steven only)
Era 2: 2 (Steven + variant)
Era 3: 3 (Steven + Paul Evans joins)
Era 4: 3 (Steven + Copilot bot + variants)
Era 5: Steven variants plus mixed-tool branch artifacts; author fields matter less than worktree/PR evidence
```

**Surprise**: No external community PRs visible. All work is internal (human + AI).

---

## Technological Transitions

### Parser Version Lineage
- **Era 1-2**: v1 (C) being wrapped, v2 (Pest) being improved
- **Era 3**: v3 (Recursive Descent) emerging, v2 becoming secondary
- **Era 4-5**: v3 dominates, v2 kept for benchmarking, v1 archived

### Architecture Decision Documentation
- **Era 1**: No ADRs
- **Era 2**: No explicit ADRs, but "codex/" suggests automated planning
- **Era 3**: ADRs designed and written (ADR-005, ADR-008)
- **Era 4**: ADRs indexed and catalogued (documentation push)
- **Era 5**: ADRs used as reference, not created (stable architecture)

### Skill System Evolution
- **Era 1-3**: No skills (monolithic agent)
- **Era 4**: Emerging (infrastructure preparation)
- **Era 5**: 8 skills, 10 commands, 48 hooks (mature)

---

## Key Insights

### 1. **Velocity Is Not Linear**
- Opus direct coding (Era 1-2): ~14 commits/day
- Copilot fleet (Era 4): ~36 merged commits/day, with a 152-commit peak day
- Claude agents (Era 5): session-based bursts, not a continuous daily cadence
- **Lesson**: Not "faster is better." Era 5's selective Claude runs are more sustainable and higher-quality than a continuous firehose.

### 2. **Architecture Phase Required Slowdown**
- Era 3 (2.8 commits/day) was intentional: ADRs written, mutation tests added, refactoring done.
- This "slowdown" enabled Era 4-5 to sustain high velocity without quality degradation.
- **Lesson**: Build infrastructure before scaling.

### 3. **Branch Naming as Evolutionary Signal**
- `codex/` = Copilot era (generic code-generation tool)
- `worktree-agent-HASH` = Claude era (isolated agents, deterministic naming)
- **Lesson**: Tool choice reveals in branching strategy.

### 4. **Merge Queue is the Bottleneck**
- 3-wide merge queue means ~9 optimal coding agents
- Era 5 spawned ~100 agents; excess overflow to GitHub issue queue
- **Lesson**: Don't scale agents beyond CI capacity.

### 5. **Steven Remains Human Bottleneck**
- In all 5 eras, final merges are done by Steven
- Copilot (Era 4): Steven merges Copilot's work
- Claude agents (Era 5): Steven merges agent work
- **Lesson**: Scaling agents ≠ scaling human review. Steven is the quality gate.

### 6. **Bot vs Agent Distinction**
- **Copilot (Era 4)**: Generates code at high volume, human selects/merges
- **Claude agents (Era 5)**: Plan-then-execute, constrain scope, preserve institutional knowledge via memory
- **Lesson**: Agents > bots for sustained development.

---

## Amusing Artifacts & Artifacts

### Commit Message Gems

**Era 1** (Opus-as-helper):
- "final: apply pr-cleanup-agent fixes for production-ready merge"
  - _Agent references appearing in Era 1 commits!_

**Era 2** (Merge chaos):
- 4+ commits on same day resolving same conflicts
- "Merge branch 'master' into codex/X", then 5 mins later "Merge branch 'master' into codex/X" again
  - _Suggests rebase loops, not moving forward_

**Era 3** (Philosophical):
- "restore architectural integrity for Issue #146"
  - _Suggests refactoring for principle, not just velocity_

**Era 4** (Copilot flood):
- "auto-format with cargo fmt" (repeated 50+ times)
- "docs: expand ADR coverage summaries" (repeated)
  - _Suggests Copilot re-doing same work, Steven selecting best version_

**Era 5** (Meta):
- "chore(swarm): add /merge-queue skill"
- "chore(swarm): encode cycle 5 learnings into skills and templates"
- "docs: add ADR topic guide to improve ADR discoverability"
  - _Commits about the infrastructure that creates commits_

### Branch Name Evolution
- `codex/implement-is_in_hash_key_context-function` (2025-08-31)
- `codex/finish-incremental-parsing-implementation` (2025-08-31)
- `codex/extend-dead_code_detector-to-analyze-code` (2025-09-05)
- `worktree-agent-ab615443` (2026-03-05) ← Paradigm shift!

### Hidden Signals
1. **"final: apply pr-cleanup-agent" in Era 1** (2025-08-31)
   - Agents existed before Era 2 officially started
   - Suggests Steven testing agents before deployment

2. **Copilot-swe-agent[bot] with only 1 commit** (Era 4)
   - Bot registered but Steven does all merges
   - suggests experimental/trial phase for Copilot

3. **Archived agents (54 definitions)**
   - `.claude/agents4/`: canonical Q3 three-phase swarm
   - `.claude/agents5/` and `.claude/agents6/`: transition/evolution layers
   - `.claude/agents/`: effectively the current `agents7` layer
   - The live control plane now spans agents + commands + skills + hooks, with `swarm-state/` as current-ish documentation/state

4. **Phantom bug in cycle 5**
   - Scout analysis tool itself had edge cases
   - Found "bucket #5" that wasn't real
   - Discovered DURING cycle 5, fixed in-place

---

## Release Markers

### v0.9.1 → v0.10 → v0.12.0
- **v0.9.1 close-out** (Era 3, late 2025): Mutation testing, flaky tests
- **v1.0 readiness** (Era 3 → 4): Architecture decisions + "v0.10 readiness"
- **v0.12.0 public alpha** (Era 5, 2026-03-19):
  - 97 LSP features
  - 80% CPAN corpus clean
  - Full swarm infrastructure
  - Ready for public launch

---

## Conclusion: Five Eras Tell a Story

| Era | Paradigm | Velocity | Quality | Lesson |
|-----|----------|----------|---------|--------|
| 1 | Opus Direct | 14/day | High | Single genius, slow |
| 2 | Early Swarms | 9/day | High | Coordination overhead |
| 3 | Sidechain | 2.8/day | Very High | Architecture requires slowdown |
| 4 | Copilot Fleet | 36/day | Medium | Volume ≠ quality |
| 5 | Claude Agents | Burst-based | High | Infrastructure-aware scaling |

**The arc bends toward...**
Well-designed infrastructure (worktree isolation, skills, memory, ADRs) enables selective, controlled parallelism. That matters more than raw throughput because it preserves quality, reduces merge conflict pressure, and captures institutional knowledge.

**The future**: Era 5 is the stable state. Scaling beyond 100 agents requires either (a) removing Steven as bottleneck via better CI tooling, or (b) embracing issue queue overflow as intentional.
