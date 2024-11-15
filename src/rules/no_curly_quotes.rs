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

pub fn no_curly_quotes(run: &Run, upstream: &str) -> anyhow::Result<Vec<diagnostic::Diagnostic>> {
    let config = &run.config.nocurlyquotes;

    if !config.enabled {
        trace!("'nocurlyquotes' is disabled");
        return Ok(vec![]);
    }

    let mut diagnostics: Vec<diagnostic::Diagnostic> = Vec::new();

    debug!("scanning {} files for curly quotes", run.paths.len());

    // Scan files in parallel
    let results: Result<Vec<_>, _> = run
        .paths
        .par_iter()
        .map(|path| no_curly_quotes_impl(path, run))
        .collect();

    match results {
        Ok(v) => Ok(v.into_iter().flatten().collect()),
        Err(e) => Err(e),
    }
}

fn no_curly_quotes_impl(path: &PathBuf, run: &Run) -> anyhow::Result<Vec<diagnostic::Diagnostic>> {
    let config: &crate::config::Conf = &run.config;

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
