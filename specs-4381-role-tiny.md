# Specs — Role::Tiny Support for Role Composition Diagnostics

## Feature Description

Add Role::Tiny framework support to the existing role conflict detection system (PL303). When a Perl class in the same file consumes multiple Role::Tiny roles that provide overlapping method names, the LSP emits a PL303 diagnostic.

## Behavior

### Same-File Role Conflict Detection
When `check_role_conflicts()` processes a file containing:
1. A class using `use Role::Tiny::With;` and `with 'RoleA', 'RoleB';`
2. Two or more of those roles defined in the same file using `use Role::Tiny;`
3. Those roles providing a method with the same name

Then the LSP emits diagnostic PL303 (role method conflict).

### Detection Is Framework-Agnostic
The conflict detection logic itself is unchanged — only the framework detection is extended to recognize Role::Tiny. The `with 'Role'` syntax works identically for Moose/Moo/Mouse/Role::Tiny once the framework is recognized.

### Suppression via Class Method
If the consuming class itself defines a method with the conflicting name, the diagnostic is suppressed (same behavior as Moose/Moo).

## Acceptance Criteria

1. **Role::Tiny role conflict detection:** When two Role::Tiny roles in the same file provide the same method, and a class consumes both via `with()`, a PL303 diagnostic is emitted on the `with()` call.

2. **Three-way conflict detection:** When three Role::Tiny roles provide the same method and are consumed together, PL303 is emitted.

3. **Class method suppresses conflict:** When the consuming class defines its own implementation of the conflicting method, no diagnostic is emitted.

4. **Both import styles work:** Both `use Role::Tiny;` (role definition) and `use Role::Tiny::With;` (role consumption) are recognized as the Role::Tiny framework.

5. **Existing Moose/Moo/Mouse behavior unchanged:** All existing role conflict tests continue to pass.

6. **No new diagnostic codes:** Only the existing PL303 code is used; no new codes are introduced.

## Non-Goals

- Workspace-wide role conflict detection (only same-file is covered)
- Transitive role composition detection
- Changes to the symbol table data model beyond adding Role::Tiny framework recognition
- Support for Role::Tiny's `with()` function outside of a class context (out of scope)

## Dependencies

- `perl-semantic-analyzer`: Framework enum changes in `class_model.rs` and `FrameworkKind`/symbol table changes in `symbol.rs`
- `perl-lsp-diagnostics`: Comment update in `role_conflicts.rs`, new integration tests
- No changes to `perl-parser`, `perl-workspace-index`, or other crates
