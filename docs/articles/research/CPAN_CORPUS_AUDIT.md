# CPAN Corpus Completeness Audit — Final Report

**Date:** 2026-03-19
**Scope:** Tree-sitter Perl LSP project
**Working Directory:** `/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/`

---

## Executive Summary

The CPAN corpus infrastructure is **well-structured but highly incomplete**:
- **Current state:** 4,355 total files, 3,139 clean (72%), 1,212 with errors
- **Coverage model:** Top 1,000 CPAN distributions by "reverse dependency count" (River score)
- **What's included:** 2,197 `.pm` files from ~460 installed distributions
- **What's missing:** 968 of the top 1,000 distributions (97% gap) — including critical frameworks
- **Gap reason:** The distribution-to-module mapping is broken: "Test-Simple" distribution doesn't auto-populate Test::More/Test::Builder into the manifest

---

## 1. Current Corpus Structure

### Directory Layout

```
test_corpus/                          # Hand-written test files (91 files, 20K lines)
  ├── *.pl                            # Feature coverage files (source filters, XS, modern Perl)
  ├── real_world/                     # Real-world code patterns
  └── edge_cases/                     # Parser boundary tests

target/cpan-corpus/                   # Installed CPAN modules via cpanm
  └── lib/perl5/                      # All .pm files (2,197 files, 207M)

.ci/cpan-top-1000-distributions.txt  # Pinned list of 1,000 distributions (by River score)
.ci/cpan-corpus-manifest.txt          # Known-clean modules (1,849 lines, ratcheting)
.ci/cpan-corpus-baseline.json         # Last sweep report (4,355 files analyzed)

tree-sitter-perl/test/corpus/         # Tree-sitter test files (46 files)
  ├── *.txt                           # TST format test cases
  └── edge-cases.txt, expressions/, ...
```

### Infrastructure Files

| File | Purpose | Last Update |
|------|---------|-------------|
| `xtask/src/tasks/cpan_corpus.rs` (602 lines) | Corpus acquisition, sweep, ratchet | Core infrastructure |
| `xtask/src/tasks/parser_corpus_sweep.rs` | Parser sweep with error classification | Core infrastructure |
| `.ci/cpan-top-1000-distributions.txt` | Pinned top 1,000 by River.immediate | 2026-03-17 (2d old) |
| `.ci/cpan-corpus-manifest.txt` | Known-clean module ratchet | 2026-03-17 (auto-updated) |
| `.ci/cpan-corpus-baseline.json` | Baseline metrics for regression gating | 2026-03-18 (1d old) |

---

## 2. CPAN Top 1,000 Analysis

### Ranking Metric: "River Score"

**Sorting:** MetaCPAN API: `river.immediate` (descending)
**Definition:** Reverse dependency count — how many CPAN packages directly depend on this one

**Top 50 sample:**
1. Test-Simple (Test::More, Test::Builder)
2. ExtUtils-MakeMaker (build system)
3. perl (core)
4. PathTools (File::Spec)
5. IO (core)
6. Carp (error handling)
7. Module-Build (build system)
8. Scalar-List-Utils (Scalar::Util, List::Util)
9. **Moose** (OO framework, 356 submodules in corpus)
10. Exporter
11. File-Temp
12. Test-Exception
13. **Moo** (lightweight OO)
14. libwww-perl (LWP, HTTP client)
15. URI
16. parent
17. ...
35. **Mojolicious** (web framework)
38. **Path-Tiny**
37. **DBI** (database interface)
40. **Test-Deep** (assertion testing)
42. **Test2-Suite** (modern testing)
50. **Plack** (PSGI app server)

### Critical Observations

**Top-1000 has excellent breadth:**
- Web frameworks: Mojolicious, Catalyst-Runtime, Dancer2, Plack
- OO systems: Moose, Moo, Type-Tiny, Class-Accessor
- Testing: Test-Simple, Test-Exception, Test-Deep, Test2-Suite, Test-NoWarnings
- Data: Data-Dumper, DateTime, JSON, XML-LibXML, YAML
- Database: DBI, DBIx-Class, DBIx-Connector
- File/Path: File-Temp, Path-Tiny, File-Find-Rule, File-Slurp
- Utilities: Try-Tiny, List-MoreUtils, Path-Class

