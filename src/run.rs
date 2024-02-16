use crate::config::Conf;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct Run {
    pub paths: HashSet<PathBuf>,
    pub config: Conf,
}
