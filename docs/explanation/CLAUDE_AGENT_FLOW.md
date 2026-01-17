# Claude Agent PR Review Flow Design

This document defines the improved Claude Code agent flow for PR reviews with local verification and GitHub integration.

## Flow Overview

```
pr-initial-reviewer → [test-runner-analyzer → context-scout → pr-cleanup-agent]* → pr-finalize → pr-merger → pr-doc-finalize
```

## Agent Definitions

### 1. pr-initial-reviewer (Entry Point)
**Purpose**: Fast initial analysis to catch obvious issues and direct the review flow.

**Updated Description**:
```
Use this agent when a pull request is first opened or when new commits are pushed to an existing PR. This agent provides fast, cost-effective initial analysis to catch obvious issues early and guides the next steps in the review process.

The agent will:
- Perform quick static analysis of changes
- Identify potential issues (compilation, style, obvious bugs)
- Leave GitHub comments summarizing findings
- Recommend next agent in the flow based on analysis

Flow Direction:
- If tests are likely to fail: → test-runner-analyzer
- If architecture understanding needed: → context-scout  
- If obvious fixes needed: → pr-cleanup-agent
- If changes look clean: → pr-finalize

Examples: 
<example>Context: New PR opened with parser changes
user: "PR #123 just opened with regex parsing improvements"
assistant: "I'll use pr-initial-reviewer to analyze the changes and determine the review path"
<commentary>New PR needs initial triage to determine review approach</commentary></example>

<example>Context: New commits pushed to existing PR
user: "Just pushed 3 commits addressing reviewer feedback"  
assistant: "Let me run pr-initial-reviewer to assess the updates and guide next steps"
<commentary>Updated PR needs re-evaluation to continue review flow</commentary></example>
```

### 2. test-runner-analyzer (Verification Loop)
**Purpose**: Local test execution and failure analysis with GitHub status updates.

**Updated Description**:
```
Use this agent for local test execution, diagnosis of test failures, and verification that changes work correctly. Focuses on local verification instead of relying on GitHub CI due to billing concerns.

The agent will:
- Run comprehensive test suites locally (cargo test, cargo check, etc.)
- Analyze and diagnose any failures in detail
- Update GitHub PR status with test results
- Recommend next steps based on test outcomes

Flow Direction:
- If tests pass: → pr-finalize
- If tests fail with clear fixes: → pr-cleanup-agent
- If tests fail needing context: → context-scout
- If tests fail but PR has potential: Push branch, update status, pause for later

Examples:
<example>Context: PR needs verification after initial review
user: "Initial review suggests testing the LSP changes"
assistant: "I'll use test-runner-analyzer to run the LSP test suite and verify functionality"
<commentary>Need to verify specific functionality works correctly</commentary></example>

<example>Context: After cleanup, need to verify fixes
user: "Applied fixes for compilation errors, need to verify"
assistant: "Using test-runner-analyzer to run tests and confirm fixes resolved the issues"
<commentary>Verification step in cleanup loop</commentary></example>
```

### 3. context-scout (Intelligence Gathering)
**Purpose**: Targeted reconnaissance to understand code context for informed decisions.

**Updated Description**:  
```
Use this agent when you need to quickly understand existing code patterns, architecture, or implementation details to make informed review decisions. Focuses on gathering intelligence needed for the current PR review.

The agent will:
- Map existing implementations and patterns relevant to PR changes
- Understand architectural context and dependencies
- Identify potential integration points or conflicts
- Provide actionable insights for the review process

Flow Direction:
- After gathering context: → pr-cleanup-agent (if fixes needed)
- After gathering context: → test-runner-analyzer (if verification needed)
- After gathering context: → pr-finalize (if just needed understanding)

Examples:
<example>Context: Complex PR changing parser architecture
user: "PR modifies core parsing logic, need to understand impact"
assistant: "I'll use context-scout to map the parsing architecture and identify all affected components"
<commentary>Need architectural understanding before making review decisions</commentary></example>

<example>Context: Test failures need context to fix
user: "Tests failing but unclear why, need to understand the codebase"
assistant: "Using context-scout to understand the failing test context and identify root causes"
<commentary>Test failures require codebase knowledge to resolve</commentary></example>
```

### 4. pr-cleanup-agent (Issue Resolution)
**Purpose**: Comprehensive issue resolution with GitHub integration.

**Updated Description**:
```
Use this agent to systematically address all PR issues including test failures, reviewer feedback, compilation errors, and documentation gaps. Integrates with GitHub to leave comments and track progress.

The agent will:
- Address reviewer feedback systematically
- Fix compilation errors and test failures  
- Update documentation as needed
- Leave GitHub comments explaining changes made
- Coordinate with other agents for complex issues

Flow Direction:
- After fixes applied: → test-runner-analyzer (verify fixes work)
- If needs more context: → context-scout (understand before fixing)
- If all issues resolved: → pr-finalize
- If issues too complex: Update GitHub status, push progress, pause

Examples:
<example>Context: Reviewer feedback needs comprehensive response
user: "Got reviewer feedback about LSP implementation issues"
assistant: "I'll use pr-cleanup-agent to systematically address all feedback and coordinate fixes"
<commentary>Comprehensive issue resolution with multiple reviewer points</commentary></example>

<example>Context: Tests failing after initial analysis
user: "Tests are failing and need fixes before merge"
assistant: "Using pr-cleanup-agent to diagnose and fix all test failures systematically"
<commentary>Need systematic approach to resolve multiple test issues</commentary></example>
```