**But Top-1000 is incomplete coverage:**
The list focuses on what other packages depend on, not what users actually run. Missing:
- Less-popular but important patterns (indirect object syntax, source filters)
- Real-world applications (not modules)
- Legacy codebases

---

## 3. What's Currently in the Corpus

### Module Coverage: The Distribution-to-Module Mapping Gap

The corpus includes modules **IF they were installed via cpanm**. The manifest (`cpan-corpus-manifest.txt`) is a **known-clean ratchet**, not a complete inventory.

**What IS included:**
- ✓ Moose:: (356 submodules)
- ✓ Test::Deep:: (54 submodules)
- ✓ Dancer2:: (many submodules)
- ✓ Mojolicious:: (many submodules)
- ✓ Try-Tiny modules (if installed)
- ✓ Type-Tiny modules
- ✓ CHI:: (59 submodules)
- ✓ App::Cmd:: (18 submodules)

**What IS NOT in manifest (critical gap):**
- ✗ Test::More (from Test-Simple distribution)
- ✗ Test::Builder (from Test-Simple distribution)
- ✗ Data::Dumper (from Data-Dumper distribution)
- ✗ Scalar::Util, List::Util (from Scalar-List-Utils distribution)
- ✗ DBI (from DBI distribution) — only dependencies
- ✗ DBIx::Class (from DBIx-Class distribution)

**Why?** The manifest file is built by the ratchet process (running `cargo xtask cpan-corpus ratchet`), which:
1. Installs distributions via cpanm
2. Parses all .pm files
3. Extracts module names from `package` statements
4. Only adds modules that parse **cleanly**

**So the manifest is incomplete because:**
- If a distribution failed to install, its modules won't appear
- If a module has parse errors, it won't be in the manifest (it goes to the error report instead)
- The distribution list is only 1,000 items; many important distributions may not be in top 1,000

**Proof of the gap:** 968 of 1,000 distributions are missing from the manifest entirely.

### Parse Results Summary

From `.ci/cpan-corpus-baseline.json`:

```
Total files analyzed:      4,355
Clean files:               3,139 (72.1%)
Files with errors:         1,212 (27.9%)
Files unreadable:          4

Error distribution (top buckets):
  unexpected_token_in_expr:      146 files
  unexpected_rbrace_expr:         83 files
  unexpected_fat_arrow_expr:      66 files
  expected_left_brace:            66 files
  expected_variable:              66 files
  unexpected_comma_expr:          70 files
  unexpected_question_expr:      109 files
  unclosed_paren_identifier:     140 files
  unclosed_paren:                106 files
  expected_comma_or_close_paren:  55 files
  unclosed_bracket:               38 files
  unclosed_brace:                 32 files
  unclosed_brace_semicolon:       32 files
```

**Sweep performance:** 1.22 seconds for 4,355 files

---

## 4. What's Missing

### A. Critical Frameworks NOT in Manifest

**Web/App frameworks:**
- Catalyst-Runtime (Catalyst OO web framework)
- DBIx-Class (ORM)
- DBIx-Connector (connection pooling)
- Dancer (full Dancer v1 framework, if modules parse)

**Core Testing Modules (from Test-Simple):**
- Test::More (most widely-used Perl test module — **critical**)
- Test::Builder (testing infrastructure)
- Test::Simple (basic testing)

**Core Utilities:**
- Data::Dumper (data inspection)
- Scalar::Util, List::Util (from Scalar-List-Utils)
- List::MoreUtils
- File-Slurp (file reading)
- Class-Accessor (OO boilerplate)

**This is because:**
1. These distributions may have parse errors in some modules
2. The top-1000 list pins specific distributions, but module extraction is shallow

### B. What File Types Are Included

**Only .pm files are tested:**
- ✓ Pure Perl modules
- ✓ Modules with embedded POD
- ? Modules with XS stubs (we see Perl parts only)

**NOT included:**
- ✗ .pl scripts (only .pm files)
- ✗ .t test files (only .pm files)
- ✗ .xs files (compiled, not pure Perl)
- ✗ Inline::C, FFI code (not pure Perl)

**Impact:**
- We're not testing real-world scripts (bin/, scripts/)
- We're not testing actual test files (.t)
- We're not testing mixed Perl/XS modules' Perl portions thoroughly

