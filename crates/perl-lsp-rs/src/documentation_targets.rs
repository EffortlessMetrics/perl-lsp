//! Canonical documentation targets for Perl editor surfaces.
//!
//! Keep construction of editor-facing documentation URIs in one place so hover,
//! document links, and virtual documents do not drift as perldoc graph support
//! grows.

/// A validated Perl documentation name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PerlDocumentationTarget {
    name: String,
    section: Option<String>,
}

impl PerlDocumentationTarget {
    /// Build a documentation target for a Perl module, pragma, or perldoc topic.
    ///
    /// The current editor contract uses raw `perldoc://Name::Space` and
    /// `https://metacpan.org/pod/Name::Space` strings. This validator keeps that
    /// behavior for simple Perl names while rejecting empty or path-like input.
    pub(crate) fn new(name: &str) -> Option<Self> {
        Self::with_section(name, None)
    }

    /// Build a documentation target for a Perl module, pragma, or perldoc topic
    /// plus an optional POD section.
    pub(crate) fn with_section(name: &str, section: Option<&str>) -> Option<Self> {
        let trimmed = name.trim();
        if !is_supported_perl_doc_name(trimmed) {
            return None;
        }

        let section = match section {
            Some(section) => {
                let trimmed_section = section.trim();
                if trimmed_section != section {
                    return None;
                }
                if !is_supported_pod_section_name(trimmed_section) {
                    return None;
                }
                Some(trimmed_section.to_string())
            }
            None => None,
        };

        Some(Self { name: trimmed.to_string(), section })
    }

    /// Build a documentation target from a virtual perldoc URI.
    pub(crate) fn from_perldoc_uri(uri: &str) -> Option<Self> {
        let target = uri.strip_prefix("perldoc://")?;
        if target != target.trim() || uri.chars().any(char::is_whitespace) {
            return None;
        }

        let (name, section) = match target.split_once('#') {
            Some((name, fragment)) => (name, Some(decode_pod_section_fragment(fragment)?)),
            None => (target, None),
        };

        Self::with_section(name, section.as_deref())
    }

    /// Build a documentation target from a simple POD `L<>` module target.
    ///
    /// This intentionally accepts only module-like names and the core pragma
    /// targets that virtual perldoc already enriches. Section-only links,
    /// URLs, and empty labels are left to the client as plain POD text.
    #[cfg(test)]
    pub(crate) fn from_simple_pod_link_target(target: &str) -> Option<Self> {
        let candidate = simple_pod_link_candidate(target)?;
        Self::from_pod_link_candidate(candidate, None)
    }

    /// Build a documentation target from a workspace POD `L<>` target.
    ///
    /// This accepts the same module targets as `from_simple_pod_link_target`,
    /// plus local section links such as `L</reset>` by anchoring them to the
    /// current workspace module.
    pub(crate) fn from_workspace_pod_link_target(
        target: &str,
        current_module: &str,
    ) -> Option<Self> {
        let candidate = simple_pod_link_candidate(target)?;
        Self::from_pod_link_candidate(candidate, Some(current_module))
    }

    fn from_pod_link_candidate(candidate: &str, current_module: Option<&str>) -> Option<Self> {
        if let Some(section) = candidate.strip_prefix('/') {
            return Self::with_section(current_module?, Some(section));
        }

        if let Some((name, section)) = candidate.split_once('/') {
            let name = name.trim();
            if is_supported_core_pragma_pod_target(name) || name.contains("::") {
                return Self::with_section(name, Some(section));
            }
            return None;
        }

        if is_supported_core_pragma_pod_target(candidate) || candidate.contains("::") {
            Self::new(candidate)
        } else {
            None
        }
    }

    /// Return the validated perldoc target name.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Return the requested POD section, when the target names one.
    pub(crate) fn section(&self) -> Option<&str> {
        self.section.as_deref()
    }

    /// Return the virtual perldoc document URI.
    pub(crate) fn perldoc_uri(&self) -> String {
        match self.section() {
            Some(section) => {
                format!("perldoc://{}#{}", self.name, encode_pod_section_fragment(section))
            }
            None => format!("perldoc://{}", self.name),
        }
    }

    /// Return the MetaCPAN POD URI for this target.
    pub(crate) fn metacpan_pod_uri(&self) -> String {
        format!("https://metacpan.org/pod/{}", self.name)
    }

    /// Return the perl.org perldoc URI for core pragmas/topics.
    pub(crate) fn perl_org_perldoc_uri(&self) -> String {
        format!("https://perldoc.perl.org/{}", self.name)
    }

    /// Return a markdown link to the virtual perldoc document.
    pub(crate) fn virtual_perldoc_markdown_link(&self) -> String {
        format!("[Open virtual perldoc]({})", self.perldoc_uri())
    }

    /// Return a markdown link to the MetaCPAN POD page.
    pub(crate) fn metacpan_markdown_link(&self, label: &str) -> String {
        format!("[{label}]({})", self.metacpan_pod_uri())
    }

    /// Return a markdown link to the perl.org perldoc page.
    pub(crate) fn perl_org_perldoc_markdown_link(&self) -> String {
        format!("[perldoc {}]({})", self.name, self.perl_org_perldoc_uri())
    }
}

/// Construct a MetaCPAN POD URI for a validated Perl documentation name.
pub(crate) fn metacpan_pod_uri(name: &str) -> Option<String> {
    PerlDocumentationTarget::new(name).map(|target| target.metacpan_pod_uri())
}

fn simple_pod_link_candidate(target: &str) -> Option<&str> {
    if let Some((label, link_target)) = target.split_once('|') {
        if label.trim().is_empty() {
            return None;
        }
        Some(link_target.trim())
    } else {
        Some(target.trim())
    }
}

fn is_supported_perl_doc_name(name: &str) -> bool {
    if name.is_empty()
        || name.contains(char::is_whitespace)
        || name.contains(['/', '\\', '<', '>', '|', '#', '?'])
    {
        return false;
    }

    name.split("::").all(is_perl_doc_name_segment)
}

fn is_perl_doc_name_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_supported_pod_section_name(section: &str) -> bool {
    !section.is_empty()
        && section.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ' '))
}

fn encode_pod_section_fragment(section: &str) -> String {
    let mut encoded = String::with_capacity(section.len());
    for ch in section.chars() {
        if ch == ' ' {
            encoded.push_str("%20");
        } else {
            encoded.push(ch);
        }
    }
    encoded
}

fn decode_pod_section_fragment(fragment: &str) -> Option<String> {
    let mut decoded = String::with_capacity(fragment.len());
    let mut chars = fragment.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let high = chars.next()?;
            let low = chars.next()?;
            decoded.push(char::from(hex_pair_value(high, low)?));
        } else {
            decoded.push(ch);
        }
    }

    if is_supported_pod_section_name(&decoded) { Some(decoded) } else { None }
}

fn hex_pair_value(high: char, low: char) -> Option<u8> {
    let high = high.to_digit(16)?;
    let low = low.to_digit(16)?;
    u8::try_from((high * 16) + low).ok()
}

fn is_supported_core_pragma_pod_target(target: &str) -> bool {
    matches!(target, "strict" | "warnings")
}

#[cfg(test)]
#[path = "documentation_targets_tests.rs"]
mod tests;
