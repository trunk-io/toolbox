use serde::Serialize;
use serde_sarif::sarif;
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

#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub line: u64,
    pub character: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone, Serialize)]
pub struct Replacement {
    pub deleted_region: Range,
    pub inserted_content: String,
}
#[derive(Clone, Serialize)]
pub struct Diagnostic {
    pub path: String,
    pub range: Option<Range>,
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub replacements: Option<Vec<Replacement>>,
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

impl Diagnostic {
    pub fn to_sarif(&self) -> sarif::Result {
        let mut physical_location = sarif::PhysicalLocationBuilder::default();
        physical_location.artifact_location(
            sarif::ArtifactLocationBuilder::default()
                .uri(self.path.clone())
                .build()
                .unwrap(),
        );

        if let Some(range) = &self.range {
            physical_location.region(
                sarif::RegionBuilder::default()
                    .start_line(range.start.line as i64 + 1)
                    .start_column(range.start.character as i64 + 1)
                    .end_line(range.end.line as i64 + 1)
                    .end_column(range.end.character as i64 + 1)
                    .build()
                    .unwrap(),
            );
        }

        let fixes = if let Some(replacements) = &self.replacements {
            let mut fixes = Vec::new();
            for replacement in replacements {
                fixes.push(replacement.to_fix(self));
            }
            Some(fixes)
        } else {
            None
        };

        sarif::ResultBuilder::default()
            .level(self.severity.to_string())
            .locations([sarif::LocationBuilder::default()
                .physical_location(physical_location.build().unwrap())
                .build()
                .unwrap()])
            .fixes(fixes.unwrap_or_default())
            .message(
                sarif::MessageBuilder::default()
                    .text(self.message.clone())
                    .build()
                    .unwrap(),
            )
            .rule_id(self.code.clone())
            .build()
            .unwrap()
    }
}

impl Replacement {
    pub fn to_fix(&self, diag: &Diagnostic) -> sarif::Fix {
        sarif::FixBuilder::default()
            .artifact_changes([sarif::ArtifactChangeBuilder::default()
                .artifact_location(
                    sarif::ArtifactLocationBuilder::default()
                        .uri(diag.path.clone())
                        .build()
                        .unwrap(),
                )
                .replacements(vec![sarif::ReplacementBuilder::default()
                    .deleted_region(
                        sarif::RegionBuilder::default()
                            .start_line(self.deleted_region.start.line as i64)
                            .start_column(self.deleted_region.start.character as i64 + 1)
                            .end_line(self.deleted_region.end.line as i64)
                            .end_column(self.deleted_region.end.character as i64 + 1)
                            .build()
                            .unwrap(),
                    )
                    .inserted_content(
                        sarif::ArtifactContentBuilder::default()
                            .text(self.inserted_content.clone())
                            .build()
                            .unwrap(),
                    )
                    .build()
                    .unwrap()])
                .build()
                .unwrap()])
            .build()
            .unwrap()
    }
}
