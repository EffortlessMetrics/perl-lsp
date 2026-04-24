# Zero-Panic: Reliability and Security in a Language Server

A language server is the most intimate piece of infrastructure a developer touches.
It runs inside your editor, on every keystroke, with access to your filesystem.
If it panics, the process dies. Completions vanish. Diagnostics disappear.
Goto-definition stops working. The developer doesn't get an error dialog --
they get silence. And silence from a tool you depend on is worse than a crash
you can see.

This is the story of how perl-lsp enforces zero panics in production code,
and the defense-in-depth security architecture that surrounds it.

---

## The Policy

Seven constructs are banned from production code:

- `unwrap()`
- `expect()`
- `panic!()`
- `todo!()`
- `unimplemented!()`
- `std::process::abort()`
- `dbg!()`

Additionally, `std::process::exit()` is restricted to `bin/` entry points and
`lifecycle.rs` (the LSP shutdown path).

These are not guidelines. They are `deny`-level Clippy lints, enforced at
compile time across the entire workspace:

```toml
# Cargo.toml — workspace lints
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
```

Every crate in the workspace inherits these lints. A single `unwrap()` in any
library crate is a compile error, not a warning. There is no "fix it later."

## Why It Matters for a Language Server

Most programs crash visibly. A web server returns a 500. A CLI prints a stack
trace. The user sees what happened and can react.

A language server crash is different. The editor spawns the LSP process in the
background. When that process panics, the user's experience degrades silently:

- **Completions stop appearing** -- the user thinks the project is misconfigured.
- **Diagnostics go stale** -- errors from three edits ago persist on screen.
- **Navigation breaks** -- goto-definition returns nothing, so the user opens
  `grep` instead.
- **Formatting fails** -- the user assumes perltidy is broken.

The editor may restart the server, but restart is not instant. There's a gap
where the developer has no language intelligence at all. If the panic was
triggered by a specific file, the server crashes again on restart, entering
a crash loop.

A language server must handle every input gracefully. Malformed Perl, corrupt
URIs, unexpected protocol messages, filesystem race conditions -- all of these
must produce a `Result::Err`, never a panic.

## Enforcement Layers

The zero-panic policy is enforced at four layers, each catching what the
previous one misses.

### Layer 1: Clippy Lints (Compile Time)

The workspace-level `deny` lints make banned constructs a hard compile error.
`cargo clippy --workspace` catches every violation before a single test runs.

Tests are exempted via `cfg_attr`:

```rust
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
```

This is deliberate. Test code uses `unwrap()` and `panic!()` because test
failures should be loud and immediate. Production code uses `?` and `Result`
because production failures must be recoverable.

### Layer 2: CI Gates (Pre-Merge)

The CI pipeline runs `cargo clippy --workspace` as part of every PR check.
Even if a developer's local toolchain is misconfigured, the CI gate blocks
the merge. The relevant gate tiers:

| Tier | Command | When |
|------|---------|------|
| A (PR-fast) | `just pr-fast` | Every PR push |
| B (Merge gate) | `just ci-gate` | Before merge |

Both tiers include Clippy. A PR cannot merge with a banned construct.

### Layer 3: Code Review

Every PR is reviewed before merge. Reviewers check for banned constructs,
but more importantly, they check for *semantic* violations that Clippy
cannot catch -- like error handling that swallows errors silently, or
`match` arms that return meaningless defaults instead of propagating failures.

### Layer 4: CLAUDE.md as AI-Agent Enforcement

perl-lsp is developed using AI agents in a swarm architecture. Every agent
reads `CLAUDE.md` at startup, which contains the banned-construct policy.
This means the policy is enforced not just by human reviewers, but by
every AI agent that writes code for the project. The agents run
`cargo clippy` as part of their verification step before creating PRs.

## The Error Handling Pattern

With `unwrap()` and `expect()` banned, every fallible operation must be
handled explicitly. The project uses a consistent pattern:

**The `?` operator** is the primary tool. Functions return `Result<T, E>` or
`Option<T>`, and `?` propagates errors to the caller:

```rust
pub fn validate_workspace_path(
    path: &Path,
    workspace_root: &Path,
) -> Result<PathBuf, WorkspacePathError> {
    let workspace_canonical = workspace_root.canonicalize().map_err(|error| {
        WorkspacePathError::PathOutsideWorkspace(format!(
            "Workspace root not accessible: {} ({error})",
            workspace_root.display()
        ))
    })?;
    // ...
    Ok(final_path)
}
```

**`ok_or_else()`** converts `Option<T>` to `Result<T, E>` when absence is
an error:

```rust
let args: EvaluateArguments = match arguments.and_then(|v| serde_json::from_value(v).ok()) {
    Some(a) => a,
    None => {
        return DapMessage::Response {
            seq,
            request_seq,
            success: false,
            // ...
            message: Some("Missing arguments".to_string()),
        };
    }
};
```

