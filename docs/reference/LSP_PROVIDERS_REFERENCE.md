# LSP Providers Reference
<!-- Labels: docs:reference, lsp:providers, parser:comprehensive-improvements -->

**Reference Documentation** - Complete technical specifications for Perl LSP providers

This document provides comprehensive API reference for the seven LSP provider modules that deliver enhanced editor integration features for Perl development. Each provider integrates with the Parse → Index → Navigate → Complete → Analyze workflow to provide specialized functionality.

## Table of Contents

- [Document Links Provider](#document-links-provider)
- [Code Lens Provider](#code-lens-provider)
- [Inlay Hints Provider](#inlay-hints-provider)
- [Document Highlights Provider](#document-highlights-provider)
- [Folding Ranges Provider](#folding-ranges-provider)
- [Type Definition Provider](#type-definition-provider)
- [Implementation Provider](#implementation-provider)

---

## Document Links Provider

**Module**: `crates/perl-parser/src/document_links.rs`
**LSP Method**: `textDocument/documentLink`
**LSP Specification**: [LSP 3.17 Document Link](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentLink)

### Purpose

Provides clickable links in Perl source code for module imports and file requirements, enabling quick navigation to module definitions or CPAN documentation. Automatically resolves local module paths and creates MetaCPAN links for external dependencies.

### LSP Workflow Integration

- **Parse**: Scans source text for `use` and `require` statements (line-based parsing)
- **Index**: Resolves module paths against workspace roots and common Perl library directories
- **Navigate**: Creates URI links to local files or MetaCPAN documentation
- **Complete**: No direct integration
- **Analyze**: No direct integration

### Configuration

The provider requires workspace root URLs for proper module path resolution:

```rust
use perl_parser::document_links::compute_links;
use url::Url;

let workspace_roots = vec![
    Url::parse("file:///workspace/project").unwrap(),
];
```

### Key Methods

#### `compute_links`

```rust
pub fn compute_links(uri: &str, text: &str, roots: &[Url]) -> Vec<Value>
```

Computes document links for a given Perl document.

**Arguments**:
- `uri`: The URI of the document being processed
- `text`: The content of the document
- `roots`: A slice of workspace root URLs to resolve modules against

**Returns**: Vector of `serde_json::Value` objects representing LSP DocumentLink structures

**Algorithm**:
1. Scans text line-by-line for `use` and `require` statements
2. Extracts module names (e.g., `Foo::Bar` from `use Foo::Bar;`)
3. Filters out core pragmas (`strict`, `warnings`, `utf8`, etc.)
4. Attempts local resolution through `lib/`, `blib/lib/`, workspace root
5. Falls back to MetaCPAN link for unresolved modules (`https://metacpan.org/pod/{module}`)

### Supported Patterns

The provider recognizes these Perl import patterns:

```perl
# Module imports - creates links to local files or MetaCPAN
use Data::Dumper;
use MyApp::Module;

# File requirements - resolves relative to workspace roots
require "lib/helper.pl";
require 'config/settings.pm';
```

### Excluded Patterns

Core Perl pragmas are excluded from linking (30 pragmas total):

```perl
# No links created for these
use strict;
use warnings;
use feature;
use parent;
use base;
# ... (see is_pragma function for complete list)
```

### Example Usage

```rust
use perl_parser::document_links::compute_links;
use url::Url;

let source = r#"
use Data::Dumper;
require JSON::XS;
use MyApp::Controller;
"#;

let uri = "file:///project/script.pl";
let roots = vec![Url::parse("file:///project").unwrap()];

let links = compute_links(uri, source, &roots);

// Links contain JSON-RPC formatted DocumentLink objects:
// {
//   "range": {"start": {"line": 1, "character": 4}, "end": {"line": 1, "character": 16}},
//   "target": "https://metacpan.org/pod/Data::Dumper",
//   "tooltip": "Open Data::Dumper"
// }
```

### Performance Characteristics

- **Parsing Speed**: Line-based scanning, <100μs for typical files
- **Memory Footprint**: Minimal - no AST required, operates on text
- **Workspace Resolution**: First-match algorithm for module paths

### Test Coverage

**Test File**: `crates/perl-lsp/tests/lsp_document_links_test.rs`

**Coverage**:
- Basic URL handling (Windows/Unix paths)
- Relative path resolution
- Module link creation

**Known Limitations**:
- Test coverage is basic (URL validation only)
- No integration tests for full LSP workflow
- Module resolution logic not directly testable from external tests

---

## Code Lens Provider

**Module**: `crates/perl-parser/src/code_lens_provider.rs`
**LSP Method**: `textDocument/codeLens`, `codeLens/resolve`
**LSP Specification**: [LSP 3.17 Code Lens](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_codeLens)

### Purpose

Displays inline actionable information above code elements such as reference counts for subroutines/packages and "Run Test" buttons for test functions. Supports two-phase resolution: initial lens extraction and lazy reference counting.

### LSP Workflow Integration

- **Parse**: AST traversal to identify subroutines, packages, test functions
- **Index**: Reference counting via workspace index for "X references" lenses
- **Navigate**: Provides commands for finding references and running tests
- **Complete**: No direct integration
- **Analyze**: Test function detection uses naming convention heuristics

### Configuration

No external configuration required. Integrates with workspace index for reference counting:

```rust
use perl_parser::code_lens_provider::{CodeLensProvider, resolve_code_lens};

let provider = CodeLensProvider::new(source.to_string());
```

### Key Types

#### `CodeLens`

```rust
pub struct CodeLens {
    pub range: Range,                    // LSP range to display lens
    pub command: Option<Command>,        // Executable command (resolved)
    pub data: Option<Value>,             // Data for lazy resolution
}
```

#### `Command`

```rust
pub struct Command {
    pub title: String,                   // Display text
    pub command: String,                 // Command identifier
    pub arguments: Option<Vec<Value>>,   // Command arguments
}
```

### Key Methods

#### `CodeLensProvider::new`

```rust
pub fn new(source: String) -> Self
```

Creates a new code lens provider with source text for position calculations.

#### `CodeLensProvider::extract`

```rust
pub fn extract(&self, ast: &Node) -> Vec<CodeLens>
```

Extracts code lenses from an AST through recursive traversal.

**Supported Node Types**:
- `NodeKind::Subroutine`: Adds "Run Test" lens for test functions, "X references" lens for all subs
- `NodeKind::Package`: Adds "X references" lens

**Test Function Detection Patterns**:
- `test_*` - Standard test prefix
- `*_test` - Alternative test suffix
- `t_*` - Short test prefix
- `test` - Standalone test function
- `ok_*`, `is_*`, `like_*`, `can_*` - Test::More style

#### `resolve_code_lens`

```rust
pub fn resolve_code_lens(lens: CodeLens, reference_count: usize) -> CodeLens
```

Resolves a code lens by adding reference count command.

**Algorithm**:
1. Checks if lens has `data` but no `command` (unresolved references lens)
2. Extracts symbol name from `data`
3. Creates command with formatted reference count
4. Returns resolved lens with `editor.action.findReferences` command

#### `get_shebang_lens`

```rust
pub fn get_shebang_lens(source: &str) -> Option<CodeLens>
```

Detects shebang line and returns "Run Script" lens if present.

**Detection Pattern**: `#!/...perl...` at start of file

### Example Usage

```rust
use perl_parser::{Parser, code_lens_provider::{CodeLensProvider, resolve_code_lens}};

let source = r#"#!/usr/bin/perl
package MyApp;

sub test_basic {
    ok(1, "test passes");
}

sub helper {
    return 42;
}
"#;

let mut parser = Parser::new(source);
let ast = parser.parse().unwrap();

let provider = CodeLensProvider::new(source.to_string());
let mut lenses = provider.extract(&ast);

// Add shebang lens
if let Some(shebang) = get_shebang_lens(source) {
    lenses.push(shebang);
}

// Resolve reference lenses (typically done by LSP server)
for lens in &mut lenses {
    if lens.command.is_none() {
        *lens = resolve_code_lens(lens.clone(), 3); // 3 references found
    }
}

// Lenses now contain:
// - "▶ Run Script" at line 0 (shebang)
// - "▶ Run Test" above test_basic
// - "3 references" for test_basic
// - "3 references" for helper
// - "3 references" for MyApp package
```

### Performance Characteristics

- **AST Traversal**: Single-pass recursive descent, <1ms for typical files
- **Position Calculation**: UTF-8 byte offset to line/column conversion, ~1μs per position
- **Memory Usage**: ~100 bytes per lens (typical file has 5-20 lenses)

### Test Coverage

**Test File**: `crates/perl-lsp/tests/lsp_code_lens_reference_test.rs`

**Coverage**:
- Code lens extraction for subroutines (test and non-test)
- Shebang detection and "Run Script" lens
- Reference counting resolution
- Integration with LSP server protocol

**Test Scenarios**:
- Multiple function calls with reference counting
- Unused functions (0 references)
- Test function naming patterns
- Shebang presence/absence

---

## Inlay Hints Provider

**Module**: `crates/perl-parser/src/inlay_hints_provider.rs`
**LSP Method**: `textDocument/inlayHint`
**LSP Specification**: [LSP 3.17 Inlay Hint](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_inlayHint)

### Purpose

Displays inline type information and parameter names to improve code readability without cluttering the source. Shows parameter labels for function calls, inferred types for variables, and intermediate types for method chains.

### LSP Workflow Integration

- **Parse**: AST traversal to identify function calls, variable declarations, method chains
- **Index**: Uses builtin signatures from `builtin_signatures_phf` for parameter names
- **Navigate**: No direct integration
- **Complete**: No direct integration
- **Analyze**: Type inference for Perl expressions (arrays, hashes, function return types)

### Configuration

#### `InlayHintConfig`

```rust
pub struct InlayHintConfig {
    pub parameter_hints: bool,    // Enable/disable parameter name hints
    pub type_hints: bool,          // Enable/disable type annotation hints
    pub chained_hints: bool,       // Enable/disable chained method type hints
    pub max_length: usize,         // Maximum hint label length (default: 30)
}

impl Default for InlayHintConfig {
    fn default() -> Self {
        Self {
            parameter_hints: true,
            type_hints: true,
            chained_hints: true,
            max_length: 30,
        }
    }
}
```

### Key Types

#### `InlayHint`

```rust
pub struct InlayHint {
    pub position: Position,         // Where to display the hint
    pub label: String,              // Hint text
    pub kind: InlayHintKind,        // Type or Parameter
    pub tooltip: Option<String>,    // Additional information
    pub padding_left: bool,         // Add space before hint
    pub padding_right: bool,        // Add space after hint
}
```

#### `InlayHintKind`

```rust
pub enum InlayHintKind {
    Type = 1,       // Type annotation
    Parameter = 2,  // Parameter name
}
```

### Key Methods

#### `InlayHintsProvider::new`

```rust
pub fn new(source: String) -> Self
```

Creates provider with default configuration.

#### `InlayHintsProvider::with_config`

```rust
pub fn with_config(source: String, config: InlayHintConfig) -> Self
```

Creates provider with custom configuration.

#### `InlayHintsProvider::extract`

```rust
pub fn extract(&self, ast: &Node) -> Vec<InlayHint>
```

Extracts all inlay hints from the AST.

#### `InlayHintsProvider::extract_range`

```rust
pub fn extract_range(&self, ast: &Node, range: Range) -> Vec<InlayHint>
```

Extracts inlay hints within a specific range (performance optimization for visible viewport).

### Supported Patterns

#### Parameter Hints

Shows parameter names for function calls using builtin signatures or common function knowledge:

```perl
# Displays: push(ARRAY: @array, list: "value")
push(@array, "value");

# Displays: substr(string: $str, offset: 0, length: 5, replacement: "new")
substr($str, 0, 5, "new");

# Displays: open(FILEHANDLE: FH, mode: "<", filename: "file.txt")
open(FH, "<", "file.txt");
```

**Smart Filtering**: Skips hints for clear arguments:
- Short string literals (<20 chars, alphanumeric)
- Long descriptive variable names (>5 chars)

#### Type Hints

Shows inferred types for variable declarations:

```perl
# Displays: my $arr: ARRAY = [1, 2, 3];
my $arr = [1, 2, 3];

# Displays: my $hash: HASH = { key => "value" };
my $hash = { key => "value" };

# Displays: my $result: ARRAY = split(/,/, $input);
my $result = split(/,/, $input);
```

**Type Inference Rules**:
- `[]` → `ARRAY`
- `{}` → `HASH`
- `"..."` → `string`
- `42` → `number`
- `qr//` → `Regexp`
- `split`, `keys`, `values`, `grep`, `map` → `ARRAY`
- `new` → `object`

#### Chained Hints

Shows intermediate types in method chains:

```perl
# Displays: $result /* ARRAY */ ->map(...)
my $result = split(/,/, $input)->map(...);
```

### Example Usage

```rust
use perl_parser::{Parser, inlay_hints_provider::{InlayHintsProvider, InlayHintConfig}};

let source = r#"
my $arr = [1, 2, 3];
push(@array, "value");
substr($string, 0, 5, "replacement");
"#;

let mut parser = Parser::new(source);
let ast = parser.parse().unwrap();

// Use default configuration
let provider = InlayHintsProvider::new(source.to_string());
let hints = provider.extract(&ast);

// Or customize configuration
let config = InlayHintConfig {
    parameter_hints: true,
    type_hints: false,  // Disable type hints
    chained_hints: true,
    max_length: 20,
};
let custom_provider = InlayHintsProvider::with_config(source.to_string(), config);
let custom_hints = custom_provider.extract(&ast);

// Convert to JSON for LSP
for hint in hints {
    let json = hint.to_json();
    println!("{}", json);
}
```

### Performance Characteristics

- **AST Traversal**: Single-pass recursive descent
- **Hint Generation**: <100μs per function call/variable declaration
- **Range-Based Extraction**: Optimized for viewport rendering (~50% reduction for visible area)
- **Memory Usage**: ~80 bytes per hint

### Test Coverage

**Test File**: `crates/perl-parser/src/inlay_hints_provider.rs` (internal tests)

**Coverage**:
- Parameter hints for builtin functions (`push`, `substr`, `open`)
- Type hints for arrays, hashes, function calls
- Smart filtering for clear arguments
- Inferred return types

**Test Status**:
- Tests acknowledge new AST structure may not fully support inlay hints yet
- Empty results acceptable during AST migration
- Focus on ensuring no crashes and proper structure

---

## Document Highlights Provider

**Module**: `crates/perl-parser/src/document_highlight.rs`
**LSP Method**: `textDocument/documentHighlight`
**LSP Specification**: [LSP 3.17 Document Highlight](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentHighlight)

### Purpose

Highlights all occurrences of a symbol when the cursor is positioned on it, distinguishing between read and write access. Supports variables (scalars, arrays, hashes), functions, methods, and identifiers with proper scope awareness.

### LSP Workflow Integration

- **Parse**: AST traversal to find all symbol occurrences
- **Index**: No workspace indexing (single-document scope)
- **Navigate**: Provides visual feedback for symbol usage
- **Complete**: No direct integration
- **Analyze**: Write/read access detection through parent node analysis

### Key Types

#### `DocumentHighlight`

```rust
pub struct DocumentHighlight {
    pub location: SourceLocation,       // Byte offset range
    pub kind: DocumentHighlightKind,    // Text, Read, or Write
}
```

#### `DocumentHighlightKind`

```rust
pub enum DocumentHighlightKind {
    Text = 1,   // Regular text occurrence
    Read = 2,   // Read access to symbol
    Write = 3,  // Write access (declaration, assignment, increment)
}
```

### Key Methods

#### `DocumentHighlightProvider::new`

```rust
pub fn new() -> Self
```

Creates a new stateless document highlight provider.

#### `DocumentHighlightProvider::find_highlights`

```rust
pub fn find_highlights(
    &self,
    ast: &Node,
    source: &str,
    byte_offset: usize,
) -> Vec<DocumentHighlight>
```

Finds all highlights for the symbol at the given position.

**Algorithm**:
1. Find the node at the cursor byte offset
2. Extract symbol information (name, sigil, type)
3. Recursively collect all matching symbols in the AST
4. Determine highlight kind (Write/Read) based on parent context
5. Deduplicate by location, preferring Write over Read

**Write Access Detection**:
- Variable declarations (`my $x`, `our @ISA`)
- Left side of assignments (`$x = ...`)
- Increment/decrement operations (`$x++`, `--$x`)

**Read Access**: All other occurrences

### Supported Symbol Types

#### Variables

```perl
my $foo = 42;    # Write
print $foo;      # Read
$foo = $foo + 1; # Write (LHS), Read (RHS)
my $bar = $foo * 2; # Read
```

**Symbol Info**:
- Name: `foo`
- Sigil: `$`
- Highlights: All 5 occurrences with appropriate kinds

#### Functions

```perl
sub calculate {  # Declaration
    return 42;
}

my $result = calculate();  # Call
calculate();               # Call
print calculate();         # Call
```

**Symbol Info**:
- Name: `calculate`
- Sigil: None
- Highlights: All 4 occurrences (declaration + 3 calls)

#### Methods

```perl
sub process { ... }  # Declaration

$obj->process();     # Call
$other->process();   # Call
```

**Symbol Info**:
- Name: `process`
- Sigil: None
- Is Method: true
- Highlights: Only method calls (not function declarations)

### Parent Context Analysis

The provider uses parent node analysis to determine highlight kinds:

```rust
// Declaration context
NodeKind::VariableDeclaration { variable, .. } => Write

// Assignment context
NodeKind::Assignment { lhs, .. } => Write (if match on LHS), Read (otherwise)

// Increment/decrement context
NodeKind::Unary { op: "++"|"--", operand } => Write
```

### Deduplication Strategy

When the same position has multiple potential highlights (e.g., assignment LHS is both declaration and write):

1. Group by `(start, end)` byte offset
2. Prefer `Write (3)` over `Read (2)` over `Text (1)`
3. Sort final results by start position

### Example Usage

```rust
use perl_parser::{Parser, document_highlight::DocumentHighlightProvider};

let source = r#"
my $foo = 42;
print $foo;
$foo = $foo + 1;
my $bar = $foo * 2;
"#;

let mut parser = Parser::new(source);
let ast = parser.parse().unwrap();

let provider = DocumentHighlightProvider::new();

// Find highlights for $foo at byte offset 4 (first occurrence)
let highlights = provider.find_highlights(&ast, source, 4);

// Results:
// [
//   DocumentHighlight { location: 3..7 (my $foo), kind: Write },
//   DocumentHighlight { location: 23..27 (print $foo), kind: Read },
//   DocumentHighlight { location: 31..35 ($foo = ...), kind: Write },
//   DocumentHighlight { location: 38..42 (... $foo + 1), kind: Read },
//   DocumentHighlight { location: 59..63 ($foo * 2), kind: Read },
// ]
```

### Performance Characteristics

- **Node Finding**: Single AST traversal, <50μs for typical files
- **Symbol Matching**: O(n) where n = AST node count
- **Deduplication**: HashMap-based, O(h) where h = highlight count
- **Memory Usage**: ~50 bytes per highlight

### Test Coverage

**Test File**: `crates/perl-lsp/tests/lsp_document_highlight_test.rs`

**Coverage**:
- Scalar variable highlighting with write/read detection
- Subroutine call highlighting
- Method call highlighting (incomplete implementation acknowledged)
- Statement modifier highlighting (Issue #191 fixes)
- Empty results for non-symbol positions

**Known Issues**:
- Test acknowledges some patterns may return 0 results during implementation refinement
- Focus on API correctness rather than complete coverage

---

## Folding Ranges Provider

**Module**: `crates/perl-parser/src/folding.rs`
**LSP Method**: `textDocument/foldingRange`
**LSP Specification**: [LSP 3.17 Folding Range](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_foldingRange)

### Purpose

Enables code folding/collapsing in editors for logical code sections including subroutines, blocks, control structures, classes, heredocs, and import statement groups. Optimized for large Perl files with byte offset-based ranges.

### LSP Workflow Integration

- **Parse**: AST traversal for structural elements + lexer for heredocs
- **Index**: No workspace indexing (single-document scope)
- **Navigate**: Provides visual code organization
- **Complete**: No direct integration
- **Analyze**: Groups consecutive import statements automatically

### Key Types

#### `FoldingRange`

```rust
pub struct FoldingRange {
    pub start_offset: usize,            // Starting byte offset
    pub end_offset: usize,              // Ending byte offset
    pub kind: Option<FoldingRangeKind>, // Optional classification
}
```

**Performance Note**: Uses byte offsets (not line numbers) for <1μs range calculation per fold region. LSP server converts to line numbers during serialization.

#### `FoldingRangeKind`

```rust
pub enum FoldingRangeKind {
    Comment,  // Multi-line comments, POD, DATA sections
    Imports,  // Consecutive use/require statements
    Region,   // Code blocks, subs, classes, heredocs
}
```

### Key Methods

#### `FoldingRangeExtractor::new`

```rust
pub fn new() -> Self
```

Creates a new folding range extractor with empty state.

#### `FoldingRangeExtractor::extract`

```rust
pub fn extract(&mut self, ast: &Node) -> Vec<FoldingRange>
```

Extracts all folding ranges from the AST.

**Algorithm**:
1. Clear previous ranges
2. Recursively visit all AST nodes
3. Create ranges for foldable constructs (multi-line only)
4. Return cloned vector

#### `FoldingRangeExtractor::extract_heredoc_ranges`

```rust
pub fn extract_heredoc_ranges(text: &str) -> Vec<FoldingRange>
```

Extracts heredoc folding ranges using the lexer (separate from AST).

**Rationale**: Heredocs may span multiple tokens and require special lexer-based handling for accurate byte offset ranges.

### Supported Constructs

#### Packages

```perl
package MyApp::Controller {
    # Package body
    sub handler { ... }
}

# Or bare packages (multi-line scope)
package MyApp::Model;
# ... module content ...
```

**Fold Range**: Entire package declaration to closing brace or next package

#### Subroutines and Methods

```perl
sub calculate {
    my $x = shift;
    return $x * 2;
}

method process($arg) {
    return $arg + 1;
}
```

**Fold Range**: Subroutine/method body block

#### Control Structures

```perl
if ($condition) {
    # Then branch
} elsif ($other) {
    # Elsif branch
} else {
    # Else branch
}

while ($x < 10) {
    # Loop body
}

for (my $i = 0; $i < 10; $i++) {
    # Loop body
}

foreach my $item (@list) {
    # Loop body
}
```

**Fold Range**: Each block (if/elsif/else, while, for, foreach)

#### Phase Blocks

```perl
BEGIN {
    # Initialization
}

END {
    # Cleanup
}

CHECK { ... }
INIT { ... }
```

**Fold Range**: Phase block body

#### Classes (Corinna/Object::Pad)

```perl
class MyClass {
    field $x;
    method process { ... }
}
```

**Fold Range**: Class body

#### Arrays and Hashes

```perl
my @array = (
    "element1",
    "element2",
    "element3",
);

my %hash = (
    key1 => "value1",
    key2 => "value2",
);
```

**Fold Range**: Array/hash literal with elements

#### Import Groups

```perl
use strict;
use warnings;
use Data::Dumper;
use JSON::XS;
use MyApp::Config;
# Folded as single group
```

**Algorithm**:
1. Track consecutive `use`/`no` statements
2. Create single folding range spanning first to last import
3. Reset on non-import statement
4. Mark as `FoldingRangeKind::Imports`

#### DATA Sections

```perl
__DATA__
Large data content
can be folded
```

**Fold Range**: DATA section body (marked as Comment)

#### Heredocs

```perl
my $text = <<'EOF';
Multi-line
heredoc content
EOF
```

**Extraction**: Uses `FoldingRangeExtractor::extract_heredoc_ranges(text)` with lexer-based detection

### Example Usage

```rust
use perl_parser::{Parser, folding::{FoldingRangeExtractor, FoldingRangeKind}};

let source = r#"
package MyApp;
use strict;
use warnings;
use Data::Dumper;

sub calculate {
    my $x = shift;
    if ($x > 0) {
        return $x * 2;
    }
    return 0;
}

my @data = (
    1, 2, 3,
    4, 5, 6,
);
"#;

let mut parser = Parser::new(source);
let ast = parser.parse().unwrap();

let mut extractor = FoldingRangeExtractor::new();
let mut ranges = extractor.extract(&ast);

// Add heredoc ranges
let heredoc_ranges = FoldingRangeExtractor::extract_heredoc_ranges(source);
ranges.extend(heredoc_ranges);

// Results include:
// - Package body (Region)
// - Import group: use strict + use warnings + use Data::Dumper (Imports)
// - Subroutine calculate body (Region)
// - If block inside calculate (Region)
// - Array literal @data (Region)
```

### Performance Characteristics

- **Memory Footprint**: 24 bytes per range (optimized for large files)
- **Range Calculation**: <1μs per fold region
- **AST Traversal**: Single-pass recursive descent
- **LSP Serialization**: Direct mapping to protocol types with line conversion

### Test Coverage

**Test File**: `crates/perl-lsp/tests/lsp_folding_ranges_test.rs`

**Coverage**:
- Subroutine folding
- Nested block folding (if, while inside subs)
- Import group detection (consecutive use statements)
- Multi-statement blocks

**Test Expectations**:
- At least 4 ranges for nested structures
- Proper JSON array response format

---

## Type Definition Provider

**Module**: `crates/perl-parser/src/type_definition.rs`
**LSP Method**: `textDocument/typeDefinition`
**LSP Specification**: [LSP 3.17 Type Definition](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_typeDefinition)

### Purpose

Provides go-to-type-definition functionality for Perl objects and blessed references, navigating from variable usage to the package/class definition. Supports constructor calls, method calls, blessed references, and `isa` checks.

### LSP Workflow Integration

- **Parse**: AST analysis to extract type information from expressions
- **Index**: Searches across open documents for package definitions
- **Navigate**: Creates LocationLink to package declarations
- **Complete**: No direct integration
- **Analyze**: Type inference from blessed references and constructor patterns

### Key Methods

#### `TypeDefinitionProvider::new`

```rust
pub fn new() -> Self
```

Creates a new stateless type definition provider.

#### `TypeDefinitionProvider::find_type_definition`

```rust
pub fn find_type_definition(
    &self,
    ast: &Node,
    line: u32,
    character: u32,
    uri: &str,
    documents: &HashMap<String, String>,
) -> Option<Vec<LocationLink>>
```

Finds type definition for a position in the AST.

**Returns**: `LocationLink` array with package definition locations, or `None` if no type found

### Supported Patterns

#### Constructor Calls

```perl
my $obj = Package::Name->new();
#         ^^^^^^^^^^^^^ - Extracts "Package::Name"
```

**Type Extraction**:
- Pattern: `Identifier -> Identifier("new")`
- Result: Package name from left side of `->`

#### Method Calls

```perl
$obj->method();
# Type inferred from $obj declaration (limited support)
```

**Type Extraction**:
- Attempts to trace `$obj` variable to its declaration
- Limited to simple cases (`$self`, `$this` recognized)

#### Blessed References

```perl
bless {}, 'MyClass';
#         ^^^^^^^^ - Extracts "MyClass"

bless $ref, $class;
#           ^^^^^^ - Extracts from variable/identifier
```

**Type Extraction**:
- Pattern: `bless` function call with 2 arguments
- Second argument is the package name (string or identifier)

#### ISA Checks

```perl
$obj isa MyClass
#       ^^^^^^^^ - Extracts "MyClass"
```

**Type Extraction**:
- Pattern: `Binary { op: "isa", right: ... }`
- Right side is package name

#### Package-Qualified Identifiers

```perl
Package::Name::method()
# ^^^^^^^^^^^^^ - Extracts "Package::Name"
```

**Type Extraction**:
- Pattern: Identifier containing `::`
- Package is everything except the last component

### Example Usage

```rust
use perl_parser::Parser;
use perl_lsp::features::type_definition::TypeDefinitionProvider;
use std::collections::HashMap;

let source = r#"
package MyClass;

sub new {
    my $class = shift;
    bless {}, $class;
}

sub method {
    print "Hello\n";
}

package main;

my $obj = MyClass->new();
$obj->method();
"#;

let mut parser = Parser::new(source);
let ast = parser.parse().unwrap();

let provider = TypeDefinitionProvider::new();
let uri = "file:///test.pl";

let mut documents = HashMap::new();
documents.insert(uri.to_string(), source.to_string());

// Find type definition for MyClass->new() at line 14, character 10
let result = provider.find_type_definition(&ast, 14, 10, uri, &documents);

// Result contains LocationLink to "package MyClass;" declaration
assert!(result.is_some());
let locations = result.unwrap();
assert_eq!(locations.len(), 1);
```

### Performance Characteristics

- **Type Extraction**: Pattern matching on AST nodes, <10μs
- **Package Search**: Linear AST traversal, <100ms for typical files
- **Cross-Document Search**: O(d × n) where d = documents, n = AST nodes
- **Memory Usage**: Minimal (stateless provider)

### Limitations

- **Object Type Inference**: Limited to simple variable patterns (`$self`, `$this`)
- **Cross-File Resolution**: Searches only open documents, not full workspace
- **Position Calculation**: Simplified implementation (Issue #196 improvements planned)
- **Return Values**: May return dummy `(0, 0)` positions in some cases

### Test Coverage

**Test File**: `crates/perl-lsp/tests/lsp_type_definition_tests.rs`

**Coverage**:
- Basic package definition finding
- Constructor call type extraction
- CRLF and emoji position handling
- Response format validation (array or null)
- Non-dummy position verification

**Known Issues**:
- Tests focus on API correctness over complete functionality
- Position calculations need refinement (acknowledged in tests)
- Type inference for complex variable tracking not implemented

---

## Implementation Provider

**Module**: `crates/perl-parser/src/implementation_provider.rs`
**LSP Method**: `textDocument/implementation`
**LSP Specification**: [LSP 3.17 Implementation](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_implementation)

### Purpose

Finds implementations of types and methods in Perl code, including subclasses that inherit from a base class using `@ISA` or `use parent`, and overridden methods in derived classes. Supports workspace-wide implementation search with optional workspace indexing.

### LSP Workflow Integration

- **Parse**: AST analysis to identify packages, inheritance, methods
- **Index**: Optional workspace index for cross-file inheritance tracking
- **Navigate**: Provides go-to-implementation functionality
- **Complete**: No direct integration
- **Analyze**: Inheritance analysis for implementation relationships

### Key Types

#### `ImplementationProvider`

```rust
pub struct ImplementationProvider {
    workspace_index: Option<std::sync::Arc<WorkspaceIndex>>,
}
```

**Workspace Integration**: Optional `WorkspaceIndex` for comprehensive cross-file implementation finding (5MB memory overhead for implementation metadata).

### Key Methods

#### `ImplementationProvider::new`

```rust
pub fn new(workspace_index: Option<std::sync::Arc<WorkspaceIndex>>) -> Self
```

Creates a new implementation provider with optional workspace indexing.

**Examples**:

```rust
use perl_parser::implementation_provider::ImplementationProvider;

// Without workspace indexing (single-file analysis)
let provider = ImplementationProvider::new(None);

// With workspace indexing (cross-file inheritance)
use std::sync::Arc;
use perl_parser::workspace_index::WorkspaceIndex;
let workspace_index = Arc::new(WorkspaceIndex::new());
let provider = ImplementationProvider::new(Some(workspace_index));
```

#### `ImplementationProvider::find_implementations`

```rust
pub fn find_implementations(
    &self,
    ast: &Node,
    line: u32,
    character: u32,
    uri: &str,
    documents: &HashMap<String, String>,
) -> Vec<LocationLink>
```

Finds implementations at the given position.

**Algorithm**:
1. Find the node at cursor position
2. Extract implementation target (Package, Method, BlessedType)
3. Search documents for implementations
4. Return LocationLink array

### Supported Inheritance Patterns

#### `use parent`

```perl
package Animal;
sub speak { die "Abstract" }

package Dog;
use parent 'Animal';
#          ^^^^^^^^ - Inheritance detected
sub speak { "Woof!" }
```

**Detection**:
- Pattern: `Use { module: "parent", args: ["Animal"] }`
- All args checked against base package name

#### `use base`

```perl
package Cat;
use base 'Animal';
#        ^^^^^^^^ - Inheritance detected
sub speak { "Meow!" }
```

**Detection**: Same as `use parent`

#### `@ISA` Assignment

```perl
package Bird;
our @ISA = ('Animal');
#          ^^^^^^^^^^ - Inheritance detected
sub speak { "Chirp!" }

# Or variable form
our @ISA = ($parent_class);
```

**Detection**:
- Pattern: `VariableDeclaration { declarator: "our", variable: "@ISA", initializer: ... }`
- Initializer checked for parent package name

### Implementation Finding Strategies

#### Package Implementations (Subclasses)

Finds all classes that inherit from a base package:

```rust
// For base package "Animal", finds:
// - Dog (use parent 'Animal')
// - Cat (use parent 'Animal')
// - Bird (our @ISA = ('Animal'))
```

**Algorithm**:
1. Parse all documents in workspace
2. Find packages with inheritance from base
3. Use workspace index if available for optimized search
4. Return LocationLink to package declarations

#### Method Implementations (Overrides)

Finds all overridden methods in subclasses:

```rust
// For Animal::speak, finds:
// - Dog::speak
// - Cat::speak
// - Bird::speak
```

**Algorithm**:
1. Find all package implementations (subclasses)
2. For each subclass, search AST for method with same name
3. Return LocationLink to method declarations

### Example Usage

```rust
use perl_parser::{Parser, implementation_provider::ImplementationProvider};
use std::collections::HashMap;

let source = r#"
package Animal;
sub new { bless {}, shift }
sub speak { die "Abstract method" }

package Dog;
use parent 'Animal';
sub speak { "Woof!" }

package Cat;
use parent 'Animal';
sub speak { "Meow!" }

package main;
my $pet = Animal->new();
"#;

let mut parser = Parser::new(source);
let ast = parser.parse().unwrap();

let provider = ImplementationProvider::new(None);
let uri = "file:///test.pl";

let mut documents = HashMap::new();
documents.insert(uri.to_string(), source.to_string());

// Find implementations of Animal at line 1 (package declaration)
let implementations = provider.find_implementations(&ast, 1, 8, uri, &documents);

// Results contain LocationLink to Dog and Cat package declarations
assert_eq!(implementations.len(), 2);
```

### Performance Characteristics

- **Implementation Finding**: <100ms for typical inheritance hierarchies
- **Memory Usage**: <5MB for implementation metadata
- **Workspace Indexing**: Leverages cached inheritance relationships
- **Cross-Document Search**: O(d × n) where d = documents, n = AST nodes per document

### Test Coverage

**Test File**: `crates/perl-lsp/tests/lsp_implementation_tests.rs`

**Coverage**:
- Finding subclasses via `use parent`
- Finding method overrides in derived classes
- Response format validation (array or null)
- Position verification (non-dummy coordinates)

**Test Scenarios**:
- Multiple subclasses inheriting from base
- Multiple method overrides across inheritance hierarchy
- No implementations (returns empty array or null)

---

## Common Patterns Across Providers

### Position Handling

All providers use consistent byte offset → UTF-16 line/column conversion:

```rust
use perl_parser::position::offset_to_utf16_line_col;

let (line, col) = offset_to_utf16_line_col(source_text, byte_offset);
```

**Performance**: ~1μs per conversion with proper UTF-8/UTF-16 handling

### AST Traversal

Providers implement recursive descent traversal with pattern matching:

```rust
fn visit_node(&self, node: &Node, context: &mut Context) {
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                self.visit_node(stmt, context);
            }
        }
        NodeKind::Subroutine { body, .. } => {
            // Process subroutine
            self.visit_node(body, context);
        }
        // ... other patterns
        _ => {}
    }
}
```

### LSP Response Format

All providers return JSON-compatible structures using `serde_json`:

```rust
use serde_json::{Value, json};

let response = json!({
    "range": {
        "start": { "line": start_line, "character": start_col },
        "end": { "line": end_line, "character": end_col }
    },
    "kind": kind_value
});
```

### Error Handling

Providers gracefully handle missing data:

```rust
// Return empty Vec instead of error
pub fn find_symbols(&self, ...) -> Vec<Symbol> {
    match self.internal_find(...) {
        Some(results) => results,
        None => Vec::new(),  // Graceful degradation
    }
}

// Return None for optional results
pub fn find_definition(&self, ...) -> Option<Location> {
    let node = self.find_node_at_position(...)?;
    self.extract_definition(node)
}
```

## Testing Best Practices

### Integration Testing

All providers have LSP integration tests using the test harness:

```rust
use support::lsp_harness::LspHarness;

let mut harness = LspHarness::new();
harness.initialize(None).expect("Failed to initialize");
harness.open(uri, source).expect("Failed to open file");

let response = harness.request("textDocument/METHOD", params)
    .expect("Request failed");
```

### Test Structure

Provider tests follow consistent patterns:

1. **Setup**: Initialize LSP server and open test document
2. **Request**: Send LSP method request with test position
3. **Validate**: Check response format (array/null/object)
4. **Verify**: Validate content (ranges, kinds, positions)

### Position Testing

Tests verify proper UTF-16 position handling:

```rust
// Test with CRLF line endings and emojis
let source = "package Test;\r\n# 🎉 Comment\r\nsub new { ... }\r\n";

let response = harness.request(...).expect("Failed");

// Verify non-dummy positions
if let Some(locations) = response.as_array() {
    for loc in locations {
        let line = loc["range"]["start"]["line"].as_u64().unwrap();
        let char = loc["range"]["start"]["character"].as_u64().unwrap();
        assert!(line > 0 || char > 0, "Expected non-(0,0) position");
    }
}
```

## Performance Optimization

### Viewport-Based Extraction

Providers support range-based extraction for visible editor regions:

```rust
// Only extract hints/ranges in visible viewport
let range = Range::new(
    Position::new(visible_start_line, 0),
    Position::new(visible_end_line, 0),
);

let hints = provider.extract_range(ast, range);  // ~50% reduction
```

### Caching Strategies

- **Code Lens**: Two-phase resolution (extract → lazy resolve with reference count)
- **Folding Ranges**: Byte offset storage for fast LSP line conversion
- **Type Definition**: Stateless provider (no caching needed)

### Memory Management

- **Shared State**: Use `Arc<WorkspaceIndex>` for cross-provider workspace access
- **Minimal Cloning**: Return references where possible, clone only for LSP serialization
- **Lazy Evaluation**: Code lens resolution deferred until client requests

## Migration Notes

### From v2 to v3 Parser

Several providers acknowledge ongoing AST migration:

```rust
// Inlay hints test (inlay_hints_provider.rs)
// Note: Inlay hints may not work with new AST structure yet
// For now just ensure it doesn't crash - empty result is acceptable
```

**Impact**:
- **Code Lens**: Fully migrated, all tests passing
- **Document Links**: Fully migrated (line-based, no AST dependency)
- **Inlay Hints**: Partial migration, some patterns may return empty results
- **Document Highlights**: Fully migrated with Issue #191 fixes
- **Folding Ranges**: Fully migrated with lexer integration
- **Type Definition**: Basic migration complete, position calculation improvements planned
- **Implementation**: Fully migrated, workspace index integration ready

## See Also

- [LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md) - Server architecture and request handling
- [Workspace Navigation Guide](WORKSPACE_NAVIGATION_GUIDE.md) - Cross-file navigation features
- [API Documentation Standards](API_DOCUMENTATION_STANDARDS.md) - Documentation requirements for providers
- [Position Tracking Guide](POSITION_TRACKING_GUIDE.md) - UTF-16/UTF-8 position conversion details

## Related Issues

- **Issue #191**: Document Highlights fixes for statement modifiers and regex operations
- **Issue #196**: Type Definition position calculation improvements
- **Issue #207**: DAP integration (separate from LSP providers)
- **Issue #160/SPEC-149**: API documentation infrastructure enforcement

---

**Document Version**: 0.9.x
**Last Updated**: 2025-01-31
**Maintainer**: Perl LSP Documentation Team
