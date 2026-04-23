//! Prompt construction from inline completion context.

use crate::providers::inline_completion::PreparedInlineCompletionContext;

/// Build an OpenAI-compatible prompt from the prepared context.
///
/// Returns a `(system, user)` message pair suitable for the chat completions API.
pub fn build_fim_prompt(context: &PreparedInlineCompletionContext) -> (String, String) {
    let mut system = String::from(
        "You are a Perl code completion assistant. Complete the code at the cursor position. \
         Return ONLY the completion text, no explanation, no markdown.",
    );

    // Add context about the current scope
    if let Some(ref pkg) = context.current_package {
        system.push_str(&format!("\nCurrent package: {pkg}"));
    }
    if let Some(ref func) = context.current_function {
        system.push_str(&format!("\nInside subroutine: {func}"));
    }
    if !context.imports.is_empty() {
        system.push_str(&format!("\nImported modules: {}", context.imports.join(", ")));
    }

    // Build the user message with the code context
    let mut user = String::new();
    if let Some(ref prev) = context.previous_non_empty_line {
        user.push_str(prev);
        user.push('\n');
    }
    user.push_str(&context.prefix);
    user.push_str("<CURSOR>");

    (system, user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_prompt_includes_prefix() {
        let ctx = PreparedInlineCompletionContext {
            prefix: "my $x = ".to_string(),
            current_line: "my $x = ".to_string(),
            previous_non_empty_line: Some("use strict;".to_string()),
            current_function: Some("new".to_string()),
            current_package: Some("MyClass".to_string()),
            variables: vec!["$self".to_string()],
            imports: vec!["strict".to_string()],
        };
        let (system, user) = build_fim_prompt(&ctx);
        assert!(system.contains("Perl"));
        assert!(system.contains("MyClass"));
        assert!(system.contains("new"));
        assert!(user.contains("my $x = "));
        assert!(user.contains("<CURSOR>"));
        assert!(user.contains("use strict;"));
    }
}
