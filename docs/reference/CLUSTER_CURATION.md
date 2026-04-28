# Cluster Curation — Codex/Jules Ensemble Methodology

**Audience:** Ensemble-curator agents, orchestrators, and reviewers handling high-volume
external-AI-agent PR inflow (Codex 4-shot bursts, Jules single-prompt runs, Hermes
planning artifacts, Factory Droid, Aider, Claude Code).

**Related agents:** `ensemble-curator` (`.claude/agents/ensemble-curator.md`)  
**Related skills:** `/ensemble-detect`, `/cluster-triage`, `/hallucination-check`  
**Related articles:** `docs/articles/FOUR_WAY_ENSEMBLE_PATTERN.md`,
`docs/articles/BROAD_SCOPE_LAYER_DIVERSITY.md`,
`docs/articles/CODEX_HALLUCINATION_TRIAGE.md`  
**Cross-references:** #7073 (salvage-classify skill), #7061 (typed routing labels),
`docs/articles/FOUR_WAY_ENSEMBLE_PATTERN.md` (ensemble vocabulary and Monte Carlo framing)

---

## 1. What a Cluster Is

A **cluster** is a set of N >= 2 PRs generated from the same external-agent prompt or
task dispatch — intentionally or accidentally — that address the same feature area.

### Accidental vs. intentional clusters

| Type | Description | Default handling |
|---|---|---|
| **Intentional ensemble** | One surface deliberately spawns N independent implementations of the same spec ("4-shot is search, not waste"). PRs may diverge in approach, structure, or scope slice. | File-path triage → pick winner(s) → extract loser edges → cross-pollinate |
| **Accidental duplication** | Two operators or runs independently tackle the same narrow fix. PRs converge on the same diff. | Pick winner (usually the earlier one) → close dupes after extracting any novel tests |

The distinction matters because intentional ensemble outputs are **design-space exploration**.
Each PR asks "where does this problem live?" — and the variance across answers is signal, not
waste. Closing them all without reading means discarding a free architectural map. See
`FOUR_WAY_ENSEMBLE_PATTERN.md` for the full Monte Carlo framing.

### Cluster vs. concurrent work

Two PRs touching disjoint files on disjoint features at the same time are **not** a cluster.
They are concurrent single-threaded pipelines running in parallel. The key diagnostic is
overlap in file paths (see section 3).

---

## 2. Detection

Before triaging any PR from an external-agent author, run `/ensemble-detect`. The full
algorithm is in `.claude/commands/ensemble-detect.md`. The four signals in priority order:

### 2a. Shared task ID in body

Codex embeds a `task_e_<id>` marker in the PR body. All PRs from one Codex dispatch share
the same ID. Find siblings:

```bash
TASK_ID=$(gh pr view <N> --json body -q .body | grep -oE 'task_e_[a-z0-9]{8,}' | head -1)
gh pr list --state open --limit 100 --search "$TASK_ID" --json number,title
```

### 2b. Creation-time burst

PRs from the same source created within 10-15 minutes of each other:

```bash
gh pr list --state open --limit 100 --json number,createdAt,author \
  --author <author> --jq '.[] | select(.createdAt > "WINDOW_START" and .createdAt < "WINDOW_END") | .number'
```

### 2c. Branch-name pattern

External-agent branches follow predictable patterns with shared prefixes and varying suffixes:

| Source | Pattern |
|---|---|
| Codex | `codex/improve-<topic>-<suffix>` |
| Jules | `jules/<topic>` |
| Hermes | `hermes/<topic>` |
| Factory Droid | `droid/<topic>` |

Siblings share the `codex/improve-<topic>` prefix with different suffixes.

### 2d. Title stem match

Titles differing only by a stem verb (`add` / `improve` / `expand` / `support`) against
otherwise identical structure are cluster siblings.

### Detection output

When you find a cluster, note the cluster size N, the shared topic, and the task_id (if
present). Route the whole cluster to section 3 before processing any individual PR. Processing
cluster PRs one at a time produces contradictory verdicts and burns N times the triage cost.

---

## 3. Curation Steps

### 3a. Map files per PR first

**File-path triage comes before title triage.**

```bash
for pr in <PR-LIST>; do
  echo "=== PR #$pr ==="
  rtk gh pr diff $pr --name-only
done
```

Sort the file lists. PRs with disjoint file sets are **layer-diverse siblings** — they
explore different parts of the problem, not different solutions to the same part.

