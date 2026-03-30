# The Best Perl LSP: Comprehensive Vision Document

**Research Completed**: 2026-03-19
**Vision Scope**: v0.12.0 (now) → v1.0.0 (production leadership)
**Benchmark**: rust-analyzer, typescript-language-server, gopls

---

## Executive Summary

perl-lsp today is a **credible public alpha** with solid fundamentals:
- **100% LSP feature coverage** (97 features, all GA)
- **Sub-millisecond parsing** (931ns incremental)
- **72% CPAN clean parse rate** (3139/4355 top-1000 modules)
- **90% parser test coverage** across native Perl 5.8-5.40 syntax
- **Native DAP debugger** with breakpoints, stepping, variables

The path to "the best Perl LSP" requires **leadership in three dimensions**:

### 1. **Real-World Reliability** (Current Gap)
- 72% CPAN clean → **95%+ CPAN clean** (parser hardening)
- No Moose/Moo/Class::Accessor intelligence → **Full DSL coverage**
- Basic static analysis → **Real perlcritic integration + dead-code detection**

### 2. **Perl-Specific Superpowers** (Competitive Edge)
- Auto-module resolution (we have bare `use` detection)
- **Perldoc hover** (click a function, get CPAN documentation)
- **@INC resolution** (find installed modules, understand PERL5LIB)
- **Debugger that just works** (perl -d integration, one-click debug)
- **Cross-file refactoring** (rename a package, rename imports everywhere)

### 3. **Platform & Development Ecosystem** (Replication Value)
- The **swarm methodology** as a replicable pattern for building complex tools in parallel
- **Feature governance via `features.toml`** — proof that declarative capability tracking scales
- **Corpus-driven development** — public evidence that you can build Perl tooling on real CPAN code, not labs
- **Rust library form** (`perl-parser` as an embeddable, published crate) — anyone can build on it

---

## Section 1: What Makes Competitors Best-in-Class?

### rust-analyzer Leadership Model

**Incremental Computation** (Salsa-based)
- Changes flow through the query graph; only affected nodes recompute
- Time: ~20-50ms full project recheck on medium codebases
- Payoff: IDE never blocks; real-time type info

**Proc Macro Expansion**
- Rust macros are not syntax sugar; they're compile-time plugins
- rust-analyzer expands them and tracks hygiene, bindings, and error scopes
- Payoff: Developers understand what the IDE sees vs. what the compiler sees

**Type Inference Pipeline** (Chalk-based)
- Entire project types are inferred in parallel; results cached
- Failure modes are clear (unresolved type → red squiggle + explanation)
- Payoff: Trust in diagnostics; fewer false positives

**Assist Ecosystem** (Code Actions at Scale)
- 100+ assists covering common refactors, completions, and transformations
- Each assist is testable, composable, and has clear scope
- Payoff: IDE feels like a teammate, not a tool

**MIR-Based Diagnostics**
- Translates to mid-level IR to catch patterns earlier (borrow checker-like issues)
- Payoff: Warnings feel semantic, not syntactic

**Inlay Hints + Semantic Tokens**
- Parameter names on function calls, inferred types on variables, scope highlights
- Payoff: Code clarity without noise; developers control detail level

---

### typescript-language-server Leadership Model

**Type Inference Across Projects**
- Infers types even in files that don't have explicit annotations
- Handles generic inference, union narrowing, type guards
- Payoff: Completion knows what `.foo` returns even without JSDoc

**Refactoring Operations**
- Extract function, extract variable, move symbol to file, rename across projects
- All respect module boundaries, imports, and exports
- Payoff: Developers refactor with confidence; IDE updates imports automatically

**Auto-Import with Module Resolution**
- Knows about node_modules, monorepos, barrel exports
- Completion includes: "Add import for X? Yes / No"
- Payoff: Zero manual import overhead

**Call & Type Hierarchy**
- Jump to callers, jump to implementations, see inheritance chain
- Works across npm packages and TypeScript versions
- Payoff: Understand code flow and architecture without grep

**Project References & Composite Builds**
- Multi-project workspaces with clear boundaries
- Changes ripple correctly; builds happen in dependency order
- Payoff: LSP becomes part of build, not just editor polish

---

### gopls Leadership Model

