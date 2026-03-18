#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::quote_parser::{
    extract_regex_parts, extract_substitution_parts, extract_substitution_parts_strict,
    extract_transliteration_parts,
};
use perl_parser::Parser;

const MAX_SEGMENT_BYTES: usize = 256;
const VALID_SUB_MODIFIERS: &[char] = &[
    'g', 'i', 'm', 's', 'x', 'o', 'e', 'r', 'a', 'd', 'l', 'u', 'n', 'p', 'c',
];
const VALID_TR_MODIFIERS: &[char] = &['c', 'd', 's', 'r'];
const PAIRED_DELIMITERS: &[(char, char)] = &[('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];
const UNPAIRED_DELIMITERS: &[char] = &['/', '!', '#', '%', '~', '|', ':'];

fn bounded_utf8_lossy(data: &[u8]) -> String {
    let capped = if data.len() <= MAX_SEGMENT_BYTES { data } else { &data[..MAX_SEGMENT_BYTES] };
    String::from_utf8_lossy(capped).into_owned()
}

fn sanitize_segment(input: &str, forbidden: &[char]) -> String {
    input
        .chars()
        .filter(|ch| !ch.is_control())
        .map(|ch| if forbidden.contains(&ch) { '_' } else { ch })
        .collect()
}

fn decorate_nested(segment: &str, open: char, close: char, selector: u8) -> String {
    match selector % 4 {
        0 => segment.to_string(),
        1 => format!("{open}{segment}{close}"),
        2 => format!(r#"{segment}\\{close}{open}"#),
        _ => format!("{open}{segment}{close}{segment}"),
    }
}

fn valid_modifiers(raw: &[u8], allowed: &[char]) -> String {
    raw.iter()
        .take(8)
        .map(|byte| allowed[usize::from(*byte) % allowed.len()])
        .collect()
}

fn assert_within_bounds(source: &str, parts: &[&str]) {
    for part in parts {
        let _ = part.len() <= source.len();
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let pivot_one = usize::from(data[0]) % data.len();
    let pivot_two = if data.len() > 1 { usize::from(data[1]) % data.len() } else { 0 };
    let split_low = pivot_one.min(pivot_two);
    let split_high = pivot_one.max(pivot_two);

    let regex_seed = bounded_utf8_lossy(&data[..split_low]);
    let pattern_seed = bounded_utf8_lossy(&data[split_low..split_high]);
    let replacement_seed = bounded_utf8_lossy(&data[split_high..]);

    for &(open, close) in PAIRED_DELIMITERS {
        let regex_body = decorate_nested(&sanitize_segment(&regex_seed, &[open, close]), open, close, data[0]);
        let substitution_pattern = decorate_nested(
            &sanitize_segment(&pattern_seed, &[open, close]),
            open,
            close,
            data.get(1).copied().unwrap_or_default(),
        );
        let substitution_replacement = decorate_nested(
            &sanitize_segment(&replacement_seed, &[open, close]),
            open,
            close,
            data.get(2).copied().unwrap_or_default(),
        );

        let regex_input = format!("qr{open}{regex_body}{close}{}", valid_modifiers(data, VALID_SUB_MODIFIERS));
        let substitution_input = format!(
            "s{open}{substitution_pattern}{close}{open}{substitution_replacement}{close}{}",
            valid_modifiers(&data[1..], VALID_SUB_MODIFIERS)
        );
        let transliteration_input = format!(
            "tr{open}{substitution_pattern}{close}{open}{substitution_replacement}{close}{}",
            valid_modifiers(&data[2..], VALID_TR_MODIFIERS)
        );

        let (regex_pattern, regex_body_out, regex_modifiers) = extract_regex_parts(&regex_input);
        assert_within_bounds(&regex_input, &[&regex_pattern, &regex_body_out, &regex_modifiers]);

        let (sub_pattern, sub_replacement, sub_modifiers) = extract_substitution_parts(&substitution_input);
        assert_within_bounds(&substitution_input, &[&sub_pattern, &sub_replacement, &sub_modifiers]);

        if let Ok((strict_pattern, strict_replacement, strict_modifiers)) =
            extract_substitution_parts_strict(&substitution_input)
        {
            assert_within_bounds(
                &substitution_input,
                &[&strict_pattern, &strict_replacement, &strict_modifiers],
            );
        }

        let (tr_search, tr_replace, tr_modifiers) = extract_transliteration_parts(&transliteration_input);
        assert_within_bounds(&transliteration_input, &[&tr_search, &tr_replace, &tr_modifiers]);

        for program in [
            format!("my $regex = {regex_input};"),
            format!("$_ =~ {substitution_input};"),
            format!("$value =~ {transliteration_input};"),
        ] {
            let mut parser = Parser::new(&program);
            let _ = parser.parse();
        }
    }

    for &delimiter in UNPAIRED_DELIMITERS {
        let regex_body = sanitize_segment(&regex_seed, &[delimiter]);
        let substitution_pattern = sanitize_segment(&pattern_seed, &[delimiter]);
        let substitution_replacement = sanitize_segment(&replacement_seed, &[delimiter]);

        let regex_input = format!("m{delimiter}{regex_body}{delimiter}{}", valid_modifiers(data, VALID_SUB_MODIFIERS));
        let substitution_input = format!(
            "s{delimiter}{substitution_pattern}{delimiter}{substitution_replacement}{delimiter}{}",
            valid_modifiers(&data[1..], VALID_SUB_MODIFIERS)
        );
        let transliteration_input = format!(
            "y{delimiter}{substitution_pattern}{delimiter}{substitution_replacement}{delimiter}{}",
            valid_modifiers(&data[2..], VALID_TR_MODIFIERS)
        );

        let (regex_pattern, regex_body_out, regex_modifiers) = extract_regex_parts(&regex_input);
        assert_within_bounds(&regex_input, &[&regex_pattern, &regex_body_out, &regex_modifiers]);

        let (sub_pattern, sub_replacement, sub_modifiers) = extract_substitution_parts(&substitution_input);
        assert_within_bounds(&substitution_input, &[&sub_pattern, &sub_replacement, &sub_modifiers]);

        let _ = extract_substitution_parts_strict(&substitution_input);

        let (tr_search, tr_replace, tr_modifiers) = extract_transliteration_parts(&transliteration_input);
        assert_within_bounds(&transliteration_input, &[&tr_search, &tr_replace, &tr_modifiers]);

        for program in [
            format!("if ($value =~ {regex_input}) {{ print 'match'; }}"),
            format!("$value =~ {substitution_input};"),
            format!("$value =~ {transliteration_input};"),
        ] {
            let mut parser = Parser::new(&program);
            let _ = parser.parse();
        }
    }
});
