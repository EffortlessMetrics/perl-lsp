#[cfg(test)]
mod tests {
    use crate::{PerlLexer, TokenType};

    #[test]
    fn test_format_body_infinite_loop() {
        let input = "format body without terminator\nthis goes on forever\n";
        let mut lexer = PerlLexer::new(input);

        // Manually enter format mode to test
        lexer.enter_format_mode();

        // Set a timeout using a different thread
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            panic!("parse_format_body took >2s - probable infinite loop");
        });

        // This should complete quickly or return an error
        let token = lexer.next_token();

        // Cancel the timeout if we got here
        drop(handle);

        // Should get an error token
        assert!(matches!(token, Some(t) if matches!(t.token_type, TokenType::Error(_))));
    }
}
