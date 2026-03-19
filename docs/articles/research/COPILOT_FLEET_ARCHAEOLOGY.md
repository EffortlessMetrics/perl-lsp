# Copilot Fleet Archaeology
## The February 2026 Firehose Before The Claude Swarm

This note traces the Copilot CLI fleet/autopilot era from the tracked commit stream.

The main question is simple:

- did the burst begin on 2026-02-26 or 2026-02-27?
- what changes once `Co-authored-by: Copilot` starts appearing in the commit bodies?
- how does this era differ from the later Claude Code swarm?

The commit history answers those questions more cleanly than memory does.

---

## 1. The Burst Starts On 2026-02-27, Not 2026-02-26

I found no tracked commits dated 2026-02-26 in the relevant slice.

The first visible burst day in the commit stream is **2026-02-27**. That day already contains:

- direct feature and fix commits
- release-chain prep
- performance/security work split across named concern branches
- merge commits from local worktree branches back into `origin/master`

Representative 2026-02-27 commits:

- `c38a89a52` - `Bolt: Optimize symbol extraction regex compilation`
- `7192b776e` - `Sentinel: Fix path traversal in debug adapter launch`
- `e93ae7cb1` - `Sentinel: Fix safe evaluation bypass for iterator/IO ops`
- `9d4f58062` - `perf(semantic-analyzer): avoid deep cloning AST nodes in subroutine analysis`
- `3f06b4bd8` - `Sentinel: Fix Argument Injection in TestRunner`
- `e11378ef9` - `perf(scope-analyzer): optimize unused parameter detection`
- `7031c2f48` - `feat(ux): add context-aware states to status menu`
- `679ecacb8` - `Add inlineValues lifecycle coverage`

The same day also shows merge commits into concern-named branches such as:

- `bolt/optimize-symbol-extractor-regex-...`
- `palette-context-aware-status-menu-...`
- `palette-improve-run-tests-ux-...`
- `palette-ux-run-tests-context-...`
- `perf/optimize-regex-compilation-...`
- `sentinel/fix-argument-injection-...`

That branch naming matters. The work is not yet the later stateful swarm model. It is still a burst of concern-specific worktrees and rebases.

Maintainer recollection, consistent with the branch-family shape here: some of these concern-lane PRs may have originated from the January Jules-style backlog and then been reviewed or merged during the Copilot CLI maintainership burst. The git history alone does not settle that provenance with certainty, so this should be read as informed context rather than a proved claim.

---

## 2. 2026-02-28 Is The Release Campaign Switch

The next day changes shape.

On **2026-02-28**, the log pivots into a much denser release/public-crates-io campaign:

- release readiness and orchestration hardening
- v0.10.0 version bump and release candidate work
- workflow hardening
- publish allowlist expansion
- docs and README refreshes
- microcrate inventory and publishing support

Representative 2026-02-28 commits:

- `107cdc516` - `release: crates.io public release readiness and orchestration hardening (#871)`
- `77fd4978b` - `release: v0.10.0 release candidate - build fixes, version bump, code quality (#881)`
- `7200c142b` - `release: bump workspace package/dependency versions to 0.10.0 (#879)`
- `4a1e76a7c` - `Add SRP microcrate discovery command`
- `0ed1798e4` - `xtask: add SRP microcrate inventory report (#933)`
- `797fff93f` - `chore(release): align turnkey PR-driven 0.x.y workflow`
- `9ccc58fed` - `fix(release): make turnkey orchestrator base-branch agnostic`

This is not just more volume. It is a different batch shape:

- more PR-style units
- more release plumbing
- more generated or campaign-driven work
- less of the small concern-lane cadence seen on 2026-02-27

The release campaign is the best evidence that the 2/27 firehose was already in motion, but 2/28 is where it becomes a coordinated public-release push.

Maintainer recollection, framed as inference: this is likely the point where the remaining Jules-style backlog had largely cleared and Copilot CLI shifted toward broader maintainership goals. That fits the visible pivot from concern-lane work into release orchestration, workflow hardening, and public-release preparation.

---

## 3. The Copilot Trailer Boundary

The first visible `Co-authored-by: Copilot` trailers appear in the 2026-02-28 batch, not on 2026-02-27.

The `git log` slice from 2026-02-27 through 2026-03-01 shows the trailer lines in the 2026-02-28 batch. That is the point where the attribution style becomes explicit in the commit bodies.

