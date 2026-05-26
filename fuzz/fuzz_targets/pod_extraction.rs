#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_pod::{extract_pod, PodDoc};

const MAX_INPUT_BYTES: usize = 4096;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };
    String::from_utf8_lossy(capped)
}

fn assert_doc_well_formed(doc: &PodDoc, source: &str) {
    // Every extracted field must be a substring-like fragment of the input, in
    // the weak sense that it cannot exceed the source length. (POD extraction
    // trims and concatenates - a strict substring check would over-constrain.)
    let len = source.len();
    if let Some(name) = &doc.name {
        assert!(name.len() <= len, "name field longer than source");
    }
    if let Some(syn) = &doc.synopsis {
        assert!(syn.len() <= len, "synopsis field longer than source");
    }
    if let Some(desc) = &doc.description {
        assert!(desc.len() <= len, "description field longer than source");
    }
    for (key, body) in &doc.methods {
        assert!(key.len() <= len, "method key longer than source");
        assert!(body.len() <= len, "method body longer than source");
    }
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    // Pass 1: arbitrary input. Must not panic on any byte sequence - POD
    // extraction is a permissive line-oriented scanner that historically has
    // had off-by-one trouble with truncated directives like `=cut\0` or `=head`
    // without a following digit.
    let doc1 = extract_pod(&input);
    assert_doc_well_formed(&doc1, &input);
    let _ = doc1.is_empty();

    // Pass 2: extract from a well-formed POD wrapper around the input. This
    // exercises the *transition* code paths (`=pod` start, `=cut` end, body
    // accumulation across `=head` boundaries) that the raw input rarely hits.
    let wrapped = format!(
        "=pod\n\n=head1 NAME\n\nFuzz::Module - {input}\n\n=head1 SYNOPSIS\n\n  {input}\n\n=head1 DESCRIPTION\n\n{input}\n\n=head2 method_a\n\n{input}\n\n=cut\n"
    );
    let doc2 = extract_pod(&wrapped);
    assert_doc_well_formed(&doc2, &wrapped);

    // Pass 3: list-oriented POD (=over / =item / =back) is a separate state
    // machine inside the extractor; make sure arbitrary content inside lists
    // doesn't break it.
    let listy = format!(
        "=head1 DESCRIPTION\n\n=over 4\n\n=item *\n\n{input}\n\n=item *\n\n{input}\n\n=back\n\n=cut\n"
    );
    let doc3 = extract_pod(&listy);
    assert_doc_well_formed(&doc3, &listy);

    // Pass 4: multiple =cut/restart cycles. Some past bugs only surfaced when
    // POD reopened after closing.
    let restarting = format!(
        "=head1 NAME\n\n{input}\n\n=cut\n\nsub code {{}}\n\n=head1 NAME\n\n{input}\n\n=cut\n"
    );
    let doc4 = extract_pod(&restarting);
    assert_doc_well_formed(&doc4, &restarting);
});
