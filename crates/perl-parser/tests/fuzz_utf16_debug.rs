use perl_parser::position::{offset_to_utf16_line_col, utf16_line_col_to_offset};

#[test]
fn debug_utf16_roundtrip_failure() {
    let text = "a😀b\r\nc😀d";
    println!("Text: {:?}", text);
    println!("Text bytes: {:?}", text.as_bytes());
    println!("Text length: {}", text.len());

    // Examine each byte position
    for offset in 0..=text.len() {
        let (line, col) = offset_to_utf16_line_col(text, offset);
        let roundtrip = utf16_line_col_to_offset(text, line, col);
        println!("Offset {} -> (line={}, col={}) -> roundtrip={}", offset, line, col, roundtrip);

        if offset == 2 {
            println!(">>> FAILING CASE at offset 2 <<<");

            // Examine character boundaries around this position
            println!("Character boundary at 0: {}", text.is_char_boundary(0));
            println!("Character boundary at 1: {}", text.is_char_boundary(1));
            println!("Character boundary at 2: {}", text.is_char_boundary(2));
            println!("Character boundary at 3: {}", text.is_char_boundary(3));
            println!("Character boundary at 4: {}", text.is_char_boundary(4));
            println!("Character boundary at 5: {}", text.is_char_boundary(5));

            // Show the breakdown by characters
            for (i, ch) in text.chars().enumerate() {
                let utf8_len = ch.len_utf8();
                let utf16_len = ch.len_utf16();
                println!(
                    "Char {}: {:?} - UTF-8 bytes: {}, UTF-16 units: {}",
                    i, ch, utf8_len, utf16_len
                );
            }

            // Demonstrate the specific issue with offset 2
            println!("\n--- Analysis of offset 2 ---");
            println!("Offset 2 is mid-emoji (😀 spans bytes 1-4)");
            println!("Position conversion at offset 2: line={}, col={}", line, col);
            println!("Roundtrip conversion: {}", roundtrip);

            // Show what the algorithm thinks
            let lines: Vec<&str> = text.split_inclusive('\n').collect();
            println!("Lines split_inclusive: {:?}", lines);

            // Trace through the algorithm for offset 2
            let mut acc = 0usize;
            for (line_idx, line_text) in text.split_inclusive('\n').enumerate() {
                let next = acc + line_text.len();
                println!(
                    "Line {}: {:?}, acc={}, next={}, contains offset 2: {}",
                    line_idx,
                    line_text,
                    acc,
                    next,
                    offset >= acc && offset < next
                );

                if offset >= acc && offset < next {
                    let rel = offset - acc;
                    println!("Relative position in line: {}", rel);
                    println!("Is char boundary: {}", line_text.is_char_boundary(rel));

                    if !line_text.is_char_boundary(rel) {
                        // This is the problematic path
                        let mut char_start = rel;
                        while char_start > 0 && !line_text.is_char_boundary(char_start) {
                            char_start -= 1;
                        }
                        println!("Found char start at: {}", char_start);
                        let char_at_start = &line_text[char_start..];
                        let first_char = char_at_start.chars().next().unwrap();
                        println!("Character: {:?}", first_char);
                        println!("UTF-16 units: {}", first_char.len_utf16());
                    }
                }
                acc = next;
            }
        }
    }
}

#[test]
fn test_simple_emoji_utf16() {
    let text = "😀";
    println!("Simple emoji test: {:?}", text);

    for offset in 0..=text.len() {
        let (line, col) = offset_to_utf16_line_col(text, offset);
        let roundtrip = utf16_line_col_to_offset(text, line, col);
        println!("Offset {} -> (line={}, col={}) -> roundtrip={}", offset, line, col, roundtrip);
    }
}
