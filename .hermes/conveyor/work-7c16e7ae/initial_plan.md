# Initial Plan — work-7c16e7ae

## Issue
GitHub #3431: Missing Perl snippets for modern syntax and frameworks

## Approach

### Scope: Fix both VS Code snippets AND LSP completion snippets

The issue mentions VS Code extension snippets, but investigation revealed TWO independent snippet systems that BOTH have gaps. The fix must update both because these are separate code paths — VS Code native snippets are served by the VS Code extension directly, while LSP snippets are served by the perl-lsp server via the `textDocument/completion` protocol. Updating only one would leave the other incomplete.

1. **`vscode-extension/snippets/perl.json`** — VS Code native snippets (for VS Code users)
2. **`crates/perl-lsp-completion/src/completion/snippets.rs`** — LSP completion snippets (for all LSP clients, including VS Code if using the LSP server)

Additionally, `crates/perl-lexer/src/keywords/mod.rs` needs updating to add modern keywords (`class`, `method`, `field`, `defer`, `given`, `when`, `catch`, `finally`) to `LSP_COMPLETION_KEYWORDS` because the parser already tokenizes these keywords (they're in `LEXER_KEYWORDS`) but the LSP completion layer doesn't surface them — users typing `class <tab>` get no completion suggestion even though the parser understands it.

### Implementation Details

#### A. Add Modern Perl Keywords to `LSP_COMPLETION_KEYWORDS` (crates/perl-lexer/src/keywords/mod.rs)

Add these keywords to `LSP_COMPLETION_KEYWORDS` (must remain sorted):
- `class` — Perl 5.38+ class declaration
- `method` — Perl 5.38+ method within class
- `field` — Perl 5.38+ field declaration
- `defer` — Perl 5.36+ deferred block
- `given` — Perl 5.10+ switch (experimental)
- `when` — Perl 5.10+ case (experimental)
- `catch` — Perl 5.34+ try/catch
- `finally` — Perl 5.34+ try/catch/finally

Note: `state` and `say` are already present in `LSP_COMPLETION_KEYWORDS`.

#### B. Add LSP Snippets (crates/perl-lsp-completion/src/completion/snippets.rs)

Add new `Snippet` entries to the `SNIPPETS` const array:

**Modern Perl 5.38+ Class:**
- `perlclass` — `class ${1:Name} { ... }`
- `perlmethod` — `method ${1:name}($self, $arg) { ... }`
- `perlfield` — `field $${1:name} :param;`

**Perl 5.36+ Defer:**
- `deferblock` — `defer { $0 };`

**Perl 5.10+ Given/When:**
- `givenwhen` — `given ($1) { when ($2) { $0 } default { } }`

**Try/Catch (5.34+):**
- `trycatch` — `try { $1 } catch ($e) { $0 }`
- `tryfinally` — `try { $1 } finally { $0 }`

**Moo/Moose Method Modifiers:**
- `haslazy` — `has '${1:attr}' => ( is => 'rw', lazy => 1, builder => '_build_${1:attr}' );`
- `hasbuilder` — `_build_${1:attr} method`
- `aroundmod` — `around ${1:method} { my ($self, @args) = @_; $0 }`
- `beforemod` — `before ${1:method} => sub { $0 };`
- `aftermof` — `after ${1:method} => sub { $0 };`

**Role Composition:**
- `withrole` — `with '${1:Role::Name}' => sub { $0 };`

**Test::More Extensions:**
- `tmskip` — `SKIP: { skip("${1:reason}", ${2:1}); $0 }`
- `tmtodo` — `TODO: { todo_skip("${1:reason}"); $0 }`
- `tmbail` — `BAIL_OUT("${1:reason}");`
- `tmplan` — `plan tests => ${1:n};`
- `tmthrows` — `throws_ok { $1 } qr/${2:pattern}/, '${3:description}';`

**DBI Patterns:**
- `dbiconnect` — `my $dbh = DBI->connect($dsn, $user, $pass, { RaiseError => 1 }) or die $DBI::errstr;`
- `dbitransact` — `eval { $dbh->begin_work; $1; $dbh->commit; } or do { $dbh->rollback; die $@; };`

**State Variable:**
- `statevar` — `state \$${1:var} = ${2:value};`

**Say (already a keyword, add snippet):**
- `say` — `say "${1:text}";`

#### C. Add VS Code Snippets (vscode-extension/snippets/perl.json)

Add corresponding entries to the VS Code JSON. Convert LSP Tabstop syntax to VS Code format (`${1:default}` → `${1:default}` — same).

Same categories as LSP snippets above.

### Task Breakdown

1. **Update `LSP_COMPLETION_KEYWORDS`** in `crates/perl-lexer/src/keywords/mod.rs` to add modern Perl keywords
2. **Update LSP snippets** in `crates/perl-lsp-completion/src/completion/snippets.rs` with ~15-20 new snippets
3. **Update VS Code snippets** in `vscode-extension/snippets/perl.json` with corresponding entries
4. **Run tests**:
   - `cargo test -p perl-lexer` — ensure keyword tests pass
   - `cargo test -p perl-lsp-completion` — ensure snippet tests pass
   - `cargo test -p perl-lsp` — integration tests
5. **Run linting**: `cargo clippy -p perl-lsp-completion -p perl-lexer`

## Risks

1. **Duplicate triggers**: Must ensure new snippet triggers don't conflict with existing ones in either snippet system because LSP snippet tests enforce uniqueness — a duplicate trigger would fail CI. LSP snippet tests already enforce this via `no_duplicate_triggers` test.

2. **Keyword ordering**: `LSP_COMPLETION_KEYWORDS` must remain alphabetically sorted because the keyword list is validated with `assert_sorted_unique`. Adding keywords out of order will fail tests.

3. **VS Code snippet JSON validity**: Invalid JSON will break VS Code extension because VS Code reads this file at startup. Use existing snippet entries as templates to ensure correct structure.

4. **Perl version compatibility**: Some snippets (e.g., `class`, `method`, `field`) require Perl 5.38+ because the native class syntax was introduced in that version. The `doc` field should mention the version requirement to avoid user confusion.

5. **LSP vs VS Code snippet format differences**: Both use `${N:default}` tabstop syntax, so conversion is straightforward, because the Tabstop syntax is standardized in LSP and VS Code. Care must be taken with escape sequences (e.g., `\$` in LSP vs `\$` in VS Code — same).

## Out of Scope

- Parser changes (parsing already supports 5.38+ features)
- Semantic analysis changes
- Adding snippets for other editors (Emacs, Neovim) — those use LSP or their own config
- Adding completion for Moose/Moo `has` option keys (already exists via `is_has_options_key_context`)
- Changing the VS Code extension's `package.json` snippet registration (just updating the JSON file)
