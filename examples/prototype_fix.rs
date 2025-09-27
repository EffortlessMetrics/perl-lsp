// Add this method to parser.rs after the parse_subroutine method

    /// Check if the content in parentheses looks like a prototype
    /// This is a heuristic check - prototypes only contain specific characters
    fn looks_like_prototype(&mut self) -> ParseResult<bool> {
        // Save current state
        let saved_peeked = self.tokens.peeked.clone();
        let saved_peeked_second = self.tokens.peeked_second.clone();
        
        // We need to look past the opening paren
        if self.tokens.peek()?.kind != TokenKind::LeftParen {
            return Ok(false);
        }
        
        // Consume the paren temporarily
        self.tokens.next()?;
        
        // Check first token after paren
        let first_token = match self.tokens.peek() {
            Ok(t) => t,
            Err(_) => {
                // Restore state
                self.tokens.peeked = saved_peeked;
                self.tokens.peeked_second = saved_peeked_second;
                return Ok(false);
            }
        };
        
        let looks_like_proto = match first_token.kind {
            // These are prototype sigils
            TokenKind::Dollar | TokenKind::At | TokenKind::Percent | 
            TokenKind::Star | TokenKind::BitwiseAnd | TokenKind::Backslash => true,
            // Empty parens could be either
            TokenKind::RightParen => true,
            // Semicolon for optional params
            TokenKind::Semicolon => true,
            // Anything else is likely a signature
            _ => false,
        };
        
        // If it might be a prototype and starts with &, check if & is followed by @ or other sigils
        if looks_like_proto && first_token.kind == TokenKind::BitwiseAnd {
            // Look at the next token
            self.tokens.next()?; // consume the &
            if let Ok(next_token) = self.tokens.peek() {
                match next_token.kind {
                    TokenKind::At | TokenKind::Dollar | TokenKind::Percent | 
                    TokenKind::Star | TokenKind::RightParen => {
                        // This is definitely a prototype like &@ or &$ 
                    }
                    _ => {
                        // & followed by something else, might not be a prototype
                    }
                }
            }
        }
        
        // Restore state
        self.tokens.peeked = saved_peeked;
        self.tokens.peeked_second = saved_peeked_second;
        
        Ok(looks_like_proto)
    }

// Then modify the parse_subroutine method to use this:
        // Parse optional prototype or signature after attributes
        let (params, prototype) = if self.peek_kind() == Some(TokenKind::LeftParen) {
            // Check if this looks like a prototype
            if self.looks_like_prototype()? {
                // Parse as prototype
                let proto = self.parse_prototype()?;
                // Store prototype as an attribute
                attributes.push(format!("prototype({})", proto));
                (Vec::new(), Some(proto))
            } else {
                (self.parse_signature()?, None)
            }
        } else {
            (Vec::new(), None)
        };

// Add the parse_prototype method:
    /// Parse a prototype string from parentheses
    fn parse_prototype(&mut self) -> ParseResult<String> {
        self.expect(TokenKind::LeftParen)?;
        let mut proto = String::new();
        
        loop {
            match self.peek_kind() {
                Some(TokenKind::RightParen) => {
                    self.tokens.next()?; // consume )
                    break;
                }
                Some(TokenKind::Dollar) => {
                    proto.push('$');
                    self.tokens.next()?;
                }
                Some(TokenKind::At) => {
                    proto.push('@');
                    self.tokens.next()?;
                }
                Some(TokenKind::Percent) => {
                    proto.push('%');
                    self.tokens.next()?;
                }
                Some(TokenKind::Star) => {
                    proto.push('*');
                    self.tokens.next()?;
                }
                Some(TokenKind::BitwiseAnd) => {
                    proto.push('&');
                    self.tokens.next()?;
                }
                Some(TokenKind::Semicolon) => {
                    proto.push(';');
                    self.tokens.next()?;
                }
                Some(TokenKind::Backslash) => {
                    proto.push('\\');
                    self.tokens.next()?;
                }
                _ => {
                    return Err(ParseError::syntax(
                        "Invalid character in prototype",
                        self.current_position()
                    ));
                }
            }
        }
        
        Ok(proto)
    }