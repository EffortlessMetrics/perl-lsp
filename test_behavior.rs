use perl_parser::quote_parser::*;

fn main() {
    // Test specific behaviors that are failing
    println!("extract_transliteration_parts(\"tr/abc/xyz/g\"): {:?}", extract_transliteration_parts("tr/abc/xyz/g"));
    println!("extract_transliteration_parts(\"tr{{{{}}}}{{{{}}}}\"): {:?}", extract_transliteration_parts("tr{{{{}}}}{{{{}}}}"));
    println!("extract_regex_parts(\"x\"): {:?}", extract_regex_parts("x"));
    println!("extract_substitution_parts(\"s(a)(b)\"): {:?}", extract_substitution_parts("s(a)(b)"));
    println!("extract_substitution_parts(\"s/abc/defg/hi\"): {:?}", extract_substitution_parts("s/abc/defg/hi"));
}
