# ADR-0047: BDD Test Execution Integration for VS Code

## Status
Proposed

## Context

The perl-lsp VS Code extension currently supports Test::More/Test2 tests via `PerlTestAdapter` in the Test Explorer panel. Issue #3550 requests VS Code Test Explorer integration for Test::BDD::Cucumber `.feature` files, enabling BDD tests to appear in the Test Explorer with clickable results linking back to `.feature` files.

The existing codebase has substantial Gherkin infrastructure:
- `gherkinProviders.ts` provides document symbols, folding ranges, and step definition linking for `.feature` files
- `gherkinStepDefinitions.ts` provides step definition auto-generation
- The extension activates on `onLanguage:gherkin`
- `buildOutline()` in `gherkinProviders.ts` parses feature files into a hierarchical OutlineNode tree

However, critical verification findings reveal:
1. **Scenario Outline expansion is NOT implemented** — `buildOutline()` parses Examples tables but does not expand them into individual test items
2. **Background tracking is NOT implemented** — `buildOutline()` does not associate Background nodes with subsequent Scenario siblings
3. **BDD runner command is unverified** — `prove -l -r features/` is likely wrong; `prove` is designed for `.t` files
4. **JSON output format is unverified** — Test::BDD::Cucumber's default output is TAP-like, not JSON
5. **perl-tdd-support crate cannot be reused** — It is designed for `.t` files only

## Decision

### 1. Parallel BddTestAdapter Architecture

Create a new `BddTestAdapter` class in `src/bddTestAdapter.ts` as a separate class from `PerlTestAdapter`. This separation is justified because:
- Different file patterns (`*.t` vs `*.feature`)
- Different parsing logic (Gherkin vs TAP/subtest)
- Different test runners (prove for Perl tests, prove/yath/pdc for BDD)
- Different result parsing (TAP for Perl tests, TAP/JSON for BDD)

Shared parsing utilities will be extracted to `src/gherkin/parser.ts`.

### 2. Parser Utility Extraction and Enhancement

Extract `buildOutline()` and related parsing functions from `gherkinProviders.ts` into a new `src/gherkin/parser.ts` utility module. This shared utility will be used by both the existing Gherkin providers and the new BddTestAdapter.

**Required enhancements to the parser:**
- **Scenario Outline expansion**: Add `expandScenarioOutline(node: OutlineNode): OutlineNode[]` function that converts each Examples row into a separate outline node with modified label `<scenario name> (<row values>)`
- **Background tracking**: Track the nearest preceding Background node when building the test item hierarchy, associating each Background with subsequent Scenario siblings

### 3. TAP-Based Result Parsing (Primary)

Use TAP (Test Anything Protocol) parsing as the primary result parsing approach, mirroring the existing `PerlTestAdapter` pattern. This is the proven approach in the codebase and provides resilient error handling without depending on unverified JSON output.

### 4. Runner Command with Auto-Detection

Implement BDD test execution with auto-detection of the available runner:
1. Attempt `prove -lvr features/` (prove with lib, verbose, recursive)
2. Fall back to `yath` if prove fails
3. Report actionable error if no runner is available

**Note**: The exact command interface requires empirical verification with a real Test::BDD::Cucumber project. This ADR assumes `prove -lvr features/` as the starting point with the understanding that verification may reveal a different command is needed.

### 5. JSON Parsing as v2 Enhancement

Defer JSON output parsing to a v2 enhancement. The JSON format is unverified and may require specific version/configuration. Implement TAP parsing first, then add JSON as an enhancement once the basic flow works and JSON format is verified.

## Consequences

### Positive
- Fills a genuine DX gap for BDD-focused Perl teams
- Extends existing Gherkin infrastructure investment
- Mirrors established VS Code Test Controller API pattern
- TAP fallback ensures resilient error handling
- Scenario Outline expansion provides correct test granularity

### Negative
- Doubles the test adapter maintenance surface (two file watchers, two parsing paths, two runner interfaces)
- BDD runner command (`prove -lvr features/`) is unverified and needs empirical testing
- Background step tracking requires explicit implementation not present in current `buildOutline()`
- Sets precedent for feature additions outside roadmap cycle

### Tradeoffs
- **Complexity vs Reuse**: Separate adapter adds code but avoids mixing concerns
- **TAP vs JSON**: TAP is proven but less structured; JSON is more structured but unverified
- **Roadmap vs Feature Value**: Feature is valuable but outside current v0.12.x roadmap focus

## Alternatives Considered

### Alternative 1: Extend PerlTestAdapter to Handle Both .t and .feature Files
**Rejected**: Different file patterns, parsing logic, and runners justify separate classes. Mixing BDD and TAP concerns increases complexity and maintenance burden. The existing PerlTestAdapter is well-tested; adding BDD logic risks regressions.

### Alternative 2: JSON-Only Result Parsing
**Rejected**: JSON output format from Test::BDD::Cucumber is unverified. TAP parsing is the proven approach in existing PerlTestAdapter. JSON parsing can be added as v2 enhancement once format is verified.

### Alternative 3: Reuse perl-tdd-support Rust Crate
**Rejected**: The perl-tdd-support crate's TestRunner is designed for `.t` files only (discovers `test_*` functions, runs via `prove`). BDD-specific semantics are not supported. BddTestAdapter must be implemented in TypeScript like PerlTestAdapter.

### Alternative 4: JSON as Primary with TAP Fallback
**Rejected**: Given the unverified status of JSON output, depending on it as primary is risky. TAP is proven and provides reliable fallback.

## Dependencies

- **perl-lsp vscode-extension**: All implementation is in TypeScript within the VS Code extension
- **perl-tdd-support**: Not applicable — BDD tests cannot use this crate
- **Test::BDD::Cucumber**: Must be installed in the Perl environment
- **prove or yath**: Required runner (auto-detected)
- **VS Code Test Controller API**: Uses existing TestController and TestItem APIs