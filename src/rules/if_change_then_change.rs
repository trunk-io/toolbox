use anyhow::Context;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use regex::Regex;

use crate::diagnostic;
use crate::git::Hunk;

#[derive(Debug)]
pub struct IctcBlock {
    pub path: PathBuf,
    pub begin: i64,
    pub end: i64,
    pub thenchange: PathBuf,
}

lazy_static::lazy_static! {
    static ref RE_BEGIN: Regex = Regex::new(r"(?i) *(//|#) *ifchange").unwrap();
    static ref RE_END: Regex = Regex::new(r"(?i) *(//|#) *thenchange (.*)").unwrap();

}

pub fn ictc(hunks: &Vec<Hunk>) -> anyhow::Result<Vec<diagnostic::Diagnostic>> {
    // TODO(sam): this _should_ be a iter-map-collect, but unclear how to apply a reducer
    // between the map and collect (there can be multiple hunks with the same path)
    let mut modified_lines_by_path: HashMap<PathBuf, HashSet<i64>> = HashMap::new();
    for h in hunks {
        modified_lines_by_path
            .entry(h.path.clone())
            .or_insert_with(HashSet::new)
            .extend(h.begin..h.end);
    }
    let modified_lines_by_path = modified_lines_by_path;

    let mut blocks: Vec<IctcBlock> = Vec::new();
    for path in modified_lines_by_path.keys() {
        let mut ifttt_begin: i64 = -1;

        let in_file = File::open(&path).with_context(|| format!("failed to open: {:#?}", path))?;
        let in_buf = BufReader::new(in_file);

        for (i, line) in lines_view(in_buf)
            .context(format!("failed to read lines of text from: {:#?}", path))?
            .iter()
            .enumerate()
            .map(|(i, line)| (i + 1, line))
        {
            if RE_BEGIN.find(line).is_some() {
                ifttt_begin = i as i64;
            } else if let Some(end) = RE_END.captures(line) {
                if ifttt_begin != -1 {
                    let block = IctcBlock {
                        path: path.clone(),
                        begin: ifttt_begin,
                        end: i as i64,
                        thenchange: PathBuf::from(end.get(2).unwrap().as_str()),
                    };

                    let block_lines = HashSet::from_iter(block.begin..block.end);

                    if !block_lines.is_disjoint(
                        modified_lines_by_path
                            .get(&block.path)
                            .unwrap_or(&HashSet::new()),
                    ) {
                        blocks.push(block);
                    }

                    ifttt_begin = -1;
                }
            }
        }
    }

    let blocks_by_path: HashMap<&PathBuf, &IctcBlock> =
        blocks.iter().map(|b| (&b.path, b)).collect();

    let ret: Vec<diagnostic::Diagnostic> = blocks
        .iter()
        .filter(|b| blocks_by_path.get(&b.thenchange).is_none())
        .map(|b| diagnostic::Diagnostic {
            range: diagnostic::Range {
                path: b.path.to_str().unwrap().to_string(),
                start: diagnostic::Position {
                    line: b.begin as u64,
                    character: 0,
                },
                end: diagnostic::Position {
                    line: b.end as u64,
                    character: 0,
                },
            },
            severity: diagnostic::Severity::Error,
            code: "if-change-then-change".to_string(),
            message: format!(
                "Expected change in {} because {} was modified",
                b.thenchange.display(),
                b.path.display(),
            ),
        })
        .collect();

    debug!("ICTC blocks are:\n{:?}", blocks);

    Ok(ret)
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
