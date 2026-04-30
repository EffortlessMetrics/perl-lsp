# ADR-0017: First-Run Error Classification and Messaging

## Status
Proposed

## Context

Issue #4178 identifies 7 first-run error scenarios in perl-lsp where users see nothing, generic messages, or log-only errors instead of actionable messaging. After investigation:

- **4 of 7 are already addressed** in the current codebase (parse error hints, missing module @INC context, workspace building notification, permission denied)
- **2 genuinely remain unaddressed**:
  1. **Gap 1 (HIGH)**: Perl not installed — `WARN_ONCE_ROOT_UNDETECTED.call_once(|| { tracing::warn!(...) })` writes to server log only; user sees nothing
  2. **Gap 5 (MEDIUM)**: DAP launch — `Err(e.to_string())` returns raw I/O error `"No such file or directory: 'perl'"` instead of actionable guidance

### Gap 1: Why the Initial Fix Was Infeasible

The initial plan proposed replacing `tracing::warn!()` with `self.show_message(...)` inside the `call_once` closure. This fails because:

```rust
// Current code — module-level static
static WARN_ONCE_ROOT_UNDETECTED: Once = Once::new();
WARN_ONCE_ROOT_UNDETECTED.call_once(|| {
    tracing::warn!("perl-lsp: workspace root not detected — ...");
});
```

The `call_once` closure is `'static` and captures nothing from the environment — it cannot access `self`. The `LspServer` methods `resolve_module_path` and `resolve_module_path_with_uri` have access to `self`, but the closure does not.

### Gap 5: Raw I/O Error Instead of Actionable Message

```rust
// process.rs:365 — current code
Err(e) => Err(e.to_string())  // produces "No such file or directory: 'perl'"
```

When `perl -d` fails to spawn, the raw I/O error string is returned. The user receives `"No such file or directory: 'perl'"` with no remediation guidance.

## Decision

### Gap 1: Use Instance-Level AtomicFlag for One-Time Warning

Add `root_undetected_shown: Arc<AtomicBool>` to the `LspServer` struct. Replace the `WARN_ONCE_ROOT_UNDETECTED.call_once(|| { tracing::warn!(...) })` pattern with:

```rust
if self.root_undetected_shown.fetch_or(true, Ordering::SeqCst) == false {
    let _ = self.show_message(MessageType::Warning, 
        "perl-lsp: workspace root not detected — module resolution disabled. \
        To enable: open the project folder in your editor (File > Open Folder) \
        rather than individual files. This warning appears once per server session.");
}
```

**Rationale**: This approach:
- Allows `self.show_message(...)` to be called (closure captures `self` via the method)
- Uses atomic check-and-set to ensure exactly-once semantics
- Changes semantics from "once per process" to "once per server session", which is more correct for the stated use case
- Follows existing patterns in `LspServer` (which already has many `Arc<AtomicBool>` fields like `client_supports_pull_diagns`)
- Is purely additive — no existing APIs or protocols are modified

### Gap 5: Detect ErrorKind::NotFound for Actionable DAP Error

In `process.rs:365`, replace `Err(e) => Err(e.to_string())` with:

```rust
Err(e) => {
    if e.kind() == std::io::ErrorKind::NotFound {
        Err("Perl interpreter not found on PATH. Ensure 'perl' is installed and on your system PATH.".to_string())
    } else {
        Err(e.to_string())
    }
}
```

**Rationale**: 
- Directly addresses the user-facing gap: raw I/O error vs. actionable guidance
- Localized change with no architectural impact
- `ErrorKind::NotFound` is the correct kind to check when a command cannot be found on PATH
- Easy to test by temporarily renaming `perl`

## Consequences

### Benefits

1. **First-run users get actionable guidance**: Instead of silent failure or cryptic errors, users see clear messages explaining the problem and how to fix it
2. **Reduces support burden**: Actionable error messages reduce "it doesn't work" issues filed without understanding why
3. **No API changes**: Both changes use existing methods (`show_message()`, error matching) — purely additive
4. **No protocol changes**: All changes are internal to the server implementation
5. **Follows existing patterns**: `LspServer` already uses `Arc<AtomicBool>` for instance-level flags

### Tradeoffs

1. **Semantic shift (Gap 1)**: Changes from "once per process" to "once per server session". If multiple `LspServer` instances run in one process, each shows the message once. This is arguably more correct for the use case (each server session should warn once).

2. **Minor latency (Gap 1)**: `fetch_or` on `AtomicBool` adds a small atomic operation to each module resolution call. The `tracing::warn!` was already fire-and-forget; the atomic check is similarly cheap.

3. **Channel timing (Gap 1)**: If `show_message` fails because the LSP channel isn't initialized, the message is silently dropped. This is acceptable — the previous `tracing::warn!` was also fire-and-forget.

4. **DAP permission denied (Gap 5)**: The fix only handles `NotFound`. If `perl` is on PATH but not executable, the error remains a raw I/O error. This is a separate edge case deferred to future work.

## Alternatives Considered

### Alternative 1: Keep Module-Level static Once, Deferred Flag Pattern

Keep `WARN_ONCE_ROOT_UNDETECTED` as-is, but add a deferred notification:
```rust
WARN_ONCE_ROOT_UNDETECTED.call_once(|| {
    set_deferred_warning("perl-lsp: workspace root not detected — ...");
});
```

**Rejected**: More complex — requires adding deferred warning infrastructure. The `Arc<AtomicBool>` approach is simpler and directly callable.

### Alternative 2: Centralized Error Message Service (Option A from Issue #4178)

Create a centralized `ErrorMessageService` that aggregates and deduplicates all error messages.

**Rejected**: Too high-cost. The issue explicitly considered this and recommended Option B (per-subsystem approach). The current changes are localized and sufficient.

### Alternative 3: Only Fix Gap 5, Defer Gap 1

Focus on the DAP fix only and defer the `LspServer` refactoring.

**Rejected**: Gap 1 is the higher-priority issue — it affects the most fundamental onboarding scenario (users opening a single file without a workspace). The architectural refactoring is straightforward once the correct approach is specified.

### Alternative 4: Use tokio::sync::OnceCell Instead of Arc<AtomicBool>

Use `tokio::sync::OnceCell` for the instance-level flag.

**Rejected**: `Arc<AtomicBool>` is simpler for this use case (just check-and-set, no async). The existing codebase uses `Arc<AtomicBool>` for similar patterns. No benefit to introducing `OnceCell`.

## Deferred Decisions

1. **Missing module diagnostic severity (Gap 3)**: Whether `Warning` vs `Error` severity is appropriate for missing required modules. Deferred to team/product decision.

2. **Editor-agnostic messaging**: The Gap 1 message mentions "File > Open Folder" which is VS Code-specific. Whether the message should be editor-agnostic is deferred.

3. **DAP PermissionDenied handling**: Whether to also detect `ErrorKind::PermissionDenied` for the case where `perl` is found but not executable is deferred to future work.
