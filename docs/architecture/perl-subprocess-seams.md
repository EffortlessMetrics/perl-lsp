# Perl subprocess seams

## Principle

External Perl subprocesses are compatibility / oracle / bootstrapping
**seams**. Every such seam needs an explicit ambient-input contract AND a
stated path to internalization where practical.

Long-term direction: `perl-lsp` should internalize as much Perl handling
as practical. Each subprocess seam is a place where the LSP's static
analysis can be undermined by ambient state — `PERL5LIB`, `cwd`, `HOME`,
the shell's perlbrew shims, a `local::lib` setup, locale settings, or a
sitecustomize hook.

Tracking: **#8555**. Implementation companion: **#8551** (`PerlOracleEnv`
struct + per-call-site migration).

---

## Why this is a seam, not a detail

A subprocess that asks Perl for facts is **not** an implementation detail
of the function that spawns it. It is a boundary between the LSP's static
world (config + parsed source) and the runtime world (whatever Perl says
when invoked). The 2026-05-11 #8493 incident is the canonical case: the
LSP config said `usePerl5lib=false`, the completion and resolver paths
honored it, but the **interpreter startup `@INC` probe inherited
`PERL5LIB` from the LSP process's own environment** and leaked it back
into `@INC`. The seam was invisible; the contract was implicit; the
ambient leak was silent.

Every "ask Perl" seam has the same shape:

```
LSP process env  ─┬→  perl  ─→  stdout  ─→  LSP's claim about Perl
                  └→  (also: cwd, HOME, PATH, perlbrew shims, ...)
```

If the seam does not declare which ambient inputs are allowed, the
subprocess silently uses whatever the LSP process inherited from VS Code
/ the shell / the OS, and the user's config is effectively ignored.

---

## Ambient inputs to enumerate per seam

Any of the following can affect a Perl subprocess's behavior and MUST be
explicitly allowed or denied:

### Perl-specific environment

- `PERL5LIB` — additional `@INC` paths
- `PERL5OPT` — interpreter command-line options
- `PERL_MM_OPT`, `PERL_MB_OPT` — module-build configuration
- `PERL_LOCAL_LIB_ROOT`, `PERL_LOCAL_LIB_PREFIX` — `local::lib` activation
- `PERL_NO_SITECUSTOMIZE` (inverse) — sitecustomize.pl behavior

### Perl-version-manager shims

- `PERLBREW_ROOT`, `PERLBREW_HOME`, `PERLBREW_PATH` — perlbrew
- `PLENV_ROOT`, `PLENV_SHIMS` — plenv
- `ASDF_DATA_DIR`, `ASDF_DIR` — asdf

### Process environment

- `PATH` — which `perl` binary gets resolved (use an explicit binary path
  instead of `$PATH` lookup wherever possible)
- `HOME` — controls `~/.perldb`, `~/.cpan`, `~/.cpanm`, sitecustomize
  search
- `cwd` — controls `.` in `@INC`, relative path resolution

### Locale and platform

- `LANG`, `LC_*` — affects regex behavior, sort order, string encoding
- Filesystem case sensitivity — affects module resolution on macOS/Windows
- Shell behavior — affects how arguments are quoted on Windows

---

## Per-seam declaration shape

Each seam in the workspace MUST declare:

| Field | Meaning |
|---|---|
| **Purpose** | What it asks Perl for (one sentence) |
| **Allowed ambient inputs** | What flows through, with rationale |
| **Stripped ambient inputs** | What's denied (default: everything not allowed) |
| **Timeout** | Bounded subprocess execution (no unbounded waits) |
| **Cache key** | What makes a result re-usable |
| **Invalidation trigger** | What causes a re-probe |
| **Fallback behavior** | What happens when the subprocess fails / times out / produces malformed output |
| **User-visible warning** | What surfaces in diagnostics / logs |
| **Internalization path** | One of: `Permanent`, `Bridge`, `Oracle`, `Candidate` |

