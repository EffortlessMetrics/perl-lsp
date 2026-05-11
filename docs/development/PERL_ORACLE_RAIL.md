# Perl-Oracle Subprocess Environment Burndown

> **Substrate (already built)**: startup `@INC` probe with timeout + env stripping landed via #8493 (PERL5LIB gating), #8497 (bounded startup probe to prevent first-hit LSP stall), #8518 (warn-on-timeout); architecture doc PR #8558 (open) codifies the Perl subprocess ambient-input contract.
> **Connector gap**: an explicit `PerlOracleEnv` wrapper with deny-all-ambient defaults that every `ask Perl` subprocess in perl-lsp routes through, so ambient leak vectors (`PERL5LIB`, `PERL_UNICODE`, `PERL_HASH_SEED`, etc.) cannot bypass policy by going through a fresh `Command`.
> **0.14.0 upside**: deterministic, auditable Perl subprocess behavior. Users running perl-lsp in unusual shells, with workspace-local `PERL5LIB` exports, or with Carton/Carmel/local-lib environments get the same `@INC` semantics LSP analysis assumes — no silent drift, no first-hit stalls, no unexplained module-resolution divergence between editor and CLI.

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1 — architecture doc | n/a (PR-only) | yes | #8558 (open) | n/a (doc only) |
| 2 — inventory ask-Perl seams | #8620 | yes (`builder-ready`) | — | seam inventory committed under `docs/architecture/` |
| 3 — `PerlOracleEnv` v1 + startup-probe migration | #8622 | yes (`builder-ready`) | — | `cargo test -p perl-lsp-rs-core --lib config -- --nocapture --test-threads=2` (poisoned-env tests) |
| 4 — per-call-site seam migrations | deferred — successors filed after #8622 lands | no | — | per-seam unit tests |

## Exit criteria

- [ ] All phases land or are explicitly deferred with a successor.
- [ ] Receipt command in this doc reproduces the closeout proof.
- [ ] Status doc updated (`docs/project/status/*.md` regenerated post-merge).
- [ ] Claim boundary recorded.

## Claim boundary

**This rail proves**: every `ask Perl` subprocess that *routes through `PerlOracleEnv`* uses the explicit, deny-all-ambient contract. Phase 3 migrates the startup `@INC` probe — the single highest-traffic seam — as the canonical reference adoption.

**This rail does NOT prove**: that *every* call site in the workspace has been migrated. Per-call-site coverage is owned by Phase 4 successors, which are filed only after Phase 3 lands so they can branch from a stable `PerlOracleEnv` API. Until Phase 4 closes for a given crate, that crate's Perl subprocesses may still leak ambient env vars.

## Receipts

```bash
# Phase 3 receipt: poisoned-env tests around the startup @INC probe.
cargo test -p perl-lsp-rs-core --lib config -- --nocapture --test-threads=2

# Spot-check that PerlOracleEnv (when landed) is the only Command spawner.
rg -n 'Command::new\("perl"\)' crates/ | rg -v 'perl_oracle_env'

# Per-phase issue status.
gh issue view 8620
gh issue view 8622
```

## Related

- Umbrella issue: #8551 (`arch(perl-oracle): explicit ambient-environment contract for every \`ask Perl\` subprocess`).
- Architecture / spec docs: PR #8558 (architecture doc), `docs/architecture/` (post-merge location).
- Status doc: `docs/project/status/index.md`.
- Adjacent rails: `docs/development/FILE_POLICY_RAIL.md` (non-rust policy surfaces; `PERL5LIB` / Carton manifests overlap), `docs/development/CI_UX_RAIL.md` (contributors need clear signal when a Perl-oracle probe times out in CI).

## Do not combine

- Do not combine with: dependency bumps, Rust 1.95 lint cleanup, Codecov rollout, file-policy promotion, `@INC` consumer fixes (those land independently).
- Do not bundle Phase 4 seam migrations into Phase 3 — each Phase 4 PR is per-crate so reviewer can audit ambient-env decisions one seam at a time.
- Do not weaken `PerlOracleEnv` defaults to `allow-by-default`; the doctrine is deny-all-ambient with explicit opt-in.

## Lane assignment

**Builder (sonnet)** owns Phase 3 onward. The connector is small enough for one builder pass but touches policy-sensitive code (subprocess spawning, env construction, poison testing) — sonnet's verification rigor matters here.

Phase 4 successors may be parallelized across crates by spawning multiple builder agents in worktrees, since `PerlOracleEnv` is a stable API surface by then.
