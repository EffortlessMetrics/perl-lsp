# Review Gates

This document describes the advisory AI review gates running on PRs in this
repository.

## ub-review

`ub-review` builds a targeted evidence packet for unsafe/native-boundary code
changes and posts one grouped Pull Request Review with sensor and model-lane
findings. It is advisory: the job runs with `continue-on-error: true` and is
not listed as a required branch-protection check. A failing or skipped
ub-review job does not block merge.

### What it reviews

`ub-review` targets the unsafe-boundary review surface:

- `cargo-allow` — exception ledger drift (new `unsafe` blocks added without a
  matching `deny.toml` allow entry, or existing entries silently broadened).
- `ripr` — changed Rust behavior exposed to weak or absent test oracles.
- `unsafe-review` — changed unsafe code without a reviewable safety contract,
  precondition guard, layout/alignment witness, or aliasing/lifetime evidence.
- `tokmd` — deterministic LLM-ready diff context packet.
- `ast-grep` — cheap structural route scans on changed source.
- `actionlint` — workflow changes.

Missing sensor output is recorded as missing evidence, never as clean evidence.

### The unsafe-review boundary: coverage instrument vs cockpit

`unsafe-review` is a **reviewability** sensor, not a soundness prover. It asks:
does the changed unsafe code have the artifacts a reviewer needs to assess
safety? Those artifacts are: a `SAFETY:` comment, precondition guards, layout
or alignment witnesses, aliasing/lifetime evidence, a local test that reaches
the unsafe block, and a route to a meaningful oracle.

This is intentionally narrower than Miri or ASAN. Miri proves a specific
concrete execution path is UB-free; `unsafe-review` asks whether the *review*
cockpit is equipped. A passing `unsafe-review` run means a human or model
reviewer can assess the change; it does not mean the change is sound. A failing
run means the safety evidence the reviewer would use is absent.

Heavy witnesses (builds, tests, Miri, ASAN, mutation testing) are disabled by
default (`allow-heavy: false`) and are never enabled in the advisory phase.

### Configuration

`policy/ub-review.toml` is the repo-local config passed to the action. It
selects the `bun-ub-v0` profile as the closest available preset for Rust repos
(a perl-lsp-specific profile is planned for PR 2 of the ub-review program).
Runner-size profiles and per-tool sensor thresholds are PR 2 and PR 3 scope.

### Invocation method

The workflow uses `EffortlessMetrics/ub-review` as a GitHub composite action.
The action's `install-mode=auto` first tries to download a Linux x64 release
archive; if no release asset exists for the current ref, it falls back to a
source build from the action repository. No public release asset exists for
the current pinned SHA (`804d198b5a15a0df94bb4f43750dba71165916cd`), so first
runs perform a source build. This is slower (~5 min extra) but deterministic.

**Runner routing (temporary):** The router currently forces GitHub-hosted
runners for all ub-review jobs. Self-hosted CX runners are Docker-only
(no host-level Rust), so `install-mode=auto`'s source-build fallback fails
with "cargo is unavailable and rustup is not installed" (evidence: PR #1218,
run 27086277244, adoption datum #3). The CX job YAML is preserved in the
workflow for a 3-line revert once EffortlessMetrics/ub-review#343 ships a
release artifact.

### Secrets

`MINIMAX_API_KEY` is required for model review lanes. `OPENCODE` is an optional
fallback. Both are org-level secrets. The `Secret preflight` step in the
workflow fails clearly if `MINIMAX_API_KEY` is absent before any model call.
Neither secret value is echoed in logs.

### Reproducing locally

```bash
# Install ub-review (from source if no release is available):
cargo install --git https://github.com/EffortlessMetrics/ub-review ub-review

# Run the advisory sensors without model lanes:
ub-review run \
  --config policy/ub-review.toml \
  --profile gh-runner \
  --base origin/main \
  --head HEAD \
  --out target/ub-review \
  --posting artifact-only \
  --model-mode off

# Read the summary:
cat target/ub-review/running-summary.md
```

To include model lanes, set `MINIMAX_API_KEY` in the environment and pass
`--posting review --model-mode auto`.

### Program plan

The ub-review integration ships as a three-PR program:

| PR | Scope |
|----|-------|
| PR 1 (this) | Advisory workflow scaffold, fork-policy gate, router, lane whitelist, docs. |
| PR 2 | Runner-size profiles and perl-lsp-specific review profile. |
| PR 3 | Unsafe-review sensor policy and per-tool thresholds for this repo's actual unsafe surface. |

Promotion from advisory to a required gate requires evidence from real PR runs,
calibration of false-positive rates, and a scope decision on which unsafe
surfaces warrant blocking.
