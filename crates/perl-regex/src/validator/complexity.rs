use crate::error::RegexError;

use super::{
    config::RegexValidationConfig,
    group::GroupStack,
    scanner::{RegexEvent, RegexScanner},
    unicode_property::UnicodePropertyCounter,
};

pub(crate) fn check_complexity(
    pattern: &str,
    start_pos: usize,
    config: &RegexValidationConfig,
) -> Result<(), RegexError> {
    let mut scanner = RegexScanner::new(pattern);
    let mut groups = GroupStack::new();
    let mut unicode_properties = UnicodePropertyCounter::new(config.max_unicode_properties);

    while let Some(event) = scanner.next_event() {
        match event {
            RegexEvent::UnicodeProperty { offset } => unicode_properties.observe(start_pos + offset)?,
            RegexEvent::GroupStart { kind, offset } => {
                groups.push(kind, offset, start_pos, config)?;
            }
            RegexEvent::Alternation { offset } => groups.observe_alternation(offset, start_pos, config)?,
            RegexEvent::GroupEnd => groups.pop(),
        }
    }

    Ok(())
}