**Fast Semantic Analysis**
- 100ms full-package type checking for most code
- Incremental checking for file-level changes
- Payoff: No lag on save; diagnostics instant

**Symbol Documentation**
- Hover on any symbol → full godoc for that symbol with examples
- Works across vendored, external, and builtin packages
- Payoff: IDE is your documentation

**Import Management**
- Organizes imports (unused removal, standard library grouping)
- "Add missing import" code action
- Payoff: Never think about import management again

**Build System Integration**
- Understands go.mod, build tags, vendored code
- Respects GOPATH and go modules
- Payoff: Works in any Go project without configuration

**Call Graph Analysis**
- Find all callers of a function
- Trace callstacks; find dead code
- Payoff: Understand impact of changes before refactoring

---

## Section 2: What Would Make perl-lsp Best-in-Class FOR PERL?

### Dimension 1: Real-World Reliability

#### Parser + Semantic Robustness

**Today**: 72% CPAN clean
**Done**: 95%+ CPAN clean (parser hardens on real projects)

What must happen:
- Fix 5,000+ remaining parse failures in CPAN top-1000 without regressions
- Hang-risk candidates must be eliminated or bounded
- Edge cases (heredocs, regex quoting, interpolation) must be bulletproof

**Business case**: A Perl developer opens their project in perl-lsp; diagnostics are trustworthy, not noisy. No "internal parser error" surprises.

#### Moose/Moo/Class::Accessor Intelligence

**Today**: Parser recognizes these frameworks syntactically
**Done**: LSP understands attributes, roles, inheritance, method modifiers

What must happen:
- `use Moose` → Recognize `has`, `extends`, `with`, method modifiers
- Hover on `my $obj->foo` → Find the `has 'foo'` declaration
- Rename `has 'foo'` → Updates all references to `$obj->foo`, `$self->foo`, etc.
- Go-to-definition for `with 'Some::Role'` → Find the role file

**Business case**: Moose is pervasive in production Perl. Not understanding it is a blocker for enterprise adoption.

#### use/require + @INC Resolution

**Today**: Bare `use` detection (import diagnostics)
**Done**: Full module resolution with inheritance

What must happen:
- `use parent 'Parent::Class'` → Hover shows parent methods
- Find references to a sub → Checks parent classes too
- Go-to-definition for inherited sub → Finds it in parent module
- Resolver respects PERL5LIB, local lib, cpanm-installed modules

**Business case**: Developers understand code flow across package boundaries. Refactoring is safe.

#### Perlcritic Integration

**Today**: None (parsing only)
**Done**: Real static analysis diagnostics

What must happen:
- `use strict; my $x = 1; $x;` → "Useless use of variable in void context"
- `qw(foo bar)` style violations → Warnings with quick-fix
- Bareword issues, regex safety, complexity metrics
- Full perlcritic rules catalog, configurable per project (.perlcriticrc)

**Business case**: Perl developers get real warnings, not just parse errors. Helps them write better code.

#### Dead-Code Detection

**Today**: None
**Done**: Identify unused subs, unused lexicals, unreachable code

What must happen:
- `sub never_called { }` → Gray highlight + "Dead code" diagnostic
- `my $unused = 1;` → Warning + quick-fix (remove)
- Respect main package exports (subs called by name strings)
- Understand &EXPORT, &EXPORT_OK, %EXPORT_TAGS

**Business case**: Refactoring becomes safe. Developers know which code to delete.

---

### Dimension 2: Perl-Specific Superpowers

#### Perldoc on Hover

**Today**: None
**Done**: Click a builtin function, see the perldoc

What must happen:
- Hover on `print` → Show perldoc for `print`
- Hover on `Cwd::abs_path` → Show perldoc for abs_path
- Integrate with installed CPAN modules (via `perldoc` or parsed docs)
- Search CPAN if not locally installed

**Business case**: Perl documentation is rich but scattered. LSP bridges that gap.

Example:
```perl
my @files = glob("*.pl");  # Hover → perldoc -f glob
use Cwd;
my $dir = abs_path(".");   # Hover → perldoc Cwd::abs_path
```

#### Test Runner Integration

**Today**: None
**Done**: Run tests from the IDE

