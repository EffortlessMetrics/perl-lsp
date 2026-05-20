use crate::{error::RegexError, syntax::cursor::quoted_literal_end};

use super::{
    config::RegexValidationConfig,
    group::{GroupStack, GroupType},
};

pub(crate) fn check_complexity(
    pattern: &str,
    start_pos: usize,
    config: &RegexValidationConfig,
) -> Result<(), RegexError> {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let mut stack = GroupStack::new();
    let mut unicode_property_count = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if let Some(end) = quoted_literal_end(bytes, i) {
                    i = end;
                    continue;
                }

                if i + 1 < bytes.len() {
                    match bytes[i + 1] {
                        b'p' | b'P' => {
                            i += 2;
                            if i < bytes.len() && bytes[i] == b'{' {
                                unicode_property_count += 1;
                                if unicode_property_count > config.max_unicode_properties {
                                    return Err(RegexError::syntax(
                                        format!(
                                            "Too many Unicode properties in regex (max {})",
                                            config.max_unicode_properties
                                        ),
                                        start_pos + i - 2,
                                    ));
                                }
                            }
                            continue;
                        }
                        _ => {
                            i += 2;
                            continue;
                        }
                    }
                }
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
                let mut group = GroupType::Normal;
                if i + 1 < bytes.len() && bytes[i + 1] == b'?' {
                    i += 2;
                    if i < bytes.len() && bytes[i] == b'<' {
                        i += 1;
                        if i < bytes.len() && (bytes[i] == b'=' || bytes[i] == b'!') {
                            i += 1;
                            group = GroupType::Lookbehind;
                        }
                    } else if i < bytes.len() && bytes[i] == b'|' {
                        i += 1;
                        group = GroupType::BranchReset { branch_count: 1 };
                    }
                } else {
                    i += 1;
                }
                stack.push(group, i - 1, start_pos, config)?;
                continue;
            }
            b'|' => stack.observe_alternation(i, start_pos, config)?,
            b')' => stack.pop(),
            _ => {}
        }
        i += 1;
    }
    Ok(())
}
