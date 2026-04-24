# Perl Deserves Better Tooling: Building A Modern LSP For A Language Everyone Forgot

*Published March 2026. This is not a sales piece. It is an honest accounting of where Perl tooling stands, why it got here, and what we built to change it.*

---

## The Gap

78% of Perl developers use no language server.

That number comes from the 2025 Perl IDE Survey (602 respondents). It is not that Perl developers have decided they prefer working without completions, navigation, and inline diagnostics. It is that nothing good enough existed to make the friction worth it.

Vim dominates at 28.4% of editors in use. Emacs follows. VSCode is growing. But across all of them, the majority of Perl developers open a file, write code, and find out about mistakes when they run it. The feedback loop that developers in other ecosystems have come to expect — type the wrong thing and see a red underline immediately — is simply absent for most Perl work.

Four language servers exist: Perl::LanguageServer (293K VSCode installs, last major update 27 months ago), PerlNavigator (53K installs, actively maintained, but Perl-based), PLS (9.7K installs, clean codebase, limited feature coverage), and coc-perl (a thin wrapper for Neovim). Combined, they reach roughly 350,000 installs — with significant overlap from developers who tried each one.

All three independent servers share one fundamental characteristic: they require Perl to function.

PerlNavigator calls `perl -c` for syntax checking. Perl::LanguageServer is written in Perl and requires CPAN modules. PLS is the same. This requirement creates a class of problems that no amount of feature development can address: the tool only works if your Perl installation is configured correctly, your dependencies are present, and your platform behaves like the author expected.

The 78% gap is largely explained by that friction.

---

## Why Perl Is Hard

Before explaining what we built, it is worth being honest about why building it was difficult. The problems are not engineering laziness. Perl is genuinely one of the hardest mainstream languages to parse statically.

Larry Wall famously said "only perl can parse Perl." He was not exaggerating.

Most programming languages have context-free grammars. You can write a formal grammar, generate a parser, and the output is deterministic. Perl is different at a fundamental level.

Consider the `/` character. In `$x / $y`, it is division. In `if (/error/)`, it is the start of a regex. In `$x /= 2`, it is division-assign. The same character has three completely different meanings, and the correct interpretation depends on what came before it in the token stream. You cannot tokenize Perl without simultaneously parsing it.

Then there is `{ }`. In `$ref = { key => 'value' }`, braces construct a hash reference. In `map { $_ * 2 } @list`, they are a block. In statement position, `{ ... }` is a bare block. The parser must decide which interpretation is correct based on context — context that a grammar generator cannot express.

`$/` looks like division. `$$` looks like a dereference. `$^W` looks like XOR. These are all valid Perl special variables that a naive parser would mangle.

Then there is the escape hatch that makes static analysis fundamentally incomplete: source filters. A module can call `use Filter::Simple` and transform the source code text before Perl's own parser sees it. At that point, a static parser is reading code that is not valid Perl — it is pre-filter text that only becomes valid Perl after a Perl module executes. You cannot statically analyze source-filtered code. This is not a parser bug. It is a property of the language.

We tried three different parsing approaches before landing on one that works.

**Tree-sitter** (v1) produces a C parser from a JavaScript grammar specification. It works brilliantly for Python, JavaScript, and Rust. It does not work for Perl. Tree-sitter grammars are context-free. Perl's grammar is context-sensitive. The `/` ambiguity alone defeats any context-free approach — you cannot express "this is a regex if the previous token was a keyword" in a context-free grammar.

**Pest** (v2) is a Parsing Expression Grammar library for Rust. PEGs are more powerful than context-free grammars — they support ordered choice and unlimited lookahead. We hoped this would be enough. It was not. PEGs parse top-down with backtracking but cannot carry parser state between alternatives. When the parser sees `{ ... }`, a PEG can try "parse as hash" then "parse as block," but it cannot carry the information "we just saw `sort`" into the choice. It took five days to reach this conclusion. Both approaches are kept in the repository for benchmark comparison.

**Recursive descent** (v3, current) works for Perl because it allows full stateful control. A `LexerMode` state machine tracks whether to expect a term or an operator, resolving the `/` ambiguity at the token level. The parser passes context through function arguments: when it sees `sort`, it calls `parse_builtin_block()` instead of the generic `parse_hash_or_block()`. Arbitrary lookahead handles indirect object syntax. IDE-friendly error recovery produces partial ASTs even mid-edit, so completions work when you are in the middle of typing a statement.

The tradeoff is maintenance cost. A generated parser gets correctness guarantees from its grammar definition. A hand-written parser must encode every rule manually. For Perl, that tradeoff is worth it.

---

## What perl-lsp Does

perl-lsp is a Language Server Protocol implementation for Perl, written in Rust. Every capability catalogued in `features.toml` — 119 entries spanning LSP, DAP, and perl-lsp extensions — has a wired-up implementation at GA maturity. That number is capability enumeration, not a claim about per-capability correctness, edge-case completeness, or subjective UX quality. Some capabilities are sharper than others, and we are transparent about which ones are still rough.

