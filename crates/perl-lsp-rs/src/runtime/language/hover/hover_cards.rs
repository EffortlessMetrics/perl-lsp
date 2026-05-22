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
