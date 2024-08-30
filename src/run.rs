use crate::config::Conf;
use std::collections::HashSet;
use std::path::PathBuf;

use clap::builder::PossibleValue;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OutputFormat {
    Sarif,
    Text,
}

impl ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Sarif, Self::Text]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::Sarif => PossibleValue::new("sarif"),
            Self::Text => PossibleValue::new("text"),
        })
    }
}

#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "Trunk Technologies Inc.")]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommands>,

    pub files: Vec<String>,

    #[clap(long)]
    #[arg(default_value_t = String::from("HEAD"))]
    pub upstream: String,

    #[clap(long, default_value = "sarif")]
    pub output_format: OutputFormat,

    #[clap(long, default_value = "")]
    /// optional cache directory location
    pub cache_dir: String,

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
    pub is_upstream: bool,
    pub config_path: String,
}