The parser does not call Perl. It is a hand-written recursive descent parser in Rust with a stateful lexer that handles Perl's context-sensitive grammar entirely in static analysis. A single native binary handles parsing, semantic analysis, and all LSP features.

The feature list is not a promise — it is tracked in `features.toml` with test coverage for each capability:

- **Navigation**: go to definition, find references, call hierarchy, type hierarchy, workspace symbols
- **Completion**: 150+ built-in Perl functions, workspace symbols across all project files, module names
- **Diagnostics**: static analysis, undefined subroutine detection, variable scope problems
- **Editing**: rename across the workspace, code actions, formatting (via Perl::Tidy integration), inlay hints
- **Semantic tokens**: syntax highlighting with semantic context (not just regex-based tokenization)
- **Code lens**: inline contextual information without cluttering the source view
- **Debugging**: a bundled DAP (Debug Adapter Protocol) server for step debugging

Compare this to PerlNavigator: go to definition, completion, diagnostics, some navigation. Perl::LanguageServer: similar, with debugging via `Devel::Perl5Db`. Both are production-tested tools. perl-lsp has more feature coverage, at the cost of being newer.

The codebase enforces a zero-panic policy in production code: no `unwrap()`, no `expect()`, no `panic!()`. The parser returns `Result` types throughout, with structured error recovery that produces partial ASTs rather than crashes. An LSP server that panics on malformed input is useless — recovering gracefully and continuing to provide completions for the rest of the file is the only acceptable behavior.

The CPAN corpus currently sits at 95.3% clean parses across 9,372 real files from the CPAN top-1000 distributions. That is a file-level clean parse rate — the share of files the parser processes without recording errors — not a measure of semantic AST fidelity, cross-file analysis, or any LSP-level correctness. It is a floor that says the parser does not choke on most real Perl, not a ceiling on what the LSP does with it. That number has been rising continuously — it was 50% a few months ago. More on how we got here below.

---

## What Makes It Different

The single biggest difference is the absence of a runtime dependency.

perl-lsp does not require Perl. Install the VSCode extension and it works. No `cpanm`, no `perl -v`, no configuration. No matching your Perl version to the extension's expectations. No wondering which CPAN modules need to be installed before the tool will start.

This matters most for three groups of developers:

**Front-end or full-stack developers** who maintain Perl backend code but do not have Perl set up as their primary development language. Currently, these developers get no tooling. With perl-lsp, they install one extension and get completions and navigation immediately.

**Developers on Windows**. PerlNavigator has known Windows limitations due to Perl runtime differences. Perl::LanguageServer does not support Windows at all. perl-lsp is a Rust binary that compiles identically on Linux, macOS, and Windows.

**Code reviewers**. PerlNavigator executes `BEGIN` blocks during syntax checking. That is fine for your own code where you trust the execution. For a third-party PR, executing arbitrary `BEGIN` blocks is a security risk. perl-lsp performs zero code execution. You can analyze untrusted Perl safely.

The architecture is also complete in a different sense. Because the parser is pure Rust with full AST output, features that require a complete syntax tree are possible: rename-across-files, call hierarchy, type hierarchy, semantic tokens. PPI (the parser underlying Perl::LanguageServer and PLS) deliberately produces a "document model" rather than a full AST. Rename and call hierarchy are not possible on top of a document model. They are not missing because of engineering inattention — they are missing because the foundation does not support them.

---

## The Features That Matter

Feature lists are easy to generate. Here are five things a Perl developer would notice immediately.

**Completion with import awareness.** When you `use List::Util qw(sum first any)`, the completion provider knows which functions you imported. Type `su` and you see `sum` from `List::Util` alongside Perl builtins — not every export from every installed module. The workspace index tracks what your code actually imports.

**Missing module detection.** If you write `use My::Module` and that module does not exist in the workspace, you get a diagnostic immediately. Not when you run the code. Not when CI fails. On the line where you typed it. This catches the fat-fingered module name before it becomes a debugging session.

**die to croak modernization.** A common code action suggestion: replace `die "error: $msg"` with `Carp::croak("error: $msg")`. `croak` reports errors from the caller's perspective, which is almost always what you want in library code. The code action fires when the diagnostic identifies a `die` in a function that is likely called from external code.

**POD sections in outline.** The document symbol provider includes POD documentation sections alongside subroutines and packages in the file outline. When you have a module with extensive documentation, navigating it from the outline panel works for both code and docs without switching between the file and perldoc output.

**Regex pattern snippets.** Typing a regex triggers snippet completions for common patterns: email validation, IP addresses, URL matching, date formats. These are not magic — they are common patterns with their flags documented. For developers who write Perl regularly, having them as completions reduces the time spent looking up regex syntax.

---

## The Corpus Story

Here is the honest version of how we got to 95.3%.

