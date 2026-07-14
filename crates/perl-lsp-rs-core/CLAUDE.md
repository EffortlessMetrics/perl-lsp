# CLAUDE.md (perl-lsp-rs-core)

## Role

Core LSP server logic: the absorbed implementation of nearly every LSP
provider, protocol type, and server runtime concern, previously split across
roughly thirty separate crates that were folded into this one over several
absorption waves ("G1a"/"G1b" providers, "G2" runtime, "G3"
transport/tooling/uri).

## Owns

By module (see `src/lib.rs` doc comments for the authoritative list):

- `capability_map` -- feature-catalog-to-client-capability translation
- `config` -- runtime configuration loading, validation, compat adapters
- `critic_parser` -- Perl::Critic output parsing
- `feature_catalog` / `features` -- feature model, IDs, registry, capability gating
- `governance` -- feature-profile / rollout policy APIs
- `hashing` -- shared hashing helpers
- `performance` -- caches and allocation strategies for large workspaces
- `platform` -- cross-platform interpreter/toolchain detection
- `protocol` -- JSON-RPC/LSP types (capabilities, errors, jsonrpc, methods)
- `providers` -- every LSP request/notification provider (completion,
  navigation, code_actions, diagnostics, rename, formatting,
  semantic_tokens, folding, inlay_hints, ...)
- `runtime` -- cancellation, resource limits, input validation, process
  launcher, text-editing utilities
- `tooling` -- perlcritic/perltidy integration glue
- `transport` -- Content-Length message framing for stdio/socket
- `uri` -- URI parsing/conversion for protocol-facing code

## Does not own

- The tower-lsp `LanguageServer` trait wiring and binary entrypoint --
  that's `perl-lsp-rs`.
- Perl parsing itself -- delegates to `perl-parser-core` / `perl-parser` /
  `perl-lexer`.
- Module resolution -- delegates to `perl-module`.
- The DAP protocol -- that's `perl-dap`, which depends on this crate only
  for shared runtime/protocol types, not for LSP behavior.

## Neighbors

- Upstream: `perl-parser-core`, `perl-parser`, `perl-lexer`, `perl-ast`,
  `perl-module`, `perl-position-tracking`, `perl-semantic-analyzer`,
  `perl-semantic-facts`, `perl-subprocess-runtime`, `perl-lsp-perltidy`,
  `perl-diagnostics`, `perl-workspace`, `perl-pragma`, `perl-symbol`.
- Downstream: `perl-lsp-rs` (primary consumer, wires this into the actual
  server), `perl-dap` (shared protocol/runtime types).

## Read first

- `src/lib.rs` -- module map with one-line doc comments per module.
- `src/providers/mod.rs` -- provider grouping and absorption-wave history.
- `src/runtime/mod.rs` -- absorption notes, including why
  `perl-lsp-transport` was deliberately NOT absorbed (dependency cycle with
  `perl-lsp-protocol`).
- `src/transport/mod.rs` -- framing implementation.

## Focused validation

`cargo test -p perl-lsp-rs-core`. Look specifically for:

- `tests/*_module_shape.rs` -- assert absorbed-module boundaries haven't
  regressed.
- `tests/g3_*.rs` and `tests/wave_final_absorption_tests.rs` -- structural
  guarantees from the absorption waves; run these before changing module
  layout.

## Review hotspots

This crate absorbed many previously-independent crates -- the main risk is
cross-module coupling introduced by an "easy" absorption. The `runtime`
module doc explicitly flags `text_utils` as provider-adjacent code kept here
for organizational reasons only, and `perl-lsp-transport` as deliberately
excluded due to a dependency cycle -- don't move code across that boundary
without re-reading the cycle rationale first.

## Claim boundary

Reflects module structure as authored in `lib.rs` doc comments and
`Cargo.toml` dependencies. Makes no claim about current provider completeness
or feature-flag rollout state -- see `features.toml` for that.