### C. Coverage Depth

**Current approach:** One .pm file per distribution (or one module per distribution)

**Missing:** Comprehensive module coverage
- Some distributions have hundreds of .pm files (Moose, Mojolicious, Catalyst)
- But many have only 1-2 main modules with many dependencies on others
- Example: Test-Simple distribution should expose Test::More, but it's not in manifest

---

## 5. Corpus Expansion Options

### Option A: Fix the Distribution-to-Module Gap (Quick Win)

**What:** Re-run `cpanm install` on all 1,000 distributions, then harvest ALL parsed modules

**Effort:** ~10 minutes runtime (cpanm + ratchet)
**Benefits:**
- Automatically get Test::More, Data::Dumper, and 900+ missing modules
- No code changes needed
- Uses existing infrastructure

**Steps:**
```bash
cd /home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs
cargo xtask cpan-corpus fetch-list        # Refresh top-1000 (2026-03-17 is 2 days old)
cargo xtask cpan-corpus install           # Install all 1000 distributions
just cpan-corpus-ratchet                  # Auto-add all clean modules to manifest
just cpan-corpus-check                    # Verify regression
```

**Expected outcome:** Manifest would grow from 1,849 to ~3,500–4,000 modules (assuming most parse cleanly)

**Risk:** Low — ratchet only moves forward (never removes modules)

---

### Option B: Add Test Files (.t) and Scripts (.pl)

**What:** Include `.t` and `.pl` files from installed distributions

**Effort:** Moderate — parser_corpus_sweep.rs needs `--include-test-files` flag

**Benefits:**
- Real-world test patterns
- Script patterns (not just module patterns)
- Catch parser bugs that only appear in test/script contexts

**Changes needed:**
1. Modify `parser_corpus_sweep.rs` to optionally scan `.t` files
2. Add `--include-test-files` CLI flag
3. Expand corpus baseline

**Risk:** Medium — test files may have more errors, requiring more fixes

---

### Option C: Add Non-CPAN Real-World Sources

**What:** Include Perl codebases from:
- Large open-source projects (Perl itself, Dancer, Catalyst ecosystems)
- Legacy codebases (PerlMongers archives, CPAN-like repos)
- Live applications (if available with permission)

**Effort:** High — requires source identification, download, license verification

