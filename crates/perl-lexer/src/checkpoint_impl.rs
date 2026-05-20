use crate::*;

impl Checkpointable for PerlLexer<'_> {
    fn checkpoint(&self) -> LexerCheckpoint {
        use checkpoint::CheckpointContext;

        // Determine the checkpoint context based on current state
        let context = if matches!(self.mode, LexerMode::InFormatBody) {
            CheckpointContext::Format {
                start_position: self.position.saturating_sub(100), // Approximate
            }
        } else if !self.delimiter_stack.is_empty() {
            // We're in some kind of quote-like construct
            CheckpointContext::QuoteLike {
                operator: String::new(), // Would need to track this
                delimiter: self.delimiter_stack.last().copied().unwrap_or('\0'),
                is_paired: true,
            }
        } else {
            CheckpointContext::Normal
        };

        LexerCheckpoint {
            position: self.position,
            mode: self.mode,
            delimiter_stack: self.delimiter_stack.clone(),
            in_prototype: self.in_prototype,
            prototype_depth: self.prototype_depth,
            after_sub: self.after_sub,
            after_arrow: self.after_arrow,
            hash_brace_depth: self.hash_brace_depth,
            after_var_subscript: self.after_var_subscript,
            paren_depth: self.paren_depth,
            current_pos: self.current_pos,
            eof_emitted: self.eof_emitted,
            context,
        }
    }

    fn restore(&mut self, checkpoint: &LexerCheckpoint) {
        self.position = checkpoint.position;
        self.mode = checkpoint.mode;
        self.delimiter_stack.clone_from(&checkpoint.delimiter_stack);
        self.in_prototype = checkpoint.in_prototype;
        self.prototype_depth = checkpoint.prototype_depth;
        self.after_sub = checkpoint.after_sub;
        self.after_arrow = checkpoint.after_arrow;
        self.hash_brace_depth = checkpoint.hash_brace_depth;
        self.after_var_subscript = checkpoint.after_var_subscript;
        self.paren_depth = checkpoint.paren_depth;
        self.current_pos = checkpoint.current_pos;
        self.eof_emitted = checkpoint.eof_emitted;

        // Handle special contexts
        use checkpoint::CheckpointContext;
        if let CheckpointContext::Format { .. } = &checkpoint.context {
            // Ensure we're in format body mode
            if !matches!(self.mode, LexerMode::InFormatBody) {
                self.mode = LexerMode::InFormatBody;
            }
        }
    }

    fn can_restore(&self, checkpoint: &LexerCheckpoint) -> bool {
        // Can restore if the position is valid for our input
        checkpoint.position <= self.input.len()
    }
}
