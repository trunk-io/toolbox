use git2::{Delta, DiffOptions, Repository};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Hunk {
    pub path: PathBuf,
    pub begin: i64,
    pub end: i64,
}

#[derive(Debug)]
pub struct NewOrModified {
    // Set of modified line ranges in new/existing files
    pub hunks: Vec<Hunk>,

    // Set of new/modified files
    pub paths: HashSet<PathBuf>,
}

pub fn modified_since(upstream: &str) -> anyhow::Result<NewOrModified> {
    let repo = Repository::open(".")?;

    // ifchange
    let _ = "asdf";
    // do not land
    // thenchange src/main.rs
    let upstream_tree = repo.find_reference(upstream)?.peel_to_tree()?;

    let mut diff_opts = DiffOptions::new();
    diff_opts.include_untracked(true);

    let diff = repo.diff_tree_to_workdir_with_index(Some(&upstream_tree), Some(&mut diff_opts))?;

    let mut ret = NewOrModified {
        hunks: Vec::new(),
        paths: HashSet::new(),
    };
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
                    let path = delta.new_file().path().unwrap().to_path_buf();

                    ret.paths.insert(path.clone());

                    ret.hunks.push(Hunk {
                        path,
                        begin: hunk.new_start() as i64,
                        end: (hunk.new_start() + hunk.new_lines()) as i64,
                    });
                }
                _ => (),
            }
            true
        }),
        None,
    )?;

    Ok(ret)
}
