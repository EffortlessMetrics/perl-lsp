# Copilot Instructions

This file provides guidance to GitHub Copilot when working with code in this repository.

**Latest Release**: 0.12.0
**API Stability**: See [docs/reference/STABILITY.md](docs/reference/STABILITY.md)
**Metrics**: See [docs/project/CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) for computed status

## Quick Reference

```bash
# Canonical local gate (REQUIRED before push)
nix develop -c just ci-gate

# Build and run LSP server
cargo build -p perllsp --release
./target/release/perllsp --stdio

# Run all tests
cargo test --workspace --lib
```

## Crate Structure

The workspace contains 80+ crates organized in tiers. Key crates:

| Crate | Path | Purpose |
|-------|------|---------|
| **perl-parser** | `/crates/perl-parser/` | Main parser library (v3 recursive descent) |
| **perl-lsp** | `/crates/perl-lsp/` | Standalone LSP server binary |
| **perl-dap** | `/crates/perl-dap/` | Debug Adapter Protocol (bridge mode) |
| **perl-lexer** | `/crates/perl-lexer/` | Context-aware tokenizer |
| **perl-parser-core** | `/crates/perl-parser-core/` | Core parsing infrastructure |
| **perl-workspace-index** | `/crates/perl-workspace-index/` | Workspace symbol indexing |
| **perl-semantic-analyzer** | `/crates/perl-semantic-analyzer/` | Semantic analysis |
| **perl-corpus** | `/crates/perl-corpus/` | Test corpus |
| **perl-parser-pest** | `/crates/perl-parser-pest/` | Legacy Pest parser |

### Crate Families

| Family | Count | Purpose |
|--------|-------|---------|
| `perl-module-*` | 13 | Module resolution microcrates |
| `perl-lsp-*` | 21 | LSP feature providers |
| `perl-lsp-feature-*` | 7 | Feature governance subsystem (subset of `perl-lsp-*`) |
| `perl-dap-*` | 4 | Debug adapter components |
| `perl-ts-*` | 5 | Tree-sitter integration |
| `perl-workspace-*` | 4 | Workspace discovery and indexing |
| Core leaf crates | ~30 | Token, AST, quote, regex, heredoc, error, etc. |

## Essential Commands

### Build

```bash
cargo build -p perllsp --release      # LSP server
cargo build -p perl-parser --release  # Parser library
cargo install --path crates/perllsp   # Install from source
```

### Test

```bash
cargo test                            # All tests
cargo test -p perl-parser             # Parser tests
cargo test -p perl-lsp-rs                # LSP tests
cargo test test_name                  # Run single test by name
cargo test -p perl-parser -- test_name --exact  # Run exact test in crate

# LSP tests with threading constraints
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# Semantic definition tests (resource-efficient mode)
just ci-lsp-def
```

### Benchmarks and Fuzzing

```bash
just benchmarks                       # Run all benchmarks
cargo bench -p perl-parser            # Parser benchmarks
just fuzz-bounded                     # Bounded fuzz run (60s per target)
just mutation-subset                  # Mutation testing subset
```

### Dead Code Detection

```bash
just dead-code                        # Full dead code report
just dead-code-report                 # Generate JSON report
just dead-code-strict                 # Run in strict mode (fail on any dead code)
cargo machete                         # Check unused dependencies (fast)
```

### Lint and Format

```bash
cargo fmt --all                       # Format code
cargo clippy --workspace              # Lint all crates
cargo clippy --workspace --lib        # Lint libraries only (faster)
```

### Health and Status

```bash
just health                           # Show codebase metrics
just status-check                     # Verify computed metrics are current
bash scripts/ignored-test-count.sh    # Show ignored test counts
just debt-report                      # Show technical debt status
just debt-check                       # Verify debt budget compliance
```

### Supply Chain Security

```bash
just sbom                             # Generate SBOM (both formats)
just sbom-spdx                        # Generate SBOM in SPDX format
just sbom-cyclonedx                   # Generate SBOM in CycloneDX format
just sbom-verify                      # Verify SBOM generation
just security-audit                   # Run security audit (cargo-audit)

# Verify release artifact provenance
gh attestation verify <artifact> --owner EffortlessMetrics
```

### Code Coverage

```bash
just coverage                         # Generate HTML coverage report locally
just coverage-summary                 # Show coverage summary in terminal
just coverage-lcov                    # Generate lcov.info for CI
```

### SemVer Checking

