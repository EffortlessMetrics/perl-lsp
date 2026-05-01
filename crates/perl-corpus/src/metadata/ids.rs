use std::collections::HashMap;

pub fn slugify_title(title: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in title.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    slug
}

pub fn generate_id(
    supplied_id: Option<&str>,
    file_stem: &str,
    title: &str,
    section_index: usize,
    auto_ids: &mut HashMap<String, usize>,
) -> String {
    if let Some(id) = supplied_id.filter(|id| !id.is_empty()) {
        return id.to_string();
    }

    let title_slug = slugify_title(title);
    let base = if title_slug.is_empty() {
        format!("section-{section_index}")
    } else {
        title_slug
    };
    let base_id = format!("{file_stem}.{base}");
    let count = auto_ids.entry(base_id.clone()).or_insert(0);
    let id = if *count == 0 { base_id } else { format!("{base_id}-{}", *count + 1) };
    *count += 1;
    id
}
