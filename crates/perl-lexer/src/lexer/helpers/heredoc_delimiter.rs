use crate::PerlLexer;

impl PerlLexer<'_> {
    #[inline]
    pub(crate) fn parse_quoted_heredoc_delimiter(
        &mut self,
        quote: char,
        text: &mut String,
    ) -> Option<String> {
        text.push(quote);
        self.advance();

        let mut delim = String::new();
        while self.position < self.input.len() {
            let Some(ch) = self.current_char() else {
                break;
            };

            if ch == quote {
                text.push(ch);
                self.advance();
                return Some(delim);
            }

            if ch == '\n' || ch == '\r' {
                return None;
            }

            delim.push(ch);
            text.push(ch);
            self.advance();
        }

        None
    }
}
