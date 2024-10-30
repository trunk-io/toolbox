// trunk-ignore-all(trunk-toolbox/do-not-land,trunk-toolbox/todo)
extern crate regex;

use crate::diagnostic;
use crate::run::Run;
use anyhow::Context;
use log::{debug, trace};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

lazy_static::lazy_static! {
    static ref DNL_RE: Regex = Regex::new(r"(?i)(DO[\s_-]*NOT[\s_-]*LAND)").unwrap();
    static ref TODO_RE: Regex = Regex::new(r"(?i)(TODO|FIXME)(\W+.*)?$").unwrap();
}

pub fn is_binary_file(path: &PathBuf) -> std::io::Result<bool> {
    let mut file = File::open(path)?;
    let mut buffer = [0; 4096];
    let n = file.read(&mut buffer)?;
    Ok(buffer[..n].contains(&0))
}

pub fn is_ignored_file(path: &Path) -> bool {
    // Filter out well known files that should have the word donotland in them (like toolbox.toml)
    path.file_name().map_or(false, |f| f == "toolbox.toml")
}

// Checks for $re and other forms thereof in source code
//
// Note that this is named "pls_no_land" to avoid causing DNL matches everywhere in trunk-toolbox.
pub fn pls_no_land(run: &Run) -> anyhow::Result<Vec<diagnostic::Diagnostic>> {
    let dnl_config = &run.config.donotland;
    let todo_config = &run.config.todo;

    // Avoid opening the file if neither are enabled.
    if !dnl_config.enabled && !todo_config.enabled {
        trace!("'donotland' is disabled");
        trace!("'todo' is disabled");
        return Ok(vec![]);
    }

    debug!("scanning {} files for pls_no_land", run.paths.len());

    // Scan files in parallel
    let results: Result<Vec<_>, _> = run
        .paths
        .par_iter()
        .map(|path| pls_no_land_impl(path, run))
        .collect();

    match results {
        Ok(v) => Ok(v.into_iter().flatten().collect()),
        Err(e) => Err(e),
    }
}

fn pls_no_land_impl(path: &PathBuf, run: &Run) -> anyhow::Result<Vec<diagnostic::Diagnostic>> {
    let config: &crate::config::Conf = &run.config;

    if is_binary_file(path).unwrap_or(true) {
        debug!("ignoring binary file {}", path.display());
        return Ok(vec![]);
    }

    if is_ignored_file(path) {
        debug!("ignoring ignored file {}", path.display());
        return Ok(vec![]);
    }

    let in_file = File::open(path).with_context(|| format!("failed to open: {:#?}", path))?;
    let mut in_buf = BufReader::new(in_file);

    let mut first_line = vec![];
    in_buf.read_until(b'\n', &mut first_line)?;

    if first_line.is_empty() {
        return Ok(vec![]);
    }

    trace!("scanning contents of {}", path.display());

    let first_line_view = String::from_utf8(first_line)
        .with_context(|| format!("could not read first line of {:#?}", path))?;
    let lines_view = in_buf
        .lines()
        .collect::<std::io::Result<Vec<String>>>()
        .with_context(|| format!("failed to read lines of text from {:#?}", path))?;

    let mut ret = Vec::new();

    for (i, line) in [first_line_view]
        .iter()
        .chain(lines_view.iter())
        .enumerate()
    {
        // DO-NOT-LAND should always fire and not use HTL semantics. So only generate
        // those warnings on the current code and skip the upstream
        if !line.contains("trunk-ignore(|-begin|-end|-all)\\(trunk-toolbox/(do-not-land)\\)")
            && config.donotland.enabled
            && !run.is_upstream()
        {
            if let Some(m) = DNL_RE.find(line) {
                ret.push(diagnostic::Diagnostic {
                    path: path.to_str().unwrap().to_string(),
                    range: Some(diagnostic::Range {
                        start: diagnostic::Position {
                            line: i as u64,
                            character: m.start() as u64,
                        },
                        end: diagnostic::Position {
                            line: i as u64,
                            character: m.end() as u64,
                        },
                    }),
                    severity: diagnostic::Severity::Error,
                    code: "do-not-land".to_string(),
                    message: format!("Found '{}'", m.as_str()),
                    replacements: None,
                });
            }
        }
        if !line.contains("trunk-ignore(|-begin|-end|-all)\\(trunk-toolbox/(todo)\\)")
            && config.todo.enabled
        {
            if let Some(m) = TODO_RE.captures(line) {
                let token = &m[1];
                ret.push(diagnostic::Diagnostic {
                    path: path.to_str().unwrap().to_string(),
                    range: Some(diagnostic::Range {
                        start: diagnostic::Position {
                            line: i as u64,
                            character: m.get(1).unwrap().start() as u64,
                        },
                        end: diagnostic::Position {
                            line: i as u64,
                            // Remove one since we also check for a nonalpha character after the token.
                            character: m.get(1).unwrap().end() as u64,
                        },
                    }),
                    // Lower severity than DNL
                    severity: diagnostic::Severity::Warning,
                    code: "todo".to_string(),
                    message: format!("Found '{}'", token),
                    replacements: None,
                });
            }
        }
    }

    Ok(ret)
}
