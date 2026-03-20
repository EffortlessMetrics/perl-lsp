# Perldoc Hover Integration Research

**Date**: 2026-03-19
**Status**: Exploration Complete
**Effort Estimate**: 2-3 weeks for full integration

---

## 1. Current Hover State

### What Works Now
- **Module hovers** (`use Module`): Shows resolved path (PR #2211)
  - File-based resolution: `crates/perl-module-resolution/src/lib.rs`
  - Displays: `**Module::Name** → Resolved: /path/to/Module.pm`
  - Handles both URI and filesystem resolution

- **Symbol hovers** (variables, subs, packages): Shows semantic info
  - Type information: Scalar/Array/Hash/Sub/Package/Constant/Label/Format
  - Declaration context
  - Attributes
  - User documentation (from code comments)
  - Source: `crates/perl-lsp/src/runtime/language/hover.rs` (lines 89-161)

- **Builtin hovers**: Shows Perl builtin function signatures
  - Hard-coded database of 100+ builtin functions (lines 732-926)
  - Source: `crates/perl-builtins/src/`
  - Limited documentation: just signature + basic description

- **Current infrastructure for documentation**:
  - `SemanticAnalyzer::find_definition()` - extracts symbol info
  - `get_builtin_documentation()` - returns signature + description
  - POD extraction regex in `perl-semantic-analyzer` - parses `=head1 NAME` sections
  - Builtin signatures via `perl-builtins` and `perl-builtins-phf` crates

### What's Missing
- **Module documentation** when hovering `use DBI; use Moose;`
  - Currently shows only path, not synopsis or description

- **Method call documentation** when hovering `$dbh->prepare($sql)`
  - Currently shows nothing useful for CPAN methods

- **Method documentation** for imported methods (e.g., `use Moose;` then `has 'name';`)

---

## 2. POD Integration Points Found

### Existing POD Infrastructure
- `perl-dap-breakpoint/src/validator.rs`: POD region detection
  - Functions: `find_pod_regions()`, `is_pod_directive()`
  - Detects `=pod`, `=head*`, `=over`, `=item`, `=cut`
  - Used to prevent breakpoints in POD sections

- `perl-semantic-analyzer/src/analysis/semantic.rs`: POD extraction
  - `extract_pod_name_section()` - parses NAME sections from POD
  - Regex pattern: `=head1\s+NAME\s+(.*?)(?:=head|\Z)`
  - Currently used for package documentation in hover

- `perl-lsp/src/fallback/text.rs`: Basic POD folding support
  - Recognizes POD blocks for code folding ranges

### Can Be Leveraged
- `resolve_module_path()` and `resolve_module_uri()` already find `.pm` files
- Just need to **parse POD from resolved .pm files** for documentation

---

## 3. How to Get POD Documentation

### Option A: Shell out to `perldoc -T Module`
**Pros:**
- Simple, uses native Perl
- Accurate (same as users' `perldoc` command)
- Handles all Perl versions

**Cons:**
- Requires Perl to be installed on developer's machine
- Subprocess latency (~100-500ms per call)
- High latency on hover (user experience issue)
- Not reproducible in offline/CI environments
- Race conditions if perldoc hangs

**Implementation**:
```rust
let output = std::process::Command::new("perldoc")
    .arg("-T")  // plaintext
    .arg("Module::Name")
    .output()?;
```

**Verdict**: **Not viable for hover** (interactive, must be <50ms). Could be async background cache.

---

### Option B: Parse POD from resolved .pm files
**Pros:**
- No external dependency on Perl
- Fast (file I/O only, no subprocess)
- Works offline
- Can be cached in memory during session
- Full control over formatting

**Cons:**
- Need to implement POD parser
- Must handle `=head1`, `=head2`, `=over`, `=item`, `=back`, `=cut`
- Different from `perldoc -T` formatting (needs paging, filtering)

**Existing parser code**:
- `perl-dap-breakpoint/src/validator.rs` detects POD block boundaries
- Can reuse `=head`, `=pod`, `=cut` detection

**Effort**: Medium (write POD→Markdown converter, ~300-500 lines Rust)

**Verdict**: **Best for production** (fast, reliable, cached). Primary approach.

---

### Option C: Pre-index POD at workspace init
**Pros:**
- Can index all installed modules at startup (once)
- Very fast hover (O(1) lookup)
- Can include MetaCPAN data if desired

**Cons:**
- Startup time overhead (scan `@INC` for .pm files)
- Needs persistent storage (JSON cache file)
- Cache invalidation on `@INC` change
- Hard to maintain sync with Perl versions

**Verdict**: **Secondary optimization** (v2). First ship B, then optimize with C.

---

### Option D: Fetch from MetaCPAN API
**Pros:**
- Rich, pre-formatted documentation
- Includes examples, links
- Version-specific docs

**Cons:**
- Network latency (500ms-2s typical)
- Requires internet connection
- Rate limiting
- Not for local-only modules

**Verdict**: **Consider as enhancement** (async background fetch, fallback to local).

---

## 4. Implementation Roadmap

### Phase 1: Parser POD from Resolved .pm Files (v1.0)

**Step 1**: Extend module resolution to return file path
- Currently: `resolve_module_path()` returns `PathBuf`
- Need: Pass path to new POD extractor

**Step 2**: Implement POD parser
```rust
// crates/perl-pod-parser/src/lib.rs
pub struct PodSection {
    level: u8,      // 1-4 for =head1, =head2, etc.
    title: String,
    content: String,
}

pub fn extract_pod(source: &str) -> Vec<PodSection> { ... }
```

**Step 3**: Hook into hover for modules
- When hovering `use DBI;`:
  1. Resolve to `/path/to/DBI.pm`
  2. Extract POD NAME, SYNOPSIS sections
  3. Format as Markdown for hover

- When hovering `$dbh->prepare()`:
  1. Resolve `DBI` module
  2. Parse methods from POD (look for `=head2 prepare` or `=item prepare()`)
  3. Show method docs

**Step 4**: Cache POD in session
```rust
// In LspServer
pod_cache: Arc<RwLock<HashMap<String, Vec<PodSection>>>>
```

---

## 5. What Users Will See

### Today (Current)
```
Hover: use DBI;
→ **DBI**
→ Resolved: /usr/lib/perl/DBI.pm
```

### After v1.0
```
Hover: use DBI;
→ **DBI**
→ Resolved: /usr/lib/perl/DBI.pm
→
→ ## Synopsis
→ DBI is a database interface for Perl. It allows you to...
→
→ ## Description
→ The DBI is a Perl module that provides a consistent interface...
→ (truncated for display, ~200 chars)
```

### After v2.0 (with method hovers)
```
Hover: $dbh->prepare($sql)
→ **Method**
→ `prepare($sql_string)`
→
→ **DBI::st** — Prepare a statement for execution...
→
→ prepare() returns a statement handle...
```

### After v3.0 (with MetaCPAN fallback)
```
Hover: use Moose;
→ **Moose**
→ Resolved: /usr/lib/perl/Moose.pm
→
→ Moose — A postmodern object system for Perl
→ (Moose is a complete object system for Perl...)
→
→ [View on MetaCPAN](https://metacpan.org/pod/Moose)
```

---

## 6. Integration Points

### Files to Modify
1. **`crates/perl-lsp/src/runtime/language/hover.rs`** (lines 227-286)
   - Extend `build_module_hover()` to fetch POD
   - Add method hover detection

2. **New crate: `crates/perl-pod-parser/`**
   - POD extraction and formatting
   - Reusable across LSP providers

3. **`crates/perl-lsp/src/lib.rs`**
   - Add POD cache to `LspServer` state

### Integration with Existing Code
- **Module resolution** (`perl-module-resolution`): Already finds .pm paths
- **Semantic analyzer** (`perl-semantic-analyzer`): Can parse method definitions
- **Builtin docs** (`perl-builtins`): Fallback for standard functions

---

## 7. Effort & Risk Assessment

### Phase 1 (Base POD Parsing)
- **Scope**: Parse POD from .pm files, show in module hovers
- **Effort**: 3-4 days
- **Risk**: Low (file parsing, no external deps)
- **Files**: +600 lines Rust, mostly in new crate

### Phase 2 (Method Hovers)
- **Scope**: Extract methods from POD, show on `$obj->method` hover
- **Effort**: 2-3 days
- **Risk**: Medium (AST traversal for method context)
- **Files**: +300 lines

### Phase 3 (MetaCPAN Fallback)
- **Scope**: Async fetch from MetaCPAN for missing local docs
- **Effort**: 2 days
- **Risk**: Medium (network errors, rate limits)
- **Files**: +200 lines

### Total: 7-9 days for full integration

---

## 8. Recommended Next Steps

### Immediate (This Week)
1. **Create `perl-pod-parser` crate**
   - Implement POD section extraction
   - Add tests with sample .pm files
   - Reuse code from `perl-dap-breakpoint/src/validator.rs`

2. **Extend hover.rs**
   - Modify `build_module_hover()` to fetch POD sections
   - Add POD caching to `LspServer`
   - Test with DBI, Moose, Data::Dumper

3. **Create GitHub issue**: "Feature: Perldoc hover for CPAN modules"
   - Link to this research
   - Clarify scope for each phase

### Short-term (Next 1-2 weeks)
- Phase 2: Method hover support
- Phase 3: MetaCPAN fallback
- Documentation and user guides

---

## 9. Confidence Assessment

- **POD parsing from files**: 95% confidence (proven approach)
- **Module-to-POD hookup**: 90% confidence (module resolution already works)
- **Method extraction**: 70% confidence (requires AST navigation)
- **MetaCPAN integration**: 80% confidence (standard API, but network dependencies)

**Overall readiness**: Ready to start Phase 1 immediately.