What must happen:
- Gutter button: "Run this test file" → Opens integrated terminal with `prove`
- Gutter button: "Run this test function" → `prove -m` on the specific test
- Inline test results: show pass/fail/SKIP next to each test
- Capture output in a panel below the editor

**Business case**: Test development becomes IDE-first. TDD workflows feel native.

#### Cross-File Refactoring

**Today**: Rename works; imports are basic
**Done**: Rename propagates through inheritance, roles, and exports

What must happen:
- Rename sub `foo` in Package::A
- Automatically rename all `&foo` calls and `&Package::A::foo` references
- Update any `use Package::A qw(foo)` to `use Package::A qw()` (or remove import)
- Rename a class → Rename in `use parent`, `@ISA`, `use base`

**Business case**: Safe refactoring at scale. Developers can restructure without fear.

#### CPAN Integration Layer

**Today**: None
**Done**: Completion shows CPAN modules, with version info

What must happen:
- Completion on `use ` → Lists installed modules + top 100 CPAN modules by downloads
- Show module version, author, last-update date
- Quick-fix: "Add missing module to cpanfile?" (for dependency management)
- Dependency resolution: "This module requires X version Y; install?"

**Business case**: Perl's CPAN is its superpower. Make it discoverable.

---

### Dimension 3: Debugger That Just Works

#### One-Click Debug

**Today**: DAP works; requires setup
**Done**: Open any .pl file, press "Run and Debug", it works

What must happen:
- Detect if file is executable Perl or a test
- Auto-configure PERL5LIB, include paths, and shebang handling
- Respect local::lib and cpanm-installed modules
- Breakpoints just work (no recompile cycle)

**Business case**: Perl developers who've never used a debugger can debug in the IDE. Lower barrier to adoption.

#### Live Variable Inspection

**Today**: Works; basic
**Done**: Hover over any variable, see live value with deep inspection

What must happen:
- `my $obj = Foo->new()` → Hover shows object properties
- `@array` → Shows all elements with indices
- `%hash` → Shows all k/v pairs
- Nested structures: click to expand deeply

**Business case**: Debugging becomes intuitive. "What's in this variable right now?"

#### Conditional Breakpoints

**Today**: Hit-count breakpoints exist
**Done**: Expression-based breakpoints with full power

What must happen:
- `break if $x > 100` → Breakpoint with condition
- Conditions can reference any variable in scope
- Condition failures don't crash debugger

**Business case**: Developers can debug faster without manual loops.

---

## Section 3: Feature Maturity Scorecard

### Current State (v0.12.0 alpha)

| Category | Feature | Status | Maturity | Notes |
|----------|---------|--------|----------|-------|
| **Parsing** | Perl 5.8-5.40 syntax | GA | 95% | 72% CPAN clean |
| **LSP Core** | 97 features | GA | 100% | All advertised features implemented |
| **Diagnostics** | Syntax + parse errors | GA | 80% | Missing: perlcritic, dead code, use warnings |
| **Navigation** | Go-to-def, references, rename | GA | 85% | Missing: cross-inheritance, @INC resolution |
| **Completion** | Symbols, builtins, keywords | GA | 80% | Missing: CPAN module completion |
| **Formatting** | Perl::Tidy integration | GA | 90% | Missing: config-per-project |
| **Semantics** | Workspace symbol index | GA | 70% | Missing: Moose/Moo intelligence |
| **Debugging** | DAP native adapter | GA | 80% | Missing: expression evaluation, locals |
| **Perl-specific** | Test runner, perldoc, @INC | Preview | 20% | All missing |

### Roadmap to Leadership (v0.15.0)

| Milestone | Parser % | Diagnostics | Refactoring | Debugging | Perl Features | Target Release |
|-----------|----------|-------------|-------------|-----------|--------------|-----------------|
| **v0.12.0** (now) | 72% CPAN | Syntax only | Rename | Basic DAP | None | Current |
| **v0.13.0** | 85% CPAN | +Perlcritic, dead code | +Cross-file | +Evaluate | +Perldoc | Q2 2026 |
| **v0.14.0** | 92% CPAN | +Warnings, strict | +Full refactor | +Locals, watch | +Test runner, CPAN search | Q3 2026 |
| **v0.15.0** | 97% CPAN | All diagnostics | Full refactoring | Full debugger | Full Perl integration | Q4 2026 |

