use proptest::prelude::*;

use super::qw::identifier;

/// Generate a small integer literal
fn small_int() -> impl Strategy<Value = i32> {
    1i32..10
}

/// Generate a loop label
fn label() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("OUTER".to_string()),
        Just("INNER".to_string()),
        Just("LOOP".to_string()),
        identifier().prop_map(|s| s.to_uppercase()),
    ]
}

/// Generate a loop control keyword
fn loop_control() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("next"), Just("last"), Just("redo"),]
}

/// Generate a condition expression
fn condition() -> impl Strategy<Value = String> {
    prop_oneof![
        (identifier(), small_int()).prop_map(|(var, n)| format!("${} < {}", var, n)),
        (identifier(), small_int()).prop_map(|(var, n)| format!("${} > {}", var, n)),
        (identifier(), small_int()).prop_map(|(var, n)| format!("${} == {}", var, n)),
        (identifier(), small_int()).prop_map(|(var, n)| format!("${} != {}", var, n)),
        identifier().prop_map(|var| format!("defined ${}", var)),
        identifier().prop_map(|var| format!("${}", var)),
    ]
}

/// Generate a simple statement for loop body
fn body_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        identifier().prop_map(|var| format!("${}++;", var)),
        identifier().prop_map(|var| format!("${}--;", var)),
        (identifier(), small_int()).prop_map(|(var, n)| format!("${} += {};", var, n)),
        identifier().prop_map(|var| format!("print ${};", var)),
        Just("print $_;".to_string()),
    ]
}

/// Generate a while loop with optional control statements
fn while_loop() -> impl Strategy<Value = String> {
    (
        identifier(),
        small_int(),
        condition(),
        body_statement(),
        prop::option::of((loop_control(), condition())),
    )
        .prop_map(|(var, init, cond, body, ctrl)| {
            let control = ctrl.map(|(kw, c)| format!("\n    {} if {};", kw, c)).unwrap_or_default();
            format!("my ${} = {};\nwhile ({}) {{\n    {}{}\n}}\n", var, init, cond, body, control)
        })
}

/// Generate an until loop
fn until_loop() -> impl Strategy<Value = String> {
    (identifier(), small_int(), condition(), body_statement()).prop_map(
        |(var, init, cond, body)| {
            format!("my ${} = {};\nuntil ({}) {{\n    {}\n}}\n", var, init, cond, body)
        },
    )
}

/// Generate a for loop (C-style)
fn for_c_style() -> impl Strategy<Value = String> {
    (identifier(), small_int(), small_int(), body_statement()).prop_map(
        |(var, start, end, body)| {
            format!(
                "for (my ${} = {}; ${} < {}; ${}++) {{\n    {}\n}}\n",
                var, start, var, end, var, body
            )
        },
    )
}

/// Generate a foreach loop
fn foreach_loop() -> impl Strategy<Value = String> {
    (
        identifier(),
        identifier(),
        small_int(),
        small_int(),
        body_statement(),
        prop::option::of((loop_control(), condition())),
    )
        .prop_map(|(var, _arr, start, end, body, ctrl)| {
            let control = ctrl.map(|(kw, c)| format!("\n    {} if {};", kw, c)).unwrap_or_default();
            format!("for my ${} ({}..{}) {{\n    {}{}\n}}\n", var, start, end, body, control)
        })
}

/// Generate a foreach with continue block
fn foreach_with_continue() -> impl Strategy<Value = String> {
    (identifier(), small_int(), small_int(), body_statement(), body_statement()).prop_map(
        |(var, start, end, body, cont_body)| {
            format!(
                "for my ${} ({}..{}) {{\n    {}\n}} continue {{\n    {}\n}}\n",
                var, start, end, body, cont_body
            )
        },
    )
}

/// Generate a labeled loop with control
fn labeled_loop() -> impl Strategy<Value = String> {
    (label(), identifier(), small_int(), small_int(), body_statement(), loop_control()).prop_map(
        |(lbl, var, start, end, body, ctrl)| {
            format!(
                "{}: for my ${} ({}..{}) {{\n    {}\n    {} {} if ${} == {};\n}}\n",
                lbl,
                var,
                start,
                end,
                body,
                ctrl,
                lbl,
                var,
                (start + end) / 2
            )
        },
    )
}

