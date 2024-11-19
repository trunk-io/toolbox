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

    debug!("scanning {} files for curly quotes", run.paths.len());

    // Scan files in parallel
    let results: Result<Vec<_>, _> = run
        .paths
        .par_iter()
        .map(|path| no_curly_quotes_impl(path))
        .collect();

    match results {
        Ok(v) => Ok(v.into_iter().flatten().collect()),
        Err(e) => Err(e),
    }
}

fn no_curly_quotes_impl(path: &PathBuf) -> anyhow::Result<Vec<Diagnostic>> {
    let in_file = File::open(path).with_context(|| format!("failed to open: {:#?}", path))?;
    let mut in_buf = BufReader::new(in_file);

    let mut first_line = vec![];
    in_buf.read_until(b'\n', &mut first_line)?;

    trace!("scanning contents of {}", path.display());

    let lines_view = in_buf
        .lines()
        .collect::<std::io::Result<Vec<String>>>()
        .with_context(|| format!("failed to read lines of text from {:#?}", path))?;

    let mut ret = Vec::new();

    for (i, line) in lines_view.iter().chain(lines_view.iter()).enumerate() {
        let mut pos = 0;
        let mut char_positions = Vec::new();
        // Iterate through the line and find positions of “ or ”
        while let Some(start) = line[pos..].find(|c| c == '“' || c == '”') {
            let char_pos = (pos + start) as u64;
            char_positions.push(char_pos);
            pos = pos + start + 1;
        }

        if char_positions.is_empty() {
            continue;
        }

        // Build an array of replacements for each character in char_positions
        let replacements: Vec<Replacement> = char_positions
            .iter()
            .map(|&char_pos| Replacement {
                deleted_region: Range {
                    start: Position {
                        line: i as u64,
                        character: char_pos,
                    },
                    end: Position {
                        line: i as u64,
                        character: char_pos,
                    },
                },
                inserted_content: "\"".to_string(),
            })
            .collect();

        ret.push(Diagnostic {
            path: path.to_str().unwrap().to_string(),
            range: Some(Range {
                start: Position {
                    line: i as u64,
                    character: *char_positions.first().unwrap(),
                },
                end: Position {
                    line: i as u64,
                    character: *char_positions.last().unwrap(),
                },
            }),
            severity: Severity::Error,
            code: "no-curly-quotes".to_string(),
            message: format!("Found curly quote on line {}", i),
            replacements: Some(replacements),
        });
    }

    Ok(ret)
}
