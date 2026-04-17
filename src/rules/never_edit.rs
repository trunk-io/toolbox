use crate::config::NeverEditConf;
use crate::git::FileStatus;
use crate::run::Run;
use glob_match::glob_match;
use path_clean::PathClean;

use log::debug;
use log::trace;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::path::Path;

use crate::diagnostic;
use crate::git;

/// Strip leading `./` segments so paths and patterns align with git's
/// workspace-relative form.
fn strip_leading_dot_slash(s: &str) -> String {
    let mut p = s.replace('\\', "/");
    while p.starts_with("./") {
        p = p[2..].to_string();
    }
    p
}

/// Prefixes of the repository root suitable for stripping from absolute
/// never-edit glob patterns (raw workdir and canonicalized when available).
fn workdir_prefix_strings(workdir: &Path) -> Vec<String> {
    let mut v = vec![workdir
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()];
    if let Ok(c) = std::fs::canonicalize(workdir) {
        let s = c
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_string();
        if !v.contains(&s) {
            v.push(s);
        }
    }
    v
}

/// Never-edit glob patterns are always interpreted relative to the repository
/// root: leading `./` is removed, and absolute patterns that start with the
/// workdir are converted to a workspace-relative glob.
fn normalize_never_edit_glob_pattern(pattern: &str, workdir: &Path) -> String {
    let p = strip_leading_dot_slash(pattern);
    if !Path::new(&p).is_absolute() {
        return p;
    }
    let p = p.replace('\\', "/");
    for pref in workdir_prefix_strings(workdir) {
        if p == pref {
            return String::new();
        }
        let with_slash = format!("{pref}/");
        if let Some(suffix) = p.strip_prefix(&with_slash) {
            return suffix.to_string();
        }
    }
    p
}

/// Resolve a path passed on the CLI (or elsewhere) to a `/`-separated path
/// relative to the repository root, so it can be matched against normalized
/// never-edit globs.
fn repo_relative_posix(file_path: &str, workdir: &Path) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let raw = Path::new(file_path);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        cwd.join(raw)
    };
    let cleaned = joined.clean();

    let work_base = if workdir.is_absolute() {
        workdir.to_path_buf()
    } else {
        cwd.join(workdir)
    }
    .clean();

    let cleaned_abs = std::fs::canonicalize(&cleaned).unwrap_or_else(|_| cleaned.clone());
    let work_abs = std::fs::canonicalize(&work_base).unwrap_or_else(|_| work_base.clone());

    cleaned_abs
        .strip_prefix(&work_abs)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}

fn matches_never_edit(rel_path: &str, config: &NeverEditConf, workdir: &Path) -> bool {
    for glob_path in &config.paths {
        let pat = normalize_never_edit_glob_pattern(glob_path, workdir);
        if glob_match(&pat, rel_path) {
            log::info!(
                "matched: {} (normalized {:?}) with {}",
                glob_path,
                pat,
                rel_path
            );
            return true;
        }
    }
    false
}

pub fn is_never_edit(file_path: &str, config: &NeverEditConf) -> bool {
    let Ok(workdir) = git::repo_workdir(None) else {
        return false;
    };
    let Some(rel) = repo_relative_posix(file_path, &workdir) else {
        return false;
    };
    matches_never_edit(&rel, config, &workdir)
}

pub fn never_edit(run: &Run, upstream: &str) -> anyhow::Result<Vec<diagnostic::Diagnostic>> {
    let config = &run.config.neveredit;

    if !config.enabled {
        trace!("'neveredit' is disabled");
        return Ok(vec![]);
    }

    if run.is_upstream() {
        trace!("'neveredit' skipped on upstream baseline run");
        return Ok(vec![]);
    }

    let mut diagnostics: Vec<diagnostic::Diagnostic> = Vec::new();

    if config.paths.is_empty() {
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

    let workdir = git::repo_workdir(None)?;

    // Validate patterns against the list of git-tracked files anchored at the
    // workspace root rather than walking the filesystem from the process cwd.
    // This keeps validation in lockstep with matching, which uses workspace-relative paths.
    debug!("verifying protected paths are valid and exist");
    let tracked = git::tracked_files(None).unwrap_or_default();
    for glob_path in &config.paths {
        let pat = normalize_never_edit_glob_pattern(glob_path, &workdir);
        let matches_something = tracked.iter().any(|file| {
            let tr = strip_leading_dot_slash(&file.replace('\\', "/"));
            glob_match(&pat, &tr)
        });
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

    // Build up list of files that are being checked and are protected
    let protected_files: Vec<_> = run
        .paths
        .par_iter()
        .filter_map(|file| {
            file.to_str().and_then(|file_str| {
                let rel = repo_relative_posix(file_str, &workdir)?;
                if matches_never_edit(&rel, config, &workdir) {
                    Some(rel)
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
        if let Some(status) = modified.paths.get(Path::new(protected_file)) {
            match status {
                FileStatus::Modified => {
                    let replacements = build_restore_replacement(upstream, protected_file);
                    diagnostics.push(diagnostic::Diagnostic {
                        path: protected_file.clone(),
                        range: None,
                        severity: diagnostic::Severity::Error,
                        code: "never-edit-modified".to_string(),
                        message: "file is protected and should not be modified".to_string(),
                        replacements,
                    });
                }
                FileStatus::Deleted => {
                    let replacements = build_restore_replacement(upstream, protected_file);
                    diagnostics.push(diagnostic::Diagnostic {
                        path: protected_file.clone(),
                        range: None,
                        severity: diagnostic::Severity::Warning,
                        code: "never-edit-deleted".to_string(),
                        message: "file is protected and should not be deleted".to_string(),
                        replacements,
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

/// Build a replacement that restores the upstream version of a file.
/// For modified files, this replaces the entire local content with the upstream content.
/// For deleted files, this inserts the upstream content to recreate the file.
fn build_restore_replacement(
    upstream: &str,
    file_path: &str,
) -> Option<Vec<diagnostic::Replacement>> {
    let upstream_text = match git::get_upstream_content(upstream, file_path, None) {
        Ok(content) => content,
        Err(e) => {
            debug!("failed to get upstream content for {}: {}", file_path, e);
            return None;
        }
    };

    // Determine the region to delete based on local file content.
    // If the file still exists (modified), delete all its content.
    // If the file was deleted, use an empty region (0,0)-(0,0).
    let deleted_region = if let Ok(current_text) = std::fs::read_to_string(file_path) {
        let lines: Vec<&str> = current_text.split('\n').collect();
        let last_line_idx = lines.len().saturating_sub(1);
        let last_line_len = lines.last().map_or(0, |l| l.len());
        diagnostic::Range {
            start: diagnostic::Position {
                line: 0,
                character: 0,
            },
            end: diagnostic::Position {
                line: last_line_idx as u64,
                character: last_line_len as u64,
            },
        }
    } else {
        diagnostic::Range {
            start: diagnostic::Position {
                line: 0,
                character: 0,
            },
            end: diagnostic::Position {
                line: 0,
                character: 0,
            },
        }
    };

    Some(vec![diagnostic::Replacement {
        deleted_region,
        inserted_content: upstream_text,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_glob_strips_dot_slash() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            normalize_never_edit_glob_pattern("./src/**", tmp.path()),
            "src/**"
        );
        assert_eq!(
            normalize_never_edit_glob_pattern("././src/foo", tmp.path()),
            "src/foo"
        );
    }
}
