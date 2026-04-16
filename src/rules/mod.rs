use crate::diagnostic;
use crate::run::Run;

pub mod if_change_then_change;
pub mod never_edit;
pub mod no_curly_quotes;
pub mod pls_no_land;

pub type RuleFn = fn(&Run, &str) -> anyhow::Result<Vec<diagnostic::Diagnostic>>;

pub const RULES: &[(&str, RuleFn)] = &[
    ("pls_no_land", pls_no_land::pls_no_land),
    ("if_change_then_change", if_change_then_change::ictc),
    ("never_edit", never_edit::never_edit),
    ("no_curly_quotes", no_curly_quotes::no_curly_quotes),
];
