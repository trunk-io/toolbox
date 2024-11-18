use crate::run::Run;
use anyhow::Context;
use sha2::{Digest, Sha256};

use log::{debug, trace, warn};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, Position, Range, Replacement, Severity};
use crate::git;

#[derive(Debug, Clone)]
pub struct TextBlock {
    pub location: Range,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct RemoteLocation {
    pub repo: String,
    pub path: String,
    pub lock_hash: String,
    // content and location specifying this remote location in the original file
    pub block: TextBlock,
}

impl RemoteLocation {
    pub fn new(line: &str, block: &IctcBlock) -> Self {
        let capture = RE_BEGIN.captures(line).unwrap();

        // get the content of the second capture group which must be a remote file
        let rule = capture
            .get(2)
            .with_context(|| "expected at least 3 captures")
            .unwrap()
            .as_str()
            .trim();

        let parts: Vec<&str> = rule.split_whitespace().collect();
        if parts.len() < 2 {
            panic!("Entry must contain at least two parts separated by space");
        }

        let path_parts: Vec<&str> = parts[1].splitn(2, '#').collect();
        let path = path_parts[0].to_string();
        let lock_hash = if path_parts.len() > 1 {
            path_parts[1].to_string()
        } else {
            String::new()
        };

        // Calculate where in the string 'line' the string 'rule' appears
        let rule_position = line.find(rule).expect("Rule not found in line") as u64;

        let block = TextBlock {
            location: Range {
                start: Position {
                    line: block.begin.unwrap(),
                    character: rule_position,
                },
                end: Position {
                    line: block.begin.unwrap(),
                    character: rule_position + rule.len() as u64,
                },
            },
            text: rule.to_string(),
        };

        Self {
            repo: parts[0].to_string(),
            path,
            lock_hash,
            block,
        }
    }

    fn extract_repo_name(repo_url: &str) -> Option<String> {
        if repo_url.contains("github.com") {
            let re = Regex::new(r"^git@github\.com:[\w\.-]+/(?P<name>[\w\.-]+)\.git$").unwrap();
            if let Some(captures) = re.captures(repo_url) {
                return Some(captures["name"].to_string());
            }
        }
        None
    }

    fn repo_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.repo);
        let result = hasher.finalize();
        let hash_string = format!("{:x}", result);
        let hash_string = &hash_string[..32]; // Get the first 32 characters
        hash_string.to_string()
    }

    pub fn repo_dir_name(&self) -> String {
        if let Some(repo_name) = Self::extract_repo_name(&self.repo) {
            return format!("{}-{}", repo_name, &self.repo_hash());
        }
        self.repo_hash()
    }

    pub fn local_dir(&self, run: &Run) -> PathBuf {
        let mut cache_dir = PathBuf::from(run.cache_dir.clone());

        if !cache_dir.exists() {
            // put the cache in the temp directory
            cache_dir = env::temp_dir();
            if !cache_dir.exists() {
                cache_dir = env::current_dir().expect("Failed to get current directory");
            }
        }
        cache_dir.join(self.repo_dir_name())
    }

    pub fn get_replacement_for_hash(&self, new_hash: &str) -> Replacement {
        if self.lock_hash.is_empty() {
            // nothing to delete - just an insertion
            // find insertion point which is at the end of the file name
            Replacement {
                deleted_region: Range {
                    start: Position {
                        line: self.block.location.start.line,
                        character: self.block.location.end.character,
                    },
                    end: Position {
                        line: self.block.location.start.line,
                        character: self.block.location.end.character,
                    },
                },
                inserted_content: format!("#{}", new_hash),
            }
        } else {
            // find the location of the original hash and replace it with the new hash
            Replacement {
                deleted_region: Range {
                    start: Position {
                        line: self.block.location.start.line,
                        character: self.block.location.end.character - self.lock_hash.len() as u64,
                    },
                    end: Position {
                        line: self.block.location.start.line,
                        character: self.block.location.end.character,
                    },
                },
                inserted_content: new_hash.to_string(),
            }
        }
    }
}

#[derive(Debug)]
pub enum ThenChange {
    RemoteFile(RemoteLocation),
    RepoFile(PathBuf),
    MissingIf,
    MissingThen,
}

#[derive(Debug)]
pub enum IfChange {
    RemoteFile(RemoteLocation),
    RepoFile(PathBuf),
}

#[derive(Debug)]
pub struct IctcBlock {
    pub path: PathBuf,
    pub begin: Option<u64>,
    pub end: Option<u64>,
    pub ifchange: Option<IfChange>,
    pub thenchange: Option<ThenChange>,
}

