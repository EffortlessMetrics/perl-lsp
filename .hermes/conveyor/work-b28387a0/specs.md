# Specification: BDD Test Execution Integration

## Feature Description

Enable VS Code Test Explorer integration for Test::BDD::Cucumber `.feature` files, allowing developers to run BDD tests directly from VS Code with results displayed in the Test Explorer panel. Feature files appear under a dedicated test controller, scenarios are listed as individual tests, and failed tests link back to the specific line in the `.feature` file.

## Non-Goals

This specification does NOT include:
- Debug actions for BDD tests (Run profile only, no Debug)
- Installation verification of Test::BDD::Cucumber
- Integration with other BDD frameworks (Test::BDD::Cucumber only)
- Running individual steps within a scenario
- Test::BDD::Cucumber JSON output parsing (deferred to v2)
- Integration with perl-tdd-support Rust crate (not applicable for BDD)

## Feature Hierarchy

### Test Explorer Structure
```
Perl Tests (PerlTestController)
└── *.t files → subtests (existing)

BDD Tests (BddTestController) [NEW]
└── features/*.feature
    ├── Feature: Login
    │   ├── Background (executed before each scenario)
    │   ├── Scenario: successful login
    │   ├── Scenario: failed login with wrong password
    │   └── Scenario Outline: login with users
    │       ├── Examples: user <user1> (expanded)
    │       ├── Examples: user <user2> (expanded)
    │       └── Examples: user <user3> (expanded)
```

## Acceptance Criteria

### AC1: Feature File Discovery
`.feature` files are discovered via file system watcher on `**/*.feature` and appear in the VS Code Test Explorer under the BddTestController.

### AC2: Scenario Individual Tests
Each Scenario within a Feature file is listed as an individual test item in the Test Explorer.

### AC3: Scenario Outline Expansion
Scenario Outlines are expanded into individual test items, one per Examples row. Each expanded test has a label combining the Scenario name and the Example values (e.g., "Scenario Outline: login with users (user1)").

### AC4: Background Step Execution
Background steps are executed before each Scenario in the Feature (or Rule), correctly handling Gherkin semantics. The Background steps are prepended to each scenario's execution context.

### AC5: Test Execution via Detected Runner
Tests execute via an auto-detected BDD runner (prove/yath/pdc). The adapter attempts runners in order and reports an actionable error if none are available.

### AC6: Pass/Fail Status Display
Test results display pass/fail status in the Test Explorer, with failed tests showing error messages.

### AC7: Failed Test Navigation
Clicking a failed test opens the corresponding `.feature` file at the line of the failed step, using `vscode.TestMessage` with location set to the step's line number.

### AC8: Configuration Option for Runner Preference
A configuration option `perl.bddRunner` allows users to specify preferred runner (`auto`, `prove`, `yath`, `pdc`), defaulting to `auto`.

## Technical Implementation

### File Structure
```
vscode-extension/src/
├── testAdapter.ts          # Existing PerlTestAdapter (unchanged)
├── bddTestAdapter.ts       # NEW: BddTestAdapter class
└── gherkin/
    ├── parser.ts            # NEW: Extracted from gherkinProviders.ts
    ├── providers.ts        # MODIFIED: Use shared parser
    └── stepDefinitions.ts  # Existing (unchanged)
```

### Key Classes

#### BddTestAdapter
```typescript
export class BddTestAdapter {
  private controller: vscode.TestController;
  private runnerProfile: vscode.TestRunProfile;
  private featureWatcher: vscode.FileSystemWatcher;
  private runProfile: vscode.TestRunProfile;

  // Discovery
  discoverFeatureFiles(): Promise<void>;
  parseFeatureFile(uri: vscode.Uri): Promise<OutlineNode>;
  createTestItems(node: OutlineNode, parent: vscode.TestItem): void;

  // Execution
  runHandler(request: vscode.TestRunRequest, token: CancellationToken): void;
  executeTests(cmd: string, cwd: string): Promise<string>;
  parseTAPOutput(output: string): TAPResult[];

  // Expansion
  expandScenarioOutline(node: OutlineNode): OutlineNode[];

  // Background
  trackBackground(node: OutlineNode): BackgroundNode | null;
  prependBackgroundToScenario(scenario: OutlineNode, bg: BackgroundNode): void;
}
```

#### Parser Utility (gherkin/parser.ts)
```typescript
export interface OutlineNode {
  kind: 'feature' | 'scenario' | 'background' | 'examples' | 'step';
  label: string;
  location?: Location;
  children: OutlineNode[];
  // For Scenario Outlines:
  examples?: ExamplesTable;
  // For Background tracking:
  isBackground?: boolean;
}

export function buildOutline(document: TextDocument): OutlineNode;
export function expandScenarioOutline(node: OutlineNode): OutlineNode[];
export function getBackgroundForScenario(
  scenario: OutlineNode,
  tree: OutlineNode
): BackgroundNode | null;
```

### Data Flow

1. **Discovery**: File watcher detects `*.feature` → `parseFeatureFile()` → `buildOutline()` → `createTestItems()`
2. **Expansion**: If Scenario Outline → `expandScenarioOutline()` creates one node per Examples row
3. **Background**: When creating test items, `getBackgroundForScenario()` retrieves applicable Background
4. **Execution**: User clicks Run → `runHandler()` → `executeTests()` → parse TAP output → update test items
5. **Navigation**: Failed step → `vscode.TestMessage` with location → VS Code opens file at line

### Configuration

```json
{
  "perl.bddRunner": {
    "type": "string",
    "enum": ["auto", "prove", "yath", "pdc"],
    "default": "auto",
    "description": "Runner for BDD tests. 'auto' detects available runner."
  },
  "perl.bddFeaturePattern": {
    "type": "string",
    "default": "**/*.feature",
    "description": "Glob pattern for discovering .feature files."
  }
}
```

## Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| VS Code | ^1.75.0 | Test Controller API |
| Test::BDD::Cucumber | any | BDD framework |
| prove (Perl::Harness) | any | Primary test runner |
| yath | any | Alternative runner |

## Edge Cases

| Case | Handling |
|------|----------|
| No .feature files in workspace | Adapter creates empty controller, no items in Test Explorer |
| No BDD runner available | Show actionable error message listing missing runners |
| Feature file outside workspace | Use `vscode.workspace.getWorkspaceFolder(uri)` for correct cwd |
| Background after Scenario | Invalid Gherkin; parser should handle gracefully (Background ignored) |
| Empty Examples table | Scenario Outline produces zero test items |
| Unicode in scenario names | Test item labels handle UTF-8 |
| Running single scenario | Support via `request.tests` containing single scenario URI |
| Cancellation | Respect `CancellationToken` to abort running tests |

## Verification

1. **Unit tests**: `src/test/bddTestAdapter.test.ts` with mocked VS Code APIs
2. **Integration tests**: Real .feature files in test fixture workspace
3. **Manual verification**:
   - Open workspace with `.feature` files
   - Verify Test Explorer shows Feature/Scenario hierarchy
   - Run scenario and verify pass/fail reporting
   - Click failed test and verify navigation to correct line