We started with a question: how do you know your parser works? Unit tests cover the constructs you thought to test. Real Perl is messier, more creative, and more surprising than any test author imagines. The answer is to test against reality.

The CPAN corpus is 9,372 Perl files from CPAN's top-1000 distributions. These are production modules written by hundreds of different authors — web frameworks, bioinformatics, Unicode processing, database access layers, testing infrastructure, everything. They represent real Perl as it is actually written.

Every PR runs the full corpus in CI. The baseline is ratcheted: the number of cleanly parsed files can only increase. If a change regresses parsing on a module that was previously clean, CI fails. New clean files are added to the baseline automatically after each fix wave.

Starting at 50% clean parses, we identified the top error buckets — families of related parse failures with common root causes. The largest bucket (`unexpected_token_in_expr`) decomposed into ten subcategories when examined carefully. Each subcategory drove a targeted fix. Each fix was validated against the CPAN corpus before merge.

Four months ago, recursive descent on Perl was a research question. Now 8,931 of 9,372 files from the CPAN top-1000 corpus — 95.3% — produce a clean AST with no recorded parse errors. The remaining 4.7% breaks down as: roughly 1-2% source-filtered code that is fundamentally incompatible with static analysis, 1-2% complex runtime-dependent constructs, and 1-2% genuinely fixable parser gaps we have not reached yet.

"Clean parse" here means exactly one thing: the parser read the file end to end without recording an error. It does not mean the resulting AST is semantically perfect, and it does not mean every downstream LSP feature behaves correctly on that file. It means the foundation is solid enough that the rest of the stack has something to work with.

The corpus is not a metric we report. It is a gate that every change must pass.

---

## What's Next

v0.13.0 ships with a 95.3% file-level clean parse rate on the CPAN top-1000 corpus, meeting the milestone target. The remaining fixable buckets are harder — complex expression nesting, postfix operator chains, ternary operator edge cases in deeply nested contexts — but they are bounded and well-characterized. v0.14.0 targets 98% or higher on the same file-level metric.

Beyond corpus coverage, three feature areas are in active development:

**Moose and Moo class intelligence.** Moose and Moo are the dominant OOP frameworks in modern Perl. Their DSL — `has`, `extends`, `with`, `before`, `after`, `around` — is used by thousands of CPAN modules. The current parser handles these as function calls. v0.13.0 will understand Moose class structure: attribute declarations, role composition, method modifiers. This enables go-to-definition from a `has` declaration to the accessor, hover showing the type constraint, and completion of Moose methods.

**Template::Toolkit support.** Template::Toolkit templates are a distinct format — `[% ... %]` syntax embedded in HTML or other text. The LSP server will gain basic syntax awareness for TT2/TT3 templates, reducing the noise of parse errors in template files.

**DBI SQL awareness.** Inline SQL strings in DBI calls (`$dbh->prepare("SELECT ...")`) are a natural candidate for embedded language support. Basic SQL syntax highlighting and maybe completions for column names in typed schemas.

Community feedback shapes which of these ships first. If you have a Perl codebase with specific pain points that none of the existing tools address, that is worth knowing.

---

## Try It

The VSCode extension is available in the marketplace. Search "perl-lsp" or install directly.

For other editors: the LSP binary (`perl-lsp`) implements the standard Language Server Protocol and works with any LSP-capable editor. Configuration for Neovim (via `nvim-lspconfig`), Emacs (via `lsp-mode` or `eglot`), Helix, and Zed is documented in `docs/EDITORS/`.

To report an issue or suggest a feature: the GitHub repository has an active issue tracker. If a specific CPAN module parses incorrectly, filing an issue with the module name is the fastest path to a fix — the corpus testing infrastructure means we can reproduce and validate the fix against the exact module.

Perl has a 35-year track record of doing what it was designed to do well. The language did not need better tooling to survive. But the developers writing Perl today deserve the same quality of IDE experience that developers in other languages take for granted. That is what this project is building toward.

---

*What the numbers measure. The 119 capability count (88 LSP + 24 DAP + 7 perl-lsp extensions) is verified against `features.toml` and means every catalogued capability has a wired-up implementation. The earlier 102 figure circulated briefly as a draft number until PR #4107's DAP catalog audit surfaced 14 uncatalogued handlers already implemented in `dispatch.rs`. The count does not measure per-capability correctness, edge-case completeness, or subjective UX quality. The 95.3% figure (8,931/9,372) is a file-level clean parse rate on the CPAN top-1000 corpus, reflecting the ratcheted CI baseline as of April 2026 — the share of files the parser processes without recording errors, not a measure of semantic AST fidelity, cross-file analysis, or any LSP-level correctness. End-to-end LSP correctness is not currently captured by any single automated metric; it is tracked through targeted test suites per capability. Install counts sourced from the VSCode Marketplace. All competitive analysis sourced from `docs/articles/COMPETITIVE_ANALYSIS.md`.*
