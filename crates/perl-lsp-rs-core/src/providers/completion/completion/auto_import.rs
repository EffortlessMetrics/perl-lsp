//! Auto-import: generate `use Module;` edits for completion items.
//!
//! When a user completes a method or function from an unimported module, this
//! module generates an `additionalTextEdits` entry that inserts the appropriate
//! `use` statement at the top of the file, after any existing `use` block.
//!
//! # Rules
//! - If `use ModuleName;` (or `use ModuleName qw(...)` or `use ModuleName ()`)
//!   already appears in the file, no edit is produced.
//! - The insertion point is the byte offset immediately after the last existing
//!   `use` or `require` statement line, or line 0 if there are none.
//! - The inserted text is `"use ModuleName;\n"`.

use perl_parser_core::SourceLocation;

/// Determine whether `module` is already imported in `source`.
///
/// Returns `true` if a `use <module>` or `require <module>` line already
/// exists, so no duplicate edit is generated.
pub fn module_already_imported(source: &str, module: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim();
        // Match `use Module;`, `use Module ();`, `use Module qw(...)`, etc.
        if let Some(rest) = trimmed.strip_prefix("use ") {
            let rest = rest.trim_start();
            if let Some(after) = rest.strip_prefix(module) {
                // Make sure the next char (if any) is a space, semicolon, or end-of-token.
                if after.is_empty()
                    || after.starts_with(';')
                    || after.starts_with(' ')
                    || after.starts_with('\t')
                    || after.starts_with('(')
                {
                    return true;
                }
            }
        }
        if let Some(rest) = trimmed.strip_prefix("require ")
            && let Some(after) = rest.trim_start().strip_prefix(module)
            && (after.is_empty() || after.starts_with(';') || after.starts_with(' '))
        {
            return true;
        }
    }
    false
}

/// Find the byte offset after the last `use`/`require` statement block.
///
/// Returns the byte offset at the start of the line immediately after the last
/// `use` or `require` line.  If no such lines exist, returns `0` (insert at
/// the very beginning of the file).
pub fn find_use_block_end(source: &str) -> usize {
    let mut last_use_line_end: Option<usize> = None;
    let mut offset = 0usize;

    for line in source.lines() {
        let trimmed = line.trim();
        let line_byte_len = line.len() + 1; // include the '\n'

        let is_use_line = trimmed.starts_with("use ")
            || trimmed.starts_with("require ")
            || trimmed == "use strict"
            || trimmed == "use warnings"
            || trimmed == "use utf8"
            || trimmed == "#!/usr/bin/perl"
            || trimmed.starts_with('#')
            || trimmed.is_empty();

        if is_use_line {
            // We keep tracking: the actual `use`/`require` lines advance the
            // insertion candidate; blank lines and comments extend the "preamble"
            // window but we only commit if a real use statement has been seen.
            let is_real_use = trimmed.starts_with("use ") || trimmed.starts_with("require ");
            if is_real_use {
                last_use_line_end = Some(offset + line_byte_len);
            }
        }

        offset += line_byte_len;
    }

    // Return the byte offset right after the last use/require line.
    // If the file has no `use` block at all, insert at the top.
    last_use_line_end.unwrap_or(0)
}

/// Build a `(SourceLocation, text)` edit that inserts `use ModuleName;\n`
/// at the appropriate position in `source`.
///
/// Returns `None` if `module` is already imported.
pub fn build_auto_import_edit(source: &str, module: &str) -> Option<(SourceLocation, String)> {
    if module.is_empty() || module_already_imported(source, module) {
        return None;
    }

    let insert_offset = find_use_block_end(source);
    let insert_text = format!("use {module};\n");

    Some((SourceLocation { start: insert_offset, end: insert_offset }, insert_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // module_already_imported
    // ------------------------------------------------------------------

    #[test]
    fn already_imported_use_plain() {
        let src = "use strict;\nuse DBI;\nmy $x = 1;\n";
        assert!(module_already_imported(src, "DBI"));
    }

    #[test]
    fn already_imported_use_qw() {
        let src = "use List::Util qw(sum min max);\n";
        assert!(module_already_imported(src, "List::Util"));
    }

    #[test]
    fn already_imported_use_parens() {
        let src = "use JSON ();\n";
        assert!(module_already_imported(src, "JSON"));
    }

    #[test]
    fn not_imported_when_absent() {
        let src = "use strict;\nuse warnings;\nmy $x = 1;\n";
        assert!(!module_already_imported(src, "DBI"));
    }

    #[test]
    fn not_imported_prefix_match_only() {
        // `use DBIx::Class` should NOT match a check for `DBI`
        let src = "use DBIx::Class;\n";
        assert!(!module_already_imported(src, "DBI"));
    }

    #[test]
    fn already_imported_require() {
        let src = "require LWP::UserAgent;\n";
        assert!(module_already_imported(src, "LWP::UserAgent"));
    }

    // ------------------------------------------------------------------
    // find_use_block_end
    // ------------------------------------------------------------------

    #[test]
    fn insert_offset_after_use_block() {
        let src = "use strict;\nuse warnings;\n\nsub foo { }\n";
        // After "use warnings;\n" — that is at byte 24 (12 + 12).
        let offset = find_use_block_end(src);
        assert_eq!(&src[offset..], "\nsub foo { }\n");
    }

    #[test]
    fn insert_offset_zero_when_no_use() {
        let src = "sub foo { }\n";
        assert_eq!(find_use_block_end(src), 0);
    }

    #[test]
    fn insert_offset_single_use() {
        let src = "use strict;\nsub foo { }\n";
        let offset = find_use_block_end(src);
        assert_eq!(&src[offset..], "sub foo { }\n");
    }

    // ------------------------------------------------------------------
    // build_auto_import_edit
    // ------------------------------------------------------------------

    #[test]
    fn edit_returned_when_not_imported() {
        let src = "use strict;\n\nsub foo { DBI->connect(); }\n";
        let edit = build_auto_import_edit(src, "DBI");
        assert!(edit.is_some(), "should produce an edit");
        let (loc, text) = edit.unwrap();
        assert_eq!(text, "use DBI;\n");
        // Insert point is after "use strict;\n" (12 bytes)
        assert_eq!(loc.start, 12);
        assert_eq!(loc.end, 12, "zero-width insertion");
    }

    #[test]
    fn no_edit_when_already_imported() {
        let src = "use strict;\nuse DBI;\n\nsub foo { }\n";
        assert!(build_auto_import_edit(src, "DBI").is_none());
    }

    #[test]
    fn no_edit_for_empty_module_name() {
        let src = "use strict;\n";
        assert!(build_auto_import_edit(src, "").is_none());
    }

    #[test]
    fn edit_inserts_at_top_when_no_use_block() {
        let src = "sub foo { LWP::UserAgent->new(); }\n";
        let edit = build_auto_import_edit(src, "LWP::UserAgent");
        assert!(edit.is_some());
        let (loc, text) = edit.unwrap();
        assert_eq!(loc.start, 0);
        assert_eq!(text, "use LWP::UserAgent;\n");
    }

    #[test]
    fn edit_for_submodule_name() {
        let src = "use strict;\n";
        let edit = build_auto_import_edit(src, "JSON::MaybeXS");
        assert!(edit.is_some());
        let (_, text) = edit.unwrap();
        assert_eq!(text, "use JSON::MaybeXS;\n");
    }
}
