use crate::run::Run;
use anyhow::Context;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;

use crate::diagnostic;
use crate::git;

#[derive(Debug)]
pub enum ThenChange {
    RemoteFile(String),
    RepoFile(PathBuf),
    MissingIf,
    MissingThen,
}

#[derive(Debug)]
pub struct IctcBlock {
    pub path: PathBuf,
    pub begin: Option<u64>,
    pub end: Option<u64>,
    pub thenchange: Option<ThenChange>,
}

impl IctcBlock {
    fn get_range(&self) -> diagnostic::Range {
        diagnostic::Range {
            start: diagnostic::Position {
                line: self.begin.unwrap(),
                character: 0,
            },
            end: diagnostic::Position {
                line: self.end.unwrap(),
                character: 0,
            },
        }
    }
}

lazy_static::lazy_static! {
    static ref RE_BEGIN: Regex = Regex::new(r"(?i)^\s*(//|#)\s*ifchange(.*)$").unwrap();
    static ref RE_END: Regex = Regex::new(r"(?i)^\s*(//|#)\s*thenchange(.*)$").unwrap();
}

pub fn find_ictc_blocks(path: &PathBuf) -> anyhow::Result<Vec<IctcBlock>> {
    let mut blocks: Vec<IctcBlock> = Vec::new();

    let in_file = File::open(path).with_context(|| format!("failed to open: {:#?}", path))?;
    let in_buf = BufReader::new(in_file);

    let mut block: Option<IctcBlock> = None;

    for (i, line) in lines_view(in_buf)
        .context(format!("failed to read lines of text from: {:#?}", path))?
        .iter()
        .enumerate()
        .map(|(i, line)| (i + 1, line))
    {
        let line_no = Some(i as u64);
        if RE_BEGIN.find(line).is_some() {
            if let Some(mut block_value) = block {
                // Two if blocks in a row - report problem
                block_value.end = block_value.begin;
                block_value.thenchange = Some(ThenChange::MissingThen);
                blocks.push(block_value);
            }

            block = Some(IctcBlock {
                path: path.clone(),
                begin: line_no,
                end: None,
                thenchange: None,
            });
        } else if let Some(end_capture) = RE_END.captures(line) {
            if let Some(mut block_value) = block {
                block_value.end = line_no;
                block_value.thenchange = Some(ThenChange::RepoFile(PathBuf::from(
                    end_capture
                        .get(2)
                        .with_context(|| "expected at least 3 captures")?
                        .as_str()
                        .trim(),
                )));
                blocks.push(block_value);
                block = None;
            } else {
                // block is None and we found a IfChange without a ThenChange
                blocks.push(IctcBlock {
                    path: path.clone(),
                    begin: line_no,
                    end: line_no,
                    thenchange: Some(ThenChange::MissingIf),
                });
            }
        }
    }

    // If we have an unclosed block - record that
    if let Some(mut block_value) = block {
        block_value.end = block_value.begin;
        block_value.thenchange = Some(ThenChange::MissingThen);
        blocks.push(block_value);
    }

    Ok(blocks)
}

pub fn ictc(run: &Run, upstream: &str) -> anyhow::Result<Vec<diagnostic::Diagnostic>> {
    let config = &run.config.ifchange;

    if !config.enabled {
        return Ok(vec![]);
    }

    // Build up list of files that actually have a ifchange block - this way we can avoid
    // processing git modified chunks if none are present
    let all_blocks: Vec<_> = run
        .paths
        .par_iter()
        .filter_map(|file| find_ictc_blocks(file).ok())
        .flatten()
        .collect();

    // Fast exit if we don't have any files that have ICTC blocks - saves us calling
    // into git to get more information
    if all_blocks.is_empty() {
        return Ok(vec![]);
    }

    let modified = git::modified_since(upstream, None)?;
    let hunks = &modified.hunks;

    log::debug!("Modified stats, per libgit2:\n{:#?}", modified);

    // TODO(sam): this _should_ be a iter-map-collect, but unclear how to apply a reducer
    // between the map and collect (there can be multiple hunks with the same path)
    let mut modified_lines_by_path: HashMap<PathBuf, HashSet<u64>> = HashMap::new();
    for h in hunks {
        modified_lines_by_path
            .entry(h.path.clone())
            .or_default()
            .extend(h.begin..h.end);
    }
    let modified_lines_by_path = modified_lines_by_path;

    let mut blocks: Vec<IctcBlock> = Vec::new();

    for block in all_blocks {
        if let Some(thenchange) = &block.thenchange {
            match &thenchange {
                ThenChange::MissingIf | ThenChange::MissingThen => {
                    blocks.push(block);
                }
                _ => {
                    if let (Some(begin), Some(end)) = (block.begin, block.end) {
                        let block_lines = HashSet::from_iter(begin..end);
                        if !block_lines.is_disjoint(
                            modified_lines_by_path
                                .get(&block.path)
                                .unwrap_or(&HashSet::new()),
                        ) {
                            blocks.push(block);
                        }
                    }
                }
            }
        }
    }

    let blocks_by_path: HashMap<&PathBuf, &IctcBlock> =
        blocks.iter().map(|b| (&b.path, b)).collect();

    let mut diagnostics: Vec<diagnostic::Diagnostic> = Vec::new();

    for block in &blocks {
        if let Some(change) = &block.thenchange {
            match change {
                ThenChange::RemoteFile(_remote_file) => {
                    todo!("build support for remote file")
                }
                ThenChange::RepoFile(local_file) => {
                    // Check if the repo file exists - if it was deleted this is a warning
                    if !Path::new(local_file).exists() {
                        diagnostics.push(diagnostic::Diagnostic {
                            path: block.path.to_str().unwrap().to_string(),
                            range: Some(block.get_range()),
                            severity: diagnostic::Severity::Warning,
                            code: "if-change-file-does-not-exist".to_string(),
                            message: format!("ThenChange {} does not exist", local_file.display(),),
                        });
                    }
                    // If target file was not changed raise issue
                    if blocks_by_path.get(&local_file).is_none() {
                        diagnostics.push(diagnostic::Diagnostic {
                            path: block.path.to_str().unwrap().to_string(),
                            range: Some(block.get_range()),
                            severity: diagnostic::Severity::Error,
                            code: "if-change-then-change-this".to_string(),
                            message: format!(
                                "Expected change in {} because {} was modified",
                                local_file.display(),
                                block.path.display(),
                            ),
                        });
                    }
                }
                ThenChange::MissingIf => {
                    diagnostics.push(diagnostic::Diagnostic {
                        path: block.path.to_str().unwrap().to_string(),
                        range: Some(block.get_range()),
                        severity: diagnostic::Severity::Warning,
                        code: "if-change-mismatched".to_string(),
                        message: "Expected preceding IfChange tag".to_string(),
                    });
                }
                ThenChange::MissingThen => {
                    diagnostics.push(diagnostic::Diagnostic {
                        path: block.path.to_str().unwrap().to_string(),
                        range: Some(block.get_range()),
                        severity: diagnostic::Severity::Warning,
                        code: "if-change-mismatched".to_string(),
                        message: "Expected matching ThenChange tag".to_string(),
                    });
                }
            }
        }
    }

    debug!("ICTC blocks are:\n{:?}", blocks);

    Ok(diagnostics)
}

type LinesView = Vec<String>;

fn lines_view<R: BufRead>(reader: R) -> anyhow::Result<LinesView> {
    let mut ret: LinesView = LinesView::default();
    for line in reader.lines() {
        let line = line?;
        ret.push(line);
    }
    Ok(ret)
}
