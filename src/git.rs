use git2::{AttrCheckFlags, AttrValue, Delta, DiffOptions, Repository};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Hunk {
    pub path: PathBuf,

    /// 1-indexed line number, inclusive
    pub begin: u64,

    /// 1-indexed line number, exclusive
    pub end: u64,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Default)]
pub struct FileChanges {
    /// Set of modified line ranges in new/existing files
    pub hunks: Vec<Hunk>,

    /// Map of changed files and FileStatus
    pub paths: HashMap<String, FileStatus>,
}

fn is_lfs(repo: &Repository, path: &Path) -> bool {
    // "filter" is the primary LFS attribute, see gitattributes(5)
    // FILE_THEN_INDEX checks working tree then index; mimics git itself
    // https://github.com/libgit2/libgit2/blob/v1.5.0/include/git2/attr.h#L104-L116
    if let Ok(filter_bytes) = repo.get_attr_bytes(path, "filter", AttrCheckFlags::FILE_THEN_INDEX) {
        let filter = AttrValue::from_bytes(filter_bytes);
        filter.eq(&AttrValue::from_string(Some("lfs")))
    } else {
        false
    }
}

pub fn modified_since(upstream: &str, repo_path: Option<&Path>) -> anyhow::Result<FileChanges> {
    let path = repo_path.unwrap_or(Path::new("."));
    let repo = Repository::open(path)?;

    let upstream_tree = match repo.find_reference(upstream) {
        Ok(reference) => reference.peel_to_tree()?,
        _ => repo.revparse_single(upstream)?.peel_to_tree()?,
    };

    let diff = {
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        repo.diff_tree_to_workdir_with_index(Some(&upstream_tree), Some(&mut diff_opts))?
    };

    // Iterate through the git diff, building hunks that match the new or modified lines in the
    // diff between the upstream and the working directory. Algorithm is as follows:
    //
    //      current_hunk = None
    //      for (delta, hunk, line) in diff:
    //          if old_lineno == 0, new_lineno == 0:
    //              impossible; do nothing
    //          if old_lineno nonzero, new_lineno == 0:
    //              deleted line; do nothing
    //          if old_lineno == 0, new_lineno nonzero:
    //              new or modified line; create or append to current hunk
    //          if old_lineno nonzero, new_lineno nonzero:
    //              context line or moved line; terminate current hunk
    //
    // The reason we have to do this re-hunking is because if the line numbers of an ICTC block
    // change - likely because more lines were added to the file preceding it - libgit2 will create
    // a DiffHunk which includes the moved lines, so we can't just create one hunk per DiffHunk.
    // Instead, we have to break up DiffHunk instances in up to N hunks, since we only care about
    // the new/modified section of the diff.
    //
    // See https://docs.rs/git2/latest/git2/struct.Diff.html#method.foreach and the underlying API
    // docs at https://libgit2.org/libgit2/#HEAD/group/diff/git_diff_foreach.
    let mut ret = FileChanges::default();
    let mut maybe_current_hunk: Option<Hunk> = None;
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                if !is_lfs(&repo, path) {
                    match delta.status() {
                        Delta::Added => {
                            ret.paths
                                .insert(path.to_string_lossy().to_string(), FileStatus::Added);
                        }
                        Delta::Modified => {
                            ret.paths
                                .insert(path.to_string_lossy().to_string(), FileStatus::Modified);
                        }
                        Delta::Deleted => {
                            ret.paths
                                .insert(path.to_string_lossy().to_string(), FileStatus::Deleted);
                        }
                        _ => {}
                    }
                }
            }
            true
        },
        None,
        None,
        Some(&mut |delta, _, line| {
            if let Some(path) = delta.new_file().path() {
                if !is_lfs(&repo, path) {
                    match delta.status() {
                        Delta::Added
                        | Delta::Copied
                        | Delta::Untracked
                        | Delta::Modified
                        | Delta::Renamed => {
                            if let Some(new_lineno) = line.new_lineno() {
                                if line.old_lineno().is_none() {
                                    maybe_current_hunk = maybe_current_hunk
                                        .as_ref()
                                        .map(|current_hunk| Hunk {
                                            path: current_hunk.path.clone(),
                                            begin: current_hunk.begin,
                                            end: (new_lineno as u64) + 1,
                                        })
                                        .or_else(|| {
                                            Some(Hunk {
                                                path: path.to_path_buf(),
                                                begin: new_lineno as u64,
                                                end: (new_lineno as u64) + 1,
                                            })
                                        });
                                } else if let Some(current_hunk) = &maybe_current_hunk {
                                    ret.hunks.push(current_hunk.clone());
                                    maybe_current_hunk = None;
                                }
                            }
                        }
                        Delta::Unmodified
                        | Delta::Deleted
                        | Delta::Ignored
                        | Delta::Typechange
                        | Delta::Unreadable
                        | Delta::Conflicted => (),
                    }
                }
            }
            true
        }),
    )?;

    if let Some(current_hunk) = &maybe_current_hunk {
        ret.hunks.push(current_hunk.clone());
    }

    Ok(ret)
}
