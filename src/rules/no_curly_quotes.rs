use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::PathBuf;

use crate::run::Run;

use anyhow::Context;
use log::debug;
use log::trace;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::diagnostic::{Diagnostic, Position, Range, Replacement, Severity};

pub fn no_curly_quotes(run: &Run, _upstream: &str) -> anyhow::Result<Vec<Diagnostic>> {
    let config = &run.config.nocurlyquotes;

    if !config.enabled {
        trace!("'nocurlyquotes' is disabled");
        return Ok(vec![]);
    }

    if run.is_upstream() {
        return Ok(vec![]);
    }

    debug!("scanning {} files for curly quotes", run.paths.len());

    // Scan files in parallel
    let results: Result<Vec<_>, _> = run.paths.par_iter().map(no_curly_quotes_impl).collect();

    match results {
        Ok(v) => Ok(v.into_iter().flatten().collect()),
        Err(e) => Err(e),
    }
}

const DOUBLE_CURLY_QUOTES: [char; 4] = ['\u{201C}', '\u{201D}', '\u{201E}', '\u{201F}'];
const SINGLE_CURLY_QUOTES: [char; 2] = ['\u{2018}', '\u{2019}'];

fn no_curly_quotes_impl(path: &PathBuf) -> anyhow::Result<Vec<Diagnostic>> {
    let in_file = File::open(path).with_context(|| format!("failed to open: {:#?}", path))?;
    let in_buf = BufReader::new(in_file);

    trace!("scanning contents of {}", path.display());

    let lines_view = in_buf
        .lines()
        .collect::<std::io::Result<Vec<String>>>()
        .with_context(|| format!("failed to read lines of text from {:#?}", path))?;

    let mut ret = Vec::new();

    for (i, line) in lines_view.iter().enumerate() {
        let mut char_issues = Vec::new();

        for (pos, c) in line.char_indices() {
            if SINGLE_CURLY_QUOTES.contains(&c) {
                let char_pos = line[..pos].chars().count() as u64;
                char_issues.push((char_pos, "'"));
            }
            if DOUBLE_CURLY_QUOTES.contains(&c) {
                let char_pos = line[..pos].chars().count() as u64;
                char_issues.push((char_pos, "\""));
            }
        }

        if char_issues.is_empty() {
            continue;
        }

        // Build an array of replacements for each character in char_positions
        let replacements: Vec<Replacement> = char_issues
            .iter()
            .map(|&(char_pos, rchar)| Replacement {
                deleted_region: Range {
                    start: Position {
                        line: i as u64 + 1,
                        character: char_pos,
                    },
                    end: Position {
                        line: i as u64 + 1,
                        character: char_pos + 1,
                    },
                },
                inserted_content: rchar.to_string(),
            })
            .collect();

        ret.push(Diagnostic {
            path: path.to_str().unwrap().to_string(),
            range: Some(Range {
                start: Position {
                    line: i as u64,
                    character: char_issues.first().unwrap().0,
                },
                end: Position {
                    line: i as u64,
                    character: char_issues.last().unwrap().0 + 1,
                },
            }),
            severity: Severity::Error,
            code: "no-curly-quotes".to_string(),
            message: format!("Found curly quote on line {}", i + 1),
            replacements: Some(replacements),
        });
    }

    Ok(ret)
}