```bash
just semver-check                     # Check all published packages
just semver-check-package <name>      # Check specific package
just semver-diff <name>               # Show API diff for package
```

## Development Workflow

**Local-first development** - all gates run locally before CI:

```bash
# Install pre-push hook
bash scripts/install-githooks.sh

# Run gate before pushing (format, clippy, tests, policy)
nix develop -c just ci-gate
```

CI is optional/opt-in. The repo is local-first by design.

### CI Gate Tiers

| Tier | Command | Time | When to Use |
|------|---------|------|-------------|
| **A (PR-fast)** | `just pr-fast` | ~1-2 min | Quick iteration during development |
| **B (Merge gate)** | `just ci-gate` | ~3-5 min | Before pushing (required) |
| **C (Nightly)** | `just ci-full` | ~15-30 min | Mutation testing, fuzzing, benchmarks |

## Parser Versions

- **v3 (Native)**: Current - recursive descent parser
- **v2 (Pest)**: Legacy - kept out of default gate
- **v1 (C-based)**: Benchmarking only

## Workspace Exclusions

These directories are excluded from the default workspace (require special builds):
- `tree-sitter-perl-c/` - Requires libclang
- `fuzz/` - Specialized fuzz testing build
- `archive/` - Legacy components

## Key Paths

| What | Where |
|------|-------|
| Parser source | `crates/perl-parser/src/` |
| LSP providers | `crates/perl-lsp-*/src/` |
| LSP server binary | `crates/perl-lsp/src/` |
| DAP server | `crates/perl-dap/src/` |
| Tests | `crates/*/tests/` |
| Test corpus | `test_corpus/`, `tree-sitter-perl/test/corpus/` |
| Fuzz targets | `fuzz/fuzz_targets/` |
| VSCode extension | `vscode-extension/` |
| Documentation | `docs/` |
| Features catalog | `features.toml` |
| CI gate policy | `.ci/gate-policy.yaml` |
| Technical debt ledger | `.ci/debt-ledger.yaml` |
| Dependabot config | `.github/dependabot.yml` |
| Supply chain security | `deny.toml`, `docs/reference/SUPPLY_CHAIN_SECURITY.md` |
| Build tooling | `xtask/` |

## Architecture Patterns

### Dual Indexing (PR #122)

When implementing workspace indexing, index under both qualified and bare forms:

```rust
// Index under bare name
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());

// Index under qualified name
file_index.references.entry(qualified).or_default().push(symbol_ref);
```

### Threading Configuration

LSP tests use adaptive threading. Key environment variables:

```bash
RUST_TEST_THREADS=2     # Limit test parallelism
CARGO_BUILD_JOBS=1      # Limit build parallelism
RUSTC_WRAPPER=""        # Disable rustc wrapper
```

### Crate Dependency Tiers

The workspace uses a tiered dependency structure (see `Cargo.toml`):
- **Tier 1** (~30): Leaf crates with no internal dependencies (`perl-token`, `perl-quote`, `perl-ast`, `perl-module-token-core`, `perl-lsp-feature-ids`, etc.)
- **Tier 2** (~15): Single-level dependencies (`perl-parser-core`, `perl-lsp-transport`, `perl-tokenizer`, `perl-module-token`, `perl-module-name`, etc.)
- **Tier 3** (~15): Two-level dependencies (`perl-workspace-index`, `perl-refactoring`, `perl-module-resolution`, `perl-lsp-feature-governance`, etc.)
- **Tier 4** (~10): Three-level dependencies (`perl-semantic-analyzer`, `perl-lsp-providers`, `perl-lsp-navigation`, etc.)
- **Tier 5** (1): Task runner crates (`xtask`)
- **Tier 6** (3): Application/executable crates (`perl-parser`, `perl-lsp`, `perl-dap`)
- **Tier 7** (~8): Legacy/testing crates (`perl-parser-pest`, `perl-corpus`, `tree-sitter-perl-*`)

## Documentation

