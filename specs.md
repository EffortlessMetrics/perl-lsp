# Specs — work-d26d9915

## Feature: Document Links for Module::Runtime function calls

### Feature Description

Extend the `compute_links` function in `perl-lsp-rs-core/src/providers/document_links/mod.rs` to recognize `use_module()` and `require_module()` function calls (from `Module::Runtime`) as navigable module references, emitting clickable document links for static string literal arguments.

### Behavior

When scanning a Perl source file, the `compute_links` function will additionally detect:

1. **Bare function calls**:
   - `use_module('Some::Module')` → emits module link for `Some::Module`
   - `require_module('Some::Module')` → emits module link for `Some::Module`
   - `use_module("Some::Module")` → emits module link for `Some::Module` (double quotes)
   - `require_module("Some::Module")` → emits module link for `Some::Module` (double quotes)

2. **Qualified function calls**:
   - `Module::Runtime::use_module('Some::Module')` → emits module link for `Some::Module`
   - `Module::Runtime::require_module('Some::Module')` → emits module link for `Some::Module`

3. **Cases that MUST NOT emit links**:
   - Commented code: `# use_module('Foo')` — must be excluded
   - Dynamic/variable arguments: `use_module($variable)` — cannot be statically resolved
   - Non-string arguments: `use_module(@args)` — invalid, ignored
   - Nested quotes: `use_module('it\'s')` — not supported by simple text extraction

### Acceptance Criteria

1. **`use_module('Foo::Bar')` emits exactly one module link** with `data.type == "module"` and `data.module == "Foo::Bar"`

2. **`require_module('Baz::Qux')` emits exactly one module link** with `data.type == "module"` and `data.module == "Baz::Qux"`

3. **`Module::Runtime::use_module('A::B')` emits exactly one module link** for `A::B`

4. **Double-quoted strings work**: `use_module("Foo::Bar")` emits a module link

5. **Commented code is excluded**: `# use_module('Foo')` on the same line as a real call does not create a duplicate or spurious link

6. **Variable arguments are ignored**: `use_module($dynamic_name)` does not emit a link

7. **Existing `use`/`require` statement detection is unaffected**: `use Foo::Bar;` and `require Foo::Bar;` still work as before

### Non-Goals

- Resolving dynamic module names (`use_module($variable)`) — static analysis cannot handle this
- Supporting `module_name()` from `Module::Runtime` — different function with different semantics
- Modifying semantic analysis or completion — those already work correctly
- Handling heredoc or complex nested quote forms

### Dependencies

- No new crate dependencies required
- Uses existing `perl-module` imports already present in the file
- Uses existing `serde_json` and `url` dependencies

### Implementation Notes

The implementation follows the existing pattern for inline `require "path"` detection (lines 61–87 of `mod.rs`):

1. Per-line text scanning using `line.find("use_module(")` or `line.find("require_module(")` or `line.find("Module::Runtime::use_module(")` etc.
2. Extract the string literal argument using quote-aware parsing
3. Build a deferred module link using `make_deferred_module_link`
4. Ensure comment exclusion by verifying the match position is not after a `#` on the same line
