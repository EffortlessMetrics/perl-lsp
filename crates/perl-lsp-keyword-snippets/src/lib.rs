//! Snippet templates for Perl keyword completion.

/// Completion template metadata for a Perl keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordTemplate {
    /// Text inserted into the document when the keyword is accepted.
    pub insert_text: &'static str,
    /// Whether `insert_text` is an LSP snippet.
    pub is_snippet: bool,
}

/// Returns completion template metadata for a Perl keyword.
#[must_use]
pub fn template_for_keyword(keyword: &str) -> KeywordTemplate {
    match keyword {
        "sub" => KeywordTemplate { insert_text: "sub ${1:name} {\n    $0\n}", is_snippet: true },
        "if" => KeywordTemplate { insert_text: "if ($1) {\n    $0\n}", is_snippet: true },
        "elsif" => KeywordTemplate { insert_text: "elsif ($1) {\n    $0\n}", is_snippet: true },
        "else" => KeywordTemplate { insert_text: "else {\n    $0\n}", is_snippet: true },
        "unless" => KeywordTemplate { insert_text: "unless ($1) {\n    $0\n}", is_snippet: true },
        "while" => KeywordTemplate { insert_text: "while ($1) {\n    $0\n}", is_snippet: true },
        "for" => KeywordTemplate {
            insert_text: "for (my $i = 0; $i < $1; $i++) {\n    $0\n}",
            is_snippet: true,
        },
        "foreach" => KeywordTemplate {
            insert_text: "foreach my $${1:item} (@${2:array}) {\n    $0\n}",
            is_snippet: true,
        },
        "package" => KeywordTemplate { insert_text: "package ${1:Name};\n\n$0", is_snippet: true },
        "use" => KeywordTemplate { insert_text: "use ${1:Module};\n$0", is_snippet: true },
        _ => KeywordTemplate { insert_text: "", is_snippet: false },
    }
}

#[cfg(test)]
mod tests {
    use super::template_for_keyword;

    #[test]
    fn returns_snippet_templates_for_structured_keywords() {
        let template = template_for_keyword("sub");
        assert!(template.is_snippet);
        assert_eq!(template.insert_text, "sub ${1:name} {\n    $0\n}");
    }

    #[test]
    fn returns_non_snippet_for_plain_keywords() {
        let template = template_for_keyword("return");
        assert!(!template.is_snippet);
        assert_eq!(template.insert_text, "");
    }
}
