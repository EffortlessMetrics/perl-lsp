# perl-lsp Roo workspace rules

When working in this repository with Roo Code:

- Read `AGENTS.md` first and treat it as the canonical implementation-agent guide.
- Keep changes scoped to one concern per PR (one fix/feature/refactor).
- Do not use `unwrap()`, `expect()`, `panic!()`, `todo!()`, or `unimplemented!()` in production code.
- Do not use `dbg!()`, `println!()`, or `eprintln!()` in library code (use `tracing`).
- In tests, avoid bare `unwrap()`; prefer `Result<()>` or helpers from `perl_tdd_support`.
- Run crate-local verification before finishing:
  - `cargo test -p <crate>`
  - `cargo check --all-targets -p <crate>`
  - `cargo clippy -p <crate>`
  - `cargo xtask fmt`
- Before opening a PR, run `just pr-fast`.
- Use commit title format: `type(scope): description (#NNNN)` (use `#0000` when unknown).

Primary project docs:

- Contributor workflow: `CONTRIBUTING.md`
- AI implementation workflow: `AGENTS.md`
- Command reference: `docs/reference/COMMANDS_REFERENCE.md`
- LSP implementation patterns: `docs/reference/LSP_IMPLEMENTATION_GUIDE.md`
