# ADR: Extend NativeBuildHints to Parse LIBS, DEFINE, OBJECT, and MYEXTLIB

**Status:** Proposed

**Work Item:** work-8972b8ca

---

## Context

The `native_build_hints` module in `perl-lsp-config` currently extracts only include directories (`INC` and `extra_compiler_flags`) from `Makefile.PL` and `Build.PL`. Issue #4378 requests extending it to also parse:

- **LIBS** — library flags for native linking (e.g., `-L/path -lssl -lcrypto`)
- **DEFINE** — preprocessor macros (e.g., `DEFINE => '-DFOO=1'`)
- **OBJECT** — object file list for XS linking
- **MYEXTLIB** — external library list for XS modules

Additionally, `refresh_native_build_hints()` exists in `WorkspaceConfig` but is never invoked during workspace initialization.

### Prior Review Findings

Three review agents evaluated the initial plan and found:

1. **`.unwrap()` panic** (plan-reviewer, maintainer-vision): The integration uses `folder.path.as_ref().unwrap()` but `folder.path` is `Option<PathBuf>` and can be `None`. This violates the codebase's active ban on `unwrap()` in production code.

2. **Wrong Build.PL scope** (plan-reviewer, maintainer-vision): LIBS, DEFINE, OBJECT, and MYEXTLIB are ExtUtils::MakeMaker (Makefile.PL) concepts only. Module::Build (Build.PL) does not have these parameters. Build.PL extraction for these four parameters is unnecessary.

3. **LIBS parsing complexity** (plan-reviewer): LIBS flags can contain quoted paths with spaces (e.g., `'-L"/path/with spaces/lib" -lfoo'`). A naive whitespace split fails here.

4. **Dead code concern** (maintainer-vision): `native_build_hints` is populated but never consumed by any code path. Adding four more dead fields extends infrastructure without delivering user-facing value.

### Why Proceed Despite Dead Code Concern

The issue explicitly requests this feature. While `native_build_hints` is currently unused, the fields will be consumed by a future consumer (the issue does not specify which, but this is a reasonable feature request for XS module support). We proceed with the extension but ensure the implementation is correct and non-breaking.

---

## Decision

We extend `NativeBuildHints` with four new fields and add literal extraction for Makefile.PL only, with safe path handling in the integration point.

### 1. Extend `NativeBuildHints` Struct (Backward-Compatible)

Add four new fields to the existing struct rather than creating a new type:

```rust
pub struct NativeBuildHints {
    pub include_dirs: Vec<String>,    // existing
    pub libs_flags: Vec<String>,       // new: raw LIBS linker flags
    pub define_flags: Vec<String>,    // new: raw DEFINE preprocessor flags
    pub object_files: Vec<String>,    // new: OBJECT file list
    pub myextlib_files: Vec<String>,  // new: MYEXTLIB external libs
}
```

**Rationale:** This is backward-compatible (additive only) and keeps related fields in one struct rather than scattering them across the codebase.

### 2. Extract Only from Makefile.PL (Not Build.PL)

LIBS, DEFINE, OBJECT, and MYEXTLIB are ExtUtils::MakeMaker concepts. Build.PL (Module::Build) does not use these parameters. We only add extraction to the Makefile.PL path.

**Rationale:** Avoids unnecessary code and matches the actual semantics of Perl's build systems.

### 3. LIBS Parsing with Quote-Aware Whitespace Split

LIBS flags can contain quoted paths with spaces. We reuse the existing `parse_quoted_string()` infrastructure to handle this:

```rust
fn split_libs_flags(flags: &str) -> Vec<String> {
    // Parse tokens, preserving quoted strings as single tokens
    flags
        .split_whitespace()
        .map(|token| token.to_owned())
        .collect()
}
```

Actually, since LIBS values are typically quoted strings containing multiple flags, we need a more sophisticated approach:

```rust
fn extract_libs_flags(source: &str) -> Vec<String> {
    extract_literal_values_after_key(source, "LIBS")
        .into_iter()
        .flat_map(|value| parse_libs_value(&value))
        .collect()
}

fn parse_libs_value(value: &str) -> Vec<String> {
    // Value is a quoted string like '-L/lib -lfoo' or '-L"/path with spaces/lib" -lbar'
    // We need to extract the raw value, then optionally split by whitespace for individual flags
    // The existing infrastructure already gives us the full quoted string content
    vec![value.to_owned()]
}
```

Wait — LIBS is typically `LIBS => '-L/path -lfoo'` and the value is a single quoted string. We should preserve the entire string as one flag entry, not split it further, because the consumer may need to pass it to a linker as-is.

**Rationale:** Keeping the raw LIBS string intact is simpler and avoids incorrectly splitting linker flag groups. The consumer can parse as needed.

### 4. Safe Integration Without `.unwrap()`

In `handle_client_response()`, use `if let Some(path) = &folder.path` before calling `refresh_native_build_hints()`:

```rust
if let Some(path) = &folder.path {
    effective_config.refresh_native_build_hints(path);
}
```

**Rationale:** The codebase bans `unwrap()` in production code. `folder.path` is `Option<PathBuf>` and can be `None`.

### 5. DEFINE, OBJECT, MYEXTLIB Extraction

These follow the same pattern as INC — use `extract_literal_values_after_key()` and preserve the raw value:

- **DEFINE** → `extract_literal_values_after_key(source, "DEFINE")`
- **OBJECT** → `extract_literal_values_after_key(source, "OBJECT")`
- **MYEXTLIB** → `extract_literal_values_after_key(source, "MYEXTLIB")`

All three are typically single quoted strings or arrays of quoted strings.

---

## Consequences

### Benefits

- Extended infrastructure for XS module support in Perl projects
- Backward-compatible change (additive only)
- Reuses existing parsing primitives (`parse_quoted_string`, `parse_quoted_string_array`, `extract_literal_values_after_key`)
- No `.unwrap()` panic risk
- Correct scope (Makefile.PL only)

### Tradeoffs

- **Dead code**: `native_build_hints` is still not consumed by any downstream code. This extends infrastructure without immediate user-facing value. Future consumer needed.
- **No Build.PL support for these parameters**: As per Perl build system semantics, this is correct — not a limitation.
- **LIBS kept as raw string**: Simpler but requires consumer to parse linker flags.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `native_build_hints` remains unused | High | Low | Future consumer expected per issue scope |
| LIBS parsing edge cases | Medium | Low | Keep as raw string to avoid over-parsing |
| Build.PL confusion | Low | Low | Documentation clarifies only Makefile.PL is parsed |

---

## Alternatives Considered

### Alternative 1: Create a Separate Struct for XS Build Hints

Create a new `XsBuildHints` struct rather than extending `NativeBuildHints`.

**Rejected because:** The issue explicitly requests extending `native_build_hints`. Splitting into two structs fragments the concept. The existing struct name is misleading (it includes more than just includes) but changing it is out of scope.

### Alternative 2: Full LIBS Flag Parsing

Parse LIBS into individual `-L` and `-l` flags.

**Rejected because:** LIBS format is complex and consumer-dependent. Keeping the raw string is simpler and more flexible. If a consumer needs parsed flags, they can implement their own parsing logic.

### Alternative 3: Include Build.PL Extraction for Completeness

Add extraction for completeness even though Module::Build doesn't typically use these parameters.

**Rejected because:** Unnecessary code. The plan-reviewer correctly identified that Build.PL (Module::Build) does not use LIBS/DEFINE/OBJECT/MYEXTLIB. Adding extraction for non-existent parameters is wasted effort.

### Alternative 4: Delay Until Consumer Exists

Do not implement until a downstream consumer actually uses the data.

**Rejected because:** The issue explicitly requests this feature. Dead code concern is noted but does not justify rejecting a valid feature request. Infrastructure should be available when needed.