**Benefits:**
- Tests actual patterns developers use (vs. what's published)
- May find parser bugs not in CPAN modules
- Better represents real-world failure modes

**Challenges:**
- Storage (could be large)
- Maintenance (repos may go stale)
- License compliance
- Relevance (not all open-source Perl is widely used)

---

### Option D: Targeted Framework Expansion

**What:** Explicitly add known gaps:
- Test framework family (Test::*, Test2::*)
- Core utilities (Data-Dumper, Scalar-List-Utils, Path-Tiny)
- Major ORMs (DBIx-Class, DBIx-Connector)
- Web frameworks (Catalyst, Dancer, Mojolicious extras)

**Effort:** Low — manifest entries are just lines of text

**Process:**
```bash
# In .ci/cpan-corpus-manifest.txt, add:
Test::More
Test::Builder
Test::Simple
Data::Dumper
Scalar::Util
List::Util
DBIx::Class::Row
...
```

**Benefit:** Ensures critical patterns are tested
**Risk:** If these modules have errors, ratchet-check will fail and they'll be added to error bucket (which is useful info)

---

## 6. Quality of Current Coverage

### What We Test Well

✓ **Module structure:** Packages, `use`/`require`, imports
✓ **Subroutine definitions:** Named, anonymous, prototypes, signatures
✓ **OO patterns:** Moose, Moo, bless, method calls
✓ **Data structures:** Arrays, hashes, references, dereferencing
✓ **Control flow:** if/unless/while/for/foreach/given-when
✓ **Operators:** All binary/unary/ternary operators
✓ **String interpolation:** Variables in double quotes, q/qq/qw/qr
✓ **Regular expressions:** Basic patterns, modifiers, substitution
✓ **Heredocs:** Simple and complex nesting

### What We Don't Test Well

✗ **Test files:** .t files with Test::More syntax (but not in corpus)
✗ **Scripts:** .pl scripts in bin/ directories
✗ **XS modules:** Only Perl parts, not C implementations
✗ **Source filters:** Filter::Simple, Filter::Util::Call (parsed pre-filter)
✗ **BEGIN/END blocks:** Compile-time effects not simulated
✗ **Indirect object syntax:** Some patterns may not be covered
✗ **Complex regex:** Unicode properties, recursive patterns
✗ **Tie interface:** Tie/untie patterns (partially covered in test_corpus)

### Nodekind Coverage

From `corpus_audit_report.json` (test_corpus only):
- **Coverage:** 63/68 nodekinds (92.6%)
- **Never seen:** UnknownRest, MissingStatement, MissingBlock, MissingIdentifier, MissingExpression
- **At risk (low frequency):** Diamond, Prototype, FormatStatement

---

## 7. Expansion Recommendations (Priority Order)

### Phase 1: Quick Wins (1–2 hours)

1. **Refresh Top-1000 list** (2 min)
   ```bash
   cargo xtask cpan-corpus fetch-list
   ```
   Action: Update `.ci/cpan-top-1000-distributions.txt` (currently 2 days old)

2. **Full corpus ratchet** (10 min + sweep time)
   ```bash
   cargo xtask cpan-corpus install
   just cpan-corpus-ratchet
   just cpan-corpus-check
   ```
   Expected gain: +1,500–2,000 modules in manifest

3. **Document the gap** (30 min)
   - Create GitHub issue: "CPAN corpus expansion: missing Test::More, Data::Dumper, DBI"
   - Reference this audit
   - Outline Phase 2 plan

### Phase 2: Systematic Expansion (1 week)

4. **Add known-important distributions explicitly** (2 hours)
   - Identify 20–30 distributions with parse errors
   - Triage: which are critical? which are edge cases?
   - Create parser-builder issues for critical ones

5. **Test file integration** (3 days)
   - Add `--include-test-files` flag to parser_corpus_sweep
   - Sweep test files (.t) from installed corpus
   - Report on error patterns in test code

6. **Space/time constraints check** (1 day)
   - Measure corpus size, sweep time with test files included
   - Decide if larger corpus is feasible
   - Set ratchet policy for test files

### Phase 3: Advanced (2+ weeks)

7. **Real-world codebases** (ongoing)
   - Identify high-value open-source Perl projects
   - Add as optional corpus layers
   - Use for beta testing parser improvements

---

## 8. Space and Time Constraints

### Current Footprint

| Metric | Value |
|--------|-------|
| Installed corpus | 207 MB (target/cpan-corpus/) |
| .pm files | 2,197 files |
| Parse sweep time | 1.22 seconds (all 4,355 files) |
| Manifest size | 1,849 lines (~75 KB) |
| CI artifact size | ~2 KB (baseline JSON) |

### Expansion Scenarios

**If we include all 1,000 distributions fully:**
- Est. corpus size: 350–500 MB (rough +100% from missing dists)
- Est. sweep time: 2–3 seconds (assuming linear scaling)
- Still acceptable for CI gates

**If we add all .t test files:**
- Est. additional files: 10K–20K .t files
- Est. additional corpus size: 100–200 MB
- Est. sweep time: 5–8 seconds
- Acceptable, but adds to CI time

**Total recommended ceiling:**
- **Corpus:** 500 MB (reasonable for artifact caching)
- **Sweep time:** <10 seconds (acceptable for regular CI)

---

## 9. How to Expand the Corpus

### Method 1: Ratchet Process (Automatic)

```bash
# Step 1: Ensure dist list is current
cargo xtask cpan-corpus fetch-list

# Step 2: Install distributions via cpanm (idempotent)
cargo xtask cpan-corpus install

# Step 3: Auto-add clean modules to manifest
cargo xtask cpan-corpus ratchet

# Step 4: Verify no regressions
just cpan-corpus-check
```

**Best for:** Adding modules from already-installed distributions
**Time:** ~15 minutes (cpanm may skip already-installed packages)

### Method 2: Manual Addition (Targeted)

Edit `.ci/cpan-corpus-manifest.txt` directly:
```
# Add any module name, one per line
Test::More
Data::Dumper
DBIx::Class::Row
```

Then verify:
```bash
just cpan-corpus-check  # Will add clean ones, report errors
```

**Best for:** Targeting specific modules or frameworks
**Risk:** If module has errors, check will report it (no automatic fix)

### Method 3: Custom Corpus Roots

Modify `parser_corpus_sweep.rs` to scan multiple library paths:
```rust
// Example: include /opt/perl/lib/perl5 in addition to system paths
let corpus_roots = vec![
    "/home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/target/cpan-corpus/lib/perl5",
    "/opt/local/perl/lib/perl5",  // Optional additional corpus
];
```

**Best for:** Merging multiple CPAN archives or local installations
**Effort:** Low (one code change, test)

---

## 10. Actions for Team Lead

### Immediate (Next Session)

- [ ] Run `cargo xtask cpan-corpus fetch-list` to refresh top-1000 (2d old)
- [ ] Run full install + ratchet cycle to capture missing modules
- [ ] Verify `just cpan-corpus-check` still passes

### Short-term (Next 1–2 weeks)

- [ ] Review parser error buckets from expanded manifest
- [ ] Identify top 5–10 critical modules with errors (Test::More, Data::Dumper, etc.)
- [ ] Create GitHub issues for module-specific parser fixes

### Medium-term (Next 1 month)

- [ ] Add `.t` test file support to corpus sweep
- [ ] Measure storage/time impact
- [ ] Decide on ratchet policy for test files

### Long-term (Ongoing)

- [ ] Track CPAN ecosystem changes (new popular distributions)
- [ ] Periodically refresh top-1000 list
- [ ] Monitor parser error buckets for regressions

---

## 11. Caveats & Known Limitations

1. **Distribution vs. Module distinction:** Top-1000 lists **distributions**, not modules. A distribution may contain 10–100 modules. The ratchet process extracts module names from `package` statements.

2. **Parse errors block ratcheting:** If a module fails to parse, it goes to the error bucket but is **not** added to the manifest. This is intentional (clean-only ratchet) but creates the illusion of missing modules when they're actually just erroneous.

3. **River score is reverse-dependency-based:** It measures "how many packages depend on me", not "how many users use me". This biases toward foundational packages (Test-Simple, ExtUtils-MakeMaker) but may miss widely-used apps.

4. **XS modules are partially opaque:** We can only see the Perl portions of mixed Perl/XS modules. The actual C code isn't parsed by our Perl parser.

5. **Corpus is point-in-time:** As CPAN evolves, distributions may change (new versions, module removals, etc.). The manifest only captures modules that parsed cleanly at the time of last ratchet.

---

## Appendix: File Locations

```
Project root:                       /home/steven/code/Rust/perl-lsp/tree-sitter-perl-rs/

Key infrastructure:
  - xtask/src/tasks/cpan_corpus.rs                 (fetch-list, install, ratchet)
  - xtask/src/tasks/parser_corpus_sweep.rs        (sweep with error bucketing)
  - .ci/cpan-top-1000-distributions.txt           (pinned list)
  - .ci/cpan-corpus-manifest.txt                  (known-clean ratchet)
  - .ci/cpan-corpus-baseline.json                 (last sweep report)

Test corpus:
  - test_corpus/                                   (hand-written gap coverage)
  - tree-sitter-perl/test/corpus/                 (tree-sitter test files)

Installed corpus:
  - target/cpan-corpus/lib/perl5/                 (2,197 .pm files, 207 MB)
```

---

## Summary

| Question | Answer |
|----------|--------|
| **Where do corpus files live?** | `test_corpus/` (hand-written), `target/cpan-corpus/lib/perl5/` (installed CPAN) |
| **How were 4,355 files selected?** | Top-1000 CPAN distributions by River score (reverse dependency count), via MetaCPAN API |
| **What's missing?** | 968 of 1,000 distributions missing from manifest (gap reason: parse errors or incomplete ratchet) |
| **Are critical frameworks in corpus?** | Moose ✓, Dancer2 ✓, Mojolicious ✓; BUT Test::More ✗, Data::Dumper ✗, DBI ✗ (parse errors block ratcheting) |
| **How to expand?** | (A) Re-run ratchet to harvest error buckets, (B) add test files, (C) add non-CPAN sources |
| **Space/time constraints?** | 207 MB current, 1.22s sweep; can expand to ~500 MB and 10s sweep before hitting practical limits |

