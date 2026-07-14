# Perl Kwalitee

**Perl distribution Kwalitee** is *measurable distribution quality* for the
perl-lsp native stack — objective, checkable indicators about **how the product
is shipped**, as distinct from subjective code quality. The name is borrowed
from CPAN's `Module::CPANTS`, which coined "kwalitee" for the same idea:
quality-adjacent metrics you can actually compute.

Kwalitee is the scoreboard that answers one question: *are the native lanes
(DAP, native critic, native formatter, release archives, publish metadata)
truly shippable?*

- The [`perl-kwalitee`](../../crates/perl-kwalitee/) crate owns the durable
  contract: the indicator model, the profiles, the scoring rules, and the
  versioned receipt schema.
- The `cargo xtask perl-kwalitee` command is the repo-local wrapper that runs
  the heavier gates and wires their results, receipt paths, and repo paths into
  the crate.

## Command surface

```bash
cargo xtask perl-kwalitee check   --profile pr
cargo xtask perl-kwalitee check   --profile release --dist dist --strict
cargo xtask perl-kwalitee report  --profile release --dist dist \
  --json target/receipts/kwalitee/perl-kwalitee.json \
  --markdown target/receipts/kwalitee/perl-kwalitee.md
cargo xtask perl-kwalitee explain release.no_external_tooling
```

- **`check`** evaluates and exits non-zero on a `fail` verdict (this is the CI
  gate).
- **`report`** evaluates and writes the JSON + Markdown receipts; it does not
  fail the process (use `check` to gate). Defaults:
  `target/receipts/kwalitee/perl-kwalitee.{json,md}`.
- **`explain <id>`** prints why an indicator exists and how to fix a non-pass.

## Profiles

| Profile   | Speed | Release archives | Use |
|-----------|-------|------------------|-----|
| `pr`      | fast  | not applicable   | per-PR gate |
| `release` | strict, requires `--dist` | mandatory | release gate |
| `nightly` | broad | not applicable   | pr floor + receipt-heavy advisory rows |

Under `pr`/`nightly` the release-archive indicators are reported
`not_applicable`. Under `release`, a missing `--dist` fails the release
indicators. The nightly profile adds a set of **advisory** (non-mandatory)
receipt-backed indicators that only run under `nightly`; on `pr`/`release` they
are reported `not_applicable`.

## Indicator catalog

Weights are grouped by area so the numeric score reflects distribution-quality
priorities (product surface, release, and native tooling dominate). The score
is deliberately secondary — the important signal is the **mandatory indicator
table**.

| Indicator | Area | Mandatory | Source | Applies |
|-----------|------|-----------|--------|---------|
| `manifest.workspace_member_declared` | manifest | yes | native (root Cargo.toml) | all |
| `manifest.publish_policy_clean` | manifest | yes | native (crate Cargo.toml + allowlist) | all |
| `license.declared` | license | yes | native (crate Cargo.toml) | all |
| `product_surface.native_only` | product_surface | yes | native (first-mile surface scan) | all |
| `dap.cli_native_only` | dap | yes | native (perl-dap CLI source scan) | all |
| `release.native_binaries_present` | release | yes | external (`release artifact-check`) | release |
| `release.no_external_tooling` | release | yes | external (`release artifact-check`) | release |
| `release.checksums_valid` | release | yes | external (`release artifact-check`) | release |
| `formatter.native_default` | formatter | yes | receipt (native-tooling readiness) | all |
| `critic.native_default` | critic | yes | receipt (native-tooling readiness) | all |
| `critic.run_critic_registry_parity` | critic | yes | external (parity test) | all |
| `quality.no_new_severe_gaps` | quality | yes | receipt (quality-gate) | all |
| `docs.status_current` | docs | yes | external (`update-status --check`) | all |
| `formatter.corpus_idempotent` | formatter | **advisory** | receipt (native-format corpus) | nightly |
| `critic.no_false_positives` | critic | **advisory** | receipt (native-critic false-positive fixtures) | nightly |
| `formatter.perltidy_compat_no_external_only` | formatter | **advisory** | receipt (native-format perltidy-compat) | nightly |
| `critic.perlcritic_compat_no_external_only` | critic | **advisory** | receipt (native-tooling perlcritic-compat) | nightly |

### Nightly advisory indicators

These are non-mandatory and evaluated only under the `nightly` profile. Each
reads a JSON receipt another xtask task produces; an unhealthy result is a
`warn` (never a mandatory `fail`), a missing receipt is `unverified`:

- `formatter.corpus_idempotent` ← `native-format-corpus.json` (`passed == true`)
- `critic.no_false_positives` ← `native-critic-false-positive.json`
  (`findings_count == 0` and no suppressed findings / parse errors)
