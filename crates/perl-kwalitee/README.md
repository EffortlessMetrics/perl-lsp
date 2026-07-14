# perl-kwalitee

A **Perl distribution Kwalitee evaluator** for the perl-lsp native stack.

"Kwalitee" (from CPAN's `Module::CPANTS`) is *measurable distribution quality* —
objective, checkable indicators about how a distribution is shipped — as
distinct from subjective code quality. This crate is the scoreboard that says
whether the native lanes (DAP, native critic, native formatter, release
archives) are truly shippable.

## What it owns

- the **indicator model** (`KwaliteeIndicator`, `IndicatorStatus`, `EvidenceRef`),
- the **profiles** (`pr`, `release`, `nightly`),
- the **scoring rules** and overall verdict (`pass` / `warn` / `fail`),
- the **receipt schema** (`kind = "perl_kwalitee"`, `schema_version = 1`) and its
  JSON + Markdown renderings.

## Design: pure by construction

The crate never spawns a subprocess or touches the network. Each indicator is
evaluated either:

- from the repository filesystem (Cargo manifests, first-mile doc surfaces), or
- by reading a JSON receipt another tool produced (native-tooling readiness,
  quality-gate), or
- from an **external result** the caller supplies (`ExternalResult`) after
  running a heavier gate itself.

This keeps evaluation deterministic and unit-testable. The
`cargo xtask perl-kwalitee` command is the repo-local wrapper that runs the
heavier gates and wires their results, receipt paths, and repo paths in.

## Usage

```rust
use perl_kwalitee::{evaluate, KwaliteeOptions, KwaliteeProfile};

let options = KwaliteeOptions::new("/path/to/repo", KwaliteeProfile::Pr);
let receipt = evaluate(&options);
println!("{}", receipt.to_markdown());
```

From the command line:

```bash
cargo xtask perl-kwalitee check   --profile pr
cargo xtask perl-kwalitee check   --profile release --dist dist --strict
cargo xtask perl-kwalitee report  --profile release --dist dist \
  --json target/receipts/kwalitee/perl-kwalitee.json \
  --markdown target/receipts/kwalitee/perl-kwalitee.md
cargo xtask perl-kwalitee explain release.no_external_tooling
```

See [`docs/reference/PERL_KWALITEE.md`](../../docs/reference/PERL_KWALITEE.md) for
the full indicator catalog, profile matrix, and receipt schema.

## Publishability

The crate is currently `publish = false` while the receipt schema (v1)
stabilizes. Promoting it to a public crate is a deliberate follow-up: flip the
flag, add `perl-kwalitee` to `[workspace.metadata.publish].allow` in the root
`Cargo.toml`, and confirm with `cargo xtask publish-manifest-check`.

## License

MIT OR Apache-2.0 (workspace-inherited).
