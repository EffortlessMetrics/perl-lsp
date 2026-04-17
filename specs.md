# Specifications — Sub::Exporter Support

**Work Item**: work-9c61d264
**Issue**: [GitHub #3413 - Import/Export Gap: Sub::Exporter support missing](https://github.com/EffortlessMetrics/perl-lsp/issues/3413)

## Feature Description

Add Sub::Exporter support to the Perl LSP, enabling:
- **Goto-definition** for symbols imported via Sub::Exporter configurations
- **Completion** for Sub::Exporter-configured import symbols
- **Symbol resolution** for group/tag imports (`:default`, `:all`)

Sub::Exporter is a widely-used Perl module providing sophisticated export configuration via hashref-based APIs. It is used by Moose, Moo, Test::More, Dist::Zilla, Catalyst, and many other critical Perl ecosystem modules.

### Supported Patterns

#### Pattern 1: Simple exports array
```perl
use MyModule {
    exports => [qw(foo bar baz)],
};
```

#### Pattern 2: Groups/tags
```perl
use MyModule {
    exports => [qw(foo bar baz)],
    groups => {
        default => [qw(foo bar)],
        all => [qw(foo bar baz)],
    },
};
```

#### Pattern 3: Renaming with -as
```perl
use Module::WithSubExporter
    func1 => { -as => 'my_func1' },
    func2 => { -as => 'other_func' };
```

#### Pattern 4: Sub::Exporter -setup invocation
```perl
use Sub::Exporter -setup => {
    exports => [qw(foo bar)],
    groups => { default => [qw(foo)] },
};
```

#### Pattern 5: MethodCall import
```perl
use MyModule;
MyModule->import({ exports => [qw(foo bar)] });
```

## Acceptance Criteria

### AC1: Goto Definition for Sub::Exporter Imports
**Given** a Perl file with `use MyModule { exports => [qw(foo bar)] };`
**When** the user triggers goto-definition on `foo`
**Then** the LSP navigates to `foo`'s definition in `MyModule`

### AC2: Completion for Sub::Exporter Symbols
**Given** a Perl file with `use MyModule { exports => [qw(foo bar baz)] };`
**When** the user types `MyModule->` and triggers completion
**Then** the completion list includes `foo`, `bar`, `baz` as available methods

### AC3: Group/Tag Resolution
**Given** a Perl file with:
```perl
use MyModule {
    exports => [qw(foo bar baz)],
    groups => { default => [qw(foo bar)] },
};
```
**When** the user triggers goto-definition on a symbol imported via `:default` tag
**Then** the LSP correctly resolves the symbol

### AC4: Renaming Support in Completion
**Given** a Perl file with:
```perl
use Module::WithSubExporter
    func1 => { -as => 'my_func1' };
```
**When** the user types `my_func1` and triggers completion elsewhere
**Then** `my_func1` appears as a available symbol from `Module::WithSubExporter`

### AC5: No Regression for Existing Exporter
**Given** existing Perl code using standard Exporter (`use Foo qw(bar baz)`)
**When** goto-definition or completion is triggered
**Then** existing functionality continues to work correctly

### AC6: Sub::Exporter -setup Detection
**Given** a module using `use Sub::Exporter -setup => { exports => [...] }`
**When** another file imports from that module
**Then** goto-definition and completion work for the exported symbols

## Non-Goals (Out of Scope)

The following are explicitly **NOT** in scope for this implementation:

1. **Coderef-based exporters** — Sub::Exporter patterns like `exports => [qw(foo), bar => \&build_bar]` require runtime code evaluation to determine exported symbols. Static analysis cannot resolve these.

2. **Collector/generator patterns** — Sub::Exporter's collector mechanism for building exports dynamically is not supported.

3. **Sub::Exporter::Composable** — The compositional Sub::Exporter feature is not supported.

4. **Full group resolution for custom groups** — Only standard groups (`default`, `all`) and their mappings are guaranteed. Arbitrary custom group names may not resolve correctly in v1.

5. **CPAN metadata integration** — Using META.json or other CPAN metadata for export lists is not part of this implementation.

6. **perl-ast breaking changes** — This implementation works within the existing `perl-ast` constraints. A future ADR may address AST representation changes.

## Dependencies

### Internal Dependencies

1. **perl-semantic-analyzer** — `find_import_source()` in declaration.rs needs Sub::Exporter pattern detection and symbol extraction
2. **perl-lsp-completion** — `collect_import_symbols()` needs to handle Sub::Exporter hash configs; `collect_node_import_symbols()` should handle `HashLiteral` nodes
3. **perl-ast** — Current `NodeKind::Use { args: Vec<String> }` remains unchanged; enhancement via optional `structured_args` field is future work

### External Constraints

1. **perl-parser-core** — The LSP uses this native Rust recursive descent parser, NOT tree-sitter
2. **tree-sitter-perl** — Not on the LSP's critical path; NOT used by this implementation
3. **v1.0 stability** — Changes must be backward compatible; no breaking API changes

## Technical Approach

### Detection Strategy
Since `args: Vec<String>` loses structural information, detection uses token pattern matching:
1. Look for args starting with `{` (hash start token)
2. Check for `exports`, `-setup`, or `-as` keywords in the token stream
3. If Sub::Exporter pattern detected, mark the module for specialized handling

### Extraction Strategy
For detected Sub::Exporter patterns:
1. Re-parse relevant tokens from the flat `Vec<String>` 
2. Extract symbol names from `exports => [qw(foo bar)]` patterns
3. Build export map from `groups => {...}` definitions
4. Track `-as` renamings for completion display

### Code Path Strategy
- `use` statements: Modify `collect_import_symbols()` and `find_import_source()` to detect and handle Sub::Exporter patterns
- `MethodCall->import(...)`: Extend `collect_node_import_symbols()` to handle `HashLiteral` nodes directly (since structure is preserved in Node)

### Testing Strategy
1. **Unit tests**: Test detection and extraction functions with various Sub::Exporter patterns
2. **Integration tests**: Test goto-definition and completion with real Sub::Exporter-using modules
3. **Regression tests**: Ensure existing Exporter functionality is unaffected
4. **Edge case tests**: Test invalid/unsupported patterns gracefully degrade

## Verification

1. **Unit tests** for Sub::Exporter detection and symbol extraction
2. **Integration tests** with Moose, Moo, or Test::More (real-world Sub::Exporter usage)
3. **Completion tests** verifying symbols appear in completion lists
4. **Navigation tests** verifying goto-definition works for Sub::Exporter imports
5. **Regression tests** ensuring existing Exporter tests pass