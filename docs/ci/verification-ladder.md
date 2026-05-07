# Verification Ladder

Layered verification, ordered cheapest-to-most-expensive. Each rung answers a different
proof obligation. Ordinary PRs buy the cheap rungs; main / nightly / release / labeled
PRs buy the deep rungs.

> Companion: [cost-and-verification-policy.md](cost-and-verification-policy.md),
> [lem-budgeting.md](lem-budgeting.md), [labels.md](labels.md).

---

## Ladder

| Layer | Default PR? | Purpose |
|---|---:|---|
| `cargo check` | yes | type and feature wiring |
| `cargo fmt` | yes | mechanical consistency |
| `cargo clippy` | yes | static policy / lint guard |
| unit and oracle tests | yes | deterministic behavior proof |
| UX regression smoke | yes (LSP/UX risk) | first-five-minutes user-visible behavior |
| LSP memory smoke | yes (retained-state risk) | retained-state plateau |
| Windows guardrails | yes (platform risk) | path / sandbox / module-separator regressions |
| `ripr` | advisory first | static mutation-shaped oracle-gap signal |
| bounded property tests | selective / label | input-space confidence |
| coverage | main / `coverage` label | execution surface |
| mutation testing | nightly / `mutation` label | runtime adequacy confirmation |
| fuzzing | nightly | robustness |
| OS / hardware / Docker / model checks | main / label / release | platform and integration proof |
| release dry-run / semver | release / `release-check` label / main | publishability |

---

## Proof-obligation map

Every CI lane should answer one row of this table. If two lanes answer the same row, one
is a duplicate of the other and one belongs in `duplicate_of` in
[`policy/ci-lane-whitelist.toml`](../../policy/ci-lane-whitelist.toml).

| Failure mode | Cheapest lane that catches it | Deep lane |
|---|---|---|
| compile break | `cargo check` | all-features / all-targets |
| formatting drift | `cargo fmt` | — |
| lint / banned-pattern violation | `cargo clippy` | strict clippy |
| changed behavior lacks oracle | `ripr` advisory | mutation testing |
| unit-level regression | scoped `cargo test` | nextest workspace |
| LSP UX regression | UX regression smoke | full UX harness + real-repo latency |
| retained-state regression | LSP memory smoke | memory plateau |
| Windows path / sandbox regression | Windows guardrails | platform matrix |
| extension regression | VS Code Linux smoke | VS Code OS matrix |
| dependency vuln | cargo audit | cargo deny / Trivy |
| serialization break | schema fixtures | compatibility corpus |
| public API break | public-API check | release dry-run |
| docs break | docs gate | docs deploy |

---

## ripr's place on the ladder

`ripr` is **mutation-testing-lite at static-analysis prices**. It does not run mutants,
does not emit killed/survived counts, and does not replace mutation testing. It asks the
mutation-testing-shaped question — "is the changed behavior exposed to a meaningful test
discriminator?" — earlier and cheaper, using static analysis only.

Severity vocabulary used in PR Plan summaries:

```text
exposed              — behavior change is statically reachable and has a nearby
                       discriminating test
weakly_exposed       — reachable, weakly-discriminating test only
reachable_unrevealed — reachable, no discriminating test found
no_static_path       — analysis could not find a reachable path
infection_unknown    — could not classify infection
propagation_unknown  — could not classify propagation
static_unknown       — analysis bottomed out
```

Do **not** use runtime-mutation vocabulary (`killed`, `survived`) when reporting `ripr`
results.