### Internalization-path values

- **`Permanent`** — runtime integration that should stay external. Rare.
  Example: invoking the user-configured `perl` binary to validate that it
  exists and runs.
- **`Bridge`** — temporary compatibility, planned for replacement when
  the equivalent in-process analysis is built. Example: the startup `@INC`
  probe — eventually replaceable by modeled config + workspace heuristics.
- **`Oracle`** — differential test evidence only, never editor-runtime.
  Example: the real-Perl oracle used by parser conformance tests.
- **`Candidate`** — eligible for in-process replacement now. The
  internalization work is sized; the seam is a known wart, not a permanent
  fixture.

---

## Initial seam table (2026-05-11)

This table reflects the best-known seam set at the time of writing.
Verify against the workspace before relying on it — new seams may have
been added.

| Seam | Purpose | Ambient inputs | Timeout | Cache | Internalization |
|---|---|---|---|---|---|
| **Startup `@INC` probe** (`fetch_perl_inc` in `perl-lsp-rs-core/src/config/mod.rs`) | Discover interpreter startup `@INC` paths | `PATH`, selected Perl binary; `PERL5LIB` only when `usePerl5lib=true` | 1s | per (config, folder) | **Bridge** — replace with modeled config + workspace heuristics |
| **Real-Perl oracle** (parser conformance tests) | Differential test evidence: what does real Perl say about this source? | Controlled fixture env (PERL5LIB unset, HOME=test temp, locale=C) | bounded | test-local | **Oracle** — never editor-runtime |
| **perlcritic / perltidy** (if invoked) | External tool integration | Explicit tool path, config path | bounded | per (tool, config) | **Bridge** — native critic / formatter lanes are the long-term plan |

---

## Why declare internalization paths?

A seam without an internalization classification tends to grow new
features. Each new "ask Perl" call site that doesn't declare itself a
`Candidate` or `Bridge` becomes implicit `Permanent`, and the LSP's
externalization surface area expands silently.

Declaring the path:

- Makes the LSP's runtime dependency on external Perl visible.
- Gates new seams behind an explicit "is this `Candidate` work or
  `Permanent`?" question.
- Lets the roadmap track the trend (are we adding or removing seams over
  time?).
- Surfaces `Bridge` seams as backlog items, not implementation choices.

---

## How to add a new seam

When implementing a new "ask Perl" subprocess:

1. **Decide the internalization classification first.** If you can't
   justify `Permanent` / `Bridge` / `Oracle` / `Candidate`, the seam
   probably shouldn't exist — find an in-process alternative.
2. **Write the per-seam declaration** in `docs/architecture/perl-subprocess-seams.md`
   (this doc) before writing the code.
3. **Construct an explicit `PerlOracleEnv`** (see **#8551**) instead of a
   bare `Command::new("perl")`. Default policy: deny-all-ambient.
4. **Write a poisoned-env test** that asserts the subprocess output does
   NOT reflect a poisoned ambient input the contract denies.
5. **Document the cache key and invalidation trigger.** Subprocess
   results that get cached without an invalidation trigger silently drift
   from reality.

---

## Related

- **#8555** — tracking issue (this doc).
- **#8551** — `PerlOracleEnv` struct implementation + per-call-site migration.
- **#8493** — the `PERL5LIB` env-leak incident that surfaced the cross-cutting rule.
- **#8525**, **#8537** — downstream @INC strictness work that benefited from explicit env handling.
- **`docs/devex/freshness-check.md`** + **#8546** — adjacent silent-failure-mode story (different domain; same lesson about implicit ambient state). The `docs/devex/freshness-check.md` spec is the merged-doctrine version of the original orchestrator memory.

## Claim boundary

Architecture and policy. No code change in this doc; the implementation
of `PerlOracleEnv` and the per-call-site migration lives in **#8551**. This
doc is the framing; #8551 is the proof.
