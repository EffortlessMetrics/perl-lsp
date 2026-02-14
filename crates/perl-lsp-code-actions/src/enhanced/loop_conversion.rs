//! Loop conversion code actions

use crate::types::{CodeAction, CodeActionEdit, CodeActionKind};
use perl_lsp_rename::TextEdit;
use perl_parser_core::ast::{Node, NodeKind};

/// Convert old-style for loops to modern foreach
pub fn convert_loop_style(node: &Node, source: &str) -> Option<CodeAction> {
    if let NodeKind::For { init, condition, update, body, .. } = &node.kind {
        // Check if it's a C-style for loop that can be converted
        if let Some(converted) = try_convert_c_style_loop(init, condition, update, body, source) {
            return Some(CodeAction {
                title: "Convert to foreach loop".to_string(),
                kind: CodeActionKind::RefactorRewrite,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit { location: node.location, new_text: converted }],
                },
                is_preferred: false,
            });
        }
    }

    // Check for foreach that could be improved
    if let NodeKind::Foreach { variable, list, body, continue_block } = &node.kind {
        // Check if using implicit $_
        if let NodeKind::Variable { name, sigil } = &variable.kind
            && name == "_"
            && sigil == "$"
        {
            let list_text = &source[list.location.start..list.location.end];
            let body_text = &source[body.location.start..body.location.end];

            return Some(CodeAction {
                title: "Use explicit loop variable instead of $_".to_string(),
                kind: CodeActionKind::RefactorRewrite,
                diagnostics: Vec::new(),
                edit: CodeActionEdit {
                    changes: vec![TextEdit {
                        location: node.location,
                        new_text: format!("foreach my $item ({}) {}", list_text, body_text),
                    }],
                },
                is_preferred: false,
            });
        }
    }

    None
}

/// Try to convert C-style for loop to foreach
pub fn try_convert_c_style_loop(
    init: &Option<Box<Node>>,
    condition: &Option<Box<Node>>,
    update: &Option<Box<Node>>,
    body: &Node,
    source: &str,
) -> Option<String> {
    // Pattern: for (my $i = 0; $i < @array; $i++)
    // Convert to: foreach my $item (@array)

    // Check that we have all parts of a C-style for loop
    if let (Some(init), Some(condition), Some(update)) = (init, condition, update) {
        // Check if init is "my $i = 0" pattern
        if let NodeKind::VariableDeclaration { variable, initializer, .. } = &init.kind {
            // Get iterator variable name
            if let NodeKind::Variable { name: iter_name, .. } = &variable.kind {
                // Check if initialized to 0
                if let Some(init_val) = initializer
                    && let NodeKind::Number { value } = &init_val.kind
                    && value == "0"
                    // Check if condition is $i < @array or $i < scalar @array
                    && let NodeKind::Binary { op, left, right } = &condition.kind
                    && op == "<"
                    // Check if left is our iterator variable
                    && let NodeKind::Variable { name: left_name, .. } = &left.kind
                    && left_name == iter_name
                {
                    // Check if right is an array
                    let array_name = if let NodeKind::Variable { sigil, name, .. } = &right.kind {
                        if sigil == "@" { Some(format!("@{}", name)) } else { None }
                    } else if let NodeKind::FunctionCall { name: func_name, args } = &right.kind {
                        if func_name == "scalar" && args.len() == 1 {
                            if let NodeKind::Variable { sigil, name, .. } = &args[0].kind {
                                if sigil == "@" { Some(format!("@{}", name)) } else { None }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(array) = array_name {
                        // Check if update is $i++ or ++$i
                        let is_increment = match &update.kind {
                            NodeKind::Unary { op, operand } => {
                                matches!(op.as_str(), "++" | "--")
                                    && if let NodeKind::Variable { name, .. } = &operand.kind {
                                        name == iter_name
                                    } else {
                                        false
                                    }
                            }
                            _ => false,
                        };

                        if is_increment {
                            // Replace array subscripts in body with $item
                            let body_text = &source[body.location.start..body.location.end];
                            let modified_body = body_text
                                .replace(
                                    &format!("${}[${}", array.trim_start_matches('@'), iter_name),
                                    "$item",
                                )
                                .replace(
                                    &format!("${}[${}]", array.trim_start_matches('@'), iter_name),
                                    "$item",
                                );

                            return Some(format!("foreach my $item ({}) {}", array, modified_body));
                        }
                    }
                }
            }
        }
    }

    None
}
