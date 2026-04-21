use clap::Parser;
use confique::Config;
use horton::config::Conf;
use horton::diagnostic;
use horton::rules::RULES;
use horton::run::{Cli, OutputFormat, Run, Subcommands};

use anyhow::Context;
use log::{debug, warn};
use serde_sarif::sarif;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

use log::LevelFilter;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
};

fn generate_line_string(original_results: &diagnostic::Diagnostics) -> String {
    return original_results
        .diagnostics
        .iter()
        .map(|d| {
            if let Some(range) = &d.range {
                format!(
                    "{}:{}:{}: {} ({})",
                    d.path, range.start.line, range.start.character, d.message, d.severity
                )
            } else {
                format!("{}: {} ({})", d.path, d.message, d.severity)
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
}

fn generate_sarif_string(
    original_results: &diagnostic::Diagnostics,
    run_context: &Run,
    start_time: &Instant,
) -> anyhow::Result<String> {
    // TODO(sam): figure out how to stop using unwrap() inside the map() calls below
    let mut results: Vec<sarif::Result> = original_results
        .diagnostics
        .iter()
        .map(|d| d.to_sarif())
        .collect();

    let r = sarif::ResultBuilder::default()
        .message(
            sarif::MessageBuilder::default()
                .text(format!(
                    "{:?} files processed in {:?} files:[{}]",
                    run_context.paths.len(),
                    start_time.elapsed(),
                    run_context
                        .paths
                        .iter()
                        .map(|p| p.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .build()
                .unwrap(),
        )
        .rule_id("toolbox-perf")
        .level(diagnostic::Severity::Note.to_string())
        .build()
        .unwrap();

    results.push(r);

    let run = sarif::RunBuilder::default()
        .tool(
            sarif::ToolBuilder::default()
                .driver(
                    sarif::ToolComponentBuilder::default()
                        .name("trunk-toolbox")
                        .build()
                        .unwrap(),
                )
                .build()
                .unwrap(),
        )
        .results(results)
        .build()?;
    let sarif_built = sarif::SarifBuilder::default()
        .version("2.1.0")
        .runs([run])
        .build()?;

    let sarif = serde_json::to_string_pretty(&sarif_built)?;
    Ok(sarif)
}

fn find_toolbox_toml() -> Option<String> {
    let files = ["toolbox.toml", ".config/toolbox.toml"];

    for file in &files {
        if Path::new(file).exists() {
            return Some(file.to_string());
        }
    }

    None
}

fn run() -> anyhow::Result<()> {
    let start = Instant::now();
    let mut cli: Cli = Cli::parse();

    if let Some(Subcommands::Genconfig {}) = &cli.subcommand {
        Conf::print_default();
        return Ok(());
    }

    if let Err(e) = validate_results_path(cli.results.as_deref()) {
        eprintln!("Toolbox cannot run: {}", e);
        std::process::exit(1);
    }
    cli.cache_dir = validate_cache_dir(cli.cache_dir);

    let mut ret = diagnostic::Diagnostics::default();

    // If not configuration file is provided the default config will be used
    // some parts of toolbo can run with the default config
    let toolbox_toml: String = match find_toolbox_toml() {
        Some(file) => file,
        None => "no_config_found.toml".to_string(),
    };

    let config = Conf::builder()
        .env()
        .file(&toolbox_toml)
        .load()
        .unwrap_or_else(|err| {
            eprintln!("Toolbox cannot run: {}", err);
            std::process::exit(1);
        });

    let upstream_mode = cli.upstream_mode || cli.cache_dir.ends_with("-upstream");

    let run = Run {
        paths: cli.files.into_iter().map(PathBuf::from).collect(),
        config,
        config_path: toolbox_toml,
        cache_dir: cli.cache_dir.clone(),
        upstream_mode,
    };

    let mut results: Vec<anyhow::Result<Vec<diagnostic::Diagnostic>>> =
        RULES.iter().map(|_| Ok(vec![])).collect();

    rayon::scope(|s| {
        for (result, (_, rule_fn)) in results.iter_mut().zip(RULES.iter()) {
            let run = &run;
            let upstream = cli.upstream.as_str();
            s.spawn(move |_| {
                *result = rule_fn(run, upstream);
            });
        }
    });

    for (i, result) in results.into_iter().enumerate() {
        let rule_name = RULES[i].0;
        ret.diagnostics
            .extend(result.with_context(|| format!("rule '{}' failed", rule_name))?);
    }

    let mut output_string = generate_line_string(&ret);
    if cli.output_format == OutputFormat::Sarif {
        output_string = generate_sarif_string(&ret, &run, &start)?;
    }

    if let Some(outfile) = &cli.results {
        std::fs::write(outfile, &output_string)
            .with_context(|| format!("failed to write results to {:?}", outfile))?;
    } else {
        println!("{}", output_string);
    }

    Ok(())
}

/// Validate the `--results` path before we do any real work so the caller
/// gets an actionable error instead of a late, context-free io::Error out of
/// the final write. A nonexistent parent directory is treated as a usage bug
/// (toolbox does not silently create scratch directories).
fn validate_results_path(results: Option<&str>) -> anyhow::Result<()> {
    let Some(outfile) = results else {
        return Ok(());
    };

    let path = Path::new(outfile);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            anyhow::bail!(
                "--results path {:?} is not writable: parent directory {:?} does not exist",
                outfile,
                parent
            );
        }
    }
    if path.is_dir() {
        anyhow::bail!(
            "--results path {:?} is a directory, expected a file",
            outfile
        );
    }
    Ok(())
}

/// Validate `--cache-dir`. An unusable value is not fatal - toolbox can run
/// without a cache - so we warn on stderr and blank it out so the rest of
/// the pipeline behaves as if `--cache-dir` was never set.
fn validate_cache_dir(cache_dir: String) -> String {
    if cache_dir.is_empty() {
        return cache_dir;
    }
    let path = Path::new(&cache_dir);
    if path.is_dir() {
        return cache_dir;
    }
    eprintln!(
        "warning: --cache-dir {:?} is not an accessible directory; running without a cache",
        cache_dir
    );
    String::new()
}

fn init_default_logger() {
    // Create a console appender for stdout
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%H:%M:%S)} | {({l}):5.5} | {f}:{L} | {m}{n}",
        )))
        .build();

    // Build the log4rs configuration - log only errors to stdout by default
    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Error))
        .expect("Failed to build log4rs configuration");

    log4rs::init_config(config).unwrap();
}

fn main() {
    // initialize logging from file if log4rs.yaml exists
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let log_config_path = current_dir.join("log4rs.yaml");
    if log_config_path.exists() {
        match log4rs::init_file(&log_config_path, Default::default()) {
            Ok(_) => {
                // Initialization succeeded
                debug!("logging initialized - {:?}", log_config_path);
            }
            Err(e) => {
                init_default_logger();
                warn!("Falling back to default logging setup. override with valid 'log4rs.yaml' file, {}", e);
            }
        }
    } else {
        init_default_logger();
        debug!("using default built-in logging setup - no log4rs.yaml found");
    }

    match run() {
        Ok(_) => (),
        Err(err) => {
            log::error!("{}", err);
            std::process::exit(1);
        }
    }
}
