//! XS / Perl C API completions and hover docs.
//!
//! This is intentionally small and source-gated so normal Perl files do not
//! get C API noise.

use super::{context::CompletionContext, items::CompletionItem, items::CompletionItemKind};

struct XsApiEntry {
    name: &'static str,
    kind: CompletionItemKind,
    insert_text: &'static str,
    detail: &'static str,
    signature: &'static str,
    description: &'static str,
}

const XS_API_ENTRIES: &[XsApiEntry] = &[
    XsApiEntry {
        name: "dXSARGS",
        kind: CompletionItemKind::Function,
        insert_text: "dXSARGS",
        detail: "Declare XS argument variables",
        signature: "dXSARGS",
        description: "Declare the standard XS argument variables, including `items`.",
    },
    XsApiEntry {
        name: "items",
        kind: CompletionItemKind::Variable,
        insert_text: "items",
        detail: "XS argument count",
        signature: "items",
        description: "Number of XS arguments available in the current call.",
    },
    XsApiEntry {
        name: "ST",
        kind: CompletionItemKind::Snippet,
        insert_text: "ST(${1:index})",
        detail: "Fetch an XS stack slot",
        signature: "ST(n)",
        description: "Return the stack entry at index `n`.",
    },
    XsApiEntry {
        name: "SvIV",
        kind: CompletionItemKind::Function,
        insert_text: "SvIV(${1:sv})",
        detail: "Convert SV to IV",
        signature: "SvIV(sv)",
        description: "Convert an SV to an IV value.",
    },
    XsApiEntry {
        name: "newSViv",
        kind: CompletionItemKind::Snippet,
        insert_text: "newSViv(${1:iv})",
        detail: "Create a new IV SV",
        signature: "newSViv(iv)",
        description: "Create a new scalar value from an integer.",
    },
    XsApiEntry {
        name: "newSVpv",
        kind: CompletionItemKind::Snippet,
        insert_text: "newSVpv(${1:pv}, ${2:len})",
        detail: "Create a new PV SV",
        signature: "newSVpv(pv, len)",
        description: "Create a new scalar value from a byte buffer and length.",
    },
    XsApiEntry {
        name: "sv_2mortal",
        kind: CompletionItemKind::Function,
        insert_text: "sv_2mortal(${1:sv})",
        detail: "Mark SV mortal",
        signature: "sv_2mortal(sv)",
        description: "Mark an SV for destruction at the end of the statement.",
    },
    XsApiEntry {
        name: "PUSHs",
        kind: CompletionItemKind::Snippet,
        insert_text: "PUSHs(${1:sv})",
        detail: "Push an SV onto the stack",
        signature: "PUSHs(sv)",
        description: "Push an SV onto Perl's return stack.",
    },
    XsApiEntry {
        name: "EXTEND",
        kind: CompletionItemKind::Snippet,
        insert_text: "EXTEND(${1:sp}, ${2:n})",
        detail: "Grow the stack",
        signature: "EXTEND(sp, n)",
        description: "Ensure room on the Perl stack before pushing values.",
    },
    XsApiEntry {
        name: "XSRETURN_UNDEF",
        kind: CompletionItemKind::Function,
        insert_text: "XSRETURN_UNDEF",
        detail: "Return undef from XS",
        signature: "XSRETURN_UNDEF",
        description: "Return `undef` from an XS function.",
    },
    XsApiEntry {
        name: "XSRETURN_YES",
        kind: CompletionItemKind::Function,
        insert_text: "XSRETURN_YES",
        detail: "Return true from XS",
        signature: "XSRETURN_YES",
        description: "Return a true value from an XS function.",
    },
    XsApiEntry {
        name: "PL_sv_undef",
        kind: CompletionItemKind::Constant,
        insert_text: "PL_sv_undef",
        detail: "Global undef SV",
        signature: "PL_sv_undef",
        description: "The global immortal `undef` scalar.",
    },
    XsApiEntry {
        name: "PL_sv_yes",
        kind: CompletionItemKind::Constant,
        insert_text: "PL_sv_yes",
        detail: "Global true SV",
        signature: "PL_sv_yes",
        description: "The global immortal true scalar.",
    },
    XsApiEntry {
        name: "PL_sv_no",
        kind: CompletionItemKind::Constant,
        insert_text: "PL_sv_no",
        detail: "Global false SV",
        signature: "PL_sv_no",
        description: "The global immortal false scalar.",
    },
    XsApiEntry {
        name: "PL_na",
        kind: CompletionItemKind::Constant,
        insert_text: "PL_na",
        detail: "No-length sentinel",
        signature: "PL_na",
        description: "Sentinel value for APIs that take a length argument.",
    },
];

fn source_is_xs(source: &str, filepath: Option<&str>) -> bool {
    if let Some(path) = filepath {
        let lower = path.to_ascii_lowercase();
        if lower.ends_with(".xs") || lower.ends_with(".xsi") {
            return true;
        }
        if lower.ends_with(".c")
            || lower.ends_with(".h")
            || lower.ends_with(".cc")
            || lower.ends_with(".cpp")
        {
            return source.contains("EXTERN.h")
                || source.contains("perl.h")
                || source.contains("XSUB.h")
                || source.contains("MODULE =")
                || source.contains("PACKAGE =")
                || source.contains("PPCODE:")
                || source.contains("CODE:")
                || source.contains("BOOT:");
        }
    }

    source.contains("EXTERN.h")
        || source.contains("perl.h")
        || source.contains("XSUB.h")
        || source.contains("MODULE =")
        || source.contains("PACKAGE =")
        || source.contains("PPCODE:")
        || source.contains("CODE:")
        || source.contains("BOOT:")
}

/// Detect XS-like source files by extension or common XS markers.
#[must_use]
pub fn is_xs_source(source: &str, filepath: Option<&str>) -> bool {
    source_is_xs(source, filepath)
}

/// Return hover documentation for XS / Perl C API names.
pub fn get_xs_api_documentation(name: &str) -> Option<(&'static str, &'static str)> {
    XS_API_ENTRIES
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| (entry.signature, entry.description))
}

/// Add XS / Perl C API completions in XS-like sources only.
pub fn add_xs_api_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    filepath: Option<&str>,
) {
    if !source_is_xs(source, filepath) {
        return;
    }

    for entry in XS_API_ENTRIES {
        if context.prefix.is_empty() || entry.name.starts_with(&context.prefix) {
            completions.push(CompletionItem {
                label: entry.name.to_string(),
                kind: entry.kind,
                detail: Some(entry.detail.to_string()),
                documentation: Some(entry.description.to_string()),
                insert_text: Some(entry.insert_text.to_string()),
                sort_text: Some(format!("2_xs_{}", entry.name)),
                filter_text: Some(entry.name.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{get_xs_api_documentation, is_xs_source};

    #[test]
    fn detects_xs_source_by_extension() {
        assert!(is_xs_source("", Some("example.xs")));
        assert!(is_xs_source("", Some("example.xsi")));
    }

    #[test]
    fn detects_xs_source_by_markers() {
        let source = "#include \"EXTERN.h\"\n#include \"XSUB.h\"\nMODULE = Foo PACKAGE = Foo\n";
        assert!(is_xs_source(source, Some("example.pl")));
    }

    #[test]
    fn looks_up_xs_api_docs() {
        let docs = get_xs_api_documentation("SvIV");
        assert!(docs.is_some());
    }
}
