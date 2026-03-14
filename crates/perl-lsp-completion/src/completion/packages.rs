//! Package member completion for Perl
//!
//! Provides completion for package members using workspace index integration.

use super::{
    context::CompletionContext,
    items::{CompletionItem, CompletionItemKind},
};
use perl_workspace_index::workspace_index::{
    SymbolKind as WsSymbolKind, WorkspaceIndex, WorkspaceSymbol,
};
use std::collections::HashSet;
use std::sync::Arc;

fn known_core_module_members(package_name: &str) -> &'static [(&'static str, &'static str)] {
    match package_name {
        "Cwd" => &[
            ("getcwd", "Return the current working directory."),
            ("abs_path", "Return the absolute path for a file or directory."),
            ("realpath", "Return the canonicalized path with symlinks resolved."),
        ],
        "Data::Dumper" => &[("Dumper", "Serialize Perl values into Perl source form.")],
        "Digest::MD5" => &[
            ("md5", "Compute the raw MD5 digest for the provided data."),
            ("md5_hex", "Compute the MD5 digest and return it as hexadecimal."),
            ("md5_base64", "Compute the MD5 digest and return it as base64."),
        ],
        "File::Basename" => &[
            ("basename", "Extract the filename portion from a path."),
            ("dirname", "Extract the directory portion from a path."),
            ("fileparse", "Split a pathname into filename, directory, and suffix."),
        ],
        "File::Spec" => &[
            ("catfile", "Join path parts into a platform-correct filename."),
            ("catdir", "Join path parts into a platform-correct directory."),
            ("splitpath", "Split a path into volume, directories, and file."),
            ("splitdir", "Split a directory path into its individual components."),
        ],
        "List::Util" => &[
            ("first", "Return the first value for which the block evaluates true."),
            ("max", "Return the largest value in a list."),
            ("min", "Return the smallest value in a list."),
            ("sum", "Return the numeric sum of the provided values."),
            ("reduce", "Fold a list into a single value using a block."),
            ("shuffle", "Return the list in randomized order."),
            ("uniq", "Return the unique values from the list in order."),
        ],
        "MIME::Base64" => &[
            ("encode_base64", "Encode binary data into base64 text."),
            ("decode_base64", "Decode base64 text into binary data."),
        ],
        "Scalar::Util" => &[
            ("blessed", "Return the package name if a reference is blessed."),
            ("looks_like_number", "Check whether a scalar behaves like a number."),
            ("reftype", "Return the underlying reference type."),
            ("weaken", "Turn a reference into a weak reference."),
        ],
        "Time::HiRes" => &[
            ("time", "Return the current time with sub-second resolution."),
            ("sleep", "Sleep for fractional seconds."),
            ("usleep", "Sleep for a number of microseconds."),
            ("gettimeofday", "Return the current time as seconds and microseconds."),
        ],
        _ => &[],
    }
}

fn known_core_member_documentation(package_name: &str, member_name: &str, summary: &str) -> String {
    format!(
        "Exported function `{package_name}::{member_name}` from core module `{package_name}`.\n\n{summary}\n\nSee `perldoc {package_name}`."
    )
}

fn fallback_member_documentation(package_name: &str, symbol: &WorkspaceSymbol) -> String {
    let qualified_name =
        symbol.qualified_name.clone().unwrap_or_else(|| format!("{package_name}::{}", symbol.name));

    match symbol.kind {
        WsSymbolKind::Export => {
            format!("Exported function `{qualified_name}` from package `{package_name}`.")
        }
        WsSymbolKind::Subroutine => {
            format!("Subroutine `{qualified_name}` defined in package `{package_name}`.")
        }
        WsSymbolKind::Method => {
            format!("Method `{qualified_name}` defined in package `{package_name}`.")
        }
        WsSymbolKind::Variable(_) => {
            format!("Package variable `{qualified_name}` declared in `{package_name}`.")
        }
        WsSymbolKind::Constant => {
            format!("Constant `{qualified_name}` declared in `{package_name}`.")
        }
        _ => format!("Package member `{qualified_name}` from `{package_name}`."),
    }
}

fn package_member_documentation(package_name: &str, symbol: &WorkspaceSymbol) -> Option<String> {
    symbol
        .documentation
        .clone()
        .or_else(|| Some(fallback_member_documentation(package_name, symbol)))
}

fn add_known_core_module_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    package_name: &str,
    member_prefix: &str,
) {
    let mut seen_labels: HashSet<String> =
        completions.iter().map(|item| item.label.clone()).collect();

    for (member_name, summary) in known_core_module_members(package_name) {
        if !member_name.starts_with(member_prefix)
            || !seen_labels.insert((*member_name).to_string())
        {
            continue;
        }

        completions.push(CompletionItem {
            label: (*member_name).to_string(),
            kind: CompletionItemKind::Function,
            detail: Some(package_name.to_string()),
            documentation: Some(known_core_member_documentation(
                package_name,
                member_name,
                summary,
            )),
            insert_text: Some((*member_name).to_string()),
            sort_text: Some(format!("2_{member_name}")),
            filter_text: Some((*member_name).to_string()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }
}

/// Add package member completions
pub fn add_package_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
) {
    // Only proceed if we have a workspace index to query
    let Some(index) = workspace_index else {
        return;
    };

    // Split the prefix into package name and member prefix
    let mut parts: Vec<&str> = context.prefix.split("::").collect();
    if parts.len() < 2 {
        return;
    }
    let member_prefix = parts.pop().unwrap_or("");
    let package_name = parts.join("::");

    // Query workspace index for members of the package
    let members = index.get_package_members(&package_name);
    for symbol in members {
        match symbol.kind {
            WsSymbolKind::Export | WsSymbolKind::Subroutine | WsSymbolKind::Method => {
                if symbol.name.starts_with(member_prefix) {
                    completions.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: CompletionItemKind::Function,
                        detail: Some(package_name.clone()),
                        documentation: package_member_documentation(&package_name, &symbol),
                        insert_text: Some(symbol.name.clone()),
                        sort_text: Some(format!("1_{}", symbol.name)),
                        filter_text: Some(symbol.name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                    });
                }
            }
            WsSymbolKind::Variable(_) => {
                if symbol.name.starts_with(member_prefix) {
                    completions.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: CompletionItemKind::Variable,
                        detail: Some(package_name.clone()),
                        documentation: package_member_documentation(&package_name, &symbol),
                        insert_text: Some(symbol.name.clone()),
                        sort_text: Some(format!("1_{}", symbol.name)),
                        filter_text: Some(symbol.name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                    });
                }
            }
            WsSymbolKind::Constant => {
                if symbol.name.starts_with(member_prefix) {
                    completions.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: CompletionItemKind::Constant,
                        detail: Some(package_name.clone()),
                        documentation: package_member_documentation(&package_name, &symbol),
                        insert_text: Some(symbol.name.clone()),
                        sort_text: Some(format!("1_{}", symbol.name)),
                        filter_text: Some(symbol.name.clone()),
                        additional_edits: vec![],
                        text_edit_range: Some((context.prefix_start, context.position)),
                    });
                }
            }
            _ => {}
        }
    }

    add_known_core_module_completions(completions, context, &package_name, member_prefix);
}
