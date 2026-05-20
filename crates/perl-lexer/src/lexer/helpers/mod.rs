mod arc_text;
mod balanced_segments;
mod cursor;
mod file_normalization;
mod heredoc_delimiter;
mod line_scanning;
mod operator_classification;
mod word_classification;

pub(crate) use arc_text::{empty_arc, truncate_preview};
pub(crate) use operator_classification::is_compound_operator;
pub(crate) use word_classification::{
    is_builtin_function, is_keyword_fast, is_quote_op_word_prefix,
};