### 3b. Categorize: layer-diverse vs. duplicate

| Category | Definition | Action |
|---|---|---|
| **Layer-diverse siblings** | PRs touch non-overlapping files across different crates or layers | Keep ALL — merge independently through the normal pipeline |
| **Same-layer duplicates** | 2+ PRs touch the same file and the same code region | Pick winner, close losers (section 5) |
| **Overlap-but-not-identical** | Some shared files + some unique files | Winner is the PR with the fuller implementation on shared files; unique-file content may still merge separately |

Example: the 2026-04-24 encoding cluster had 12 PRs. Only 3 (`#5740`, `#5741`, `#5743`)
were genuine duplicates on `workspace.rs`. The other 9 each touched a different layer —
URI parsing, CLI binary, code-actions pragma, position tracking, etc. — and all kept.
See `docs/articles/BROAD_SCOPE_LAYER_DIVERSITY.md` for the full breakdown.

### 3c. Pick winner for same-layer clusters

Rank duplicates by these signals, in order:

1. **Completeness** — implements the full spec, not half. Check PR body acceptance criteria.
2. **Safety** — uses `Result<()>` / `?`; no `unwrap()` / `expect()` in production code.
3. **Test coverage** — has regression tests, not only happy-path.
4. **API tightness** — no unnecessary `as usize`, no cloning where borrow works, no
   duplicated helper code.
5. **Commit cleanliness** — clean linear history vs. "fix fix fix" tail commits.

When signals are tied, prefer the earlier PR (stability bias).

### 3d. Phantom-drift pre-check on winner

Before declaring a winner, verify it does not carry a known phantom-drift file.

The canonical example: `crates/perl-dap/src/eval/validator.rs:89` (assignment operator
precedence fix) repeatedly appeared in unrelated Codex sub-tasks during the 2026-04-26
`#6853` cluster. Three independent curators caught it. An otherwise clean PR that also
carries this unrelated hunk is classified `SCOPE-DRIFT`, not `ALIGNED`.

Procedure:

```bash
gh pr diff <WINNER> --stat
# Inspect files that don't match the PR's stated scope
```

If a phantom-drift file is found: comment on the PR identifying the specific paths, leave
the PR open with a `SCOPE-DRIFT` verdict, and request the builder strip the out-of-scope
changes. Do not promote a scope-drifted PR to winner status.

---

## 4. Loser Harvest

Before closing any loser, read its diff for contributions the winner lacks. This
cross-pollination step prevents losing genuine value embedded in losing variants.

Extract and preserve:

- **Edge case tests** — test cases for variants the winner's test suite doesn't exercise
  (empty input, CRLF line endings, non-BMP Unicode, odd-length UTF-16, etc.)
- **Documented gotchas** — inline comments explaining failure modes, edge conditions, or
  design tradeoffs the winner's code lacks
- **Alternative implementations** — simpler approaches, different abstractions, or
  trait-based designs worth considering (note even if not adopted)
- **Failure-mode coverage** — error paths the winner doesn't test
- **Prompt improvement observations** — if the loser's divergence reveals a spec
  ambiguity, note it as a prompt improvement for the next dispatch

Extraction actions:

| What was found | Action |
|---|---|
| Test worth adding | Post the specific test code as a comment on the winner's PR: "Extracted from #`<loser>`; recommend adding" |
| Non-trivial edge case | Open a follow-up issue |
| Simpler implementation idea | Note in closure comment; winner author decides |
| Spec ambiguity revealed | Post on the triggering issue as a prompt improvement note |

After a cluster of 4+ PRs on a single feature, post a synthesis comment on the winning PR
summarizing: what layers the cluster covered, what approach won and why, what alternatives
were tried. This synthesis comment is the permanent record of the ensemble's design search.

---

## 5. Closure Pattern

Close losers with a structured comment that cross-references the winner, states what was
harvested (or what follow-ups were opened), and acknowledges the contribution.

Template:

```
Closing as REDUNDANT — #<WINNER> implements the same scope with <reason: more
complete / cleaner tests / fewer unsafe casts>.

What was extracted from this PR:
- `test_empty_input_edge_case`: posted on #<WINNER> as a follow-up addition
- Alternative trait-based approach: considered; winner's direct-dispatch
  is simpler for this hot path

Thank you for the contribution — the ensemble perspective helped surface the
right implementation approach.
```

### Typed routing labels (#7061, #7073)

Use one of these typed routing verdicts for every cluster disposition:

| Verdict | When |
|---|---|
| `SALVAGE_REBASE` | Winner is stale-based (branched before a cascade); rebase on current master and continue |
| `CHERRY_PICK` | Winner's approach is right but commits are entangled; cherry-pick specific commits onto a clean branch |
| `EXTRACT_TESTS` | Loser has useful tests worth adding to the winner; extract before closing |
| `EXTRACT_IMPL` | Loser has a better implementation of one function; extract and add to winner |
| `CLOSE_SUPERSEDED` | Loser is unambiguously superseded by winner; close after harvest |
| `CLOSE_OBSOLETE` | Premise obsolete (feature was already merged, or spec changed); close with explanation |
| `NEEDS_MAINTAINER_CALL` | Two PRs are intentional complements (sibling-pair pattern, section 7); escalate rather than pick |

The `SALVAGE_REBASE` and related verdicts come from the salvage-classifier doctrine:
**stale or dirty does not mean close-by-default**. Calculate rescue cost vs.
reimplementation cost. Default to preserving value.

The 2026-04-26 session example: 16 PRs classified into 11 `SALVAGE_REBASE` /
3 `CLOSE_SUPERSEDED` / 2 `NEEDS_MAINTAINER_CALL`. The 11 salvages produced usable PRs;
closure-by-default would have discarded them.

---

## 6. Hallucination Patterns

A distinct failure mode of the ensemble pattern: Codex produces coherent, clippy-clean,
well-tested Rust code that teaches perl-lsp to recognize a **name from its training
periphery** (an AI editor, a JS visual builder, an agentic coding tool) as a Perl framework
or CPAN module.

The code is idiomatic. The tests pass. The violation is against the **Perl ecosystem** — the
named module does not exist on CPAN.

### The four-PR hallucination shape

Codex tends to produce hallucinated framework support in clusters of 4, each covering a
different layer:

1. `feat(parser-core): add <Name> template extension support` — adds to `PERL_SOURCE_EXTENSIONS`
2. `feat(semantic-analyzer): add <Name> web route detection` — adds `WebFrameworkKind::<Name>`
3. `feat(semantic-analyzer): add <Name> framework aliases` — treats `use <Name>` as Moo-family
4. `fix(execute-command): skip <Name> modules in go-to-implementation`

### MetaCPAN pre-verification (mandatory)

Any PR adding to these tables requires MetaCPAN verification before it can be promoted:

- `WebFrameworkKind` enum
- `IMPLICIT_STRICT_MODULES` list
- `IMPLICIT_EXPORT_SKIP_LIST`
- `COMMON_MODULES_TIER_1`
- `PERL_SOURCE_EXTENSIONS`
- `detect_framework` / `update_framework_context` dispatch tables

```bash
curl -s "https://fastapi.metacpan.org/v1/module/_search?q=<Name>&size=3" \
  | jq '.hits.total'
```

Zero results + the name matches a known AI product → close as `HALLUCINATED`.

### Known hallucination seeds (2026-04-23/24)

| Fake Perl thing | What it actually is |
|---|---|
| OpenClaw | Agentic coding editor |
| Droid / Droid::Factory | Factory.ai terminal agent |
| Builder::IO::Fusion | builder.io JavaScript AI visual-builder |
| Google::Antigravity | Google's agentic development browser |
| Hermes Agent (framework) | Nous Research model family |
| `.mcp` as Mason extension | Anthropic MCP protocol |

### What to reject outright vs. verify

**Reject outright (no further review):**
- Name has zero CPAN hits AND matches a known AI product from the list above
- PR adds `use <Name>` → Moo-family alias for a name with no MetaCPAN presence
- Claim that "Perl 5.3x adds X" contradicts perldoc — perldoc is authoritative

**Verify before deciding:**
- Name is unfamiliar but not an obvious AI product — check MetaCPAN, then decide
- PR adds an editor integration to `docs/EDITORS/` (not framework support) — those are
  legitimate editor-setup docs for real LSP-compliant tools
- Name appears on CPAN but with very few downloads — note and flag for maintainer judgment

### Distinguishing legitimate editor docs from hallucinated framework support

**Legitimate:** `docs(editors): add <Product> setup guide` — touches `docs/EDITORS/`.  
**Hallucinated:** `feat(semantic-analyzer): add <Name> framework detection` — touches
`crates/perl-semantic-analyzer/`.

The tell: a PR touching both `docs/EDITORS/` and a semantic-analyzer crate is a mixed
signal; escalate rather than approve or close outright.

