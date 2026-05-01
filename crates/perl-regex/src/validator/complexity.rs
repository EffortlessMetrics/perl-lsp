use crate::RegexError;

use super::{
    config::RegexValidationConfig,
    group::{GroupStack, GroupType},
    unicode_property::UnicodePropertyCounter,
};

pub(crate) fn check_complexity(
    pattern: &str,
    start_pos: usize,
    config: &RegexValidationConfig,
) -> Result<(), RegexError> {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let mut groups = GroupStack::new();
    let mut unicode = UnicodePropertyCounter::new(config.max_unicode_properties);

    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if i + 1 < bytes.len() && matches!(bytes[i + 1], b'p' | b'P') {
                    i += 2;
                    if i < bytes.len() && bytes[i] == b'{' {
                        unicode.observe(start_pos + i - 2)?;
                    }
                    continue;
                }
                i += 2;
                continue;
            }
            b'[' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == b']' {
                        break;
                    } else {
                        i += 1;
                    }
                }
            }
            b'(' => {
                let mut gt = GroupType::Normal;
                let offset = i;
                if i + 1 < bytes.len() && bytes[i + 1] == b'?' {
                    i += 2;
                    if i < bytes.len() && bytes[i] == b'<' {
                        i += 1;
                        if i < bytes.len() && (bytes[i] == b'=' || bytes[i] == b'!') {
                            i += 1;
                            gt = GroupType::Lookbehind;
                        }
                    } else if i < bytes.len() && bytes[i] == b'|' {
                        i += 1;
                        gt = GroupType::BranchReset { branch_count: 1 };
                    }
                } else {
                    i += 1;
                }
                groups.push(gt, offset, start_pos, config)?;
                continue;
            }
            b'|' => groups.observe_alternation(i, start_pos, config)?,
            b')' => groups.pop(),
            _ => {}
        }
        i += 1;
    }
    Ok(())
}
