use crate::config::NeverEditConf;
use crate::git::FileStatus;
use crate::run::Run;
use glob::glob;
use glob_match::glob_match;

use log::debug;
use log::trace;
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
        trace!("'neveredit' is disabled");
        return Ok(vec![]);
    }

    let mut diagnostics: Vec<diagnostic::Diagnostic> = Vec::new();

    // We only emit config issues for the current run (not the upstream) so we can guarantee
    // that config issues get reported and not conceiled by HTL
    if config.paths.is_empty() && !run.is_upstream() {
        trace!("'neveredit' no protected paths configured");
        diagnostics.push(diagnostic::Diagnostic {
            path: run.config_path.clone(),
            range: None,
            severity: diagnostic::Severity::Warning,
            code: "never-edit-config".to_string(),
            message: "no protected paths provided in config".to_string(),
            replacements: None,
        });
        return Ok(diagnostics);
    }

    // We only report diagnostic issues for config when not running as upstream
    if !run.is_upstream() {
        debug!("verifying protected paths are valid and exist");
        for glob_path in &config.paths {
            let mut matches_something = false;
            match glob(glob_path) {
                Ok(paths) => {
                    for entry in paths {
                        match entry {
                            Ok(_path) => {
                                matches_something = true;
                                break;
                            }
                            Err(e) => println!("Error reading path: {:?}", e),
                        }
                    }
                    if !matches_something {
                        diagnostics.push(diagnostic::Diagnostic {
                            path: run.config_path.clone(),
                            range: None,
                            severity: diagnostic::Severity::Warning,
                            code: "never-edit-bad-config".to_string(),
                            message: format!("{:?} does not protect any existing files", glob_path),
                            replacements: None,
                        });
                    }
                }
                Err(_e) => {
                    diagnostics.push(diagnostic::Diagnostic {
                        path: run.config_path.clone(),
                        range: None,
                        severity: diagnostic::Severity::Warning,
                        code: "never-edit-bad-config".to_string(),
                        message: format!("{:?} is not a valid glob pattern", glob_path),
                        replacements: None,
                    });
                }
            }
        }
    }

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
        return Ok(diagnostics);
    }

    debug!(
        "tool configured for {} protected files",
        protected_files.len()
    );

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
                        message: "file is protected and should not be modified".to_string(),
                        replacements: None,
                    });
                }
                FileStatus::Deleted => {
                    diagnostics.push(diagnostic::Diagnostic {
                        path: protected_file.clone(),
                        range: None,
                        severity: diagnostic::Severity::Warning,
                        code: "never-edit-deleted".to_string(),
                        message: "file is protected and should not be deleted".to_string(),
                        replacements: None,
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
        code: "toolbox-never-edit-perf".to_string(),
        message: format!("{:?} protected files checked", protected_files.len()),
        replacements: None,
    });

    Ok(diagnostics)
}
