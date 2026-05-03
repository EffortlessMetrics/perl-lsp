# Hosted CI vs. Real User Parity

Hosted CI runners are a useful approximation of user environments. They are not user environments. Calibrating the gap, with concrete examples from the v0.13.3 install-reliability release.

## The gap

GitHub-hosted runners (`windows-latest`, `macos-latest`, `ubuntu-latest`) are managed images optimized for build throughput and CI reliability. They differ from user-grade machines in ways that *matter for install paths*:

| Dimension | Hosted runner | Real user machine |
|---|---|---|
| Defender / AV exclusions | Aggressive — temp dirs, build dirs, common archive types often excluded | Default exclusions only |
| Defender signature cache | Pre-warmed; common executable patterns already classified | Cold for any newly-published binary |
| Network | Datacenter-grade, low jitter, GitHub-internal mirrors | Residential / corporate, variable latency, real DNS |
| File system | Recently provisioned, often empty | Years of accumulated state, possible AV-quarantine artifacts |
| Power state | Always on, no sleep, full CPU | Suspend/resume, throttled CPU, contended I/O |
| Process state | Single-purpose (the test runs and the VM is destroyed) | Multi-process, possible prior version of the same binary still running |

**The dimension that bit us in v0.13.3** was the Defender combination: cold signature cache + default exclusions + freshly-extracted unsigned `.exe` in `%TEMP%`. Hosted CI's pre-warmed cache and aggressive exclusions made the same code path silently complete in milliseconds; on a real Windows 11 + Defender machine, the first install consistently failed with `EBUSY` because Defender held a read lock on the source file for 5-30 seconds.

The PR that originally added the retry-on-EBUSY logic (`#7862`) had been green on `windows-latest` for some time. It still failed for users.

## The four-tier proof stack

A release is proven when each tier shows green. The tiers are not redundant — each catches a class of failure the others can't:

```
Tier 1: Hosted CI
   Catches: code correctness, basic compatibility, schema/type validation
   Misses:  AV behavior, network jitter, residential edge cases, real lifecycle

Tier 2: Local repro on a real user-grade machine
   Catches: AV interaction, default-exclusion behavior, real Defender timing
   Misses:  cross-OS variance, distribution-channel propagation

Tier 3: Published-channel smoke (CI installing the actual published artifact)
   Catches: distribution-channel correctness (Marketplace, Open VSX, crates.io, Docker)
   Misses:  manual UX flow, multi-profile state

Tier 4: Manual user-flow smoke
   Catches: UX correctness, lifecycle behavior across multiple commands
   Misses:  nothing — this is the reference
```

Investments at each tier should be sized to the failure mode they catch. For install-path correctness on Windows, **tier 2 is the load-bearing one** — the hosted-CI smokes (tier 1) and published smokes (tier 3) both run on managed images that mask the real failure mode.

## Concrete v0.13.3 calibration

What hosted CI passed:

- `vscode-managed-binary-smoke.yml` was green on all three OSes. The source-side EBUSY was invisible.
- The `#7862` retry-on-EBUSY hardening passed CI before it merged.

What the real Windows machine showed:

- First install of `v0.13.1` binary (the test target version): consistent EBUSY on `fs.copyFileSync(extracted/perllsp.exe, globalStorage/.../perllsp.exe)`. Source path locked, not destination.
- The `#7862` retry budget (~4 seconds total) was insufficient to outlast Defender's first-time scan window. The fix that eventually shipped extended the budget to ~31 seconds.

What the published smokes showed:

- The first attempt at the Marketplace + Open VSX published smokes (run inside `Publish VSCode Extension`) failed because the install endpoints hadn't propagated yet. The same smokes dispatched 5 minutes later all passed.
- This is a *propagation lag* failure mode (different from CI parity) but the symptom looks similar — "smoke fails on real install" — which made initial triage harder.

What manual smoke would catch that even the published smokes don't:

- Existing-profile upgrade path (clean profiles vs. profiles with prior 0.13.x already installed). The published smokes use clean profiles and don't exercise the legacy flat-layout → versioned-dir migration. The unit-level migration test in `vscode-extension/src/test/downloader.test.ts` covers the code path; the manual smoke is the only place it gets exercised in actual VS Code.

## How to budget proof investment

A practical heuristic from this session:

- **For install paths** (binaries, extensions, packages, container images): all four tiers are required for a release that touches the install code. Tier 1 alone is not sufficient.
- **For internal-only changes** (parser logic, LSP providers, refactors that don't touch distribution): tier 1 + targeted tier 2 if the change has timing or AV-sensitive behavior.
- **For documentation, status, or metric changes**: tier 1 only is fine.

The default failure mode is to over-invest in tier 1 (where tooling is mature and feedback is fast) and under-invest in tiers 2-4 (where the failure modes that actually break users live).

## What to do when tier 1 and tier 2 disagree

If hosted CI says green and a real machine says red, **trust the real machine**. The hosted runner is the abstraction; the user environment is the ground truth.

The temptation is to chase the discrepancy ("why does my Windows machine differ from `windows-latest`?"). That's expensive and rarely productive. Better:

1. Reproduce the failure on the real machine until it's deterministic.
2. Identify the minimum environmental factor that reproduces it (Defender on/off, antivirus exclusions, signature cache cold/warm).
3. Codify the failure as a unit test that doesn't depend on the environmental factor (e.g., test the retry budget against a mocked EBUSY-throwing copy function rather than against real Defender).
4. Use that unit test as the regression lock; treat hosted CI as a smoke-test of integration.

This is what `vscode-extension/src/test/downloader.test.ts` does for the source-lock case: the retry budget test mocks EBUSY rather than depending on Defender. The unit test runs in milliseconds, is deterministic, and locks the regression independently of the hosted CI environment.

## Provenance

Calibration learned during the v0.13.3 install-reliability release closeout (2026-05-03). The Windows-Defender source-side EBUSY discovery is documented in `docs/forensics/2026-05-03-windows-defender-source-side-ebusy.md`. The propagation-lag distinction is in `docs/forensics/2026-05-03-marketplace-publish-vs-install-endpoint-lag.md`.
