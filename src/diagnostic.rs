use serde::Serialize;
use std::fmt;

#[derive(Clone, Serialize)]
pub enum Severity {
    Error,
    Warning,
    Note,
    None,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
            Severity::None => write!(f, "none"),
        }
    }
}

#[derive(Clone, Serialize)]
pub struct Position {
    pub line: u64,
    pub character: u64,
}

#[derive(Clone, Serialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone, Serialize)]
pub struct Diagnostic {
    pub path: String,
    pub range: Option<Range>,
    pub severity: Severity,
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Default)]
pub struct Diagnostics {
    pub diagnostics: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn to_string(&self) -> anyhow::Result<String> {
        let as_string = serde_json::to_string(&self)?;
        Ok(as_string)
    }
}
