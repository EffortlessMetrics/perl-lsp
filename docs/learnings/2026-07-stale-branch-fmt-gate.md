---
title: A red fmt gate can be stale-branch drift, not your diff
date: 2026-07-03
tags: [ci, fmt, rustfmt, stale-branch, gate-diagnosis, false-attribution]
search_terms: [gate-meta::fmt, cargo xtask fmt --check, queries.rs, wrapped line, branch behind main, freshness merge]
pr: 3340
hazard_class: false-attribution-of-ci-failure
---

## Incident

PR #3340 (a 2-file xtask/manifest change) showed `gate-meta::fmt` /
`pr-fast::fmt` (`cargo xtask fmt --check`) failing repeatedly. The changed
files were fmt-clean under the pinned stable toolchain (`cargo xtask fmt
--check` locally flagged **zero** of them). The failure was entirely in
`crates/perl-workspace/src/semantic/queries.rs` — a file the PR never touched.

## Root cause

The PR branch was cut from an **old base (17 commits behind main)**. A later
main commit (#3286) had rustfmt-normalized a test region in `queries.rs`
(unwrapping `let result = queries.dynamic_callable_may_be_visible_at(file_id,
100, "before_offset_sub");` — 99 chars, fits under `max_width = 100`). The
stale branch still carried the **pre-normalization wrapped** version, which
fails `cargo xtask fmt --check` under the pinned `rustfmt 1.95.0`. The workspace
fmt gate is workspace-wide, so a stale file anywhere fails the whole gate — and
it reads as "your PR broke fmt" when it is really "your base is old."

## Fix

Freshness merge: `git merge origin/main` into the PR branch pulled in the
fmt-clean `queries.rs` (plus the other 16 commits). `cargo xtask fmt --check`
then exits 0. No change to the PR's own files.

## Lessons

- **A workspace-wide gate failing on a file you didn't touch = suspect stale
  base first.** Confirm with `git diff origin/main -- <flagged-file>` (non-empty
  ⇒ your branch carries an older copy) before "fixing" formatting locally.
- **Do NOT reformat the flagged file to satisfy your local tool** when your
  local toolchain output diverges from main's committed state — you may be
  *reverting* a normalization main already landed. Rebase/merge to main's copy.
- The pinned toolchain matters: `rust-toolchain.toml` = stable 1.95.0,
  `rustfmt.toml` `max_width=100 use_small_heuristics="Max"`. Local `rustfmt
  --version` must match CI's before trusting a local fmt diff.
- fmt gates here are **non-required** (only `Perl LSP Rust Small Result` +
  `ripr+ New Gap Gate` block merge), but master-green discipline still wants
  them clean — and a freshness merge is the right lever, not a formatting edit.
