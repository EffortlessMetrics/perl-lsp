# Codebase Curiosities: Fun Facts and Deep Cuts from perl-lsp

*A tour through the oddities, records, and surprising patterns hiding in a Rust codebase built by humans and AI agents alike.*

---

## By the Numbers

| Metric | Value |
|--------|-------|
| First commit | July 17, 2022 |
| Total commits | 2,104 |
| Total PRs opened | 1,270 |
| PRs merged | 643 |
| PRs rejected (closed without merge) | 428 |
| Crate directories | 121 |
| Lines of Rust | ~82,000 |
| Rust test files | 463 |
| Perl test fixtures (`.pl` / `.pm` / `.t`) | 3,322 |
| Markdown files | ~1,850 |
| Per-crate `CLAUDE.md` files | 52 |
| Named releases | 11 (from `v0.1.0-pest` to `v0.10.0`) |

The word "Perl" appears 4,112 times in the Rust source. Fair enough -- it *is* a Perl LSP.

---

## The 4-Day Parser

The Pest parser-generator framework was introduced on **July 16, 2025** (commit `e7f67f39`). It was tagged as `v0.1.0-pest` on July 20. Four days later, on **July 21**, the hand-written recursive-descent parser (v3) appeared in commit `da94f7b3` ("Implement a modern two-crate architecture for Perl parsing").

By July 25, the v3 parser was already claiming "100% edge case coverage" in its release notes. The Pest grammar still exists in the repo as `perl-parser-pest`, but it has been exiled to the `archive/legacy-crates/` directory and excluded from the default workspace build.

What happened? The Pest grammar was accumulating features at a ferocious rate -- 96 commits on July 16 alone, with the grammar rapidly expanding to cover heredocs, regex, hash references, and more. But Perl's legendary parsing ambiguity (is `foo /bar/` a function call with a regex, or division?) proved hostile to PEG grammars. By July 18, commit `b45c2dee` was already titled "Implement extensive performance optimizations" -- a sign of trouble. The three-way parser comparison tool appeared on July 21 (`a3b35382`), and by that afternoon the native recursive-descent architecture was born.

The `v0.1.0-pest` tag remains in the repo as a historical marker. Versions v0.2.0, v0.3.0, and v0.4.0 were internal milestones that were never tagged. The next public tag was `v0.5.0` on August 3 -- already running on the native parser.

---

## Agent Archaeology

### The Three Personas

The project used Codex agents with three distinct personas, identifiable by emoji prefixes in their PR titles:

| Persona | Role | PRs Submitted | Merged | Rejected | Merge Rate |
|---------|------|:---:|:---:|:---:|:---:|
| **Bolt** | Performance optimization | 47 | 19 | 28 | 40% |
| **Sentinel** | Security hardening | 35 | 16 | 19 | 46% |
| **Palette** | UX and polish | 27 | 12 | 15 | 44% |