By 2026-03-04, the trailer is routine on the extended-test series:

- `ce4c1b268` - `test(dap-platform): add extended unit tests`
- `d404e05fc` - `test(lsp-formatting): add extended unit tests`
- `092d11313` - `test(lsp-inlay-hints): add extended unit tests`
- `293216b8c` - `test(lsp-rename): add extended unit tests`
- `3fcac012b` - `test(lsp-semantic-tokens): add extended unit tests`

Example body shape from that phase:

```text
test(lsp-semantic-tokens): add extended unit tests

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

The significance is not merely attribution. It marks a shift in how the work was produced:

- the commits are still human-authored at the top level
- Copilot is being attached as an explicit co-author
- the branch stream is now PR-shaped and reviewable in the open

That is different from the later Claude swarm, where the control plane itself is versioned and the work is split across commands, skills, hooks, and swarm-state rather than just co-authored commits.

Maintainer recollection: some of these trailer-bearing commits may have continued landing through PRs after the main Copilot swarm push had already tapered off. That would explain why the authorship pattern stays visible even as the burst changes shape.

---

## 4. Branch Shape And Throughput Pattern

The Copilot fleet era looks like a firehose, but it is not a random one.

The branch patterns show three distinct layers:

1. concern-specific local worktree branches on 2026-02-27
2. a release/public-crates-io aggregation flow on 2026-02-28
3. PR-batched, trailer-bearing work on 2026-03-04 and 2026-03-05

The branch names themselves reveal the operating style:

- `bolt/...`
- `palette/...`
- `sentinel/...`
- `perf/...`
- `release/public-crates-io-master`
- `codex/...`

That is the shape of a high-throughput campaign that still depends on a human merge bottleneck.

It is also why the Copilot era feels messier than the later Claude Code swarm:

- Copilot era: many short-lived branches, high PR volume, release crunch, human-managed merge flow
- Claude swarm: fewer visible bursts, more durable control-plane surfaces, persistent state, and explicit orchestration contracts

The Copilot era is about throughput.
The Claude era is about operability.

One maintainer distinction is worth preserving here: Copilot CLI was not only generating code, it was also directly reviewing, improving, and merging PRs. The difference from the later Claude Code swarms was not capability in the abstract, but control. The Claude-era control plane exposed more explicit levers around roles, isolation, memory, and enforcement.

---

## 5. Why This Differs From The Later Claude Swarm

The later Claude swarm does not just run faster. It changes the unit of organization.

In the Copilot fleet era, the primary unit is still the branch and the PR:

- create worktree branch
- generate changes
- attach Copilot co-authorship
- merge through release or integration batches

In the Claude era, the primary unit becomes the control-plane surface:

- agents
- commands
- skills
- hooks
- swarm-state

That difference shows up in the archaeology:

- Copilot era branches are named around the work
- Claude-era surfaces are named around the operating model
- Copilot-era history is mostly batch PR flow
- Claude-era history is institutional memory plus procedural reuse

The Copilot firehose mattered.
It just was not yet a self-describing swarm.

---

## 6. Evidence Pointers

Relevant commits and slices:

- `c91ff2588` - `chore: preserve rebased local history from pre-merge line (#862)` on 2026-02-27
- `c38a89a52`, `7192b776e`, `e93ae7cb1`, `9d4f58062`, `3f06b4bd8`, `e11378ef9`, `7031c2f48`, `679ecacb8` - 2026-02-27 concern-lane burst
- `38c1772e5`, `9bfceaa4c`, `c0f6fea90`, `10746c0ef`, `82554923e`, `26117573b`, `a51b92cf8` - 2026-02-27 merge commits into named concern branches
- `107cdc516`, `77fd4978b`, `7200c142b`, `4a1e76a7c`, `0ed1798e4`, `797fff93f`, `9ccc58fed` - 2026-02-28 release/public-crates-io campaign
- `ce4c1b268`, `d404e05fc`, `092d11313`, `293216b8c`, `3fcac012b` - 2026-03-04 trailer-bearing extended-test commits

The bottom line is stable:

- the burst begins on 2026-02-27
- 2026-02-28 is the release-campaign escalation
- the explicit Copilot trailer style is visible in the later batch stream
- the later Claude swarm is a different operating model, not just a larger Copilot run
