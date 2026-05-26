# Inline Completion Release Gate

This gate locks the embedded `perllsp` binary as a standard LSP 3.18
`textDocument/inlineCompletion` target for editor integration. It does not claim
full LSP 3.18 coverage or general release readiness.

Gate name:

```text
inline-completion-release-gate
```

Run the gate before release-adjacent inline-completion changes:

```bash
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_inline_completion_registration_tests --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_ai_inline_completion_tests --features expose_lsp_test_api --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs-core --lib inline_completion --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_streaming_completion_tests --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_cap_snap --profile agent --locked
./scripts/cargo-safe build -p perllsp --profile agent --locked
./scripts/cargo-safe xtask inline-completion-smoke --binary target/agent/perllsp
```

On Windows, use the `.exe` binary path. If `cargo-safe` uses an external
`CARGO_TARGET_DIR`, pass that built binary path to `inline-completion-smoke`.

The stdio smoke proves:

- static clients receive top-level `inlineCompletionProvider`;
- dynamic clients omit the static provider and receive
  `client/registerCapability` for `textDocument/inlineCompletion`;
- the dynamic registration id is `perl-inlineCompletion`;
- the dynamic registration selector includes `perl` and `perl5`;
- `experimental.inlineCompletionProvider` is absent;
- `experimental.perlInlineCompletionStream` remains a separate vendor extension
  when inline completion is enabled;
- `use ` returns the deterministic `strict;` item;
- a neutral position returns an empty item list;
- `disabledFeatures: ["lsp.inline_completion"]` removes the provider and stream
  flag, prevents dynamic registration, and rejects runtime requests.
