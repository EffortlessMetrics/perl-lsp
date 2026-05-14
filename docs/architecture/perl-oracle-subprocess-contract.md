# `perl-oracle` Subprocess Contract Inventory

**Issue:** [#8620](https://github.com/EffortlessMetrics/perl-lsp/issues/8620)
**Umbrella:** [#8551](https://github.com/EffortlessMetrics/perl-lsp/issues/8551) — `PerlOracleEnv` wrapper + per-call-site migration
**Framing:** [`docs/architecture/perl-subprocess-seams.md`](perl-subprocess-seams.md)

---

## Purpose

This file is the **authoritative inventory** of Rust call sites where the LSP or its
tooling spawns the Perl interpreter or `perldoc` as an `ask-Perl` subprocess seam. It is
the Phase 1 deliverable of #8620; Phase 2 (#8551) introduces `PerlOracleEnv` and migrates
any call site that still uses a bare `Command::new("perl")`.

This inventory does not claim every external Perl ecosystem tool. Adapters such as
`perltidy`, `perlcritic`, direct `cpanm`, `prove`, and release shell scripts remain under
their formatter, critic, corpus, or release-tooling contracts unless they invoke the Perl
interpreter through a Rust call site covered here.

**Maintenance rule:** If you add or remove a Rust `ask-Perl` interpreter or `perldoc`
call site, update this table in the same PR. Any call site that appears in a `grep` for
`Command::new("perl")`, `Command::new("perldoc")`,
`tokio::process::Command::new("perl")`, or `subprocess::Exec::cmd("perl")` across
`crates/*/src/**.rs` and `xtask/src/**.rs` but is NOT in this table is a contract
violation.

---

## Column definitions

| Column | Meaning |
|---|---|
| **Call site** | `crate/path.rs:function_name` |
| **Spawn mechanism** | How the subprocess is built (bare `Command::new`, `PerlOracleEnv`, etc.) |
| **Current env** | What ambient inputs flow through today |
| **Desired contract** | Target post-#8551: explicit allow/deny per ambient variable |
| **Timeout** | Current bound; `none` = no explicit timeout |
| **Cache key** | Whether the result is cached; what the key tuple is |
| **Internalization path** | `Permanent` · `Bridge` · `Oracle` · `Candidate` (per seams doc) |

---

## Section 1 — Editor Runtime (LSP + DAP)

These call sites execute inside the editor session on behalf of the user. They have the
highest correctness and isolation requirements because they affect real-time editor
behaviour.

---

### 1.1 Startup `@INC` probe

| Field | Value |
|---|---|
| **Call site** | `perl-lsp-rs-core/src/config/mod.rs:fetch_perl_inc` → `perl-lsp-rs-core/src/config/perl_oracle_env.rs:PerlOracleEnv::for_module_resolution` |
| **Spawn mechanism** | `Command::new(&self.perl_binary)` via `PerlOracleEnv::into_command()` |
| **Current env** | Deny-all-ambient. `PERL5LIB` passes through only when `config.use_perl5lib=true`. `PERL5OPT` always stripped. `local::lib` variables always stripped. `PATH` preserved for binary resolution. |
| **Desired contract** | Already meets the post-#8551 contract. Explicit allow: `PERL5LIB` (user-gated), `PATH`. Explicit deny: `PERL5OPT`, `PERL_MM_OPT`, `PERL_MB_OPT`, `PERL_LOCAL_LIB_ROOT`, perlbrew/plenv vars. |
| **Timeout** | 1 000 ms (`SYSTEM_INC_PROBE_TIMEOUT` at `mod.rs:807`) |
| **Cache key** | `(config.use_perl5lib, config.use_system_inc, config.perl_path, workspace_folder)` — invalidated when `use_perl5lib` toggles (see `mod.rs:669`) |
| **Internalization path** | **Bridge** — replace with modeled config + workspace heuristics; tracked in #8551 roadmap |

**Perl invocation:** `perl <perl_args> -e 'print join("\n", @INC)'`

---

### 1.2 Module-resolution `@INC` probe

| Field | Value |
|---|---|
| **Call site** | `perl-lsp-rs-core/src/config/perl_oracle_env.rs:PerlOracleEnv::for_module_resolution` |
| **Spawn mechanism** | Delegates to `for_startup_inc_probe`; same `into_command()` path |
| **Current env** | Identical to §1.1 — deny-all-ambient; `PERL5LIB` user-gated |
| **Desired contract** | Same as §1.1 — already met |
| **Timeout** | 1 000 ms (`SYSTEM_INC_PROBE_TIMEOUT`) |
| **Cache key** | Same as §1.1 (shares the cache path in `WorkspaceConfig::get_system_inc`) |
| **Internalization path** | **Bridge** — collapse into §1.1 at the cache layer; no independent subprocess needed once startup probe shares its result |

**Note:** This constructor is a gated alias of `for_startup_inc_probe`. It returns `None`
when `config.use_system_inc` is `false`, so no subprocess is spawned in that case.

---

### 1.3 `perl.debugFile` LSP command (language probe)

| Field | Value |
|---|---|
| **Call site** | `perl-lsp-rs/src/runtime/language/misc.rs` (inline `perl -d` launch) |
| **Spawn mechanism** | `PerlOracleEnv::for_language_probe(config, debug_cwd).into_command()`; unresolved Perl returns an actionable error instead of falling back to bare `Command::new("perl")` |
| **Current env** | Via oracle: deny-all-ambient; `PERL5LIB` user-gated; `PERL5OPT` stripped; `local::lib` stripped. No ambient fallback. |
| **Desired contract** | Already meets the post-#8551 permanent-subprocess contract. |
| **Timeout** | 30 s (oracle default for `for_language_probe`) |
| **Cache key** | None — on-demand per user action |
| **Internalization path** | **Permanent** — launching the user's Perl debugger is inherently external |

**Perl invocation:** `perl -d -- <file> <args>`

**Failure mode:** If no Perl binary can be resolved from config or `PATH`, the handler
returns an LSP error and refuses ambient fallback.

---

### 1.4 `perl.runFile` / `perl.runTestSub` (execute command)

| Field | Value |
|---|---|
| **Call site** | `perl-lsp-rs/src/execute_command/provider.rs:perl_command_for` |
| **Spawn mechanism** | `PerlOracleEnv::for_execute_command(config, cwd).into_command()` with fallback to bare `Command::new("perl")` when config is absent or binary resolution fails |
| **Current env** | Via oracle: deny-all-ambient base; `PERL5LIB` user-gated; `PERL5OPT` **allowed** (user scripts may use `-M` pragmas); `local::lib` **allowed** (user scripts may depend on it). Fallback path: inherits ALL ambient. |
| **Desired contract** | Same oracle path is correct. Fallback bare `Command::new("perl")` at `provider.rs:104,109` should surface a user-visible error instead of silently inheriting ambient. |
| **Timeout** | 30 s |
| **Cache key** | None — on-demand per user command invocation |
| **Internalization path** | **Permanent** — user-initiated script execution requires the user's Perl |

**Note:** Unlike probes, `for_execute_command` intentionally allows `PERL5OPT` and
`local::lib` because user scripts may legitimately rely on them. This is the correct
contract for the execute-command seam.

---

### 1.5 `perldoc` hover documentation

| Field | Value |
|---|---|
| **Call site** | `perl-lsp-rs/src/runtime/language/virtual_content.rs:fetch_perldoc` |
| **Spawn mechanism** | `PerlOracleEnv::for_perldoc(config, cwd).into_command()` |
| **Current env** | Deny-all-ambient base. `PERL5LIB` passes through only when `config.use_perl5lib=true`. `PERL5OPT` and `local::lib` variables are stripped. `PATH` is preserved for binary/helper resolution. `LC_ALL=C` is forced for stable text output. |
| **Desired contract** | Already meets the post-#8551 bridge contract. `perldoc` resolves from the configured Perl toolchain directory when possible, then falls back to `perldoc` on `PATH`. Long-term: replace with an in-process POD reader. |
| **Timeout** | 10 s (via `run_command_with_timeout` at call site) |
| **Cache key** | None currently — per-request; caching by module name would reduce subprocess churn |
| **Internalization path** | **Bridge** — replace with embedded documentation tables or bundled POD extractor that does not need a subprocess |

**Perl invocation:** `perldoc -T -- <module>` with the perldoc binary resolved from the
configured Perl toolchain when possible.

---

### 1.6 DAP pre-launch syntax check (`perl -c`)

| Field | Value |
|---|---|
| **Call site** | `perl-dap/src/debug_adapter/process.rs:check_syntax` |
| **Spawn mechanism** | `PerlOracleEnv::for_version_probe(perl_binary, cwd).into_command()` with `extra_env` from launch.json |
| **Current env** | Deny-all-ambient base; `PERL5LIB` denied; `PERL5OPT` denied; `local::lib` denied; explicit launch.json `env` entries honored via `extra_env` |
| **Desired contract** | Already meets the contract. `env_overrides` from launch.json are passed via `extra_env`, so the debug session's configured environment is available to the syntax check. |
| **Timeout** | 5 s (oracle `for_version_probe` default) |
| **Cache key** | None — invoked once per launch |
| **Internalization path** | **Bridge** — replace with parser's own syntax validation when it reaches parity; tracked as follow-up to #8551 |

**Perl invocation:** `perl -c -- <program>` (stderr contains `"<program> syntax OK"` or
error detail)

---

### 1.7 DAP debug session launcher (`perl -d`)

| Field | Value |
|---|---|
| **Call site** | `perl-dap/src/debug_adapter/process.rs:launch_perl` (after `check_syntax`) |
| **Spawn mechanism** | `PerlOracleEnv::for_version_probe(perl_binary, cwd).into_command()` with `extra_env` from launch.json, then `.arg("-d")` |
| **Current env** | Same as §1.6 — deny-all-ambient; explicit launch.json env honored via `extra_env` |
| **Desired contract** | Already meets the contract. Debug sessions use an explicit Perl binary from the launch configuration rather than ambient `PATH`. |
| **Timeout** | 30 s (startup budget; process is kept alive after spawn) |
| **Cache key** | None — one process per debug session |
| **Internalization path** | **Permanent** — DAP requires the real Perl debugger |

**Perl invocation:** `perl -d -- <program> <args>` (stdin/stdout piped; stderr piped)

---

### 1.8 DAP bridge (`perl-lsp` → `Perl::LanguageServer`)

| Field | Value |
|---|---|
| **Call site** | `perl-dap/src/bridge_adapter.rs:build_pls_dap_command` |
| **Spawn mechanism** | `PerlOracleEnv::for_dap_bridge(perl_path, cwd, perl5lib_passthrough, perl5opt_passthrough).into_command()` |
| **Current env** | Deny-all-ambient base; `PERL5LIB` conditionally allowed (debug config `perl5lib_passthrough`); `PERL5OPT` conditionally allowed (debug config `perl5opt_passthrough`); `local::lib` always denied; `PATH` preserved |
| **Desired contract** | Already meets the contract. The two passthrough flags are explicit per-session decisions rather than silent ambient inheritance. |
| **Timeout** | 30 s (startup budget; bridge process is long-running after successful handshake) |
| **Cache key** | None — one process per debug adapter session |
| **Internalization path** | **Permanent** — bridges the legacy `Perl::LanguageServer` DAP implementation for users who depend on it |

**Perl invocation:** `perl <PLS_path>` (PLS = Perl::LanguageServer)

---

### 1.9 DAP test fixture availability probe

| Field | Value |
|---|---|
| **Call site** | `perl-lsp-rs-core/src/config/perl_oracle_env.rs:PerlOracleEnv::for_dap_test_fixture` |
| **Spawn mechanism** | `Command::new("perl")` with explicit `env_remove("PERL5LIB")`, `env_remove("PERL5OPT")`, `env_remove("PERL_LOCAL_LIB_ROOT")`, `env_remove("PERL_LOCAL_LIB_PREFIX")`; does NOT use `env_clear()` so `PATH` is available for binary resolution |
| **Current env** | Partial strip: Perl-specific variables removed; `PATH`, `HOME`, and other ambient vars inherited for the availability check only. Subsequent invocations via `into_command()` apply the full deny-all-ambient policy. |
| **Desired contract** | Already correct. The availability probe is intentionally minimal — it must resolve `perl` via `PATH`, and Perl-specific variables are explicitly stripped to prevent skip/pass-rate contamination in test suites that use this constructor. |
| **Timeout** | 5 s (for subsequent `into_command()` invocations; availability check has no explicit timeout) |
| **Cache key** | None — checked once per test run at fixture setup |
| **Internalization path** | **Oracle** — test fixture gate; enables `#[test]` functions to skip gracefully when Perl is unavailable |

**Note:** This is the only `Command::new("perl")` call in `perl_oracle_env.rs` that is not
wrapped in the full `into_command()` / `configure_command()` isolation flow. It serves
solely as an availability probe; the returned `PerlOracleEnv` instance uses
`into_command()` for all actual subprocess invocations.

---

## Section 2 — TDD test support (editor-adjacent)

These call sites run inside `perl-tdd-support`, which drives Perl `.t` test files on
behalf of the LSP Test Explorer. They execute in the same process space as the LSP but
act on behalf of specific workspace test runs.

---

### 2.1 TDD test runner hermetic Perl invocation

| Field | Value |
|---|---|
| **Call site** | `perl-tdd-support/src/tdd/test_runner.rs:hermetic_perl_command` → `run_perl_test` |
| **Spawn mechanism** | `Command::new(perl_binary)` with `cmd.env_clear()` and `PATH` re-injected; explicit `-Ilib` added by caller |
| **Current env** | Hermetic: `env_clear()` removes ALL inherited env; only `PATH` (and `SYSTEMROOT` on Windows) are explicitly re-added; `PERL5LIB`, `PERL5OPT`, `HOME`, `local::lib` all absent |
| **Desired contract** | Already correct — TDD fixtures require hermetic env to prevent ambient contamination. `extra_env` may be added by callers for explicit `-I` paths. |
| **Timeout** | Per-test timeout (caller-controlled via the TAP harness) |
| **Cache key** | None — each test run is fresh |
| **Internalization path** | **Oracle** — TDD fixtures must invoke real Perl to produce TAP output |

**Perl invocation:** `perl -Ilib -- <test_file>` (hermetic env)

---

## Section 3 — xtask build and CI tooling

These call sites run inside `cargo xtask` tasks (corpus sweeps, CI health checks,
compiler oracle, CPAN install). They execute as developer tooling, never in an editor
runtime. Ambient env leakage here affects CI reproducibility but not end-user sessions.

---

### 3.1 Parser corpus sweep — version probe

| Field | Value |
|---|---|
| **Call site** | `xtask/src/tasks/parser_corpus_sweep.rs:get_perl_version` |
| **Spawn mechanism** | Bare `std::process::Command::new("perl")` |
| **Current env** | Inherits ALL ambient env |
| **Desired contract** | Strip `PERL5OPT`; set `LC_ALL=C`; no change to `PERL5LIB` (irrelevant to `$]` print). Not required to use `PerlOracleEnv` (xtask-only; not editor-runtime). |
| **Timeout** | None |
| **Cache key** | None — called once per corpus sweep |
| **Internalization path** | **Oracle** — CI tooling; never editor-runtime |

**Perl invocation:** `perl -e 'print $]'`

---

### 3.2 Parser corpus sweep — module resolution fallback

| Field | Value |
|---|---|
| **Call site** | `xtask/src/tasks/parser_corpus_sweep.rs` (module resolution block, ~line 467) |
| **Spawn mechanism** | Bare `std::process::Command::new("perl")` with explicit `PERL5LIB` set from caller-supplied paths |
| **Current env** | Explicit `PERL5LIB` from caller; other env variables inherited from xtask process |
| **Desired contract** | Caller already sets `PERL5LIB` explicitly. Add `env_clear()` + explicit `PATH` + `PERL5LIB` for full hermeticity, but this is a low-priority xtask-only call site. |
| **Timeout** | None |
| **Cache key** | Module list (reused when the same unresolved module set is queried) |
| **Internalization path** | **Oracle** — corpus tooling only |

**Perl invocation:** `perl -e 'for (qw(...)) { eval "require $_"; ...; print "$f=$INC{$f}\n" if $INC{$f} }'`

---

### 3.3 LSP test runner — Perl `.t` file execution

| Field | Value |
|---|---|
| **Call site** | `xtask/src/tasks/test_lsp.rs:test_test_runner` |
| **Spawn mechanism** | Bare `Command::new("perl")` running a `.t` file |
| **Current env** | Inherits ALL ambient env |
| **Desired contract** | No change required — this is a developer tool that intentionally uses the developer's Perl environment. Not editor-runtime. |
| **Timeout** | None |
| **Cache key** | None |
| **Internalization path** | **Oracle** — developer tooling |

**Perl invocation:** `perl <test_suite.t>`

---

### 3.4 CI doctor — Perl availability check

| Field | Value |
|---|---|
| **Call site** | `xtask/src/tasks/ci_doctor.rs:check_perl` |
| **Spawn mechanism** | Bare `Command::new("perl")` |
| **Current env** | Inherits ALL ambient env |
| **Desired contract** | No change — this is an availability probe whose purpose is to confirm that `perl` resolves on `PATH`. Ambient env is intentional. |
| **Timeout** | None |
| **Cache key** | None |
| **Internalization path** | **Oracle** — CI health check |

**Perl invocation:** `perl -v`

---

### 3.5 CPAN corpus — bootstrapped `cpanm` via Perl

| Field | Value |
|---|---|
| **Call site** | `xtask/src/tasks/cpan_corpus.rs:CpanmLauncher::command` (the `Bootstrapped` arm) |
| **Spawn mechanism** | `Command::new("perl")` running a downloaded cpanm script |
| **Current env** | Inherits ALL ambient env (CPAN tools depend on HOME, PERL_MM_OPT, etc.) |
| **Desired contract** | No change — cpanm is a full Perl module installer that legitimately uses ambient env to locate the user's CPAN config, HOME, and local directories. Stripping would break the installer. |
| **Timeout** | Per-module timeout (controlled by corpus harness) |
| **Cache key** | Distribution name / install directory |
| **Internalization path** | **Bridge** — longer-term path is a pre-built binary cpanm or a native Rust alternative; tracked separately from #8551 |

**Perl invocation:** `perl <cpanm_path> <module_or_dist>` (full ambient env)

---

### 3.6 Compiler oracle — parse-effect probe

| Field | Value |
|---|---|
| **Call site** | `xtask/src/tasks/compiler_oracle.rs:run_perl_oracle` |
| **Spawn mechanism** | `Command::new("perl")` with explicit `env_remove("PERL5OPT")`, `env_remove("PERL5LIB")`, and `env("LC_ALL", "C")` |
| **Current env** | Partial strip: `PERL5OPT` and `PERL5LIB` removed; `LC_ALL` forced to `C`; remaining ambient variables (HOME, PATH, etc.) inherited |
| **Desired contract** | Partial strip is intentional — oracle requires a predictable locale and no runtime module injection. Remaining ambient vars are acceptable for a corpus tool. |
| **Timeout** | None |
| **Cache key** | Source file content hash (oracle is deterministic for the same source) |
| **Internalization path** | **Oracle** — differential test evidence; never editor-runtime |

**Perl invocation:** `perl -I<tempdir> -e <ORACLE_PROBE> <fixture.pm>` (PERL5OPT/PERL5LIB removed, LC_ALL=C)

---

### 3.6a CPAN corpus — `run_command_with_timeout` unit tests

| Field | Value |
|---|---|
| **Call site** | `xtask/src/tasks/cpan_corpus.rs` (`#[test]` block, ~lines 991 and 1003) |
| **Spawn mechanism** | Bare `Command::new("perl")` used inside `#[cfg(test)]` functions that test the `run_command_with_timeout` utility |
| **Current env** | Inherits ALL ambient env |
| **Desired contract** | Test-only; no production change needed. These tests validate the subprocess timeout mechanism, not Perl-specific behavior. The call sites are inside `#[test]` functions and are not editor-runtime. |
| **Timeout** | Explicitly set per test: 2 s and 200 ms respectively |
| **Cache key** | N/A — test-only |
| **Internalization path** | **Oracle** — unit test harness |

**Note:** These two call sites appear in xtask source but are `#[test]`-annotated
functions (`test_run_command_with_timeout_captures_stderr` and
`test_run_command_with_timeout_kills_hung_process`). They exercise the timeout
infrastructure, not a production subprocess seam.

---

### 3.7 Compiler oracle — Perl version query

| Field | Value |
|---|---|
| **Call site** | `xtask/src/tasks/compiler_oracle.rs:query_perl_version` |
| **Spawn mechanism** | `Command::new("perl")` with explicit `env_remove("PERL5OPT")`, `env_remove("PERL5LIB")`, and `env("LC_ALL", "C")` |
| **Current env** | Same as §3.6 |
| **Desired contract** | Same as §3.6 |
| **Timeout** | None |
| **Cache key** | None — called once per oracle run |
| **Internalization path** | **Oracle** — CI tooling |

**Perl invocation:** `perl -e 'print $^V'` (PERL5OPT/PERL5LIB removed, LC_ALL=C)

---

## Section 4 — Summary table

Quick-reference. Sorted by internalization path priority (gaps first).

| # | Call site | Mechanism | PERL5LIB | PERL5OPT | local::lib | Timeout | Internalization |
|---|---|---|---|---|---|---|---|
| 1.4 | `provider.rs` (fallback path) | Bare `Command::new("perl")` | ⚠ ambient | ⚠ ambient | ⚠ ambient | 30 s | Permanent — **GAP** |
| 1.3 | `misc.rs:perl.debugFile` | `PerlOracleEnv::for_language_probe` | user-gated | ✓ denied | ✓ denied | 30 s | Permanent |
| 1.5 | `virtual_content.rs:fetch_perldoc` | `PerlOracleEnv::for_perldoc` | user-gated | ✓ denied | ✓ denied | 10 s | Bridge |
| 1.1 | `mod.rs:fetch_perl_inc` | `PerlOracleEnv::for_module_resolution` | user-gated | ✓ denied | ✓ denied | 1 000 ms | Bridge |
| 1.2 | `perl_oracle_env.rs:for_module_resolution` | `PerlOracleEnv` alias of §1.1 | user-gated | ✓ denied | ✓ denied | 1 000 ms | Bridge |
| 1.6 | `process.rs:check_syntax` | `PerlOracleEnv::for_version_probe` | ✓ denied | ✓ denied | ✓ denied | 5 s | Bridge |
| 1.7 | `process.rs:launch_perl` | `PerlOracleEnv::for_version_probe` | ✓ denied | ✓ denied | ✓ denied | 30 s | Permanent |
| 1.8 | `bridge_adapter.rs:build_pls_dap_command` | `PerlOracleEnv::for_dap_bridge` | configurable | configurable | ✓ denied | 30 s | Permanent |
| 1.4 | `provider.rs` (oracle path) | `PerlOracleEnv::for_execute_command` | user-gated | ✓ allowed | ✓ allowed | 30 s | Permanent |
| 1.9 | `perl_oracle_env.rs:for_dap_test_fixture` | `Command::new("perl")` (partial strip) | ✓ removed | ✓ removed | ✓ removed | 5 s (subsequent) | Oracle |
| 2.1 | `test_runner.rs:hermetic_perl_command` | `Command::new` + `env_clear()` | ✓ denied | ✓ denied | ✓ denied | per-test | Oracle |
| 3.1 | `parser_corpus_sweep.rs:get_perl_version` | Bare `Command::new("perl")` | ⚠ ambient | ⚠ ambient | ⚠ ambient | none | Oracle |
| 3.2 | `parser_corpus_sweep.rs` (module res.) | Bare `Command::new("perl")` + explicit PERL5LIB | explicit | ⚠ ambient | ⚠ ambient | none | Oracle |
| 3.3 | `test_lsp.rs:test_test_runner` | Bare `Command::new("perl")` | ⚠ ambient | ⚠ ambient | ⚠ ambient | none | Oracle |
| 3.4 | `ci_doctor.rs:check_perl` | Bare `Command::new("perl")` | ⚠ ambient | ⚠ ambient | ⚠ ambient | none | Oracle |
| 3.5 | `cpan_corpus.rs:CpanmLauncher` | Bare `Command::new("perl")` | ⚠ ambient | ⚠ ambient | ⚠ ambient | per-module | Bridge |
| 3.6a | `cpan_corpus.rs` (test-only) | Bare `Command::new("perl")` | ⚠ ambient | ⚠ ambient | ⚠ ambient | 2 s / 200 ms | Oracle |
| 3.6 | `compiler_oracle.rs:run_perl_oracle` | Bare `Command::new("perl")` (partial strip) | ✓ removed | ✓ removed | ⚠ ambient | none | Oracle |
| 3.7 | `compiler_oracle.rs:query_perl_version` | Bare `Command::new("perl")` (partial strip) | ✓ removed | ✓ removed | ⚠ ambient | none | Oracle |

Legend: ✓ denied/removed = no leak · user-gated = follows `usePerl5lib` config · ⚠ ambient = inherits from LSP/xtask process

---

## Section 5 — Open gaps (action items for Phase 2 / #8551)

The `perldoc` hover seam (§1.5) is wrapped by `PerlOracleEnv::for_perldoc`, and
`perl.debugFile` (§1.3) now refuses unresolved Perl instead of falling back to ambient
`perl`. The remaining editor-runtime gap is the silent execute-command fallback below.

### Gap A: `provider.rs` fallback (§1.4) — medium priority

`ExecuteCommandProvider::perl_command_for` falls back to bare `Command::new("perl")` when
no config is available. Remove fallback; surface error.

---

## Section 6 — How to verify completeness

Run the following grep from the workspace root. Every matched production call site
should already be represented in this document:

```bash
# All Command::new("perl") in crates/
grep -rn 'Command::new("perl")\|Command::new.*"perl"\b' crates/*/src/ --include='*.rs' \
  | grep -v 'target/' | grep -v '.spec/'

# All Command::new("perldoc") in crates/
grep -rn 'Command::new("perldoc")\|Command::new.*"perldoc"\b' crates/*/src/ --include='*.rs' \
  | grep -v 'target/' | grep -v '.spec/'

# All Command::new("perl") in xtask/
grep -rn 'Command::new("perl")\|Command::new.*"perl"\b' xtask/src/ --include='*.rs'

# Any tokio::process::Command::new("perl")
grep -rn 'tokio::process::Command::new.*"perl"' crates/ xtask/ --include='*.rs' \
  | grep -v 'target/'

# Any subprocess::Exec::cmd("perl")
grep -rn 'subprocess::Exec::cmd.*"perl"' crates/ xtask/ --include='*.rs' \
  | grep -v 'target/'
```

If a call site appears that is not in the table above, open a PR that adds it.

---

## Section 7 — Related

- **#8551** — `PerlOracleEnv` struct implementation and per-call-site migration (Phase 2)
- **#8555** — tracking issue for `perl-subprocess-seams.md`
- **#8620** — this document (Phase 1)
- **#8493** — the `PERL5LIB` env-leak incident that surfaced this work
- **`docs/architecture/perl-subprocess-seams.md`** — framing and policy for subprocess seams
- **`crates/perl-lsp-rs-core/src/config/perl_oracle_env.rs`** — `PerlOracleEnv` implementation
