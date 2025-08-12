use perl_parser::positions::LineStartsCache;

fn main() {
    let content = "line1\r\nline2\r\nline3";
    let cache = LineStartsCache::new(content);

    println!("Content bytes:");
    for (i, b) in content.bytes().enumerate() {
        let ch = if b == b'\r' {
            "\\r".to_string()
        } else if b == b'\n' {
            "\\n".to_string()
        } else {
            (b as char).to_string()
        };
        println!("  {}: {} (byte {})", i, ch, b);
    }

    println!("\nPositions:");
    for i in 0..=content.len() {
        let (l, c) = cache.offset_to_position(content, i);
        println!("  offset {} -> line {}, col {}", i, l, c);
    }

    // Test specific offset
    let offset = 6;
    let (l, c) = cache.offset_to_position(content, offset);
    println!("\nOffset {} maps to ({}, {})", offset, l, c);
    let rt = cache.position_to_offset(content, l, c);
    println!("Position ({}, {}) maps back to offset {}", l, c, rt);
}
