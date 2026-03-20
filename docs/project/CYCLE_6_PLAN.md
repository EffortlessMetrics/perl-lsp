---
name: Cycle 6 plan — merge drain, release, corpus 90%+, v0.13 features, community launch
description: Full cycle plan. Phase 1: drain 60+ PRs. Phase 2: publish v0.12.0. Phase 3: corpus to 90%+. Phase 4: wire v0.13 features. Phase 5: community launch. Phase 6: structural improvements.
type: project
---

## Cycle 6: Ship, Polish, Launch

### Pre-Session Checklist
```bash
just clean-worktrees          # Remove stale agent worktrees (PR #2301)
git pull origin master
gh pr list --state open --limit 200 --json number | jq length  # Check PR count
gh run list --branch master --limit 1  # Verify CI green
```

---

## Phase 1: Merge Drain (first 60 min, 1 ops agent)

**Goal:** 60+ open PRs → <15

**Priority order:**
1. Parser fixes (corpus impact): #2275, #2276, #2279, #2280, #2281, #2299, #2308
2. Corpus ratchets: #2271, #2294 (lock in 87.8%)
3. v0.13 features: #2283+#2304 (perl-pod+hover), #2284+#2300 (ClassModel+completions), #2285 (perlcritic), #2268 (parser cancellation), #2273 (debounce), #2259 (security lints), #2267 (unused imports), #2263 (check-project), #2305 (complexity hover)
4. Infrastructure: #2301 (clean-worktrees), #2298 (promotion guide), #2282 (incremental docs)
5. Articles/docs: #2274, #2277, #2278, #2286, #2292, #2295, #2302, #2303, #2306, #2309, #2310
6. Stale PRs: close anything that can't merge cleanly after 2 rebases

**Rules:** Batches of 3. Wait for CI. Parser fixes first. Close stale #2204 if unfixable.

---

## Phase 2: Publish v0.12.0 (after merge drain, 1 agent)

**Goal:** v0.12.0 on crates.io + GitHub Release + VSCode marketplace

**Steps:**
1. Verify master CI green
2. `cargo publish` in topological order (leaf crates first):
   - perl-percentile, perl-line-index, perl-source-editing, perl-diagnostics-codes
   - perl-error, perl-token, perl-ast, perl-builtins, perl-builtins-phf
   - perl-lexer, perl-heredoc, perl-quote, perl-regex
   - perl-parser-core, perl-parser
   - perl-lsp (the binary)
