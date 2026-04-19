# Task List — work-b28387a0: BDD Test Execution Integration

## Phase 1: Foundation
- [ ] Extract `buildOutline()` and related parsing from `gherkinProviders.ts` into `src/gherkin/parser.ts`
- [ ] Add `expandScenarioOutline()` function to expand Examples rows into individual nodes
- [ ] Add Background tracking logic to associate Background nodes with subsequent Scenarios
- [ ] Create `src/bddTestAdapter.ts` skeleton `BddTestAdapter` class
- [ ] Wire `BddTestAdapter` into `extension.ts` (similar to `PerlTestAdapter` registration)

## Phase 2: Test Discovery
- [ ] Add file system watcher for `**/*.feature` in `BddTestAdapter`
- [ ] Implement feature file discovery and parsing using `buildOutline()`
- [ ] Create test items for Feature → Scenario hierarchy
- [ ] Handle Scenario Outline expansion (generate one test per Examples row)
- [ ] Verify Background steps are correctly associated with scenarios

## Phase 3: Test Execution
- [ ] Implement `runHandler()` for test run requests
- [ ] Implement runner auto-detection (prove → yath → pdc fallback)
- [ ] Execute tests via detected BDD runner
- [ ] Add TAP output parsing for test results
- [ ] Map TAP results to test items (pass/fail/error)
- [ ] Handle cancellation via `CancellationToken`

## Phase 4: Error Reporting
- [ ] Map failed steps to line numbers in `.feature` files
- [ ] Create `vscode.TestMessage` with location linking to failure line
- [ ] Implement test navigation (click failed test → open file at line)
- [ ] Verify Background steps are prepended to scenario execution

## Phase 5: Polish
- [ ] Add refresh handler for manual test re-discovery
- [ ] Handle workspace folder variations (multi-root workspace support)
- [ ] Add configuration for BDD runner preference (`perl.bddRunner`)
- [ ] Add configuration for feature file pattern (`perl.bddFeaturePattern`)
- [ ] Add unit tests in `src/test/bddTestAdapter.test.ts`

## Verification
- [ ] `just devex` passes (build + unit tests)
- [ ] Manual testing: `.feature` files appear in Test Explorer
- [ ] Manual testing: Scenario Outlines expand to individual tests
- [ ] Manual testing: Failed test navigation opens correct line