# CPAN Top 1000 Corpus Strategy

> **Goal**: Parse 90%+ of the top 1,000 most-depended-upon CPAN distributions cleanly.
> **Date**: 2026-03-17
> **Depends on**: [PARSER_EDGE_CASE_ROADMAP.md](PARSER_EDGE_CASE_ROADMAP.md) (wave-based fix plan)
> **Related**: `.ci/parser-corpus-baseline.json` (system corpus baseline), `.ci/cpan-top-1000-distributions.txt` (distribution list), `.ci/cpan-corpus-manifest.txt` (known-clean CPAN subset), `.ci/common-corpus-manifest.txt` (CI-gated pinned modules)

---

## Why CPAN Top 1000

The system Perl corpus (7,095 `.pm` files from `/usr/share/perl` and friends) is a
useful regression baseline, but it skews toward core modules and whatever happens
to be installed on the CI host. It does not represent the Perl code that real users
actually work with.

The CPAN top 1000 -- the most-depended-upon distributions on MetaCPAN -- is a
better proxy for real-world coverage. If the parser handles the modules that
appear in the dependency trees of most Perl projects, users will encounter fewer
parse errors in practice. This is the corpus that matters for the LSP's
credibility as a daily-driver tool.

The 90% target is deliberately not 100%. Some CPAN distributions use source
filters, XS-only code, or generated Perl that no static parser should be expected
to handle cleanly. The long tail below 90% will be addressed on a case-by-case
basis as user reports arrive.

---

## Current State

### System Corpus Baseline

The existing system corpus sweep (`.ci/parser-corpus-baseline.json`) establishes
the starting point:

| Metric | Value |
|--------|-------|
| Total `.pm` files scanned | 7,095 |
| Unreadable (encoding) | 48 |
| Clean files (0 errors) | 5,139 (72.4%) |
| Files with errors | 1,908 (26.9%) |
| Unique first-error buckets | 28 |
| Total ERROR nodes | 28,383 |

### Common Corpus (CI-Gated)

A small set of pinned modules (`.ci/common-corpus-manifest.txt`) is verified to
parse with zero errors on every merge. This list currently contains 11 modules --
core pragmas like `XSLoader`, `bytes`, `utf8`, and stable modules like
`File::Spec`, `MIME::Base64`, and `Encode::Encoding`. It is intended to grow as
parser fixes land.

### CPAN Corpus (New)

The CPAN sprint now has three separate artifacts:

1. **Distribution list** (`.ci/cpan-top-1000-distributions.txt`) -- the top
   1,000 distributions fetched from MetaCPAN. This drives install.
2. **Full-corpus baseline** (`.ci/cpan-corpus-baseline.json`) -- the ratchet
   floor for the installed CPAN corpus. This is not committed yet; seeding it is
   part of the sprint bootstrap.
3. **Known-clean manifest** (`.ci/cpan-corpus-manifest.txt`) -- CPAN modules that
   are expected to stay zero-error once discovered. It currently starts empty.

---

## The Ratchet Protocol

All corpus metrics follow a **ratchet-only-forward** principle: measured quality
can only improve, never regress. A parser change that fixes 50 files but breaks 3
previously-clean files is rejected, even though the net effect is positive. This
eliminates the "fix one, break another" problem that plagues parser work.

### How `enforce_ratchet()` Works

The ratchet enforces five metrics simultaneously:

1. **Crash count** -- The number of catastrophic parse failures (stack overflow,
   infinite loop) must be zero. Any crash is a hard failure regardless of baseline.

2. **Unreadable files** -- The count of files that cannot be read (encoding errors)
   must not increase relative to the baseline.

3. **Clean file count** -- The number of files that parse with zero ERROR nodes
   must not decrease. This is the headline metric.

4. **Total ERROR nodes** -- The aggregate count of ERROR nodes across all files
   must not increase. This catches regressions where a file still parses but
   produces more errors than before.

5. **Per-bucket counts** -- Each error bucket (e.g., `unclosed_bracket`,
   `unexpected_fat_arrow_expr`) is tracked independently. An existing bucket's
   count must not increase. New buckets are allowed (they represent newly
   categorized errors, not regressions).