- **[CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md)** - Computed metrics and project health
- **[ROADMAP.md](docs/project/ROADMAP.md)** - Milestones and release planning
- **[COMMANDS_REFERENCE.md](docs/reference/COMMANDS_REFERENCE.md)** - Full command catalog
- **[LSP_IMPLEMENTATION_GUIDE.md](docs/reference/LSP_IMPLEMENTATION_GUIDE.md)** - Server architecture
- **[DEBT_TRACKING.md](docs/explanation/DEBT_TRACKING.md)** - Technical debt and flaky test tracking
- **[DEPENDENCY_MANAGEMENT.md](docs/how-to/DEPENDENCY_MANAGEMENT.md)** - Automated dependency updates with Dependabot
- **[DEPENDENCY_QUICK_REFERENCE.md](docs/how-to/DEPENDENCY_QUICK_REFERENCE.md)** - Quick commands for dependency management
- **[features.toml](features.toml)** - Canonical LSP capability definitions

## Truth Sources

Metrics in this project are **computed, not hand-edited**:
- `CURRENT_STATUS.md` - Auto-generated via `scripts/update-current-status.py`
- `features.toml` - Canonical LSP capability definitions
- Test output and CI receipts are the evidence for all claims

## Coding Standards

- Run `cargo clippy --workspace` before committing
- Use `cargo fmt` for consistent formatting
- **No fatal constructs in production code** - the following are banned:
  - `unwrap()`, `expect()` - use `?`, `.ok_or_else()`, or pattern matching
  - `panic!()`, `todo!()`, `unimplemented!()` - return `Result`/`Option`
  - `std::process::abort()` - never use, not even in binaries
  - `std::process::exit()` - allowed **only** in `bin/` directories and `lifecycle.rs`
  - `dbg!()` - use `tracing::debug!` instead
  - **Exception**: One centralized `#[allow(clippy::expect_used)]` for `lsp_types::Uri` fallback (see `crates/perl-lsp/src/util/uri.rs`)
  - In tests: use `Result<()>` return types, or `perl_tdd_support::must`/`must_some` helpers
- **Regex init**: Use `Option<Regex>` with `.ok()` for graceful degradation
- **Non-empty collections**: Use fixed-size arrays (`[T; N]`) for compile-time guarantees
- Prefer `.first()` over `.get(0)`
- Use `.push(char)` instead of `.push_str("x")` for single chars
- Use `or_default()` instead of `or_insert_with(Vec::new)`
- Avoid unnecessary `.clone()` on Copy types

## Contributing

1. Run `nix develop -c just ci-gate` before pushing
2. Check issues for "good first issue" labels
3. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines

| Area | Path |
|------|------|
| Parser improvements | `/crates/perl-parser/src/` |
| LSP features | `/crates/perl-lsp-*/src/` |
| CLI enhancements | `/crates/perl-lsp/src/` |
| DAP features | `/crates/perl-dap/src/` |
| Tests | `/crates/*/tests/` |

---

## Perl Language Reference

This section helps AI assistants understand the Perl code that perl-lsp parses and serves. The parser
handles Perl 5.8 through 5.40.

### Variable types and sigils

```perl
my $scalar  = "string";          # Scalar: holds a single string, number, or reference
my @array   = (1, 2, 3);         # Array: ordered list of scalars
my %hash    = (key => "value");   # Hash: unordered key-value pairs
my $ref     = \$scalar;           # Reference: scalar ref, array ref, hash ref, code ref
my $aref    = [1, 2, 3];          # Anonymous array ref (common for passing arrays)
my $href    = { key => "val" };   # Anonymous hash ref (common for passing hashes)
my $cref    = sub { ... };        # Anonymous subroutine (code ref)
my *glob    = *Other::symbol;     # Typeglob: aliasing entire symbol table entries
```

Special variables in common use:

| Variable | Meaning |
|----------|---------|
| `$_`     | Default topic variable (loop iteration, `map`, `grep`, `print`) |
| `@_`     | Subroutine argument list |
| `$!`     | `errno`: system call error string or number |
| `$@`     | `eval` error: set after a failed `eval` block |
| `$0`     | Program name |
| `$/`     | Input record separator (default: `"\n"`) |
| `$\`     | Output record separator (appended by `print`) |
| `$,`     | Output field separator (between `print` args) |
| `$;`     | Multidimensional hash key separator |
| `$?`     | Child process status |
| `$.`     | Current line number of last filehandle read |
| `@ISA`   | Inheritance list for a package |
| `@EXPORT_OK` | Symbols a module exports on request |

### Subroutine styles

```perl
# Traditional: arguments arrive in @_
sub greet {
    my ($name, $greeting) = @_;
    return "$greeting, $name";
}

# Modern signatures (v5.20+, stable in v5.36)
use feature 'signatures';
no warnings 'experimental::signatures';
sub greet($name, $greeting = "Hello") {
    return "$greeting, $name";
}

