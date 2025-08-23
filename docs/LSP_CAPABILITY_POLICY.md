# LSP Capability Policy

**Contract-driven:** A capability is advertised only after its acceptance tests land.

- **Main branch:** full surface (only features with passing tests).
- **Conservative point release:** build with `--features lsp-ga-lock` to reduce the surface to the proven "GA core".

### CI

- Default: `cargo test --workspace`
- Lock sentinel (optional):  
  `cargo test -p perl-parser --features lsp-ga-lock --test lsp_capabilities_contract_lock`

### Adding a new capability

1. Implement feature in `crates/perl-parser/src/*`.
2. Add acceptance tests in `crates/perl-parser/tests/…`.
3. Flip the advertised bit in `lsp_server.rs` **in the same PR**.
4. Update:
   - `LSP_ACTUAL_STATUS.md` (status/percent)
   - `README.md` (matrix row)
   - Contract tests (`lsp_capabilities_contract_full.rs`)