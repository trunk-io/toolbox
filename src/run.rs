use crate::config::Conf;
use std::collections::HashSet;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "Trunk Technologies Inc.")]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommands>,

    pub files: Vec<String>,

    #[clap(long)]
    #[arg(default_value_t = String::from("HEAD"))]
    pub upstream: String,

    #[clap(long)]
    /// optional path to write results to
    pub results: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Subcommands {
    // print default config for toolbox
    /// Generate default configuration content for toolbox
    Genconfig,
}

pub struct Run {
    pub paths: HashSet<PathBuf>,
    pub config: Conf,
}