# Slurpy: remaining args become an array or hash
sub log_event($level, @messages) { ... }
sub configure(%opts) { ... }

# Method: first arg is the invocant (object or class name)
sub new {
    my ($class, %args) = @_;
    return bless { %args }, $class;
}
sub name { my ($self) = @_; return $self->{name} }
```

### Object-oriented patterns

#### Traditional bless OO

```perl
package Animal;
use strict;
use warnings;

sub new {
    my ($class, %args) = @_;
    return bless {
        name  => $args{name}  // "Unknown",
        sound => $args{sound} // "...",
    }, $class;
}

sub name  { $_[0]->{name}  }        # Read-only accessor
sub sound { $_[0]->{sound} }

sub speak {
    my ($self) = @_;
    printf "%s says %s\n", $self->name, $self->sound;
}

1;
```

#### Moose / Moo (most common in CPAN code)

```perl
package Cat;
use Moose;                       # or: use Moo;

extends 'Animal';                # Inheritance
with 'Role::Printable';          # Role composition

has 'name'  => (is => 'ro',  isa => 'Str', required => 1);
has 'lives' => (is => 'rw',  isa => 'Int', default => 9);
has 'owner' => (is => 'ro',  isa => 'Maybe[Str]');
has 'toys'  => (is => 'ro',  isa => 'ArrayRef[Str]', default => sub { [] });

# Method modifiers
before 'speak' => sub { print "Preparing to speak...\n" };
after  'speak' => sub { print "Done speaking.\n" };
around 'speak' => sub {
    my ($orig, $self, @args) = @_;
    $self->$orig(@args);
};

no Moose;
__PACKAGE__->meta->make_immutable;
1;
```

Moo differences from Moose:
- No type system by default (add `use Types::Standard` for `Str`, `Int`, etc.)
- No `before`/`after`/`around` without `Moo::Role` or `Role::Tiny`
- `BUILD` runs after `new`; `BUILDARGS` transforms constructor arguments
- Lighter weight, no meta-object protocol

#### Modern Perl 5.38+ class syntax

```perl
use feature 'class';

class Point {
    field $x :param = 0;
    field $y :param = 0;

    method x { $x }
    method y { $y }

    method distance_to($other) {
        sqrt(($x - $other->x)**2 + ($y - $other->y)**2)
    }
}

class ColorPoint :isa(Point) {
    field $color :param = 'black';
    method color { $color }
}
```

### Module structure conventions

```perl
# File path: lib/Foo/Bar.pm  =>  package name: Foo::Bar
package Foo::Bar;

use strict;
use warnings;

# Optional version declaration
our $VERSION = '1.00';

# Exports (choose one style)
use Exporter 'import';
our @EXPORT    = qw(always_exported);   # Exported by default
our @EXPORT_OK = qw(opt_exported);      # Exported on request
our %EXPORT_TAGS = (
    all => [@EXPORT, @EXPORT_OK],
);

sub always_exported { ... }
sub opt_exported    { ... }
sub _private_helper { ... }   # Convention: leading _ = private

# Every module must return a true value
1;
```

### Error handling patterns

```perl
# die / eval (core)
eval {
    open my $fh, '<', $file or die "Cannot open $file: $!";
    process($fh);
};
if ($@) {
    warn "Caught error: $@";
}

# Carp (preferred in library code: errors blame the caller)
use Carp qw(croak confess carp cluck);

sub validate_arg {
    my ($arg) = @_;
    croak "validate_arg: argument must be defined"  unless defined $arg;
    carp  "validate_arg: argument looks suspicious" if $arg eq '';
}

# croak  = die  as seen from caller (use in libraries)
# confess = die  with full stack trace
# carp   = warn as seen from caller
# cluck  = warn with full stack trace

# Try::Tiny / Feature::Compat::Try (modern exception objects)
use Try::Tiny;
try {
    do_something_risky();
} catch {
    if (ref $_ && $_->isa('My::Exception')) {
        handle_my_exception($_);
    } else {
        die $_;   # Re-throw unknown exceptions
    }
} finally {
    cleanup();
};
```

### Common idioms

```perl
# Defined-or (v5.10+): prefer over || when 0 or "" are valid values
my $val = $opts{timeout} // 30;

# Chained method calls
my $result = $obj->set_name("Alice")->set_age(30)->save();

