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

/// Hand-built minimum SARIF document used when the normal output pipeline
/// can't produce one. Kept as a constant so the fallback doesn't have to
/// re-enter serde_json just to say "nothing ran".
const EMPTY_SARIF: &str = "{\"version\":\"2.1.0\",\"runs\":[]}";

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

    let outfile = cli.results.clone();
    let output_format = cli.output_format;

    // The `--results=${tmpfile}` contract (see plugin.yaml, read_output_from:
    // tmp_file) requires that toolbox always create the output file the
    // caller pointed us at - downstream readers like trunk-check unconditionally
    // read it, and if the file is missing they report the linter as failed.
    // Split the real work into `build_output` so we can synthesize a valid
    // SARIF fallback on catastrophic failure (config load, SARIF builder)
    // and still write it out before surfacing the error.
    let (output_string, exit_err) = match build_output(cli, &start) {
        Ok((output, err)) => (output, err),
        Err(err) => (fallback_output_string(output_format, &err), Some(err)),
    };

    if let Some(path) = &outfile {
        std::fs::write(path, &output_string)
            .with_context(|| format!("failed to write results to {:?}", path))?;
    } else {
        println!("{}", output_string);
    }

    match exit_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

/// Run the rule pipeline and produce the output string that should be
/// written to `--results` (or stdout).
///
/// Return shape:
/// - `Ok((output, None))` - clean run, exit 0.
/// - `Ok((output, Some(err)))` - at least one rule failed. The failures are
///   already embedded in `output` as SARIF results with rule id
///   `toolbox-rule-error`; `err` carries the non-zero-exit signal.
/// - `Err(err)` - catastrophic failure before we could produce a useful
///   output string (e.g. config file parse error). The caller is responsible
///   for synthesizing a fallback document so the output file still exists.
fn build_output(
    cli: Cli,
    start: &Instant,
) -> anyhow::Result<(String, Option<anyhow::Error>)> {
    let mut ret = diagnostic::Diagnostics::default();

    // If no configuration file is provided the default config will be used;
    // some parts of toolbox can run with the default config.
    let toolbox_toml: String = match find_toolbox_toml() {
        Some(file) => file,
        None => "no_config_found.toml".to_string(),
    };

    let config = Conf::builder()
        .env()
        .file(&toolbox_toml)
        .load()
        .with_context(|| format!("failed to load toolbox config from {:?}", toolbox_toml))?;

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

    // Individual rule failures must not abort the pipeline: the caller still
    // needs a valid output file. Convert each failure into an error-level
    // diagnostic so it rides along in the SARIF/text output, and surface the
    // accumulated failure list as the non-zero-exit signal at the end.
    let mut failed_rules: Vec<String> = Vec::new();
    for (i, result) in results.into_iter().enumerate() {
        let rule_name = RULES[i].0;
        match result {
            Ok(diagnostics) => ret.diagnostics.extend(diagnostics),
            Err(err) => {
                log::error!("rule '{}' failed: {:#}", rule_name, err);
                failed_rules.push(rule_name.to_string());
                ret.diagnostics.push(diagnostic::Diagnostic {
                    path: String::new(),
                    range: None,
                    severity: diagnostic::Severity::Error,
                    code: "toolbox-rule-error".to_string(),
                    message: format!("rule '{}' failed: {:#}", rule_name, err),
                    replacements: None,
                });
            }
        }
    }

    let output_string = match cli.output_format {
        OutputFormat::Sarif => generate_sarif_string(&ret, &run, start)?,
        OutputFormat::Text => generate_line_string(&ret),
    };

    let exit_err = if failed_rules.is_empty() {
        None
    } else {
        Some(anyhow::anyhow!(
            "rule(s) failed: {}",
            failed_rules.join(", ")
        ))
    };
    Ok((output_string, exit_err))
}

/// Produce a minimal output document describing `err` so we can still honor
/// `--results=${tmpfile}` when the normal pipeline couldn't produce anything.
fn fallback_output_string(format: OutputFormat, err: &anyhow::Error) -> String {
    match format {
        OutputFormat::Sarif => minimal_error_sarif(err),
        OutputFormat::Text => format!("error: {:#}\n", err),
    }
}

/// Hand-built SARIF (bypasses the serde_sarif builder on purpose - that
/// builder may have just been the thing that failed us).
fn minimal_error_sarif(err: &anyhow::Error) -> String {
    let doc = serde_json::json!({
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": { "name": "trunk-toolbox" } },
            "results": [{
                "ruleId": "toolbox-error",
                "level": "error",
                "message": { "text": format!("{:#}", err) },
            }],
        }],
    });
    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| EMPTY_SARIF.to_string())
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
