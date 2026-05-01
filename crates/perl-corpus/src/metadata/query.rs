use super::Section;

/// Find sections by tag
pub fn find_by_tag<'a>(sections: &'a [Section], tag: &str) -> Vec<&'a Section> {
    sections.iter().filter(|s| s.has_tag(tag)).collect()
}

/// Find sections by flag
pub fn find_by_flag<'a>(sections: &'a [Section], flag: &str) -> Vec<&'a Section> {
    sections.iter().filter(|s| s.has_flag(flag)).collect()
}
