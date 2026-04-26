# ADR/Spec Findings — work-77b3da72

## What This ADR Decides
Add Role::Tiny to both parallel framework detection pipelines (Framework enum in class_model.rs AND FrameworkKind enum in symbol.rs), enabling PL303 role conflict diagnostics for Role::Tiny role compositions.

## Key Decision
Extend BOTH `Framework` enum (class_model.rs) AND `FrameworkKind` enum (symbol.rs) to recognize Role::Tiny. The original plan only addressed class_model.rs, which would cause the feature to silently do nothing because package_kind() filters out packages not marked as SymbolKind::Role.

## Alternatives Considered
1. **Only update class_model.rs (original plan)** — Rejected because symbol.rs pipeline wouldn't mark Role::Tiny packages as SymbolKind::Role, causing package_kind() to return None and silently filter out all Role::Tiny packages.
2. **Unified framework enum** — Rejected because the two enums serve different purposes and merging would be a high-risk refactor beyond scope.

## Consequences
- Benefits: Role::Tiny users gain PL303 conflict detection, minimal/surgical changes
- Tradeoffs: Existing explicit exclusion comment in role_conflicts.rs is overturned
- Risks: False positives acceptable per existing Moose/Moo behavior; 3 new tests required

## Acceptance Criteria
1. Two+ Role::Tiny roles with same method + class consuming both → PL303 emitted
2. Three-way Role::Tiny conflict → PL303 emitted  
3. Class-defined method suppresses conflict → No diagnostic
4. Both `use Role::Tiny;` and `use Role::Tiny::With;` styles recognized
5. All existing Moose/Moo/Mouse tests pass
6. No new diagnostic codes introduced