# Dereference styles
my @copy  = @{$aref};        # Array deref
my %copy  = %{$href};        # Hash deref
my $elem  = $aref->[0];      # Arrow notation (preferred)
my $val   = $href->{key};
my $ret   = $cref->(1, 2);   # Code ref call

# Context: what you're assigning into changes what expressions return
my $count = @array;          # Scalar context: array length
my ($first, @rest) = @array; # List context: split assignment
scalar @array;               # Force scalar context explicitly
wantarray() ? @list : $count; # Detect caller's context

# grep / map (functional list processing)
my @evens  = grep { $_ % 2 == 0 } @numbers;
my @doubled = map  { $_ * 2 }     @numbers;
my %by_id  = map  { $_->id => $_ } @objects;

# sort with comparison function
my @sorted_num = sort { $a <=> $b } @numbers;      # Numeric ascending
my @sorted_str = sort { $a cmp $b } @strings;      # String ascending
my @sorted_obj = sort { $a->name cmp $b->name } @objects;
```

### Regular expressions

```perl
# Match operator: m//  (m is optional when using / delimiter)
if ($str =~ /(\w+)\s+(\d+)/) {
    my ($word, $num) = ($1, $2);
}

# Named captures (v5.10+)
if ($str =~ /(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})/) {
    printf "%s/%s/%s\n", $+{year}, $+{month}, $+{day};
}

# Substitution: s///
(my $clean = $str) =~ s/^\s+|\s+$//g;   # trim (non-destructive copy)
$str =~ s/foo/bar/gi;                    # global, case-insensitive

# Common modifiers
# /i  case-insensitive
# /g  global (all matches)
# /m  multiline (^ and $ match line boundaries)
# /s  single-line (. matches \n)
# /x  extended (whitespace and # comments ignored)
# /e  evaluate replacement as code
# /r  non-destructive (return modified copy, v5.14+)

$str =~ s/(\d+)/$1 * 2/ge;   # Double all numbers via /e

# Split and join
my @fields = split /,/, $csv_line;
my @words  = split ' ', $sentence;    # Split on any whitespace, trim leading
my $line   = join(",", @fields);
```

### File I/O

```perl
# Three-argument open (always use this form)
open my $fh, '<',    $file  or die "Cannot read $file: $!";
open my $fh, '>',    $file  or die "Cannot write $file: $!";
open my $fh, '>>',   $file  or die "Cannot append $file: $!";
open my $fh, '<:utf8', $file or die "Cannot read $file: $!";  # with encoding

# Read modes
my $line  = <$fh>;                # Read one line
chomp $line;
chomp(my @lines = <$fh>);         # Slurp all lines, strip newlines
my $text  = do { local $/; <$fh> }; # Slurp whole file into scalar

# Always close or use a block/scope
close $fh or warn "close failed: $!";
```

### Testing conventions (Test::More / Test2)

```perl
use strict;
use warnings;
use Test::More;               # or: use Test2::V0;

# Basic assertions
ok($value,              "value is true");
is($got, $expected,     "values match");
isnt($got, $bad,        "values differ");
like($str, qr/pattern/, "string matches pattern");
unlike($str, qr/nope/,  "string does not match");
cmp_ok($got, '>=', $min, "value in range");
is_deeply($got_ref, $exp_ref, "deep structure matches");

# Test::Exception
use Test::Exception;
throws_ok { risky_call() } qr/expected error/, "dies with right message";
lives_ok  { safe_call()  }                     "does not die";

