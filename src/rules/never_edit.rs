use crate::config::NeverEditConf;
use crate::git::FileStatus;
use crate::run::Run;
use glob_match::glob_match;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::diagnostic;
use crate::git;

pub fn is_never_edit(file_path: &str, config: &NeverEditConf) -> bool {
    for glob_path in &config.paths {
        if glob_match(glob_path, file_path) {
            log::info!("matched: {} with {}", glob_path, file_path);
            return true;
        }
    }
    false
}

pub fn never_edit(run: &Run, upstream: &str) -> anyhow::Result<Vec<diagnostic::Diagnostic>> {
    let config = &run.config.neveredit;

    if !config.enabled {
        return Ok(vec![]);
    }

    let mut diagnostics: Vec<diagnostic::Diagnostic> = Vec::new();

    if config.paths.is_empty() {
        diagnostics.push(diagnostic::Diagnostic {
            path: "toolbox.toml".to_string(),
            range: None,
            severity: diagnostic::Severity::Warning,
            code: "never-edit-config".to_string(),
            message: "no protected paths provided in config".to_string(),
        });
        return Ok(diagnostics);
    }
    //TODO Add warnings for any glob paths that don't resolve to an existing file or path

    // Build up list of files that are being checked and are protected
    let protected_files: Vec<_> = run
        .paths
        .par_iter()
        .filter_map(|file| {
            file.to_str().and_then(|file_str| {
                if is_never_edit(file_str, config) {
                    Some(file_str.to_string())
                } else {
                    None
                }
            })
        })
        .collect();

    // Fast exit if we don't have any files changed that are protected
    if protected_files.is_empty() {
        return Ok(vec![]);
    }

    let modified = git::modified_since(upstream, None)?;

    for protected_file in &protected_files {
        if let Some(status) = modified.paths.get(protected_file) {
            match status {
                FileStatus::Modified => {
                    diagnostics.push(diagnostic::Diagnostic {
                        path: protected_file.clone(),
                        range: None,
                        severity: diagnostic::Severity::Error,
                        code: "never-edit-modified".to_string(),
                        message: format!(
                            "{} is a protected file and should not be modified",
                            protected_file
                        ),
                    });
                }
                FileStatus::Deleted => {
                    diagnostics.push(diagnostic::Diagnostic {
                        path: protected_file.clone(),
                        range: None,
                        severity: diagnostic::Severity::Warning,
                        code: "never-edit-deleted".to_string(),
                        message: format!(
                            "{:?} is a protected file and should not be deleted",
                            protected_file
                        ),
                    });
                }
                _ => {}
            }
        }
    }

    diagnostics.push(diagnostic::Diagnostic {
        path: "".to_string(),
        range: None,
        severity: diagnostic::Severity::Note,
        code: "toolbox-perf".to_string(),
        message: format!("{:?} protected files checked", protected_files.len()),
    });

    Ok(diagnostics)
}