- `formatter.perltidy_compat_no_external_only` ←
  `native-format-perltidy-compat.json` (`external_only_count == 0`)
- `critic.perlcritic_compat_no_external_only` ← `perlcritic-compat.json`
  (`external_only_count == 0`)

#### Generating the nightly receipts

Because the crate is pure, the nightly indicators are `unverified` unless the
upstream receipts already exist on disk. The nightly CI job
(`.github/workflows/ci-nightly.yml`, "Perl Kwalitee (advisory)") generates them
before `report --profile nightly`. Every generator is native Rust — no
`perltidy`/`perlcritic` install is required — and `status`/`readiness`
aggregate the receipts above them, so they run last:

```bash
# native formatter receipts
cargo xtask native-format check
cargo xtask native-format corpus                       # → formatter.corpus_idempotent
cargo xtask native-format config \
  --receipt target/receipts/format/native-format-config.json \
  --summary target/receipts/format/native-format-config.md   # enables formatter.native_default
cargo xtask native-format perltidy-compat --profile .ci/kwalitee/perltidyrc \
  --receipt target/receipts/format/native-format-perltidy-compat.json \
  --summary target/receipts/format/native-format-perltidy-compat.md
                                                       # → formatter.perltidy_compat_no_external_only

# native critic receipts
cargo xtask native-critic check                        # default roots (status input)
cargo xtask native-critic check \
  --root xtask/tests/fixtures/native-critic/false-positive \
  --receipt target/receipts/native-tooling/native-critic-false-positive.json \
  --summary target/receipts/native-tooling/native-critic-false-positive.md
                                                       # → critic.no_false_positives
cargo xtask native-tooling perlcritic-compat --profile .ci/kwalitee/perlcriticrc \
  --receipt target/receipts/native-tooling/perlcritic-compat.json \
  --summary target/receipts/native-tooling/perlcritic-compat.md
                                                       # → critic.perlcritic_compat_no_external_only

# aggregation — must come last
cargo xtask native-tooling status \
  --receipt target/receipts/native-tooling/status.json
cargo xtask native-tooling readiness \
  --status-receipt target/receipts/native-tooling/status.json \
  --receipt target/receipts/native-tooling/readiness.json
                                          # → formatter.native_default + critic.native_default
```

The two `*-compat` commands take a required `--profile` input. They classify a
`.perltidyrc` / `.perlcriticrc` natively (they never run the real tools), so the
repo commits reference profiles at `.ci/kwalitee/perltidyrc` and
`.ci/kwalitee/perlcriticrc` — both chosen so a healthy native surface reports
`external_only_count == 0`. Keep those profiles in sync when the native
formatter/critic support surface changes.

`quality.no_new_severe_gaps` (mandatory on every profile) reads
`quality/quality-gate.json`, which depends on the RIPR receipt chain
(`cargo xtask quality-gate`) rather than the native-tooling receipts above; the
nightly job does not yet generate it, so that indicator stays `unverified`
(a `warn` under the advisory nightly profile). Wiring it is tracked separately.

### Evidence sources

The crate is **pure** — it never spawns a subprocess. Each indicator is sourced
one of three ways:

- **native** — computed by the crate from the repository filesystem (Cargo
  manifests, first-mile doc surfaces).
- **receipt** — read by the crate from an existing JSON receipt:
  - `formatter.native_default` / `critic.native_default` ←
    `target/receipts/native-tooling/readiness.json`
    (`cargo xtask native-tooling readiness`);
  - `quality.no_new_severe_gaps` ← `target/receipts/quality/quality-gate.json`
    (`cargo xtask quality-gate`).
  A receipt whose embedded commit differs from HEAD downgrades a `pass` to
  `warn` (stale evidence is not trusted).
- **external** — the xtask wrapper runs the heavier gate and feeds the result
  in:
  - `release.*` ← `cargo xtask release artifact-check --dist <dir>` (one run,
    validating binaries present + no external tooling + checksums, mapped onto
    the three release indicators);
  - `docs.status_current` ← `cargo xtask update-status --check`;
  - `critic.run_critic_registry_parity` ← `cargo test -p perl-lsp-rs --lib
    execute_command::tests::run_critic_native_matches_pull_diagnostics_registry
    -- --exact` (live-workspace only, same as `docs.status_current`).

