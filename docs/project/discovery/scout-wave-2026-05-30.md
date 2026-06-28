# Issue Discovery — Scout Wave 1 (2026-05-30)

First run of the [Issue Discovery / Bug Scout Desk](../../reference/ISSUE_DISCOVERY_DOCTRINE.md).
6 read-only scouts swept recently-hot surfaces. **0 GitHub mutations by scouts**; the
orchestrator triaged centrally and filed 6 high-confidence candidates issue-by-issue.

## Scouts dispatched

| Scout | Surface | Verdict |
|-------|---------|---------|
| `scout-dap` | DAP stack/scopes/variables/evaluate/setVariable/lifecycle/transport | Mostly corroborated existing issues (#902/#895/#898/#803); 1 downgraded lead |
| `scout-lsp` | document state, URI isolation, completion/hover/code-action/semantic-token | 2 NEW leads (needs-repro / needs-plan-review) |
| `scout-parser` | AST shape, recovery, fixtures, NodeKind | **Parser healthy** (99.3% clean, 0 active buckets); 3 low-conf fixture leads only |
| `general-purpose` (ci/ops) | workflow routing, branch triggers, path filters | 2 NEW HIGH + 1 MED |
| `general-purpose` (robustness) | panic/DoS/byte-boundary/incorrect-result | 2 NEW HIGH + 1 MED (conditional reachability) |
| `general-purpose` (docs/receipt) | status `.md` ↔ `.json` receipts, basis conflicts | 4 NEW (3 HIGH + 1 MED) |

## Filed (6 — verified by orchestrator AND dedup-clean)

| Issue | Finding | Class | Verified |
|-------|---------|-------|----------|
| [#956](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/956) | `workspace_rename.rs` byte-checks UTF-8 continuation bytes → rename corrupts identifiers adjacent to Unicode names | bug / incorrect-result | code (`refactor/workspace_rename.rs:489-492,812`) |
| [#957](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/957) | `find_pragma_insert_pos` `+1` assumes LF → CRLF pragma insert splits `\r\n` | bug / incorrect-result | code (`code_actions/modernize.rs:543-563`) |
| [#958](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/958) | Workflows trigger on nonexistent `master` (default=`main`): `post-merge-status` + `workflow-trigger-lint` dead | ci-ops | mechanically (triggers + `git remote`) |
| [#959](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/959) | `em-ci-routed-rust` GitHub fallback skips parser+LSP smoke → fork/bot PRs pass gate on `cargo check` | ci / merge-gate trust | YAML (`em-ci-routed-rust.yml:235-237/319-321/371-372/82`) |
| [#960](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/960) | `quality.md` hardcodes `<50ms / 931ns`; receipts say 53ms / 37–73µs; no receipt has 931ns | docs-drift / basis-conflict | grep (`update_status/quality.rs:136`) |
| [#962](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/962) | Feature count: 125 actual `[[feature]]` vs 119 `[meta]`/lsp.md vs 116 index.md | docs-drift / basis-conflict | `grep -c` (125 blocks / 124 advertised) |

All filed with `swarm-discovered` + `needs-plan-review` + `size/*` (never `builder-ready`).

## Held — report-only / handoff (not filed)

| Lead | Conf | Next workflow | Why held |
|------|------|---------------|----------|
| D2 — `editor_ux.md` "9 of 22 (41%)" vs 64 fixture workflows | HIGH (scout) | file next pass or needs-research | Strong but the `64` count not orchestrator-verified |
| D4 — `.ci/blockers.yaml` non-zero affected_files vs 0-dirty baseline | MED | needs-plan-review | Low-risk cleanup; corroborates clean parser |
| R3 — `IncrementalParserV2` non-char-boundary slice panic (helpers dead-code) | MED | needs-research | Conditional reachability — no live caller on default LSP path |
| LSP1 — didClose→reopen stale AST cache | MED | needs-repro | Test gap; bug not yet reproduced |
| LSP2 — codeAction/resolve lacks stale-version guard | LOW-MED | needs-plan-review | Verify resolve handler version contract first |
| DAP P1-4 — frameId scoping / no-session success / threadId / scopes validation | — | evidence-only | Corroborate existing #902/#895/#898/#803; no new issue |
| Parser P1-3 — postfix chains, tie+attrs, given/when fixtures | LOW | report-only | Speculative fixture gaps; given/when strongest (recent #822 lexer fix) |
| LSP semantic-token drift; completion bleed | LOW | report-only / discard | →#262 (unverified); completion bleed covered by merged #757 |

## Cross-scout corroboration (the wave's synthesis)

- **#958 is the root cause of #962 (and D2):** status docs are stale *because* their
  regenerator (`post-merge-status.yml`) never fires (wrong branch trigger). Fixing #958
  + regenerating addresses the generated half of #962.
- **Parser-scout "clean" + D4 agree:** the parser corpus is clean (99.3%) while
  `.ci/blockers.yaml` still lists resolved buckets — independent confirmation of staleness.
- **Systemic theme:** `master`-vs-`main` branch triggers affect ≥5 workflows
  (`post-merge-status`, `workflow-trigger-lint`, `ripr`, `pr-plan`, `tokmd`).

## Metrics

- Candidate packets produced: ~22 across 6 scouts
- Filed (high-confidence, dedup-clean): **6**
- Held as report-only / handoff: **8** (medium/low or conditional)
- Discarded as noise / already-covered: ~4 (incl. completion-bleed → #757, DAP dups)
- Duplicate-of-existing surfaced during triage: closed #781 (setVariable), merged #757 (URI isolation)
- Scout GitHub mutations: **0** (read-only contract held)

## Operational notes for next pass

- **Tooling hygiene:** general-purpose scouts run in the main working dir and left two
  0-byte stray files (`@*`, `method-`) from shell glob/redirect mishaps. Prefer worktree
  isolation or stricter quoting for grep-heavy scouts; cleaned this run.
- **Next-pass candidates:** verify + file D2; confirm R3 reachability (repro-lab); write
  a repro for LSP1. Consider creating the lane's functional labels (`docs-drift`, `ci-ops`,
  `robustness`, `candidate-issue`) so triage is filterable.
- **Wave-two surfaces (held this run):** workspace-facts (#892/#893/#894/#900/#811-813)
  and editor UX (#917/#916/#932).

---

## Held-lead verification pass (same session)

Verification of the 3 one-step-from-filing leads — **1 filed, 2 held** (verification downgraded 2):

| Lead | Outcome | Why |
|------|---------|-----|
| D4 — blockers.yaml stale | **filed #985** | Verified: `partial`/`filed` buckets vs baseline `files_with_errors: 0` |
| D2 — editor_ux scorecard | **held** | Scout's "64 workflows" unconfirmed; real discrepancy is md `22` vs json `scenario_count 30` — needs-research, not a clean file |
| R3 — IncrementalParserV2 panic | **held** | No production caller (only its own `#[cfg(test)]` constructs it) → latent, not a live bug |

## Wave 2 (workspace-facts + editor UX)

Both scouts ran in **worktree isolation** (acting on the wave-1 hygiene note). Every claim was orchestrator-verified before any GitHub write.

| Finding | Outcome | Note |
|---------|---------|------|
| WF-P1 — `WorkspaceIndex` never wires `PackageGraphIndex` → `method_candidates` empty for hover/def/refs/rename/diag | **comment on #812** | Verified root cause (queries.rs:303 / 573-576; workspace_index.rs has 0 `PackageGraphIndex` refs). Corrects #812's speculated cause. Enrich, not duplicate. |
| WF-P2 — dynamic `@ISA` (`push @ISA, $var`) silently dropped, no `DynamicBoundary` | **filed #989** | NEW; verified `collect_names_from_node` `_ => Vec::new()` (package_graph_extractor.rs:324) |
| UX-P1 — activationEvents diverges from contract test | **DISCARDED** | **Hallucination**: package.json activationEvents *exactly* matches the test (no `onStartupFinished`; `perl5` *is* expected). Verification prevented a fabricated filing. |
| UX-P2 — `disabledFeatures` change → no restart prompt | **comment on #968** | Adjacent to #968's `requiresClientRefresh` finding; one-line fix. (#968 was filed by a *concurrent* discovery session — dedup caught it.) |
| UX-P3 — Jest has no PR gate | **evidence-for #932** | Also: #931 (merged) added report-only Jest on PRs |
| UX-P4 — perltidy-missing error names wrong setting (`perltidyConfig` is a `.perltidyrc` path, not the binary) | **held** | Verified but minor: primary "cpan Perl::Tidy" advice is correct; only the parenthetical misleads |

## Concurrent-session note
A **parallel Issue-Discovery session** is active (e.g. #968 filed ~1 min after this wave, same lane signature). Dedup must run against live state per filing — it caught #968 (UX-P2) and avoided a near-duplicate. Cross-session coordination is a follow-up consideration if the lane runs multi-instance.

## Cumulative session totals
- Filed: **8** candidate issues — #956, #957, #958, #959, #960, #962 (wave-1) + #985 (held-lead) + #989 (wave-2)
- Enrichment comments on existing issues: **2** (#812 root cause, #968 sibling instance)
- Discarded as unverified/hallucinated: UX-P1; held for next pass: D2, R3, UX-P4
- Scout GitHub mutations: **0** (read-only contract held across all 8 scouts)
- Verification value: 2 of 3 held-leads downgraded, and 1 of 4 UX packets was a hallucination — reinforces "finding is cheap; being right is expensive: verify before filing."

---

## D2 resolved (verification follow-up)
D2 (editor_ux scorecard) is **dropped, not filed**: the scout's "64 fixture workflows" matches nothing (json has 3 harnesses at 29/28/17, `scenario_count: 30`; md denominator is `declared_scenario_count = 22`, measured 2026-04-26). The one concrete contradiction — `editor_ux.md` diagnostics p50 53ms vs quality.md "<50ms" — is already in #960. The remaining "22 vs 30" is a generator-semantics nuance, not a clean defect.

## Wave 3 (test-quality)
One scout (worktree-isolated). All 4 packets verified, then **consolidated into a single test-hardening issue (#992)** rather than filed separately:

| Packet | Spot | Verified |
|--------|------|----------|
| A | `start_line > 0 \|\| start_char > 0` — lsp_type_definition_tests.rs:72,109; lsp_implementation_tests.rs:88 | ✓ |
| B | `is_array() \|\| is_null()` shape-only — lsp_type_definition_tests.rs:53,146,174,207,296 (file self-documents at :58/:404; strong companions :302/:410) | ✓ |
| C | `success_count > 0` — performance_stress_edge_cases.rs:377-378 | ✓ |
| D | `is_some()` presence-only — lsp_streaming_completion_tests.rs:140-147 (strong companion :154) | ✓ |

## Cumulative session totals (updated)
- Filed: **9** candidate issues — #956, #957, #958, #959, #960, #962, #985, #989, #992
- Enrichment comments: **2** (#812 root cause, #968 sibling instance)
- Resolved-as-drop after verification: **D2** (editor_ux), **UX-P1** (hallucination); held: R3 (no caller), LSP1 (needs repro), UX-P4 (minor copy)
- Scout GitHub mutations: **0** across all **9** scouts (6 wave-1 + 2 wave-2 + 1 wave-3)
- Filed-vs-found discipline: of ~30 candidate packets produced, 9 filed, 2 folded into comments, the rest verified-down to held/dropped.

---

## Wave 4 (robustness deep-pass)
One scout (worktree-isolated), targeting LSP provider handlers + lexer/heredoc surfaces wave-1 didn't sample. Found **3 reachable server-crash panics** (non-char-boundary byte slicing of Unicode source), each rustc-confirmed; consolidated into **#998**:

| Spot | Path | Reach |
|------|------|-------|
| A — `->` trigger slice | `completion.rs:714` (`source[position-2..position]`) | textDocument/completion |
| B — word-start slice | `completion.rs:730-742` (`rfind().map(\|p\| p+1)`) | textDocument/completion |
| C — `get_text_around_offset` | `util/mod.rs:419-422` | definition (`navigation.rs:892/1037`) + references (`references.rs:339`) |

Same UTF-8 byte-boundary *class* as #956/#750 but distinct files + a worse failure mode (crash, not wrong-result). Scout also reported solid negative space (lexer `normalize_char_boundary`, semantic_tokens, perl-regex, perl-dap all safe).

## Final cumulative totals
- Filed: **10** candidate issues — #956, #957, #958, #959, #960, #962, #985, #989, #992, #998
- Enrichment comments: **2** (#812 root cause, #968 sibling instance)
- Scouts run: **10** (wave-1 ×6, wave-2 ×2, wave-3 ×1, wave-4 ×1) — **0 GitHub mutations by scouts**
- Verification stopped: 1 hallucination (UX-P1) + 2 downgraded held-leads (D2, R3) from becoming low-signal issues
- Surfaces covered: DAP, LSP, parser, CI/ops, robustness (×2), docs/receipts, workspace-facts, UX, test-quality
