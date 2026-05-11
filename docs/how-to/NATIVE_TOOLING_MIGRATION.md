# Native Tooling Migration

This guide explains how to move from shelling out to `perltidy` and
`perlcritic` toward the Rust-native formatter and native critic surfaces in
`perl-lsp`.

The migration target is native-first tooling:

- native formatting for normal editor formatting and range formatting
- native critic diagnostics for structured LSP findings, suppressions, and code
  actions
- external `perltidy` and `perlcritic` only when a project still needs exact
  legacy behavior

The current native path is useful, but it is still intentionally conservative.
Use the status and compatibility receipts below to decide whether a project can
move fully native or should keep an external compatibility path for now.

## Check Current Coverage

Start with the generated native-tooling dashboard:

```bash
cargo xtask native-tooling status \
  --markdown docs/project/status/native_tooling.md
cargo xtask native-tooling readiness \
  --markdown docs/project/status/native_tooling_readiness.md
```

The dashboard summarizes formatter fixtures, literal-preserve bailouts, native
critic rule coverage, code-action coverage, and compatibility receipt counts.
See [Native Tooling Status](../project/status/native_tooling.md) for the current
checked-in snapshot. The readiness report turns those receipt counts into
explicit default-cutover criteria; see
[Native Tooling Readiness](../project/status/native_tooling_readiness.md).
Readiness also verifies that the receipts it summarizes were generated for the
current commit, so stale local artifacts cannot silently support a cutover
claim.

For formatter proof, run:

```bash
cargo xtask native-format check
cargo xtask native-format corpus
```

For native critic proof, run:

```bash
cargo xtask native-critic check
```

For compatibility reporting, run:

```bash
perllsp --perltidy-compat-report .perltidyrc
perllsp --perlcritic-compat-report .perlcriticrc
```

Those installed-binary commands print the same native-support classification a
team needs during migration without requiring a checkout of the `xtask`
developer tooling. For receipt-backed repository proof, run:

```bash
cargo xtask native-format perltidy-compat \
  --profile .perltidyrc \
  --receipt target/receipts/format/native-format-perltidy-compat.json \
  --summary target/receipts/format/native-format-perltidy-compat.md

cargo xtask native-tooling perlcritic-compat \
  --profile .perlcriticrc \
  --receipt target/receipts/native-tooling/perlcritic-compat.json \
  --summary target/receipts/native-tooling/perlcritic-compat.md
```

These commands do not change runtime behavior. They classify existing legacy
profiles against native support so teams can migrate deliberately.
Their Markdown summaries include suggested native `.perl-lsp.toml` snippets for
compatible formatter and critic settings, plus explicit notes for options or
filters that still need manual review.

## Formatter Migration

The native formatter is the default formatter engine in the Rust formatting
provider. The subprocess-backed `PerlTidyFormatter` remains available as a
compatibility adapter and comparison surface.

Use native formatting when:

- the native-format fixture and corpus receipts pass
- the project does not require exact `perltidy` output
- `.perltidyrc` options are classified as `supported`, `approximated`, or
  `unsupported_safe`
- unsupported literal surfaces are preserved or produce explicit diagnostics

The perltidy compatibility report includes a suggested `[formatting]` block. It
maps supported options such as line width, indentation, tabs, brace placement,
else placement, keyword spacing, and trailing comma policy to native formatter
config keys. It also lists ignored execution/output flags, approximated presets,
invalid values, and external-only options separately so teams know what still
requires deliberate review.

Keep external `perltidy` compatibility when:

- review policy requires byte-for-byte `perltidy` output
- `.perltidyrc` contains options classified as `external_only`
- a project depends on layout behavior that native formatting has not learned
  yet

The native formatter should not silently rewrite risky Perl constructs. When it
cannot safely format a surface such as POD, heredocs, DATA or END sections,
regexes, substitutions, quote-like forms, or format bodies, it preserves the
source and records the unsupported surface through native-format receipts.

## Critic Migration

The native critic recommended profile is the default critic engine. Legacy
`perlcritic` remains available as an explicit compatibility adapter for teams
that still need exact Perl::Critic policy behavior.

Default native project config is equivalent to:

```toml
[diagnostics]
perlcritic = true
perlcritic_severity = 3

[critic]
engine = "native"
profile = "recommended"
exclude = ["native.documentation.require_pod_sections"]
```

Use native critic diagnostics when:

- the needed policies map to native rule IDs in the compatibility receipt
- suppressions and severity filtering match the project policy
- editor diagnostics should expose stable rule IDs, precise spans, and code
  actions

Native critic include/exclude filters use native rule IDs, not Perl::Critic
policy names. During migration, the compatibility report includes a suggested
`[critic]` TOML block with mapped native IDs for compatible `include` and
`exclude` policy names. Review any unmapped legacy filters before adding the
snippet to `.perl-lsp.toml`.

Keep external `perlcritic` compatibility when:

- the project depends on policies classified as `external_only`
- theme expansion or profile loading behavior is required
- the team needs exact Perl::Critic policy output during a transition window

To select the external adapter deliberately:

```toml
[critic]
engine = "legacy" # or "perlcritic" / "external"
```

Native critic diagnostics use `source: perl-lsp-critic` and rule IDs such as
`native.io.two_arg_open` or `native.variables.unused_lexical`. Inline
suppressions can use the native suppression prefix:

```perl
## no perl-lsp-critic native.variables.unused_lexical -- migration exception
```

Use `cargo xtask native-critic check` to run the native recommended rule set
over source files without involving editor diagnostics or the external
`perlcritic` adapter. The command emits JSON and Markdown receipts with files
checked, rules run, findings, suppressed findings, and fixable findings.

## Interpret Compatibility Results

Compatibility receipts use these classifications:

| Classification | Meaning |
| --- | --- |
| `supported` / `native_equivalent` | Native behavior directly covers the legacy option or policy. |
| `native_superset` | Native behavior covers the legacy policy and adds more precise or broader checks. |
| `approximated` | Native behavior is close, but not a one-to-one legacy match. Review before cutover. |
| `unsupported_safe` | The legacy setting has no runtime effect on native tooling or can be ignored safely. |
| `external_only` | Keep the external adapter if this behavior is required. |

After regenerating compatibility receipts, refresh the dashboard:

```bash
cargo xtask native-tooling status \
  --markdown docs/project/status/native_tooling.md
cargo xtask native-tooling readiness \
  --markdown docs/project/status/native_tooling_readiness.md
```

To verify native paths have not accidentally regressed to shelling out, run:

```bash
cargo xtask native-tooling check-defaults
```

## PR Proof

For PRs that touch native formatter, native critic, or compatibility reporting,
include the relevant receipt commands in the proof packet:

```bash
cargo xtask fmt
cargo xtask native-format check
cargo xtask native-format corpus
cargo xtask native-critic check
cargo xtask native-tooling status \
  --markdown docs/project/status/native_tooling.md
cargo xtask native-tooling readiness \
  --markdown docs/project/status/native_tooling_readiness.md
cargo xtask native-tooling check-defaults
cargo xtask check-memory-lifecycle-policy
cargo xtask check-memory-retained-owner-drift --base origin/master
cargo xtask devex pr-body --base origin/master
git diff --check
```

Do not make native tooling the default for more surfaces just because a single
fixture passes. Cutover should happen only after fixture, corpus,
compatibility, LSP, and code-action receipts are stable.
