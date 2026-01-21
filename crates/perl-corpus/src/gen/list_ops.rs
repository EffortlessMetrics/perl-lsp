use proptest::prelude::*;

use super::qw::identifier;

fn array_name() -> impl Strategy<Value = String> {
    identifier()
}

fn map_block() -> impl Strategy<Value = String> {
    (array_name(), array_name()).prop_map(|(src, dest)| {
        format!(
            "my @{} = map {{ $_ * 2 }} @{};\n",
            dest, src
        )
    })
}

fn map_empty_block() -> impl Strategy<Value = String> {
    (array_name(), array_name()).prop_map(|(src, dest)| {
        format!(
            "my @{} = map {{ }} @{};\n",
            dest, src
        )
    })
}

fn grep_block() -> impl Strategy<Value = String> {
    (array_name(), array_name()).prop_map(|(src, dest)| {
        format!(
            "my @{} = grep {{ $_ % 2 == 0 }} @{};\n",
            dest, src
        )
    })
}

fn grep_empty_block() -> impl Strategy<Value = String> {
    (array_name(), array_name()).prop_map(|(src, dest)| {
        format!(
            "my @{} = grep {{ }} @{};\n",
            dest, src
        )
    })
}

fn sort_block() -> impl Strategy<Value = String> {
    (array_name(), array_name()).prop_map(|(src, dest)| {
        format!(
            "my @{} = sort {{ $a <=> $b }} @{};\n",
            dest, src
        )
    })
}

fn sort_simple() -> impl Strategy<Value = String> {
    (array_name(), array_name()).prop_map(|(src, dest)| {
        format!(
            "my @{} = sort @{};\n",
            dest, src
        )
    })
}

/// Generate map/grep/sort list-operator statements.
pub fn list_op_in_context() -> impl Strategy<Value = String> {
    prop_oneof![
        map_block(),
        map_empty_block(),
        grep_block(),
        grep_empty_block(),
        sort_block(),
        sort_simple(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn list_ops_include_keyword(code in list_op_in_context()) {
            assert!(
                code.contains("map") || code.contains("grep") || code.contains("sort"),
                "Expected list op keyword in: {}",
                code
            );
        }
    }
}