**Graceful regex degradation** handles regex compilation failure without
panicking. Regexes are stored as `Option<Regex>` via `.ok()`:

```rust
pub static DANGEROUS_OPS_RE: Lazy<Result<Regex, regex::Error>> = Lazy::new(|| {
    let pattern = format!(r"\b(?:{})\b", DANGEROUS_OPERATIONS.join("|"));
    Regex::new(&pattern)
});

// Usage: if the regex failed to compile, skip the check rather than crash
let Some(re) = DANGEROUS_OPS_RE.as_ref().ok() else {
    return Ok(());
};
```

In tests, the `perl_tdd_support` crate provides `must()` and `must_some()`
helpers that panic with clear messages -- because test panics are the
correct behavior.

## The One Exception

The CLAUDE.md policy notes a single exemption:

> Exception: `#[allow(clippy::expect_used)]` in `crates/perl-lsp-rs/src/util/uri.rs`

This file originally contained URI parsing logic where an `expect()` was
deemed justified -- a case where the invariant was guaranteed by the type
system and a panic would indicate a logic bug in the URI library itself,
not user input. The code has since been extracted to the `perl-lsp-uri`
microcrate, and the file is now a thin re-export:

```rust
pub use perl_lsp_uri::parse_uri;
```

The exception remains documented as a matter of principle. It reminds
contributors that absolutism is the default, but pragmatism is allowed --
provided you document the justification and limit the scope. The fact that
the underlying code was refactored to no longer need the exception is a
testament to the incremental improvement process.

## Supply Chain Security