---

## Section 4: Competitive Positioning

### Head-to-Head: perl-lsp vs. PerlNavigator (VS Code)

| Dimension | perl-lsp | PerlNavigator | Winner |
|-----------|----------|---------------|--------|
| **Speed** | Native, <50ms | Perl + LSP | perl-lsp |
| **Parser coverage** | 72% CPAN, 100% repo | Unknown, estimate 60% | perl-lsp |
| **No Perl runtime needed** | Yes | No (requires Perl) | perl-lsp |
| **LSP features** | 100% (97/97) | ~60% (estimate) | perl-lsp |
| **Debugger** | Native DAP | Perl::LanguageServer | perl-lsp |
| **Moose intelligence** | Syntax only (0%) | Unknown | Unknown |
| **Perlcritic integration** | None (0%) | Likely 80%+ | PerlNavigator |
| **Test runner integration** | None (0%) | Likely 50%+ | PerlNavigator |
| **CPAN module search** | None (0%) | Likely 30%+ | PerlNavigator |

**perl-lsp's Advantage**: Architecture (native, no runtime) and debugger.
**perl-lsp's Gap**: Perl-specific features and static analysis.

**Strategy**: Close the gaps by v0.14.0, then perl-lsp becomes the obvious choice.

---

## Section 5: The "Done" Criteria

### Quantitative Metrics

#### Parser Quality
- [ ] **95%+ CPAN clean** (4,000+/4,355 modules parse without errors or timeouts)
- [ ] **99%+ repo corpus clean** (all checked-in test files parse cleanly)
- [ ] **100% Perl 5 syntax coverage** (all constructs in the spec work)
- [ ] **Zero hangs** (no infinite loops or timeouts on real code)

#### LSP Feature Completeness
- [ ] **100% LSP 3.18 support** (97/97 features, all tested)
- [ ] **<50ms response time** for completion, hover, go-to-def on medium projects
- [ ] **<500ms startup time** (including workspace index build)
- [ ] **<100MB memory** for typical projects

#### Semantic Coverage
- [ ] **Moose/Moo/Class::Accessor** fully understood (inheritance, roles, attributes)
- [ ] **use parent / use base** resolves correctly
- [ ] **@INC resolution** finds installed modules
- [ ] **Dead-code detection** works on real projects
- [ ] **Perlcritic** integration with configurable rules

#### Debugger Robustness
- [ ] **100% DAP 1.0 feature parity** with Perl debugger
- [ ] **Breakpoints, stepping, stack, scopes, variables** all work
- [ ] **Expression evaluation** in watch expressions
- [ ] **One-click debug** for .pl files (auto-config PERL5LIB, etc.)

---

### Qualitative Metrics

#### User Sentiment
- [ ] "I closed PerlNavigator and use perl-lsp full-time"
- [ ] "This is the best Perl IDE I've used"
- [ ] "Finally, Perl tooling feels modern"
- [ ] Survey: 4.5+/5.0 stars on VSCode marketplace

#### Community Adoption
- [ ] 10,000+ downloads/month on crates.io
- [ ] 100+ GitHub stars
- [ ] 20+ active issue contributors
- [ ] 5+ external PRs/month from community

#### Contributor Health
- [ ] 10+ active contributors
- [ ] <2 week PR review time
- [ ] <1 week issue response time
- [ ] Clear, up-to-date contributing guide

---

### Competitive Benchmarks