All three agents had a **golden era** (roughly PRs #473--#647, late January 2026) where their work was consistently merged. Then all three simultaneously entered a **rejection era** (PRs #771--#837, mid-February 2026) where essentially nothing got through.

### Bolt's Obsession with PHF

The performance agent, Bolt, became fixated on replacing the built-in function lookup with a PHF (perfect hash function). Across PRs #780, #782, #785, #791, #798, #804, #805, #811, #829, and #835, Bolt submitted **10 nearly identical PRs** titled variations of "Optimize is_known_function with PHF lookup" -- all rejected. Eventually PR #1167 merged the PHF extraction, but it was not a Bolt PR.

### Sentinel's Security Theatre

Sentinel the security agent found 16 genuine vulnerabilities in its early run (command injection, path traversal, eval bypass). But after those were fixed, it began hallucinating new vulnerabilities. PRs #776 through #837 contain 19 rejected "fixes" for problems like "Fix Infinite Redirect DoS in BinaryDownloader" (submitted three separate times: #771, #786, #822, #832) and "Fix safe evaluation bypass" (submitted at least six times with different phrasings).

### The 18-Way Status Menu Standoff

**18 PRs** were opened about making the VS Code status menu "context-aware." They came from all three agent personas (Palette, generic, even Sentinel got in on it). All 18 were rejected. Titles ranged from the straightforward ("Make status menu items context-aware") to the increasingly desperate ("Context-Aware Status Menu Improvements," "UX: Make status menu context-aware and improve feedback"). Not one shipped.

### The Document Links Extraction: 9 Attempts

Extracting document link detection into its own crate was attempted **9 times** across PRs #1097, #1098, #1108, #1165, #1169, #1175, #1183, #1233, and the finally-merged #1164. Eight identical-in-spirit extraction PRs were rejected before one got through.

### Three Agents, One Problem

PRs #1244, #1245, and #1246 were three different agent runs solving the same problem: adding a `just doctor` developer environment check. #1244 and #1246 were rejected; #1245 merged. The rejected ones had subtly different names -- "dev environment check and README quick-checks" vs. "quick environment check recipe" -- but were essentially the same idea racing to completion.

---

## The "Comprehensive" Signal

AI agents have a well-known fondness for the word "comprehensive." In perl-lsp:

| Buzzword | Occurrences in Commit Messages | PRs with Word in Title |
|----------|:---:|:---:|
| "comprehensive" | **277** | **107** |
| "enhance" | **271** | -- |
| "robust" | 17 | -- |
| "enterprise" | 3 | -- |

277 commits and 107 PRs with "comprehensive" in the title. The word also appears in **301 Rust source files**, mostly in test file names like `comprehensive_unit_tests.rs`.

The longest commit message in the repo is 411 characters:

> *Remove deprecated test files and parser comparison logs to streamline the codebase. This includes the deletion of various test scripts covering features such as anonymous subroutines, array and hash access, regex operations, and control flow constructs. The parser comparison file has also been removed due to its redundancy. This cleanup enhances maintainability and reduces clutter in the repository.*

Meanwhile, the shortest commit messages are all a single character: **"c"**. There are **21 commits** with just the message "c". Presumably a human in a hurry.

---

## The SRP Explosion

The crate count tells a dramatic story:

| Version / Date | Crates |
|---------------|:------:|
| `v0.1.0-pest` (Jul 2025) | 2 |
| `v0.5.0` (Aug 2025) | 6 |
| `v0.8.5` (Aug 2025) | 8 |
| Jan 2026 | 7 |
| `v0.9.1` (Feb 2026) | 50 |
| `v0.10.0` (Feb 2026) | 82 |
| HEAD (Mar 2026) | **121** |

The workspace went from 7 crates in January 2026 to **45 by February 1** and **83 by March 1**. That is roughly **2.5 new crates per day** for two months straight. The project calls this "SRP microcrate extraction" -- splitting single-responsibility modules into their own crates.

**70 microcrate extraction PRs were rejected** during this period, meaning agents submitted roughly 2 failed extractions for every one that landed.

---

## Code Curiosities

### The Smallest Crate: 47 Lines of Rust

`perl-dap-command-args` weighs in at 47 total lines of Rust, including tests. Its entire purpose: quoting command-line arguments that contain spaces. It has zero dependencies. It has its own `Cargo.toml`, `README.md`, and `LICENSE` files. The build infrastructure for this crate may well be larger than the code.

Other tiny crates: `perl-lsp-uri` (49 lines), `perl-line-index` (59 lines), `perl-workspace-ignore` (60 lines), `perl-percentile` (71 lines).

### The Infinite Loop URI Fallback

`perl-lsp-uri` contains a fallback function that tries four hardcoded URIs, then -- if *all* of those fail to parse -- enters an infinite loop trying `http://localhost/0`, `http://localhost/1`, `http://localhost/2`, and so on until one works. The comment says: "Last-resort fallback that avoids panicking if URI parser behavior changes unexpectedly." The project bans `unwrap()` and `panic!()` in production code, so this loop is the logical extreme of that policy.

### The Largest Files

| Lines | File |
|------:|------|
| 6,778 | `perl-dap/src/debug_adapter.rs` |
| 3,855 | `perl-workspace-index/src/workspace/workspace_index.rs` |
| 3,833 | `perl-ci-hygiene/src/main.rs` |
| 3,299 | `perl-parser-core/tests/comprehensive_unit_tests.rs` |
| 3,261 | `perl-refactoring/src/refactor/refactoring.rs` |
| 3,123 | `perl-lexer/src/lib.rs` |

The debug adapter at 6,778 lines is the largest file in the workspace -- nearly the size of the 9 smallest crates combined.

### The Archive

The `archive/` directory contains `legacy-crates/` and `legacy-tests/`. Inside `legacy-crates/` sits the banished Pest parser. Inside `legacy-tests/` are benchmark tests and fixtures from the parser comparison era. They are excluded from `cargo test` and the CI gate but preserved for historical reference.

### The Most-Changed File

`crates/perl-parser/src/lsp_server.rs` has been modified in **192 commits** -- more than any other file, including `CLAUDE.md` (144 changes) and `README.md` (132 changes).

---

## Records and Extremes

### Busiest Single Day: March 4, 2026

**152 commits** in one day. The day was a blitz of microcrate extractions and "comprehensive unit test" additions -- an entire test suite scaffolded across dozens of newly-extracted crates. The commit log reads like a factory assembly line: `test(tokenizer): add extended unit tests`, `test(incremental-parsing): add extended unit tests`, `test(symbol-table): add extended unit tests`, one after another after another.

### Largest PR: #858

PR #858, "fix: harden checksum verification and stabilize incremental parsing CI," added or changed **7,696 lines** -- the largest merged PR in the repo's history.

### Fastest Version Churn

Versions v0.7.2 and v0.7.3 were both tagged on **August 6, 2025** -- two releases on the same day. Then v0.8.0 arrived five days later. The stretch from v0.5.0 (Aug 3) to v0.8.5 (Aug 23) covers **six tagged releases in 20 days**.

### The Missing Versions

The version history jumps from `v0.1.0-pest` straight to `v0.5.0`, skipping v0.2 through v0.4. These were internal milestones for the native parser (the commit log references "v0.2.0 release" and "v0.3.0 release" and "v0.4.0: complete v3 parser") but they were never tagged.

Similarly, there is no `v0.6.x` in the tag list. And `v0.9.0` appears in commit messages ("release: v0.9.0 -- Semantic-Ready milestone") but the first tagged 0.9 release is `v0.9.1`.

---

## The CLAUDE.md Per-Crate Convention

**52 crates** have their own `CLAUDE.md` -- a file providing crate-specific guidance to Claude Code. These files describe the crate's tier, purpose, dependencies, key types, and commands. They range from minimal (a few lines of build instructions) to detailed architectural overviews.

Combined with the project-root `CLAUDE.md` (which itself has been modified 144 times), this forms a distributed documentation system that serves double duty: human-readable reference material and machine-readable agent context.

---

## Fun Facts and Easter Eggs

- **The 21 "c" commits**: Someone made 21 commits with the single-letter message "c." They are scattered throughout the history -- likely a human rapid-iterating with a quick alias.

- **3,322 Perl files**: The Rust project contains more Perl files (test fixtures) than lines of code in several of its smallest crates.

- **120 Cargo.toml files**: The workspace has nearly as many `Cargo.toml` build configuration files as it has `lib.rs` source files (119). The ratio of "build system" to "actual library entrypoint" is almost exactly 1:1.

- **The Codex agent rejection wall**: PRs #771 through #837 form an unbroken block of 67 consecutive PRs, overwhelmingly from the three agent personas, almost all rejected. This corresponds to the period when the low-hanging fruit had been picked and the agents kept regenerating the same stale optimizations.

- **The night shift**: The most common commit hour is midnight (133 commits at 00:xx), followed by 5 PM (119) and 1 PM (116). The least busy hours are mid-morning -- the codebase is built by night owls and AI agents that never sleep.

- **"Enterprise" count: 3**: Despite 277 uses of "comprehensive" and 271 of "enhance," the word "enterprise" appears in only 3 commit messages. Even the agents have limits.

- **Version v0.10.0 has 82 crates** -- but there are now 121 crate directories at HEAD. Thirty-nine crates were extracted in the two weeks since the last release.
