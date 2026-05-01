use crate::error::RegexError;

pub(crate) struct UnicodePropertyCounter {
    count: usize,
    max: usize,
}

impl UnicodePropertyCounter {
    pub(crate) fn new(max: usize) -> Self {
        Self { count: 0, max }
    }

    pub(crate) fn observe(&mut self, offset: usize) -> Result<(), RegexError> {
        self.count += 1;
        if self.count > self.max {
            return Err(RegexError::syntax(
                "Too many Unicode properties in regex (max 50)",
                offset,
            ));
        }
        Ok(())
    }
}