---

## 7. Sibling-Pair Detection

Some clusters of two PRs are **intentional complements**, not duplicates. One PR addresses
the primary path; the other adds a fallback, alternative, or complementary layer.

### Recognition signals

- The two PRs touch different files at the same abstraction level and their changes
  compose cleanly (neither overwrites the other's lines)
- PR bodies explicitly reference each other ("this PR handles X; #N handles Y")
- Both PRs survive the layer-diversity check (section 3b)

### Example: #5611 / #5618

These two PRs formed an intentional fallback chain: #5611 handled the primary case,
#5618 handled the fallback. Closing #5618 as a "duplicate" of #5611 would have left the
fallback path unimplemented. The correct verdict was `NEEDS_MAINTAINER_CALL`.

### Procedure for suspected sibling-pairs

1. Verify the file sets are disjoint or non-conflicting (section 3a)
2. Check that the changes compose rather than conflict
3. Post on both PRs: "Suspected sibling-pair — both may be needed. Routing to maintainer
   for final call on whether to merge both, one, or neither."
4. Apply `NEEDS_MAINTAINER_CALL` verdict
5. Do NOT close either PR pending the maintainer decision

---

## 8. Session-Scale Patterns

### High-throughput triage economics

At 100+ external-agent PRs per session:
- Title-based dup-sweeps over-close by an estimated 30-40% (layer-diverse siblings
  look like duplicates by title)
- File-path check cost: one `gh pr diff --name-only` per PR — cheap at scale
- Triage is sub-linear in cluster count when done cluster-first (one triage pass for a
  4-PR cluster is cheaper than four individual passes)
- Sort by file path within a cluster, not by title — it reveals the architectural map faster

### Variance as spec signal

When N PRs for the same issue all converge on the same implementation: the spec was
fully constrained. Day-one ratchet confidence is high.

When N PRs diverge significantly: the spec has an unresolved design decision. Do not
merge any variant until the decision is made explicitly. Return to plan-reviewer with
the N variants attached as "here are N reasonable interpretations — please select."

### Do not close for scope convenience

When 4 PRs land for the same topic and you're under throughput pressure: don't close 3
and keep 1 based on arbitrary selection. The three "losers" likely cover different layers.
Run the file-path check first. The extra 60 seconds prevents discarding stack coverage.

---

## 9. Quick Reference Checklist

Before processing any cluster:

- [ ] Ran `/ensemble-detect` — confirmed this is actually a cluster (not solo PRs)
- [ ] Fetched file list for every PR with `gh pr diff <N> --name-only`
- [ ] Sorted PRs by file set — identified layer-diverse vs. same-layer
- [ ] For same-layer: ranked by completeness, safety, tests, API tightness, commit cleanliness
- [ ] Verified winner does not carry phantom-drift files
- [ ] Ran MetaCPAN check for any PR adding to framework-detection tables
- [ ] Read loser diffs for edge cases, tests, gotchas before closing
- [ ] Posted extracted items on winner's PR (or opened follow-up issues)
- [ ] Closed losers with structured cross-ref comment
- [ ] Applied typed routing verdict (SALVAGE_REBASE / CLOSE_SUPERSEDED / NEEDS_MAINTAINER_CALL / etc.)
- [ ] Posted synthesis comment on winner for clusters of 4+ PRs

---

## 10. Cross-References

| Resource | What it covers |
|---|---|
| `docs/articles/FOUR_WAY_ENSEMBLE_PATTERN.md` | Monte Carlo framing, variance-as-spec-signal, cost-benefit math |
| `docs/articles/BROAD_SCOPE_LAYER_DIVERSITY.md` | File-path-first triage rule, encoding-cluster case study |
| `docs/articles/CODEX_HALLUCINATION_TRIAGE.md` | Full hallucination playbook and known seeds |
| `.claude/agents/ensemble-curator.md` | Agent definition — verdict enum, verification ladder, memory refs |
| `.claude/commands/ensemble-detect.md` | Cluster detection algorithm (task ID, burst, branch pattern, title stem) |
| `.claude/commands/cluster-triage.md` | File-path triage procedure and winner-ranking protocol |
| `.claude/commands/hallucination-check.md` | MetaCPAN check skill |
| #7073 | Salvage-classifier skill implementation |
| #7061 | Typed routing labels spec |
| CLAUDE.md > "Continuous Swarm Development" | Swarm pipeline entry point |
