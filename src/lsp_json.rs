extern crate serde;
extern crate serde_json;

use serde::Serialize;

#[derive(Serialize)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Serialize)]
pub struct Position {
    pub line: u64,
    pub character: u64,
}

#[derive(Serialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Serialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Severity,
    pub code: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct LspJson {
    pub diagnostics: Vec<Diagnostic>,
}

impl LspJson {
    pub fn to_string(&self) -> Result<String, String> {
        return match serde_json::to_string(&self) {
            serde_json::Result::Ok(s) => Ok(s),
            serde_json::Result::Err(err) => Err(err.to_string()),
        };
    }
}