# Always end with done_testing or a plan
done_testing();          # dynamic
# plan tests => 5;       # static alternative
```

### Perl version feature markers

```perl
use v5.10;   # say, //, given/when (deprecated), named captures, state vars
use v5.14;   # Non-destructive /r modifier, each on arrays
use v5.18;   # Hash randomization enabled by default
use v5.20;   # Subroutine signatures (experimental), hash/array slices
use v5.22;   # Double-diamond <<>> operator, hex/oct in regex
use v5.26;   # Indented heredocs <<~, /xx modifier, no more $a/$b warnings in sort
use v5.28;   # Unicode 10, bitwise string operators
use v5.32;   # Chained comparisons (1 < $x < 10), isa operator
use v5.34;   # try/catch (experimental), defer (experimental)
use v5.36;   # Signatures stable (no warning needed), strict+warnings implied by use v5.36
use v5.38;   # class/method/field syntax (experimental), bareword filehandles warned
use v5.40;   # class/field/method more stable, :param attribute
```

`use v5.36` and above automatically enable `use strict` and `use warnings` -- you do not need to add them separately.

### Common CPAN modules (frequently seen in user code)

| Module | Common exports / methods |
|--------|--------------------------|
| `Scalar::Util` | `blessed($ref)`, `reftype($ref)`, `looks_like_number($s)`, `weaken($ref)` |
| `List::Util` | `first { } @list`, `max(@list)`, `min(@list)`, `sum(@list)`, `sum0`, `any { }`, `all { }`, `none { }`, `reduce { $a op $b } @list` |
| `List::MoreUtils` | `uniq(@list)`, `zip(\@a, \@b)`, `mesh`, `each_array` |
| `Carp` | `croak`, `confess`, `carp`, `cluck` |
| `POSIX` | `floor($n)`, `ceil($n)`, `strftime($fmt, localtime)` |
| `File::Basename` | `basename($path)`, `dirname($path)`, `fileparse($path, @suffixes)` |
| `File::Path` | `make_path($dir)`, `remove_tree($dir)` |
| `File::Spec` | `File::Spec->catfile(@parts)`, `->rel2abs($path)`, `->splitpath($path)` |
| `File::Temp` | `tempfile()`, `tempdir()` |
| `Storable` | `dclone($ref)` for deep copy |
| `Data::Dumper` | `Dumper($data)` for debug inspection |
| `JSON` / `JSON::XS` / `JSON::PP` | `encode_json($ref)`, `decode_json($str)` |
| `YAML` / `YAML::XS` | `Dump($data)`, `Load($str)`, `DumpFile($path, $data)`, `LoadFile($path)` |
| `DBI` | `DBI->connect($dsn, $user, $pass)`, `$dbh->prepare($sql)`, `$sth->execute(@bind)`, `$sth->fetchrow_hashref` |
| `Moose` | `has`, `extends`, `with`, `before`/`after`/`around`, `__PACKAGE__->meta->make_immutable` |
| `Moo` | `has`, `extends`, `with` (lighter weight, no MOP) |
| `Mouse` | Drop-in Moose subset, faster loading |
| `Exporter` | `@EXPORT`, `@EXPORT_OK`, `%EXPORT_TAGS`, `import` |
| `Getopt::Long` | `GetOptions(\%opts, "verbose!", "file=s", "count=i")` |
| `Pod::Usage` | `pod2usage(2)` for usage messages from POD |

### What the LSP parser specifically handles

When writing parser tests or fixing parser bugs, note these features:

- **Heredocs**: `<<EOF`, `<<~EOF` (indented), `<<"EOF"` (interpolating), `<<'EOF'` (literal)
- **Quoting**: `q{}`, `qq{}`, `qw()`, `qr{}`, `qx{}` with any paired or repeated delimiter
- **Regex**: `m//`, `s///`, `tr///`, `y///` with arbitrary delimiters and all modifiers
- **Context-sensitive parsing**: `print LIST`, `say LIST`, `push @arr, LIST` vs `grep BLOCK LIST`
- **Format/write**: `format NAME =` ... `write NAME`
- **`BEGIN`/`END`/`CHECK`/`INIT`/`UNITCHECK`** phase blocks
- **`do FILE`**, **`require`**, **`use`** with version and import list
- **Indirect object syntax**: `new Foo @args` (legacy, still common)
- **`local`**, **`our`**, **`state`** variable declarations
- **`AUTOLOAD`**, **`DESTROY`**, **`UNIVERSAL`** special methods
- **Prototypes**: `sub foo ($$@) { }` and signatures `sub foo($a, $b) { }`
- **Attributes**: `my $x :shared = 1`, `sub foo :lvalue { }`

### Naming conventions in Perl code

| Kind | Convention | Example |
|------|------------|---------|
| Package / class | `TitleCase` with `::` separators | `MyApp::Model::User` |
| Subroutine / method | `snake_case` | `get_user`, `save_record` |
| Constant | `UPPER_SNAKE_CASE` | `MAX_RETRIES`, `DEFAULT_PORT` |
| Private method (convention) | Leading underscore | `_validate`, `_build_cache` |
| Loop variable | `$_` (default) or descriptive | `for my $item (@items)` |
| Accessor from Moose/Moo | Matches `has` attribute name | `has 'name' => ...` -> `$obj->name` |
| Class method | Called on package name | `My::Class->new(...)` |
| Instance method | Called on object | `$obj->method(...)` |