impl IctcBlock {
    fn get_range(&self) -> Range {
        Range {
            start: Position {
                line: self.begin.unwrap(),
                character: 0,
            },
            end: Position {
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

pub struct Ictc<'a> {
    run: &'a Run,
    upstream: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> Ictc<'a> {
    pub fn new(run: &'a Run, upstream: &str) -> Self {
        Self {
            run,
            upstream: upstream.to_string(),
            diagnostics: Vec::new(),
        }
    }

    pub fn run(&mut self) -> anyhow::Result<Vec<Diagnostic>> {
        let config = &self.run.config.ifchange;

        if !config.enabled {
            trace!("'ifchange' is disabled");
            return Ok(vec![]);
        }

        if self.run.is_upstream() {
            trace!("'if-change' rule doesn't run on upstream");
            return Ok(vec![]);
        }

        debug!(
            "scanning {} files for if_change_then_change",
            self.run.paths.len()
        );

        // Build up list of files that actually have a ifchange block - this way we can avoid
        // processing git modified chunks if none are present
        let all_blocks: Vec<_> = self
            .run
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

        let modified = git::modified_since(&self.upstream, None)?;
        let hunks = &modified.hunks;

        log::trace!("modified stats, per libgit2:\n{:#?}", modified);

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

        for block in &blocks {
            if let Some(change) = &block.ifchange {
                match change {
                    IfChange::RemoteFile(remote) => {
                        // remote file should be in form of
                        // {REMOTE_REPO} {REMOTE_PATH}#{LOCK_HASH}
                        if self.ifchange_remote(remote, block) {
                            // if it's ok we will keep processing the rest of the rule
                            self.thenchange(block, &blocks_by_path);
                        }
                    }
                    _ => self.thenchange(block, &blocks_by_path),
                }
            } else {
                self.thenchange(block, &blocks_by_path);
            }
        }

        log::trace!("ICTC blocks are:\n{:?}", blocks);
        Ok(self.diagnostics.clone())
    }

    pub fn build_or_get_remote_repo(
        &mut self,
        remote: &RemoteLocation,
        block: &IctcBlock,
    ) -> Result<PathBuf, Diagnostic> {
        let repo_path = remote.local_dir(self.run);

        // Check if repo_dir exists
        if repo_path.exists() {
            if !git::dir_inside_git_repo(&repo_path) {
                // must delete repo and try again
                std::fs::remove_dir_all(&repo_path)
                    .expect("Failed to remove repository and its contents");
            } else {
                return Ok(repo_path);
            }
        }

        log::debug!("cloning {:?} at {:?}", remote.repo.as_str(), repo_path);

        let result = git::clone(remote.repo.as_str(), &repo_path);
        if result.status.success() {
            return Ok(repo_path);
        }

        Err({
            block_diagnostic(
                block,
                Severity::Warning,
                "if-change-clone-failed",
                format!(
                    "Failed to clone remote repo at {}: {}",
                    remote.repo, result.stderr
                )
                .as_str(),
            )
        })
    }

    pub fn ifchange_remote(&mut self, remote: &RemoteLocation, block: &IctcBlock) -> bool {
        // get path to clone of remote repo.
        match self.build_or_get_remote_repo(remote, block) {
            Ok(path) => {
                println!("repo is cloned shallow at {:?}", path);
                // now check if the remote file has changed since the marker
                let lc = git::last_commit(&remote.local_dir(self.run), &remote.path);

                match lc {
                    Ok(commit) => {
                        let truncated_hash = &commit.hash[..10];
                        if remote.lock_hash.is_empty() {
                            // No lock hash was provided - recommend a diagnostic fix that includes the desired hash
                            let message =
                                format!("Hash for IfChange required; should be {}", truncated_hash);
                            let diagnostic = Diagnostic {
                                path: block.path.to_str().unwrap().to_string(),
                                range: Some(block.get_range()),
                                severity: Severity::Warning,
                                code: "if-change-update-lock-hash".to_string(),
                                message: message.clone(),
                                replacements: Some(vec![
                                    remote.get_replacement_for_hash(truncated_hash)
                                ]),
                            };

                            self.diagnostics.push(diagnostic);
                            return false;
                        }
                        // If the lock hash matches the last commit hash - then nothing to do
                        if commit.hash.starts_with(&remote.lock_hash) {
                            // nothing changed - all good
                            false
                        } else {
                            // commit hash has changed - must confirm we changed the code inside
                            // and also add an autofix diag to move the hash lock marker

                            let message =
                                format!("Remote file changed; new hash is {}", truncated_hash);
                            let diagnostic = Diagnostic {
                                path: block.path.to_str().unwrap().to_string(),
                                range: Some(block.get_range()),
                                severity: Severity::Warning,
                                code: "if-change-remote-updated-new-hash".to_string(),
                                message: message.clone(),
                                replacements: Some(vec![
                                    remote.get_replacement_for_hash(truncated_hash)
                                ]),
                            };

                            self.diagnostics.push(diagnostic);
                            true
                        }
                    }
                    Err(e) => {
                        self.diagnostics.push(block_diagnostic(
                            block,
                            Severity::Error,
                            "if-change-git-error",
                            format!(
                                "Failed to get last commit of remote file {}: {}",
                                remote.path, e
                            )
                            .as_str(),
                        ));
                        false
                    }
                }
            }
            Err(diag) => {
                self.diagnostics.push(diag);
                false
            }
        }
    }

    fn thenchange(&mut self, block: &IctcBlock, blocks_by_path: &HashMap<&PathBuf, &IctcBlock>) {
        if let Some(change) = &block.thenchange {
            match change {
                ThenChange::RemoteFile(_remote_file) => {
                    todo!("build support for remote file")
                }
                ThenChange::RepoFile(local_file) => {
                    // Check if the repo file exists - if it was deleted this is a warning
                    if !Path::new(local_file).exists() {
                        self.diagnostics.push(Diagnostic {
                            path: block.path.to_str().unwrap().to_string(),
                            range: Some(block.get_range()),
                            severity: Severity::Warning,
                            code: "if-change-file-does-not-exist".to_string(),
                            message: format!("ThenChange {} does not exist", local_file.display(),),
                            replacements: None,
                        });
                    }
                    // If target file was not changed raise issue
                    if blocks_by_path.get(&local_file).is_none() {
                        self.diagnostics.push(Diagnostic {
                            path: block.path.to_str().unwrap().to_string(),
                            range: Some(block.get_range()),
                            severity: Severity::Error,
                            code: "if-change-then-change-this".to_string(),
                            message: format!(
                                "Expected change in {} because {} was modified",
                                local_file.display(),
                                block.path.display(),
                            ),
                            replacements: None,
                        });
                    }
                }
                ThenChange::MissingIf => {
                    trace!("MissingIf processing...");
                    self.diagnostics.push(Diagnostic {
                        path: block.path.to_str().unwrap().to_string(),
                        range: Some(block.get_range()),
                        severity: Severity::Warning,
                        code: "if-change-mismatched".to_string(),
                        message: "Expected preceding IfChange tag".to_string(),
                        replacements: None,
                    });
                }
                ThenChange::MissingThen => {
                    self.diagnostics.push(block_diagnostic(
                        block,
                        Severity::Warning,
                        "if-change-mismatched",
                        "Expected matching ThenChange tag",
                    ));
                }
            }
        }
    }
}

pub fn block_diagnostic(block: &IctcBlock, sev: Severity, code: &str, msg: &str) -> Diagnostic {
    Diagnostic {
        path: block.path.to_str().unwrap().to_string(),
        range: Some(block.get_range()),
        severity: sev,
        code: code.to_string(),
        message: msg.to_string(),
        replacements: None,
    }
}

pub fn find_ictc_blocks(path: &PathBuf) -> anyhow::Result<Vec<IctcBlock>> {
    let mut blocks: Vec<IctcBlock> = Vec::new();

    trace!("scanning contents of {}", path.display());

    let in_file = File::open(path).with_context(|| {
        let error_message = format!("failed to open: {:#?}", path);
        warn!("{}", error_message);
        error_message
    })?;

    let in_buf = BufReader::new(in_file);

    let mut block: Option<IctcBlock> = None;

    for (i, line) in lines_view(in_buf)
        .context(format!("failed to read lines of text from: {:#?}", path))?
        .iter()
        .enumerate()
        .map(|(i, line)| (i + 1, line))
    {
        let line_no = Some(i as u64);
        if let Some(begin_capture) = RE_BEGIN.captures(line) {
            if let Some(mut block_value) = block {
                // Two if blocks in a row - report problem
                block_value.end = block_value.begin;
                block_value.thenchange = Some(ThenChange::MissingThen);
                blocks.push(block_value);
            }

            // get the content of the second capture group which should be either a remote file or blank
            let source_trigger = begin_capture
                .get(2)
                .with_context(|| "expected at least 3 captures")?
                .as_str()
                .trim();

            let mut ib = IctcBlock {
                path: path.clone(),
                begin: line_no,
                end: None,
                ifchange: None,
                thenchange: None,
            };

            ib.ifchange = if source_trigger.is_empty() {
                None
            } else if source_trigger.contains(' ') {
                // If the source trigger has a space in the middle then its in the format of a remote repo file
                Some(IfChange::RemoteFile(RemoteLocation::new(line, &ib)))
            } else {
                // Looks like a file path but it doesn't exist
                Some(IfChange::RepoFile(PathBuf::from(source_trigger)))
            };
            block = Some(ib);
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
                    ifchange: None,
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

type LinesView = Vec<String>;

fn lines_view<R: BufRead>(reader: R) -> anyhow::Result<LinesView> {
    let mut ret: LinesView = LinesView::default();
    for line in reader.lines() {
        let line = line?;
        ret.push(line);
    }
    Ok(ret)
}

// IfChange git@github.com:eslint/eslint.git LICENSE
// Content inside here
// ThenChange

// IfChange git@github.com:eslint/eslint.git LICENSE#ABCDEF
// Content inside here
// ThenChange