A ratchet violation in any of these five dimensions blocks the merge.

### Corpus Modes

The sweep infrastructure supports three related artifacts with different roles:

- **Common corpus** (`.ci/common-corpus-manifest.txt`): Strict zero-error policy.
  Every listed module must parse cleanly. Used in the Tier B merge gate via
  `just common-corpus-check`. This is the "we promise these work" list.

- **CPAN full corpus** (`.ci/cpan-corpus-baseline.json`): Ratchet enforcement
  against a committed baseline for the installed top-1000 corpus. Used in
  nightly/manual runs via `just cpan-corpus-check` after the bootstrap baseline
  is committed. This is the headline "we're working toward 90%" lane.

- **CPAN known-clean manifest** (`.ci/cpan-corpus-manifest.txt`): Strict
  zero-error subset for CPAN modules that have already gone clean. Used by
  `just cpan-corpus-check` as a second pass after the full-corpus ratchet.

---

## Tooling

The CPAN corpus workflow is a five-step pipeline. Each step has a corresponding
`just` recipe.

### Step 1: Fetch the Top 1000 List

```bash
just cpan-corpus-fetch
```

Queries the MetaCPAN API for the most-depended-upon distributions, ranked by
reverse dependency count. Writes the distribution names to
`.ci/cpan-top-1000-distributions.txt`, one per line.

### Step 2: Install Distributions Locally

```bash
just cpan-corpus-install
```

Uses `cpanm` with a `local::lib` prefix to install the listed distributions into
a local directory without polluting the system Perl. This creates a directory tree
of `.pm` files that the sweep can scan. If the checked-in distribution list is
still just the placeholder header, the install step fetches the top-1000 list
first and then continues. If `cpanm` is not installed on the host, the workflow
bootstraps a standalone `cpanm` script into `target/cpan-corpus/bin/cpanm`.
It also reuses a project-local `cpanm` cache rooted at `target/cpan-corpus/.cpanm`
so repeat runs do not need to start from a cold download state.

### Step 3: Run the Parser Sweep

```bash
just cpan-corpus-sweep
```

Invokes the `xtask parser-corpus-sweep` binary against the installed CPAN modules.
Parses every `.pm` file, counts ERROR nodes, categorizes first-error-per-file into
semantic buckets, and produces a JSON report. The sweep uses the same
infrastructure as the system corpus sweep (`xtask/src/tasks/parser_corpus_sweep.rs`),
including the progress bar, error normalization, and bucket classification.

### Step 4: Commit the First CPAN Baseline

```bash
just cpan-corpus-baseline-update
```

Writes the current installed CPAN sweep to `.ci/cpan-corpus-baseline.json`. This
is the bootstrap step that turns the CPAN lane from "exploratory" into a real
ratchet. After this file is committed, `just cpan-corpus-check` becomes a
well-defined sprint gate.

### Step 5: Ratchet Clean Modules

```bash
just cpan-corpus-ratchet
```

Identifies CPAN modules that parse cleanly and auto-appends them to the
CPAN known-clean manifest (`.ci/cpan-corpus-manifest.txt`). Once a module is
added there, `just cpan-corpus-check` verifies it continues to parse cleanly.
Promotion into the merge-gated common corpus remains a separate, deliberate step
for a smaller curated subset.

---

## Wave-Based Fix Plan

Parser improvements follow the wave structure defined in
[PARSER_EDGE_CASE_ROADMAP.md](PARSER_EDGE_CASE_ROADMAP.md). Each wave targets a
cluster of related parse failures, prioritized by files-fixed-per-fix.

### Historical Wave 1 Baseline -- Merged (PRs #1215--#1218)

Originally established the 51.1% baseline. Fixes included POD block skipping, regex false
positive nested quantifiers, `&{expr}` code dereference, and expanded builtins
with forward declarations.

### Wave 2 -- High-Impact Single Fixes (~500 files)

Four targeted fixes, each addressing a distinct parse failure pattern:

| Fix | Files | Root Cause |
|-----|-------|------------|
| Package-qualified subscript (`$Pkg::Var[i]`) | ~261 | Qualified variable name not followed by subscript |
| Fat arrow as separator (`push @a => $v`) | ~91 | `=>` not accepted as list separator in builtin args |
| `split /regex/` | ~22 | `/` after `split` treated as division |
| Statement modifiers after complex expressions | ~41 | Trailing `if`/`unless` not recognized after braces |

Wave 2 is the highest-leverage work remaining: 4 fixes for ~500 files, with item
2A alone worth 261 files.

### Wave 3 -- Expression Parsing Gaps (~300 files)

Six fixes targeting expression-level misparses: parenthesized assignment with
regex bind, `for` loops with block-taking builtins, complex ternary expressions,
`use overload`, chained method calls, and complex list/hash construction.

### Wave 4 -- Long Tail (~150 files)

Miscellaneous rare constructs: `return`/`next`/`last` in expression context,
`eval` block edge cases, `goto &sub`, unclosed-brace cascades, and various
patterns each affecting fewer than 5 files.

### Historical Projection vs Current Reality

| Milestone | Estimated Clean Rate | Delta |
|-----------|---------------------|-------|
| Wave 1 (done) | 51.1% | baseline |
| After Wave 2 | ~58% | +~500 files |
| After Wave 3 | ~62% | +~300 files |
| After Wave 4 | ~64% | +~150 files |

Those projections were made from the earlier 51.1% system baseline and have
already been surpassed by the current committed system ratchet (72.4%). The
remaining sprint question is no longer "can we improve the system corpus?" but
"what does the first real CPAN baseline look like, and which new buckets does it
surface?"

---

## Success Metrics

The CPAN corpus effort tracks four metrics:

1. **Clean parse rate** -- Percentage of `.pm` files in the CPAN top 1000 that
   parse with zero ERROR nodes. Target: 90%+.

2. **Common corpus size** -- Number of modules in the CI-gated common corpus
   manifest. Each addition represents a module that will never regress. Growth
   indicates steady, irreversible progress.

3. **CPAN known-clean manifest size** -- Number of modules in
   `.ci/cpan-corpus-manifest.txt`. This is the strict-clean working set for the
   CPAN sprint before modules graduate into the smaller common corpus.

4. **Error bucket distribution** -- The shape of the first-error-per-file
   histogram. As waves land, large buckets (e.g., `unclosed_bracket` at 544,
   `unexpected_token_in_expr` at 596) should shrink. New buckets may appear as
   the corpus expands, revealing previously unseen patterns.

---

## CI Integration

### What Runs Where

| Gate | Corpus | Enforcement | When |
|------|--------|-------------|------|
| Tier B (merge) | Common corpus | Strict zero-error | Every PR |
| Tier B (merge) | System corpus | Ratchet (5 metrics) | Every PR |
| Nightly / manual | CPAN full corpus | Ratchet (5 metrics) | Scheduled or on-demand |
| Nightly / manual | CPAN known-clean manifest | Strict zero-error | Scheduled or on-demand |

The CPAN corpus sweep is deliberately excluded from the merge gate. The top 1000
distributions represent a large install and a multi-minute parse sweep -- too slow
for the 3-5 minute Tier B budget. It runs as a nightly job or on manual trigger,
with ratchet enforcement to prevent silent regressions once the baseline is seeded.

### Why Not in the Merge Gate

Three reasons:

1. **Time budget** -- Installing and parsing 1,000 distributions takes minutes for
   install and seconds for the sweep. The merge gate targets 3-5 minutes total.

2. **External dependency** -- The MetaCPAN API and CPAN mirrors are external
   services. A mirror outage should not block merges.

3. **Churn** -- CPAN distributions update frequently. A newly published version
   of a distribution might introduce syntax that temporarily breaks parsing. This
   is noise in the merge gate but valuable signal in a nightly report.

The common corpus serves as the merge-gate proxy: a small, curated, stable subset
of modules that are known to parse cleanly and are individually meaningful.
