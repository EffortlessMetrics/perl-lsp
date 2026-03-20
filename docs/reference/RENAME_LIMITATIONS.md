# Rename Limitations

> **Feature**: `textDocument/rename` and `textDocument/prepareRename`
> **Related issue**: #2381 (this document), #433 (workspace-wide rename roadmap)

---

## Summary

| Scenario | Status |
|----------|--------|
| Single-file rename (variables, subs) | Working |
| Single-file scoped rename (respects lexical shadowing) | Working |
| Cross-file rename (same-file fallback when index not ready) | Working |
| Cross-file rename (workspace index, Ready state) | Partial |
| Package rename | Not supported |
| Import statement updates (`use Pkg qw(...)`) | Not supported |
| `@ISA` / `use parent` / `use base` updates | Not supported |
| Dynamic dispatch (`AUTOLOAD`, `eval`, `\&Pkg::func`) | Not supported |

---

## What Works Today

### Single-file rename

Renaming a symbol within the file you are editing is fully supported. The LSP
server parses the file, builds a symbol table with `perl-semantic-analyzer`, and
returns a `WorkspaceEdit` containing every occurrence of the symbol in that file.

**Guaranteed to work**:
- Scalar (`$var`), array (`@arr`), and hash (`%hash`) variables
- Named subroutines (`sub foo`)
- Sigil preservation: renaming `$count` to `total` produces `$total`, not `total`
- New-name validation: rejects empty strings, leading digits, Perl keywords, and non-identifier characters

**Example** - renaming `$count` to `$total` in a single file:

```perl
# Before
my $count = 0;
$count += 1;
print $count;

# After rename to "total"
my $total = 0;
$total += 1;
print $total;
```

### Scope-aware single-file rename

`RenameProvider::scoped_rename` (used internally) respects lexical scoping.
When an inner `my $x` shadows an outer `my $x`, renaming one does not affect the other.

```perl
my $x = 1;       # outer
if (1) {
    my $x = 2;   # inner -- treated as a separate symbol
    print $x;
}
print $x;
```

Renaming the outer `$x` leaves the inner `$x` untouched, and vice versa.

---

## What Is Partially Supported

### Cross-file rename (workspace index path)

When the workspace index has finished building (lifecycle state: `Ready`), the
LSP server uses `WorkspaceIndex::find_refs` and `find_def` to locate references
across all indexed files and returns a multi-file `WorkspaceEdit`.

This path works for straightforward sub and variable references that the index
has recorded. However, several common patterns are **not** handled:

| Gap | Description |
|-----|-------------|
| Package renames | Renaming `package Utils;` does not update `use Utils;`, `@ISA`, or file paths |
| Export list updates | `use Utils qw(old_name)` is not updated when `old_name` is renamed |
| `use parent` / `use base` | Inheritance declarations are not updated on package rename |
| `our` vs `my` scoping | Package-scoped `our $var` is not distinguished from lexically-scoped `my $var` |
| Indirect references | `\&Pkg::func`, `AUTOLOAD`, and string-based dispatch are invisible to the index |

### Degraded / building state fallback

If the workspace index is still building or unavailable, the server falls back
to single-file rename automatically. The user sees correct edits for the current
file only. A warning is printed to `stderr`:

```
Rename: workspace rename unavailable (index building), using same-file only
```

---

## What Is Not Supported

### Package rename

Renaming a `package` declaration is not supported. If you rename the symbol at
a `package Foo;` line, only the declaration token is changed. The following are
**not** updated automatically:

- `use Foo;` statements in other files
- `use parent 'Foo';` and `use base 'Foo';`
- `our @ISA = qw(Foo);` declarations
- `Foo::method()` qualified calls
- The `.pm` file path on disk

**Workaround**: Rename the package declaration manually, then use editor
find-and-replace for `use Foo` and `Foo::` occurrences, and rename the file.

### Import statement updates

If you rename a subroutine that is explicitly imported with `use Pkg qw(name)`,
the `qw` list is not updated. After the rename, callers that imported the old
name will break at runtime.

**Workaround**: Search for `qw(old_name)` occurrences after renaming.

### Dynamic dispatch patterns

The following are invisible to static analysis and will not be renamed:

```perl
eval "Utils::$method()";        # string eval
my $sub = \&Utils::old_func;    # code reference captured before rename
$dispatch{old_func}->();        # hash-based dispatch table
AUTOLOAD                        # dynamically generated method names
```

---

## Workarounds

1. **Use your editor's find-and-replace** for cross-file renames when the
   automatic rename misses occurrences. Scope the search to the relevant
   directory to avoid false matches.

2. **Run `perl -c`** after a rename to catch compile-time errors from missed
   references. Many package and import issues surface as `Can't locate` or
   `Undefined subroutine` errors.

3. **Grep for the old name** before applying a rename to understand the full
   blast radius:
   ```bash
   grep -r 'old_name' lib/ t/
   ```

4. **For module file renames**: use `perl-module-rename` (a library crate in this
   workspace) to compute line-level edits for `use`, `require`, `use parent`, and
   `use base` statements when you rename a `.pm` file.

---

## Tracking

Full cross-file rename is planned. The design specification lives at
[WORKSPACE_RENAME_SPECIFICATION.md](./WORKSPACE_RENAME_SPECIFICATION.md).
The implementation roadmap is tracked in issue #433.

Expected work breakdown:

- **Phase 1** (core infrastructure): Extend `SymbolKey` for scope tracking, add
  package declaration detection, implement `use`-statement indexing (~2-3 PRs)
- **Phase 2** (package rename): Detect package-declaration renames, update `use`
  and `@ISA` statements (~1-2 PRs)
- **Phase 3** (edge case hardening): Conflict detection, qualified reference
  disambiguation, scope-aware variable rename (~2-3 PRs)
- **Phase 4** (testing): Multi-file test corpus, performance benchmarks (~1 PR)
