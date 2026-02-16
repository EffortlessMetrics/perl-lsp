#!/bin/bash

# Update all Foreach pattern matches to include continue_block
files=(
    "crates/perl-semantic-analyzer/src/analysis/symbol.rs"
    "crates/perl-semantic-analyzer/src/analysis/scope_analyzer.rs"
    "crates/perl-semantic-analyzer/src/analysis/semantic.rs"
    "crates/perl-refactoring/src/refactor/refactoring.rs"
    "crates/perl-lsp-navigation/src/type_definition.rs"
    "crates/perl-incremental-parsing/src/incremental/incremental_document.rs"
    "crates/perl-lsp-code-actions/src/enhanced/mod.rs"
    "crates/perl-lsp-code-actions/src/enhanced/loop_conversion.rs"
    "crates/perl-workspace-index/src/workspace/workspace_index.rs"
    "crates/perl-lsp/src/features/inlay_hints_provider.rs"
    "crates/perl-lsp/src/features/document_highlight.rs"
    "crates/perl-lsp/src/call_hierarchy_provider.rs"
    "crates/perl-parser/tests/prop_complete_programs.rs"
    "crates/perl-parser/tests/prop_round_trip.rs"
    "crates/perl-parser/tests/prop_test_utils.rs"
    "crates/perl-parser/tests/prop_invariants.rs"
    "crates/perl-parser/tests/prop_corpus_invariants.rs"
    "crates/perl-parser/tests/parser_continue_redo_tests.rs"
)

for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        echo "Updating $file..."
        sed -i 's/Foreach { variable, list, body }/Foreach { variable, list, body, continue_block }/g' "$file"
        # Add handling of continue_block in the body
        sed -i '/NodeKind::Foreach { variable, list, body, continue_block } => {/,/}/ {
            /self\.visit_node_for_tests(body/ a\
                if let Some(cb) = continue_block {\
                    self.visit_node_for_tests(cb, tests);\
                }
            /self\.analyze_node(body/ a\
                if let Some(cb) = continue_block {\
                    self.analyze_node(cb, scope_id);\
                }
            /f(body)/ a\
                if let Some(cb) = continue_block {\
                    f(cb);\
                }
            /f(variable);/ a\
                if let Some(cb) = continue_block {\
                    f(cb);\
                }
            /self\.cache_node(body)/ a\
                if let Some(cb) = continue_block {\
                    self.cache_node(cb);\
                }
            /self\.collect_actions_for_range(body/ a\
                if let Some(cb) = continue_block {\
                    self.collect_actions_for_range(cb, range, actions);\
                }
            /Visit list with outer scope/ a\
                if let Some(cb) = continue_block {\
                    self.visit_node(cb);\
                }
            /Iterator is a write context/ a\
                if let Some(cb) = continue_block {\
                    self.visit_node(cb);\
                }
            /self\.visit_node(variable/ a\
                if let Some(cb) = continue_block {\
                    self.visit_node(cb, hints, range);\
                }
            /Some(vec!\[variable.as_ref(), list.as_ref(), body.as_ref()\])/ a\
                if let Some(cb) = continue_block {\
                    Some(vec![variable.as_ref(), list.as_ref(), body.as_ref(), cb.as_ref()])\
                } else {\
                    Some(vec![variable.as_ref(), list.as_ref(), body.as_ref()])\
                }
            /if let Some(result) = f(variable)/ a\
                if let Some(cb) = continue_block {\
                    if let Some(result) = f(cb) {\
                        results.push(result);\
                    }\
                }
            /f(variable)?;/ a\
                if let Some(cb) = continue_block {\
                    f(cb)?;\
                }
            /extract_shape_rec(variable/ a\
                if let Some(cb) = continue_block {\
                    extract_shape_rec(cb, out);\
                }
            /check_spans_rec(variable/ a\
                if let Some(cb) = continue_block {\
                    check_spans_rec(cb, source_len, errors);\
                }
            /f(variable)?;/ a\
                if let Some(cb) = continue_block {\
                    f(cb)?;\
                }
        }' "$file"
    fi
done

echo "All files updated!"