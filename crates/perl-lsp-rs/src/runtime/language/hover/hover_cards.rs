use super::*;

fn method_modifier_description(modifier_kind: &str) -> &'static str {
    match modifier_kind {
        "before" => "runs **before** the method — use for preconditions and logging",
        "after" => "runs **after** the method — use for postconditions and cleanup",
        "around" => {
            "wraps the method — receives `$orig` as first arg, must call `$orig->($self, @_)`"
        }
        "override" => "overrides the parent method — use to replace inherited behavior",
        "augment" => "extends the parent method — call `inner()` to invoke the next layer",
        _ => "modifies the method",
    }
}

pub(super) fn method_modifier_hover(modifier_kind: &str, method_name: &str, doc: &str) -> Value {
    let kind_label = method_modifier_description(modifier_kind);
    json!({
        "contents": {
            "kind": "markdown",
            "value": format!(
                "**Method Modifier (`{modifier_kind}`)**\n\n`{method_name}` — {kind_label}\n\n{doc}"
            ),
        },
    })
}

/// Return the timing semantics description for a Perl phase block.
///
/// Returns `None` for unrecognised phase names so callers can fall through.
fn phase_block_description(phase: &str) -> Option<(&'static str, &'static str)> {
    // Returns (short_timing, full_description)
    match phase {
        "BEGIN" => Some((
            "Compile-time execution",
            "Runs **as soon as the block is fully parsed**, at **compile time**, \
before the rest of the file compiles. \
Multiple `BEGIN` blocks run in declaration order (FIFO). \
Use for loading modules, setting up the environment, and other tasks that \
must complete before compilation continues.",
        )),
        "END" => Some((
            "Program-exit cleanup",
            "Runs at **program exit** (after the main program finishes or on `exit()`), \
in **reverse declaration order** (LIFO). \
Not called on abnormal exit via `exec` or unhandled signals. \
Use for cleanup tasks such as removing temporary files.",
        )),
        "INIT" => Some((
            "Post-compile, pre-runtime startup",
            "Runs at the **start of runtime**, after all compilation completes \
and before the main program body executes, in **declaration order** (FIFO). \
Caveat: `INIT` may be unreliable when invoked via `require` or string `eval` \
after the main compile phase.",
        )),
        "CHECK" => Some((
            "End-of-compilation hook",
            "Runs at the **end of compilation** (after all compilation, before runtime), \
in **reverse declaration order** (LIFO). \
Caveat: `CHECK` may be unreliable when invoked via `require` or string `eval` \
after the main compile phase.",
        )),
        "UNITCHECK" => Some((
            "End-of-compilation-unit hook",
            "Runs at the end of the **compilation unit** it is defined in \
(more localised and reliable than `CHECK`), in **reverse declaration order** (LIFO). \
Unlike `CHECK`, `UNITCHECK` fires for each `require`d or `eval`'d compilation unit \
independently.",
        )),
        _ => None,
    }
}

/// Build a hover card for a Perl phase block (`BEGIN`, `END`, `INIT`, `CHECK`, `UNITCHECK`).
///
/// Returns `None` when `phase` is not one of the recognised phase keywords.
pub(super) fn phase_block_hover(phase: &str) -> Option<Value> {
    let (timing, description) = phase_block_description(phase)?;
    Some(json!({
        "contents": {
            "kind": "markdown",
            "value": format!(
                "**Phase Block: `{phase}`**\n\n_{timing}_\n\n{description}\n\n\
    [perlmod — Special Subroutines](https://perldoc.perl.org/perlmod#BEGIN%2C-UNITCHECK%2C-CHECK%2C-INIT-and-END)"
            ),
        },
    }))
}
