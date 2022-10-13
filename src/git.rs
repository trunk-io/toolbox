use git2::{Delta, DiffOptions, Oid, Repository};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Hunk {
    pub path: PathBuf,
    pub begin: i64,
    pub end: i64,
}

#[derive(Debug, Default)]
pub struct NewOrModified {
    /// Set of modified line ranges in new/existing files
    pub hunks: Vec<Hunk>,

    /// Set of new/modified files
    pub paths: HashSet<PathBuf>,
}

pub fn modified_since(upstream: &str) -> anyhow::Result<NewOrModified> {
    let repo = Repository::open(".")?;

    let upstream_tree = match repo.find_reference(upstream) {
        Ok(reference) => reference.peel_to_tree()?,
        _ => repo.find_object(Oid::from_str(upstream)?, None)?.peel_to_tree()?,
    };

    let diff = {
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        repo.diff_tree_to_workdir_with_index(Some(&upstream_tree), Some(&mut diff_opts))?
    };

    let mut ret = NewOrModified::default();
    diff.foreach(
        &mut |_, _| true,
        None,
        Some(&mut |delta, hunk| {
            match delta.status() {
                Delta::Unmodified
                | Delta::Added
                | Delta::Modified
                | Delta::Renamed
                | Delta::Copied
                | Delta::Untracked => {
                    if let Some(path) = delta.new_file().path() {
                        let path = path.to_path_buf();

                        ret.paths.insert(path.clone());
                        ret.hunks.push(Hunk {
                            path,
                            begin: hunk.new_start() as i64,
                            end: (hunk.new_start() + hunk.new_lines()) as i64,
                        });
                    } else {
                        // TODO(sam): accumulate errors and return them
                        // See https://doc.rust-lang.org/rust-by-example/error/iter_result.html
                        log::error!("Found git delta where new_file had no path");
                    }
                }
                _ => (),
            }
            true
        }),
        None,
    )?;

    Ok(ret)
}