### 5. pr-finalize (NEW - Pre-merge Preparation)
**Purpose**: Final review preparation and merge readiness assessment.

**Description**:
```
Use this agent when a PR has passed testing and issue resolution, and needs final preparation for merge. Performs comprehensive final checks and prepares merge strategy.

The agent will:
- Perform final code quality checks
- Verify all tests pass and no regressions introduced
- Review API contracts and breaking changes
- Prepare merge commit message and release notes
- Ensure documentation is up to date
- Verify branch is ready for merge

Flow Direction:
- If ready for merge: → pr-merger
- If issues found: → pr-cleanup-agent
- If need more testing: → test-runner-analyzer

Examples:
<example>Context: PR has passed all checks, ready for final review
user: "PR looks good after cleanup, ready for final review"
assistant: "I'll use pr-finalize to perform comprehensive final checks and prepare for merge"
<commentary>PR needs final validation before merge execution</commentary></example>
```

### 6. pr-merger (Updated - Merge Execution)
**Purpose**: Execute merge with documentation updates and release coordination.

**Updated Description**:
```
Use this agent to execute the actual merge after all reviews are complete and the PR is finalized. Handles merge execution, immediate documentation updates, and release coordination.

The agent will:
- Execute the merge with appropriate merge strategy
- Update immediate technical documentation (API changes, etc.)
- Create/update release notes if needed
- Trigger any immediate post-merge actions
- Coordinate with pr-doc-finalize for comprehensive doc updates

Flow Direction:
- After successful merge: → pr-doc-finalize (optional, for comprehensive doc improvements)
- If merge fails: → pr-cleanup-agent (resolve conflicts)

Examples:
<example>Context: PR is finalized and ready for merge
user: "PR #42 is ready to merge after final review"
assistant: "I'll use pr-merger to execute the merge and handle immediate post-merge tasks"
<commentary>Execute the actual merge and immediate follow-up tasks</commentary></example>
```

### 7. pr-doc-finalize (NEW - Documentation Enhancement)
**Purpose**: Post-merge documentation improvement using Diataxis format.

**Description**:
```
Use this agent after a successful merge to comprehensively update and improve documentation related to the merged changes. Follows Diataxis documentation framework for systematic improvement.

The agent will:
- Update documentation affected by the merged changes
- Improve related documentation using Diataxis framework (tutorials, how-tos, reference, explanations)
- Identify and fix documentation gaps discovered during PR review
- Ensure documentation consistency across the project
- Create follow-up documentation issues if needed

Flow Direction:
- Completes the PR review cycle
- May identify follow-up work for future PRs

Examples:
<example>Context: PR merged with LSP feature additions
user: "LSP feature PR merged, need documentation updates"
assistant: "I'll use pr-doc-finalize to update all LSP documentation and improve related docs using Diataxis structure"
<commentary>Comprehensive post-merge documentation improvement</commentary></example>
```

## Flow Control Logic

### Agent Decision Matrix

| Current State | Test Status | Context Needed | Issues Present | Next Agent |
|---------------|-------------|----------------|----------------|------------|
| Initial Review | Unknown | No | Obvious issues | pr-cleanup-agent |
| Initial Review | Unknown | Yes | Any | context-scout |
| Initial Review | Unknown | No | None obvious | test-runner-analyzer |
| Context Gathered | - | - | Issues found | pr-cleanup-agent |
| Context Gathered | - | - | Need verification | test-runner-analyzer |
| Context Gathered | - | - | No issues | pr-finalize |
| Tests Running | Pass | - | None | pr-finalize |
| Tests Running | Fail | High | Present | context-scout |
| Tests Running | Fail | Low | Present | pr-cleanup-agent |
| Cleanup Done | - | - | - | test-runner-analyzer |
| Finalized | - | - | Ready | pr-merger |
| Merged | - | - | - | pr-doc-finalize |

### GitHub Integration Points

1. **pr-initial-reviewer**: Leaves summary comment with recommended next steps
2. **test-runner-analyzer**: Updates PR status checks and comments on test results
3. **pr-cleanup-agent**: Comments on each issue addressed, references specific commits
4. **pr-finalize**: Leaves final approval comment with merge recommendation
5. **pr-merger**: Updates PR with merge details and any immediate follow-ups
6. **pr-doc-finalize**: Comments on documentation improvements made

### Flexibility Guidelines

- Agents should provide recommendations, not rigid requirements
- Allow for human override at any decision point
- Support jumping to different agents when context changes
- Enable pause/resume workflow for complex issues
- Support parallel agent execution when appropriate

### Error Handling

- If agent encounters insurmountable issues: Update GitHub status, push current progress, create detailed handoff comment
- Each agent should leave clear guidance for next steps even if unable to complete
- Support resuming from any point in the flow
- Graceful degradation when GitHub API unavailable

## Implementation Notes

- All agents should have access to GitHub API for status updates and commenting
- Local verification is preferred over CI for cost reasons
- Each agent should provide clear handoff information to the next agent
- Support both automated and manual flow direction
- Maintain audit trail of all agent decisions and actions