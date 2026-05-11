# DAP Module Journey Burndown

> **Substrate (already built)**: DAP breakpoint infrastructure in `crates/perl-dap/`; the smoke-test pattern in `crates/perl-dap/tests/dap_smoke_e2e.rs` (tempdir fixture, `write()` script files, `DebugAdapter::new()`, channel event handling, `handle_*` event loop).
> **Connector gap**: a smoke receipt that proves `script.pl` → `lib/Foo.pm` path mapping resolves correctly when the debuggee imports a module. The DAP runtime can attach and set breakpoints today; what is missing is a recorded test that says script-side breakpoints survive a hop into module code.
> **0.14.0 upside**: users debugging Perl scripts that `use` workspace modules get a smoke-verified, regression-guarded path-mapping guarantee, not just an undocumented hope.

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1. Script→module resolution debug smoke | [#8621](https://github.com/EffortlessMetrics/perl-lsp/issues/8621) | yes (`builder-ready`) | _pending_ | `cargo test -p perl-dap --test dap_module_resolution_smoke` |
| 2. Follow-up fixes (deferred) | TBD | n/a | _filed only if phase 1 surfaces bugs_ | inherits phase 1 receipt |

## Exit criteria

- [ ] All phases land or are explicitly deferred with a successor.
- [ ] Receipt command in this doc reproduces the closeout proof.
- [ ] Status doc updated (`docs/project/status/dap.md` regenerated post-merge).
- [ ] Claim boundary recorded.

## Claim boundary

This rail proves that **a DAP session attached to a Perl script that `use`s a workspace module can set and hit a breakpoint inside the module's `.pm` file**, with the smoke test recording the path-mapping handshake end-to-end.

This rail does **NOT** prove:

- Breakpoints work across remote debugging boundaries or non-local Perl interpreters.
- Conditional breakpoints, logpoints, or watchpoints inside module code. Those are out of scope; only plain breakpoint hit is covered.
- Anything about DAP attach/launch semantics beyond what the existing `dap_smoke_e2e.rs` template establishes — this rail extends the template, it does not re-prove it.

## Receipts

```bash
# Phase 1 closeout
cargo test -p perl-dap --test dap_module_resolution_smoke
```

The test file `crates/perl-dap/tests/dap_module_resolution_smoke.rs` is named in #8621's implementation contract. It should follow the `dap_smoke_e2e.rs` shape: build a tempdir with `script.pl` and `lib/Foo.pm`, set a breakpoint inside `Foo.pm`, run, assert the stop event lands at the expected `lib/Foo.pm` line.

## Related

- Umbrella issue: [#8621 — test(dap): add module-resolution debug smoke for script→lib path mapping](https://github.com/EffortlessMetrics/perl-lsp/issues/8621)
- Tracker for this rollout doc: #8629
- Architecture / spec docs: `crates/perl-dap/tests/dap_smoke_e2e.rs` (template); `crates/perl-dap/src/` (DAP runtime)
- Status doc: [docs/project/status/dap.md](../project/status/dap.md)
- Adjacent rails:
  - `MODULE_COMPLETION_RAIL.md` — same conceptual journey (script `use`s module) on the editor side; DAP is the runtime side
  - `IMPORTS_RAIL.md` — once literal `require` is tracked, a follow-on DAP smoke for that path may be filed

## Do not combine

Do **not** roll this rail's PRs into:

- Conditional / logpoint / watchpoint feature work. Those each need their own smoke and acceptance.
- DAP attach or launch protocol changes. The path-mapping smoke must run against the current protocol; protocol changes are separate concerns.
- The editor-side module-resolution rails. Editor-side and runtime-side concerns share a story but live in different crates with different acceptance gates.

## Lane assignment

**Builder (sonnet)** — implementation contract in #8621. The `dap_smoke_e2e.rs` template is the canonical reference; the builder should not invent a new test harness. Phase 2 is filed only if the smoke surfaces a bug; otherwise the rail closes at phase 1.