The authoritative product-surface CI gate remains
`cargo xtask check-native-product-surface`; the crate mirrors its surface and
marker lists so `perl_kwalitee::evaluate` produces a real verdict from the repo
alone. Under the `release` profile the mirror additionally bans raw external
tool names (`Perl::LanguageServer`, `perltidy`, `perlcritic`, `Perl::Critic`,
`Devel::TSPerlDAP`, `TSPerlDAP.pm`) on first-mile surfaces — the stricter
"if it is not native, we do not ship it" bar. `BridgeAdapter` and `--bridge`
are banned on all profiles.

`dap.cli_native_only` is a native indicator that scans the `perl-dap` CLI
source for a reintroduced `--bridge` clap flag (the legacy proxy to
`Perl::LanguageServer`, removed in #3277). It looks for an actual flag
definition (`long = "bridge"`), not the string `"--bridge"`, so it does not
false-positive on the crate's own regression test.

### `critic.run_critic_registry_parity`

This indicator is **mandatory** (promoted from advisory once #3303 landed the
`NativeCriticRegistry` routing and its parity test). It asserts that the
default `perl.runCritic` command and the editor's on-type native pull
diagnostics agree (both routed through `NativeCriticRegistry`). The xtask
wrapper supplies the result by running the proof test as an external command —
`cargo test -p perl-lsp-rs --lib
execute_command::tests::run_critic_native_matches_pull_diagnostics_registry --
--exact` — and mapping its exit status onto the indicator, live-workspace only
(same `--repo-root` exemption as `docs.status_current`). A missing `cargo`/test
failure reports `fail` with the test command and stderr as evidence.

## Scoring and verdict

- a mandatory `fail` ⇒ verdict **fail**, score capped at 89;
- a mandatory `unverified` ⇒ **fail** under `--strict`, otherwise **warn**;
- warnings only ⇒ verdict **warn**, score banded to 90–99;
- all applicable indicators pass ⇒ score **100**.

`not_applicable` indicators are excluded from scoring entirely.

## Receipt schema (`schema_version = 1`)

`kind = "perl_kwalitee"`. Top-level fields:

| Field | Type | Meaning |
|-------|------|---------|
| `kind` | string | `"perl_kwalitee"` |
| `schema_version` | int | `1` |
| `generated_at` | string | RFC 3339 timestamp (caller-supplied) |
| `commit` | string | git HEAD the evaluation reflects |
| `profile` | string | `pr` / `release` / `nightly` |
| `score` | int | 0–100 |
| `verdict` | string | `pass` / `warn` / `fail` |
| `mandatory_passed` | bool | every mandatory indicator passed |
| `mandatory_failed_count` | int | mandatory indicators that failed |
| `mandatory_unverified_count` | int | mandatory indicators that are unverified (drive `fail` under `--strict`) |
| `warning_count` | int | indicators in `warn` |
| `unverified_count` | int | indicators in `unverified` |
| `indicators` | array | the full indicator table |

Each indicator:

| Field | Type | Meaning |
|-------|------|---------|
| `id` | string | stable dotted id |
| `area` | string | coarse grouping |
| `title` | string | one-line title |
| `mandatory` | bool | blocks the mandatory gate |
| `status` | string | `pass` / `fail` / `warn` / `not_applicable` / `unverified` |
| `score_weight` | int | weight in the numeric score |
| `evidence` | array | `{ kind, value }` pointers; `kind` is an open set — the evaluator emits `command` / `receipt` / `file` / `test` / `criterion` / `decision` / `note` |
| `remediation` | string? | how to fix a non-pass |

## Publishability

The crate is currently `publish = false` while schema v1 stabilizes. Promotion
to a public crate is a deliberate follow-up:

1. flip `publish = false` in `crates/perl-kwalitee/Cargo.toml`;
2. add `perl-kwalitee` to `[workspace.metadata.publish].allow` (in topological
   order — it is a leaf, so early);
3. confirm with `cargo xtask publish-manifest-check` and
   `cargo publish -p perl-kwalitee --dry-run`.

The `manifest.publish_policy_clean` indicator checks exactly this: the policy is
"clean" when it is intentional — either explicitly `publish = false` or present
in the allowlist.

## How this ties back to DAP, critic, tidy

| Lane | Kwalitee indicator |
|------|--------------------|
| Release archives include the native binaries | `release.native_binaries_present` |
| Release archives exclude PLS / external tools | `release.no_external_tooling` |
| First-mile docs stay native | `product_surface.native_only` |
| Formatter defaults native | `formatter.native_default` |
| Critic defaults native | `critic.native_default` |
| `perl.runCritic` uses the native registry | `critic.run_critic_registry_parity` |
| No new severe coverage/ripr regressions | `quality.no_new_severe_gaps` |
| Generated status docs current | `docs.status_current` |
