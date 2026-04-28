# perl-module

Perl module resolution, import analysis, and refactoring — unified facade.

This crate absorbs 13 `perl-module-*` microcrates into a single published
package. Internal functionality is organized into focused submodules; the
public API is a flat re-export surface in `api.rs`.

## Submodule Map

| Module | Responsibility |
|---|---|
| `name` | Separator normalization (`::` ↔ `'`) and module-name variant helpers |
| `path` | Bidirectional conversion between module names (`Foo::Bar`) and file paths (`Foo/Bar.pm`) |
| `token_core` | Shared primitives — `ModuleTokenSpan`, character-class predicates |
| `token_parser` | Single-line Perl module token parsing from byte offsets |
| `token` | Boundary-safe module token replacement (canonical and legacy separators) |
| `boundary` | Standalone module-token scanning with respect to package separator boundaries |
| `import` | Single-line `use`/`require` head parsing; `ModuleImportKind`, `LoadTiming`, `DispatchSemantics` |
| `import_match` | Predicate: does a source line target a module-import rename? |
| `reference` | Cursor-aware module reference extraction from `use`/`require` statements |
| `rename` | Line-edit planning for file-rename refactoring workflows (`ModuleLineEdit`) |
| `resolution` | URI and filesystem module path resolution; `use lib` / `FindBin` pragma extraction |

## Usage

```toml
[dependencies]
perl-module = { path = "../../crates/perl-module" }
```

Always import from the crate root — submodule paths are not part of the
public API and may change between releases.

```rust
use perl_module::{
    module_name_to_path, file_path_to_module_name,
    parse_module_import_head, ModuleImportKind,
    find_module_reference, ModuleReference,
};

// Convert between Perl module names and file paths
let path = module_name_to_path("Foo::Bar"); // → "Foo/Bar.pm"
let name = file_path_to_module_name("lib/Foo/Bar.pm", "lib"); // → "Foo::Bar"

// Parse an import statement
let head = parse_module_import_head("use Foo::Bar qw(baz);", 0);
```

## Design Notes

- **Facade pattern**: all public items are re-exported via `api.rs`; consumers
  never import from submodule paths directly.
- **Byte-offset API**: functions that operate on source text work with byte
  offsets rather than character offsets, matching the LSP wire format.
- **No LSP dependency**: this crate does not depend on the LSP layer; it is a
  pure Perl-semantics library usable independently of any protocol.
- **Security**: the `resolution` module uses URI-based path construction to
  avoid path-traversal vulnerabilities when resolving user-supplied module names.
