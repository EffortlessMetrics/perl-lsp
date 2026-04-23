//! Public API facade for perl-module.
//!
//! All items are re-exported from internal modules via this facade.
//! Consumers should import from `perl_module` only, not from submodules.

// name module
pub use crate::name::legacy_package_separator;
pub use crate::name::module_variant_pairs;
pub use crate::name::normalize_package_separator;

// path module
pub use crate::path::file_path_to_module_name;
pub use crate::path::module_name_to_path;
pub use crate::path::module_path_to_name;

// token_core module
pub use crate::token_core::ModuleTokenSpan;
pub use crate::token_core::has_standalone_module_token_boundaries;
pub use crate::token_core::is_module_identifier_char;
pub use crate::token_core::is_module_token_char;

// import module
pub use crate::import::DispatchSemantics;
pub use crate::import::ImportBehavior;
pub use crate::import::ImportListForm;
pub use crate::import::LoadTiming;
pub use crate::import::ModuleImportHead;
pub use crate::import::ModuleImportKind;
pub use crate::import::RequireForm;
pub use crate::import::parse_module_import_head;
pub use crate::import::resolve_known_export_tag;

// boundary module
pub use crate::boundary::ModuleTokenRange;
pub use crate::boundary::ModuleTokenRangeIter;
pub use crate::boundary::contains_standalone_module_token;
pub use crate::boundary::find_standalone_module_token_ranges;

// token module
pub use crate::token::contains_module_token;
pub use crate::token::replace_module_token;

// token_parser module
pub use crate::token_parser::parse_module_token;

// import_match module
pub use crate::import_match::line_references_module_import;

// reference module
pub use crate::reference::ModuleReference;
pub use crate::reference::ModuleReferenceKind;
pub use crate::reference::extract_module_reference;
pub use crate::reference::extract_module_reference_extended;
pub use crate::reference::find_module_reference;
pub use crate::reference::find_module_reference_extended;

// rename module
pub use crate::rename::ModuleLineEdit;
pub use crate::rename::apply_module_rename_edits;
pub use crate::rename::line_references_isa_assignment;
pub use crate::rename::line_references_package_declaration;
pub use crate::rename::line_references_qualified_call;
pub use crate::rename::plan_module_rename_edits;
pub use crate::rename::replace_module_name_prefix;

// resolution module
pub use crate::resolution::IncRoot;
pub use crate::resolution::IncRootKind;
pub use crate::resolution::ModuleUriResolution;
pub use crate::resolution::resolve_module_path;
pub use crate::resolution::resolve_module_uri;
pub use crate::resolution::resolve_module_uri_with_effective_inc;