The `deny.toml` configuration enforces supply chain policies via
[cargo-deny](https://github.com/EmbarkStudios/cargo-deny):

**Source restrictions** -- all dependencies must come from crates.io. Unknown
registries and git sources are denied outright:

```toml
[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

**License allowlist** -- only OSI-approved permissive licenses are accepted:
MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, CC0-1.0, Zlib.
No GPL, no AGPL, no SSPL.

**Advisory monitoring** -- known vulnerabilities are checked against the
RustSec advisory database. The single advisory ignore is documented with
risk assessment:

```toml
[advisories]
ignore = [
    { id = "RUSTSEC-2023-0089",
      reason = "atomic-polyfill unmaintained (postcard -> heapless -> atomic-polyfill).
               No upgrade path. Risk: low (unmaintained, not vulnerable)." },
]
```

**Duplicate detection** -- multiple versions of the same crate are warned,
with known version splits documented and tracked for upstream convergence.

## Path Traversal Prevention

The `perl-path-security` crate provides workspace-bound path validation.
Every file path that enters the LSP or DAP from user input passes through
this layer.

The defense has three checks:

**1. Character validation** -- null bytes and control characters are rejected
immediately. This prevents protocol confusion attacks where a null byte
truncates a path in C-level filesystem calls:

```rust
if let Some(path_str) = path.to_str()
    && (path_str.contains('\0') || path_str.chars().any(|c| c.is_control() && c != '\t'))
{
    return Err(WorkspacePathError::InvalidPathCharacters);
}
```

**2. Canonical resolution** -- existing paths are resolved via `canonicalize()`,
which follows symlinks and resolves `.` and `..` components. The canonical
path is then checked against the workspace root:

```rust
let canonical = resolved.canonicalize()?;
if !canonical.starts_with(&workspace_canonical) {
    return Err(WorkspacePathError::PathOutsideWorkspace(/* ... */));
}
```

**3. Non-existent path normalization** -- for paths that don't exist yet (e.g.,
completion targets), the `perl-path-normalize` crate processes path
components manually, tracking depth to prevent `..` from escaping above
the workspace root.

The final path is checked against the workspace boundary a second time after
all resolution, providing belt-and-suspenders defense.

Additional utility functions handle completion-path sanitization, blocking
absolute paths, drive prefixes, and `..` traversal in completion input.

## DAP Expression Safety

The Debug Adapter Protocol lets users evaluate expressions during debugging.
This is powerful -- and dangerous. The Perl debugger's `x` and `p` commands
execute arbitrary Perl code. Without validation, a user could type
`system('rm -rf /')` into the debug console.

The `perl-dap-eval` crate provides a `SafeEvaluator` that validates
expressions before they reach the debugger. The validation pipeline runs
six checks in order:

1. **Newline rejection** -- newlines enable command injection by breaking out
   of the debugger's single-line evaluation context.
2. **Backtick rejection** -- `` `ls` `` executes shell commands.
3. **Assignment operator rejection** -- `=`, `+=`, `.=`, etc. would mutate
   program state during inspection.
4. **Increment/decrement rejection** -- `++` and `--` are mutations.
5. **Dangerous operation detection** -- a regex matches ~80 dangerous Perl
   builtins (system, exec, eval, open, unlink, fork, etc.) with
   context-aware filtering to avoid false positives.
6. **Regex mutation rejection** -- `s///`, `tr///`, `y///` modify strings
   in place.

The context-aware filtering is critical for usability. Without it, inspecting
`$print` would be blocked because "print" matches the dangerous-operation
list. The validator recognizes:

- **Sigil-prefixed identifiers**: `$print`, `@say`, `%exit` are variable
  names, not operations.
- **Braced scalars**: `${print}` is a variable dereference.
- **Package-qualified names**: `Foo::print` is a method in package `Foo`,
  not the builtin `print`. But `CORE::print` is blocked.
- **Single-quoted strings**: `'print this'` is a literal, not a call.
- **Escape sequences**: `\s` in a regex is not `s///`.

The evaluator also blocks code-deref tricks like `&{$sub}` and method
dispatch like `->$method` that could invoke dangerous operations through
indirection.

By default, safe evaluation mode is active. Users can opt into `allowSideEffects: true`
for full access when they genuinely need to mutate state during debugging.

## "When Receipts Lie"

An AI agent once wrote a benchmark for perl-lsp that reported impressive
numbers. The benchmark compiled. The tests passed. The CI gate was green.
The numbers looked plausible.

The problem: the benchmark was measuring the wrong thing. It was timing a
code path that was dominated by setup overhead, not the operation under test.
The reported metrics were technically valid Rust timing measurements, but
they had no relationship to the performance characteristic they claimed
to measure. The benchmark was "technically correct, operationally meaningless."

This is not a hypothetical. It happened in this project. An agent produced
a benchmark, attached it to a PR, and the numbers were accepted because they
passed the CI gate. The flaw was only discovered when a human asked "what
exactly are we measuring here?" and traced the code path.

The lesson: **test output is not evidence of correctness.** A green CI gate
proves that the code compiles and the tests pass. It does not prove that
the tests test the right thing. This is why perl-lsp invests in mutation
testing.

## Mutation Testing

Traditional test coverage answers: "did the test execute this line?"
Mutation testing answers: "if I change this line, does a test fail?"

The difference matters enormously. A test suite with 100% line coverage can
still miss bugs if the tests don't assert on the right values. Mutation
testing catches this by systematically modifying the source code (inserting
"mutants") and checking whether the test suite detects each change.

perl-lsp uses `cargo-mutants` for mutation testing:

```bash
# Bounded run (~5-10 min)
just mutation-subset

# Full run (~15-30 min)
just ci-test-mutation
```

The project also maintains dedicated mutation regression harnesses that
target known survivor patterns:

```bash
just mutation-regression
# Runs:
#   cargo test -p perl-parser --test mutation_hardening_tests
#   cargo test -p perl-parser --test parser_boolean_logic_mutation_hardening
#   cargo test -p perl-lsp-rs --test mutation_survivors_elimination
```

These harnesses are tests specifically written to kill mutants that
previously survived. When a mutation testing run finds a survivor -- a
code change that no test detects -- a targeted test is written and added
to the regression harness. This ratchets the mutation score upward over time.

The adversarial framing is deliberate. A test suite is a claim about code
correctness. Mutation testing stress-tests that claim by asking: "if this
claim were wrong, would you notice?" When the answer is "no," you have
found a gap in your verification, not your code.

This is what catches the "operationally meaningless benchmark" failure mode.
A benchmark that measures setup overhead instead of the target operation
will survive mutations to the target operation -- because the mutant
doesn't change the setup path. The mutation testing framework reveals that
the benchmark, despite being green, proves nothing about the code it
claims to validate.

---

## Defense in Depth

No single layer is sufficient. Clippy lints catch syntactic violations but
miss semantic ones. CI gates catch compile-time errors but not logic bugs.
Code review catches logic bugs but is fallible. Mutation testing catches
untested logic but is expensive to run.

The reliability story of perl-lsp is not any one of these mechanisms. It is
all of them, layered:

| Layer | Catches | Misses |
|-------|---------|--------|
| Clippy deny lints | `unwrap()`, `expect()`, `panic!()` | Semantic error swallowing |
| CI gates | Compile errors, lint violations | Logic bugs with green tests |
| Code review | Logic bugs, design flaws | Human attention limits |
| AI agent enforcement | Consistent policy application | Novel violation patterns |
| Mutation testing | Untested logic, weak assertions | Performance characteristics |
| Supply chain policy | Malicious/vulnerable deps | Zero-day vulnerabilities |
| Path validation | Filesystem traversal attacks | Application-level logic bugs |
| Expression safety | Command injection, state mutation | Novel Perl eval tricks |

Each layer compensates for the blind spots of the others. The result is a
language server that handles malformed input, hostile file paths, and
dangerous debug expressions without crashing -- and a verification system
that tests whether the tests themselves are doing their job.