3. `gh release create v0.12.0 --title "v0.12.0 Public Alpha" --generate-notes`
4. Build binaries for Linux/macOS/Windows (GitHub Actions or manual)
5. Verify: `cargo install perl-lsp` installs 0.12.0
6. VSCode extension: publish to marketplace (if ready)
7. Enable GitHub Discussions (issue #2169)

---

## Phase 3: Corpus Push to 90%+ (parallel, 5 builders)

**Goal:** 87.8% → 90%+ (need ~95 more clean files)

**Current top buckets after session 3 fixes:**
- unexpected_comma_expr: ~64 remaining (partially fixed by #2308)
- unexpected_token_in_expr: ~104
- unclosed_paren: ~64
- unexpected_fat_arrow_expr: ~55
- unclosed_brace: ~46

**Strategy:**
- Scout each bucket first (5 min each)
- Builder per bucket with constrained spec
- Ratchet after each merge wave
- Target: fix 100+ files across 3-4 buckets

**After reaching 90%:** Update PUBLICATION_FACTS_LEDGER with verified number.

---

## Phase 4: Wire Remaining v0.13 Features (3 builders)

**Already built, needs merging + follow-up:**

| Feature | Built | Next Step |
|---------|-------|-----------|
| Perlcritic | PR #2285 | Merge, then: .perlcriticrc discovery, severity config |
| Perldoc hover | PRs #2283, #2304 | Merge, then: method-level docs (`$obj->method`) |
| Moose completions | PRs #2284, #2300 | Merge, then: Phase 2 inheritance resolution (2 days) |
| Parser cancellation | PR #2268 | Merge, then: wire into text_sync.rs (~5 lines) |
| Test runner | 40-60% done | Verify CodeLens→command wiring, add result→diagnostics |

**New v0.13 work:**
- Phase 2 Moose: inheritance resolution across files (2 days)
- Perldoc method hover: `$dbh->prepare` shows docs (3 days)
- Perlcritic config: .perlcriticrc discovery + severity UI (2 days)

---

## Phase 5: Community Launch (1 agent + human)

**Week 1 (after v0.12.0 published):**
1. Email Perl Weekly (editors@perlweekly.com) with:
   - "perl-lsp: Rust-native Perl language server, no Perl dependency"
   - Key stats: 87%+ CPAN, 98 features, zero panics
   - Link to GitHub + getting-started guide
2. Post on blogs.perl.org (auto-aggregated to Planet Perl)
3. Post on r/perl
4. Post on PerlMonks (Cool Use for Perl)

**Week 2-3:**
- Submit to lsp-mode (Emacs) for integration
- Submit to mason.nvim (Neovim) registry
- Contact Neil Bowers for CPAN comparison post
- Explore PTS 2026 sponsorship (April 23-26, Vienna)

**Week 4+:**
- Respond to user feedback
- File issues from bug reports
- Publish blog posts from article inventory (15+ ready)

---

## Phase 6: Structural Improvements (2-3 builders, ongoing)

**From issue backlog:**
- #2296 — Centralize CURRENT_STATUS rendering (highest-leverage swarm fix)
- #2293 — Split semantic.rs god file (3,256 LOC → 7 modules)
- #2297 — Hook reliability engineering
- #2287 — Wire dead code detector into diagnostics
- #2289 — Wire incremental parsing into text_sync
- #2291 — Audit diagnostic code usage

**From learnings:**
- Memory quarterly consolidation (156 files, need periodic merge)
- "Built but not wired" scout at session start
- Worktree cleanup at session start (just clean-worktrees)

---

## Agent Budget

| Lane | Agents | Focus |
|------|--------|-------|
| Merge-ops | 1 | Drain 60+ PRs in priority order |
| Corpus builders | 5 | Top 4 buckets → 90%+ |
| Feature wiring | 3 | Moose Phase 2, perldoc methods, perlcritic config |
| Release | 1 | cargo publish, gh release, VSCode marketplace |
| Community | 1 | Perl Weekly, blogs.perl.org, r/perl |
| Structural | 2 | CURRENT_STATUS fix, semantic.rs split |
| Corpus ratchet | 1 | After each parser merge wave |
| Improvement | 1 | Memory cleanup, "built not wired" scout |
| **Reserve** | **5** | Late-cycle routing |
| **Total** | **~20** | Lean, focused |

---

## Success Criteria

- [ ] Open PRs < 15
- [ ] Corpus ≥ 90%
- [ ] v0.12.0 published to crates.io
- [ ] GitHub Release created with binaries
- [ ] `cargo install perl-lsp` installs 0.12.0
- [ ] Perl Weekly submission sent
- [ ] blogs.perl.org post published
- [ ] GitHub Discussions enabled
- [ ] CURRENT_STATUS rendering centralized (#2296)
- [ ] semantic.rs split started (#2293)
- [ ] Moose inheritance resolution working
- [ ] Perldoc method-level hover working

---

## Key Process Rules (from session 3 learnings)

1. **Scout before building** (4:1:2 ratio)
2. **Parallel lanes, not phases** (research+build+merge+document simultaneously)
3. **Don't broadcast shutdown** (let agents idle)
4. **Check `gh pr list --limit 200`** (pagination hides real count)
5. **Stop building above 30 open PRs** (shift to merge)
6. **Run `just clean-worktrees` at session start**
7. **External analysis after major findings** (ChatGPT for invisible framings)
8. **Date-stamp all metrics** (coverage numbers move fast)
9. **Promote findings immediately** (pitfall → finding → issue → article → rule)
10. **The merge queue (3-wide) is the ceiling, not agent count**
