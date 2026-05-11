# CI Cost and Verification Policy

> **Context**: This document is part of perl-lsp's [Industrialized AI](why-industrialized.md) CI architecture. The choices here are responses to operating at 1000+ PRs/day, not premature optimization.

This repository targets CI cost per ordinary PR far below common high-volume agentic
defaults. The goal is **not** lighter verification. The goal is stronger verification per
CI minute.

> Companion docs: [lem-budgeting.md](lem-budgeting.md),
> [verification-ladder.md](verification-ladder.md),
> [labels.md](labels.md),
> [perl-lsp-rollout-plan.md](perl-lsp-rollout-plan.md).
> Anchors into the existing system: [../reference/CI_ARCHITECTURE.md](../reference/CI_ARCHITECTURE.md),
> [`.ci/gate-policy.yaml`](../../.ci/gate-policy.yaml),
> [`.ci/GATE_REGISTRY.toml`](../../.ci/GATE_REGISTRY.toml).

---

## Operating thesis

We are not reducing CI because we want less verification. We are reducing wasted CI so we
can afford more verification where it matters.

Agentic development makes code generation cheaper and faster, but verification remains
expensive. That ratio is getting worse. At high agent-driven operating volumes, 500+
GitHub contributions per day is already routine for a single contributor with current
LLM tooling, and 1,000 useful PRs/day is feasible with the broader local LLM tooling
stack enabled. Even if your repo is not at that volume today, the verification system
needs to be efficient enough to absorb agentic throughput as it grows.

OpenClaw is useful as a benchmark for the pressure, not as criticism. Their published
Blacksmith runner spend of roughly $511k maps directionally to about $20/commit since
February on Blacksmith alone. If they squash-merge PRs, commit count is a reasonable
per-PR proxy, but the figure remains directional and excludes non-Blacksmith CI cost.

We agree that agentic development needs more verification, likely more than most projects
are doing today. The question is whether the verification system is *efficient enough to
run continuously* at that volume.

```
Rust gives us fast compile-time and crate-local checks.
ripr gives mutation-testing-lite value at static-analysis prices.
LEM budgeting makes spend visible before a PR spends it.
Risk-pack routing keeps expensive lanes tied to actual risk.
```

---

## perl-lsp-specific framing

`perl-lsp` already has a tiered CI conveyor: `pr_fast`, `merge_gate`, `nightly`, `release`
(see `.ci/gate-policy.yaml`). This rollout does **not** replace it. It instruments it,
assigns costs to lanes, adds `ripr` as a static oracle-gap signal, and uses actuals to
tune default PR behavior over time.

The relationship between artifacts:

| Artifact | Role |
|---|---|
| `.ci/gate-policy.yaml` | What gates execute (enforcement source of truth) |
| `.ci/GATE_REGISTRY.toml` | Gate registry for legacy/inventory mapping |
| `policy/ci-lane-whitelist.toml` | **Why** each CI item exists, where it is allowed, who owns it |
| `policy/ci-budget.toml` | LEM bands, runner multipliers, label conventions |
| `policy/ci-lanes.toml` | Lane economics metadata (intent, base LEM, default-PR flag) |
| `policy/ci-risk-packs.toml` | When extra proof is relevant |
| `policy/ci-exceptions.toml` | Debt ledger for expensive default lanes |
| `target/ci/ci-plan.json` | What this PR is expected to run |
| `target/ci/ci-actuals.json` | What it actually spent |

---

## What a useful CI minute does

A useful CI minute either:

1. blocks a likely bad merge,
2. proves a meaningful invariant,
3. narrows a failure cause,
4. produces durable timing, flake, cache, coverage, or compatibility signal.

CI minutes spent on duplicated checks, no-op jobs, broad unrelated workflows, unnecessary
model downloads, unnecessary OS runners, or non-blocking confirmation lanes are waste.

---

## Why the target is aggressive

At expected repo volume, even sub-dollar PR economics matter. A `$20` verification cost
per PR is not operationally acceptable when hundreds of PRs/day are plausible. The
rollout target is:

```text
Default Rust PR:    < $0.50, < 35 LEM preferred
High-risk PR:       up to $1 only with explicit label / reason
Main / nightly:     preserve broad verification
```

See [lem-budgeting.md](lem-budgeting.md) for the LEM model and band table.

---

## What this rollout does not do

- Does **not** weaken merge-gate shards.
- Does **not** remove Windows guardrails or memory smoke before actuals.
- Does **not** make `ripr` blocking.
- Does **not** hard-enforce a 35 LEM budget before calibration.
- Does **not** require matrix leaf checks in branch protection.
- Does **not** treat skipped optional lanes as passed without a policy reason.

The correct frame is: **more verification, better scoped, lower wasted spend, measured
before enforced.**