#### vs. PerlNavigator
- [ ] Faster (native vs. Perl+LSP)
- [ ] No runtime dependency (users don't need Perl installed)
- [ ] Better debugger (native DAP vs. Perl::LanguageServer)
- [ ] Better parser (95%+ vs. 60% CPAN clean, estimate)

#### vs. rust-analyzer (as a gold standard)
- [ ] >90% LSP feature parity (97/97 features vs. 90+)
- [ ] Similar or faster response times (<50ms)
- [ ] Similar or better memory profile (<100MB)
- [ ] Parser as robust as rust-analyzer's (97% vs. 99.9%)

---

## Section 6: The Roadmap to Leadership

### Phase 1: v0.13.0 (Q2 2026) — Diagnostics & Static Analysis

**Goal**: Go from "syntax checker" to "code analyzer"

**Deliverables**:
- [ ] Perlcritic integration (100+ rules, configurable)
- [ ] Dead-code detection (unused subs, unused lexicals)
- [ ] Strict/warnings analysis (catch common mistakes)
- [ ] Perldoc hover integration (builtin + CPAN docs)
- [ ] CPAN module completion & version info
- [ ] Corpus ratchet to 80%+ CPAN clean
- [ ] Parser hang-risk candidates eliminated

**Success Criteria**:
- Developers get real, actionable warnings (not just parse errors)
- perl-lsp becomes a code quality tool, not just an IDE helper

---

### Phase 2: v0.14.0 (Q3 2026) — Refactoring & Debugger Hardening

**Goal**: Safe, powerful refactoring and first-class debugging

**Deliverables**:
- [ ] Cross-file refactoring (rename with import updates)
- [ ] Extract function / extract variable (across files)
- [ ] Move code refactoring (module reorganization)
- [ ] Moose/Moo full intelligence (roles, attributes, inheritance)
- [ ] Test runner integration (gutter buttons, inline results)
- [ ] Live variable inspection in debugger
- [ ] Conditional breakpoints with expressions
- [ ] Corpus ratchet to 90%+ CPAN clean

**Success Criteria**:
- Developers can safely refactor large Perl projects
- Debugging feels as good as compilation in Rust projects

---

### Phase 3: v0.15.0 (Q4 2026) — Production Leadership

**Goal**: "The best Perl LSP" — period. For any Perl use case.

**Deliverables**:
- [ ] 97%+ CPAN clean parse rate
- [ ] All diagnostics (perlcritic, dead code, strict/warnings, deprecations)
- [ ] All refactoring operations (rename, extract, move, convert)
- [ ] Full debugger feature parity (DAP 1.0)
- [ ] Full Perl integration (@INC, CPAN search, test runner)
- [ ] Stability contract: API/protocol changes only in major versions
- [ ] Production support tiers (enterprise, hobbyist)

**Success Criteria**:
- Perl developers choose perl-lsp first. Always.
- PerlNavigator users consider switching.

---

### Phase 4: Beyond v1.0.0 (2027+) — Ecosystem Leadership

**Goal**: perl-lsp becomes foundational Perl infrastructure

**Deliverables**:
- [ ] `perl-parser` stable API (v1.0 in crates.io)
- [ ] `perl-lsp` embeddable as a library (not just a server)
- [ ] Integration with CPAN tooling (cpanm, carton, cpm)
- [ ] Plugin ecosystem (external code actions, diagnostics)
- [ ] Swarm methodology documentation (replicable pattern for tool building)
- [ ] Industry adoption: 50%+ of Perl developers use perl-lsp

---

## Section 7: The Swarm Methodology as Product

### What Makes This Worth Replicating?

perl-lsp was built with a **swarm of agents** (25-100 parallel workers) coordinating through:
- **Microcrate architecture** (isolation, parallelism)
- **Issue-driven handoffs** (knowledge transfer, no blocking)
- **Skill library** (reusable mechanics, not monolithic prompts)
- **Feature governance** (declarative `features.toml`)

**Insight**: This methodology scales to complex domains. It's not Perl-specific; it's a replicable pattern for building any ambitious tool.

### How to Position This

1. **Visibility**: Document the swarm workflow in `docs/SWARM_METHODOLOGY.md`
2. **Proof**: Show the git history: 100+ agents × 50+ PRs/session = rapid iteration
3. **Teachability**: Make agent templates and skills reusable for others
4. **Talk Circuit**: "Building Perl LSP with Swarm AI" at Perl conferences

**Business case**: Other language communities need this. perl-lsp becomes a reference implementation.

---

## Section 8: Distribution & Packaging

### Current State
- Single binary (cargo install perllsp)
- VS Code extension (auto-downloads binary)
- Manual setup for Neovim, Emacs, etc.

### Future State (v0.15.0)

- [ ] Official packages for distros (apt, brew, yum, chocolatey)
- [ ] Pre-built binaries for all platforms (Linux, macOS, Windows x86/ARM)
- [ ] Official VS Code extension (signed, marketplace listing)
- [ ] Official Neovim plugin (on GitHub, nvim-treesitter integration)
- [ ] Docker image (self-contained perl-lsp server)
- [ ] Kubernetes operator (for remote development)

**Success metric**: Perl developer installs perl-lsp in <2 minutes, any platform.

---

## Section 9: Documentation & Learning

### Today's Gaps
- "How do I debug a Perl project?" → No guide
- "How do I configure perlcritic rules?" → No docs
- "How do I set up perl-lsp in $EDITOR?" → Incomplete

### v0.15.0 Target

- [ ] Comprehensive getting-started guide (every editor)
- [ ] Feature-by-feature tutorial (with examples)
- [ ] "Debugging Perl the Modern Way" (video + written guide)
- [ ] API documentation for embedding perl-parser
- [ ] Configuration guide (all LSP options, all perlcritic rules)
- [ ] Troubleshooting guide (common issues, solutions)
- [ ] Architecture documentation (for contributors)

**Success metric**: New Perl developers can adopt perl-lsp without expert help.

---

## Section 10: The Honest Assessment

### What perl-lsp Already Does Better

1. **No runtime dependency** — Parser is native Rust, not Perl
2. **Incremental parsing** — 931ns per file, not O(n) per workspace
3. **Native debugger** — DAP-based, not Perl::LanguageServer bridge
4. **LSP feature completeness** — 100% vs. ~60% competitors
5. **Modular codebase** — 128 microcrates, not monolith

### What Competitors Do Better

1. **Real static analysis** — Perlcritic is decades mature
2. **Perl-specific integration** — They ship with Perl knowledge
3. **User base** — They have paying customers, community support
4. **Debugger UX** — Perl::LanguageServer has >10 years of hardening

### The Bet

**If perl-lsp closes the static analysis and Perl-integration gaps by v0.15.0, it becomes the clear winner.**

The gap isn't technical; it's effort. The architecture is proven. The foundation is solid. The only variable is prioritization and execution.

---

## Section 11: Success Metrics Dashboard

Track these quarterly:

| Metric | Q1 2026 | Q2 2026 | Q3 2026 | Q4 2026 | Target (v1.0) |
|--------|---------|---------|---------|---------|---------------|
| **CPAN Clean %** | 72% | 80% | 90% | 97% | 97%+ |
| **LSP Features** | 100% | 100% | 100% | 100% | 100% |
| **Diagnostics Count** | 5 | 50 | 100+ | 150+ | 200+ |
| **Response Time (ms)** | <50 | <50 | <40 | <40 | <40 |
| **Startup Time (ms)** | <500 | <500 | <400 | <400 | <400 |
| **Memory (MB)** | <100 | <100 | <100 | <100 | <100 |
| **Downloads/mo** | 1000 | 2000 | 5000 | 10000 | 10000+ |
| **GitHub Stars** | 300 | 500 | 800 | 1200 | 1500+ |
| **Active Contributors** | 5 | 8 | 12 | 15 | 20+ |

---

## Section 12: Conclusion

### The Case for perl-lsp Leadership

perl-lsp has the **foundation**:
- Modern architecture (Rust, no runtime)
- Complete LSP surface (100% features)
- Real performance (sub-millisecond parsing)
- Proven scalability (swarm methodology)

perl-lsp needs **focus**:
- Parser hardening (72% → 97% CPAN)
- Static analysis (perlcritic, dead code)
- Perl-specific UX (perldoc, @INC, test runner)
- Debugger polish (expressions, locals, conditions)

**Timeline**: 12 months to leadership, 18 months to dominance (by v0.15.0).

**ROI**: Perl community finally gets a modern IDE. perl-lsp becomes foundational infrastructure for the entire ecosystem.

### The Next Meeting

Recommend aligning on:

1. **v0.13.0 scope** — Which diagnostics first? Perlcritic, dead code, or warnings?
2. **Parser hardening budget** — How many agents on CPAN bugs vs. new features?
3. **Debugger roadmap** — Is expression evaluation or test runner integration more valuable?
4. **Community feedback loop** — How do we validate decisions with real Perl developers?

---

**Document prepared for strategy discussion.**
**See [ROADMAP.md](ROADMAP.md) and [CURRENT_STATUS.md](CURRENT_STATUS.md) for tactical details.**