/// Generate nested labeled loops
fn nested_labeled_loops() -> impl Strategy<Value = String> {
    (
        label(),
        label(),
        identifier(),
        identifier(),
        small_int(),
        loop_control(),
    )
        .prop_map(|(outer, inner, var1, var2, n, ctrl)| {
            format!(
                "{}: for my ${} (1..{}) {{\n    {}: for my ${} (1..{}) {{\n        {} {} if ${} == ${};\n    }}\n}}\n",
                outer, var1, n, inner, var2, n, ctrl, outer, var1, var2
            )
        })
}

/// Generate a postfix loop
fn postfix_loop() -> impl Strategy<Value = String> {
    (identifier(), small_int(), body_statement())
        .prop_map(|(var, limit, body)| {
            prop_oneof![
                Just(format!("my ${} = 0;\n{} while ${} < {};\n", var, body, var, limit)),
                Just(format!("my ${} = {};\n{} until ${} < 1;\n", var, limit, body, var)),
            ]
        })
        .prop_flat_map(|s| s)
}

/// Generate a given/when statement
fn given_when() -> impl Strategy<Value = String> {
    (identifier(), small_int(), small_int(), small_int()).prop_map(|(var, val, case1, case2)| {
        format!(
            "use v5.10;\nmy ${} = {};\ngiven (${}) {{\n    when ({}) {{ print \"case1\"; }}\n    when ({}) {{ print \"case2\"; }}\n    default {{ print \"other\"; }}\n}}\n",
            var, val, var, case1, case2
        )
    })
}

/// Generate a try/catch/finally block
fn try_catch() -> impl Strategy<Value = String> {
    (identifier(), identifier()).prop_map(|(msg, err_var)| {
        format!(
            "try {{\n    die \"{}\";\n}} catch (${}) {{\n    warn ${};\n}} finally {{\n    print \"done\";\n}}\n",
            msg, err_var, err_var
        )
    })
}

/// Generate an eval block
fn eval_block() -> impl Strategy<Value = String> {
    (identifier(), identifier()).prop_map(|(result, msg)| {
        format!(
            "my ${} = eval {{\n    die \"{}\";\n    1;\n}};\nwarn $@ if !${};\n",
            result, msg, result
        )
    })
}

/// Generate a do block
fn do_block() -> impl Strategy<Value = String> {
    (identifier(), small_int(), small_int()).prop_map(|(var, a, b)| {
        format!("my ${} = do {{\n    my $x = {};\n    $x + {};\n}};\n", var, a, b)
    })
}

/// Generate a defer block (Perl 5.36+)
fn defer_block() -> impl Strategy<Value = String> {
    (identifier(), identifier()).prop_map(|(func, msg)| {
        format!(
            "use v5.36;\nuse feature 'defer';\nno warnings 'experimental::defer';\nsub {} {{\n    defer {{ print \"{}\\n\"; }}\n    return 1;\n}}\n",
            func, msg
        )
    })
}

/// Generate loop control samples (next/redo/continue).
pub fn loop_with_control() -> impl Strategy<Value = String> {
    prop_oneof![
        while_loop(),
        until_loop(),
        for_c_style(),
        foreach_loop(),
        foreach_with_continue(),
        labeled_loop(),
        nested_labeled_loops(),
        postfix_loop(),
        given_when(),
        try_catch(),
        eval_block(),
        do_block(),
        defer_block(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn control_flow_contains_keywords(code in loop_with_control()) {
            assert!(
                code.contains("next")
                    || code.contains("last")
                    || code.contains("redo")
                    || code.contains("continue")
                    || code.contains("for")
                    || code.contains("while")
                    || code.contains("until")
                    || code.contains("given")
                    || code.contains("when")
                    || code.contains("try")
                    || code.contains("catch")
                    || code.contains("finally")
                    || code.contains("defer")
                    || code.contains("eval")
                    || code.contains("do"),
                "Expected loop control keyword in: {}",
                code
            );
        }

        #[test]
        fn while_loops_are_valid(code in while_loop()) {
            assert!(code.contains("while"));
            assert!(code.contains("my $"));
        }

        #[test]
        fn foreach_loops_have_iterator(code in foreach_loop()) {
            assert!(code.contains("for my $"));
        }

        #[test]
        fn labeled_loops_have_label(code in labeled_loop()) {
            assert!(code.contains(":"));
            assert!(code.contains("for my $"));
        }
    }
}
