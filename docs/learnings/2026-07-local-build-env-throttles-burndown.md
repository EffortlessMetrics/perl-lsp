---
tags: [build, sccache, incremental, oom, cargo-build-jobs, performance, agent-throughput, control-plane, windows]
repos: [perl-lsp-swarm]
related: ["#3230", "#3231", "#3232"]
portable: true
article_asset: true
search_terms: [build killed, OOM, out of memory, sccache 8.6% hit rate, cargo incremental, profile agent, CARGO_BUILD_JOBS, cargo_safe, builds cold, slow local build, verify-and-land, bottleneck is infrastructure, rustc jobs]
---

# The local build environment throttled the burn-down — not codegen or CI

**Date**: 2026-07
**Hazard class**: Build-infrastructure / agent throughput
**Portable lesson**: [docs/concepts/cache-aware-agent-lanes.md](../concepts/cache-aware-agent-lanes.md)

## What happened

During an autonomous issue burn-down, the binding constraint was neither writing
the fixes nor the CI/merge gates — it was the **local compile itself**. Every
build, whether launched by a builder agent or by the orchestrator salvaging a
stalled agent's work, either got **OOM-killed mid-compile** or ran effectively
cold and blew past the tool timeout. Builder agents wrote *correct* fixes
(#3052, #3146, #2292 all merged clean on the first CI run) but repeatedly failed
to locally **verify** them — so the fixes stalled at the last-mile
build/test/clippy step for hours.

The workaround that unblocked it: push the correct-by-inspection diffs and let
Linux CI verify (it passed — the failure was purely the local Windows box).

## Root cause

Three compounding issues on an 8-core / ~7 GB-free Windows dev box:

1. **`sccache` at an 8.6% hit rate** (`sccache --show-stats`: 680 hits / 7,216
   misses). The default `dev` profile has `incremental = ON`, and **sccache
   cannot cache incremental builds** (1,102 non-cacheable "incremental"
   compilations). Incremental artifacts are also worktree-local, so nothing
   cross-pollinates between agent worktrees — the exact reuse the cache exists
   for. The cache everyone assumed was warm was almost entirely cold.
2. **Unbounded rustc parallelism** — the default `-j8` on ~7 GB free RAM spawns
   enough concurrent rustc/codegen to exhaust memory → the OS kills the compile.
3. **No `SCCACHE_BASEDIRS`** on the plain-`cargo` path → sibling-worktree paths
   differ → cache keys differ → misses even when a dep was already built next door.

The required CI gates missed all of this because they build on Linux with their
own cache; the problem lived entirely in local verification.

## Fix

The repo **already ships the fix** — `scripts/cargo-safe` (the `cargo_safe`
justfile alias) sets `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`, a shared
`SCCACHE_DIR` + `SCCACHE_BASEDIRS`, a `flock` build lock, and a disk preflight.
And `[profile.agent]` (root `Cargo.toml`) is purpose-built for this
(`incremental = false`, `debug = "line-tables-only"`). **The agent/preflight path
just bypassed all of it**, running raw `cargo test -p X` (default dev profile).

Route all local builder + preflight commands through the agent-safe path:
`cargo test/clippy -p <crate> --profile agent --locked` (incremental off → global
sccache actually hits across worktrees) with `CARGO_BUILD_JOBS` capped (no OOM).
Tracked as **#3230** (add per-crate agent-safe recipes + update CLAUDE.md
preflight + builder prompts). Related: **#3231** (evaluate `lld-link` for msvc),
**#3232** (document Windows Defender exclusions for `target/`).

## Portable lesson

At AI-scale codegen, **the local build environment — RAM, job parallelism, and
cache configuration — is a first-class throughput constraint, not a footnote.**
Production (writing the fix) is cheap and reliable; the frontier is
*verify-and-land*, and this session it was throttled *below* CI, at the developer
/ agent's own compile box. You can be bottlenecked on memory, not reasoning.

Concrete guards:
- Pin the sccache-friendly **profile** (`--profile agent` / incremental off) for
  every agent and preflight build — an incremental build silently defeats the
  shared cache. Verify with `sccache --show-stats` (a healthy repeated-build
  workload should be well above 50% hits; 8.6% means it's cold).
- **Cap `CARGO_BUILD_JOBS`** on memory-constrained boxes; unbounded `-j` + a big
  Rust workspace = OOM kills that look like mysterious "build died" failures.
- When the infra to do this already exists (here, `cargo_safe`), the fix is a
  **routing/docs change** (make the agents use it), not new tooling.
- A build that keeps dying is a *diagnostic signal*, not just an obstacle — it
  points straight at the binding constraint. Instrument it (`--show-stats`,
  which target dir, which profile) rather than retrying blindly.
