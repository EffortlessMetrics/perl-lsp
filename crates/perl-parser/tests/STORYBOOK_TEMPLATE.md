# Storybooking Template

Use this template before adding or refactoring an end-to-end user-story test.
Keep it lightweight: a few lines of planning should be enough to make the test
read clearly and stay focused on observable LSP behavior.

## Story Card

- **Story name**: `test_user_story_<capability>_<outcome>`
- **User goal**: _What is the Perl developer trying to accomplish?_
- **Protocol methods**: _Which LSP request(s)/notification(s) are involved?_
- **Coverage note**: _What gap, issue, or acceptance criterion does this close?_

## Given / When / Then

### Given
- Minimal workspace layout:
- Perl source snippet(s):
- Any configuration or indexing preconditions:

### When
- Editor action or request sequence:
- Incremental edits or follow-up requests:

### Then
- Primary editor-visible outcome:
- Secondary stable fields worth asserting:
- Negative-path expectation or sibling unhappy-path test:

## Assertion Plan

- [ ] One core assertion block per `Then`
- [ ] Assert only the protocol fields under test
- [ ] Include at least one failure-mode check
- [ ] Keep fixtures deterministic and minimal
- [ ] Update README coverage notes if the story becomes active

## Rust Test Skeleton

```rust
#[test]
fn test_user_story_<capability>_<outcome>() {
    // Story: <one-sentence user goal>
    // Given: <workspace + source setup>
    // When: <LSP request sequence>
    // Then: <editor-visible expectation>
}
```
