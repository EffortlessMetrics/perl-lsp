# ADR — Role::Tiny Support for Role Composition Diagnostics

## Status
Proposed

## Context

GitHub issue #4381 requests adding Role::Tiny framework support to the existing role conflict detection system (PL303). Currently, the system detects method conflicts when a class consumes multiple roles that provide the same method, but only for Moose/Moo/Mouse frameworks. Role::Tiny is explicitly excluded.

The codebase has **two parallel framework detection pipelines** that must both be updated:

1. **`Framework` enum** (`class_model.rs`) → `ClassModelBuilder::detect_framework()` → produces `ClassModel` objects used directly by `check_role_conflicts()`

2. **`FrameworkKind` enum** (`symbol.rs`) → `SymbolExtractor::update_framework_context()` → sets `SymbolKind::Class` or `SymbolKind::Role` in the symbol table, which `check_role_conflicts()` uses via `package_kind()` to filter role models

The existing plan only addressed pipeline #1. Without pipeline #2 changes, `package_kind()` returns `None` for Role::Tiny packages, they are silently filtered out, and no diagnostics are emitted.

## Decision

We will extend both framework detection pipelines to recognize Role::Tiny, enabling role conflict diagnostics (PL303) for Role::Tiny role compositions.

### Changes to Pipeline 1 (`class_model.rs`)

1. Add `RoleTiny` variant to `Framework` enum before `None`
2. Add match arms for `"Role::Tiny"` and `"Role::Tiny::With"` in `detect_framework()`
3. Update comment in `role_conflicts.rs` line 5 to remove "Role::Tiny" from ignore list

### Changes to Pipeline 2 (`symbol.rs`)

4. Add `RoleTiny` variant to `FrameworkKind` enum alongside existing Moose/Moo variants
5. Add `"Role::Tiny" | "Role::Tiny::With"` match arm in `update_framework_context()`
6. Add `FrameworkKind::RoleTiny => SymbolKind::Role` mapping in `upgrade_package_symbols_from_framework_flags()`

### Design Rationale

**Why two pipelines exist:** The `Framework` enum in `class_model.rs` is used by `ClassModelBuilder` to produce structured `ClassModel` objects from AST traversal — this is the class-model level used for diagnostics. The `FrameworkKind` enum in `symbol.rs` is used by `SymbolExtractor` to mark packages in the symbol table — this is the symbol-level used for lookups and filtering. Both are needed because `check_role_conflicts()` uses both the class models (via `ClassModelBuilder`) and the symbol table (via `package_kind()`).

**Why `Role::Tiny` and `Role::Tiny::With` both:** `use Role::Tiny;` declares a role package; `use Role::Tiny::With;` activates the `with()` function for consuming roles. Both are needed for the framework detection to work correctly regardless of which import style the user employs.

## Consequences

### Benefits
- Role::Tiny users gain the same PL303 conflict detection already available to Moose/Moo/Mouse users
- Method conflicts between roles consumed by the same class are detected early
- Minimal, surgical changes with no new behavioral surface area

### Tradeoffs
- The existing comment in `role_conflicts.rs` explicitly stated Role::Tiny was "intentionally ignored" — this ADR overturns that decision
- Role::Tiny roles are plain packages with subs (not a distinct role keyword), so detection is based on `use Role::Tiny;` import rather than a role declaration keyword

### Risks
- **False positives:** A package that uses `Role::Tiny` but doesn't intend to be a role could have its methods checked for conflicts — acceptable per the existing Moose/Moo behavior
- **Test coverage:** Three new integration tests must be added to verify the feature works end-to-end

## Alternatives Considered

### Alternative 1: Only Update `class_model.rs` (Original Plan)
- **Rejected because:** The plan only addressed `ClassModelBuilder` and ignored the `SymbolExtractor` pipeline. `package_kind()` would return `None` for Role::Tiny packages, silently filtering them out. The feature would do nothing.

### Alternative 2: Create a Unified Framework Enum
- **Rejected because:** The two enums serve different purposes and have different structures (`Framework` has `NativeClass`/`PlainOO` variants not in `FrameworkKind`, and vice versa). Merging them would be a large, high-risk refactor beyond the scope of this issue.

## Non-Goals
- Extending role conflict detection to workspace-wide indexing (intentional limitation preserved)
- Adding transitive role composition detection (intentional limitation preserved)
- Modifying the diagnostic output format or adding new diagnostic codes
