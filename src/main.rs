use clap::Parser;
use confique::Config;
use horton::config::Conf;
use horton::diagnostic;
use horton::rules::if_change_then_change::Ictc;
use horton::rules::never_edit::never_edit;
use horton::rules::pls_no_land::pls_no_land;
use horton::run::{Cli, OutputFormat, Run, Subcommands};

use serde_sarif::sarif;
use std::path::{Path, PathBuf};
use std::time::Instant;

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
        .map(|d| {
            let mut physical_location = sarif::PhysicalLocationBuilder::default();
            physical_location.artifact_location(
                sarif::ArtifactLocationBuilder::default()
                    .uri(d.path.clone())
                    .build()
                    .unwrap(),
            );

            if let Some(range) = &d.range {
                physical_location.region(
                    sarif::RegionBuilder::default()
                        .start_line(range.start.line as i64 + 1)
                        .start_column(range.start.character as i64 + 1)
                        .end_line(range.end.line as i64 + 1)
                        .end_column(range.end.character as i64 + 1)
                        .build()
                        .unwrap(),
                );
            }
            sarif::ResultBuilder::default()
                .level(d.severity.to_string())
                .locations([sarif::LocationBuilder::default()
                    .physical_location(physical_location.build().unwrap())
                    .build()
                    .unwrap()])
                .message(
                    sarif::MessageBuilder::default()
                        .text(d.message.clone())
                        .build()
                        .unwrap(),
                )
                .rule_id(d.code.clone())
                .build()
                .unwrap()
        })
        .collect();

    let r = sarif::ResultBuilder::default()
        .message(
            sarif::MessageBuilder::default()
                .text(format!(
                    "{:?} files processed in {:?}",
                    run_context.paths.len(),
                    start_time.elapsed(),
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
    let cli: Cli = Cli::parse();

    if let Some(Subcommands::Genconfig {}) = &cli.subcommand {
        Conf::print_default();
        return Ok(());
    }

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

    let run = Run {
        paths: cli.files.into_iter().map(PathBuf::from).collect(),
        config,
        config_path: toolbox_toml,
        cache_dir: cli.cache_dir.clone(),
    };

    let (pls_no_land_result, ictc_result): (Result<_, _>, Result<_, _>) = rayon::join(
        || pls_no_land(&run),
        || Ictc::new(&run, &cli.upstream).run(),
    );

    match pls_no_land_result {
        Ok(result) => ret.diagnostics.extend(result),
        Err(e) => return Err(e),
    }

    match ictc_result {
        Ok(result) => ret.diagnostics.extend(result),
        Err(e) => return Err(e),
    }

    //TODO: refactor this to use a threadpool for all the rules. using rayon::join() won't scale
    //beyond two things
    let ne_result = never_edit(&run, &cli.upstream);
    match ne_result {
        Ok(result) => ret.diagnostics.extend(result),
        Err(e) => return Err(e),
    }

    let mut output_string = generate_line_string(&ret);
    if cli.output_format == OutputFormat::Sarif {
        output_string = generate_sarif_string(&ret, &run, &start)?;
    }

    if let Some(outfile) = &cli.results {
        std::fs::write(outfile, output_string)?;
    } else {
        println!("{}", output_string);
    }

    Ok(())
}

fn main() {
    match log4rs::init_file("log4rs.yaml", Default::default()) {
        Ok(_) => {
            // Initialization succeeded
            println!("Logging initialized successfully.");
        }
        Err(e) => {
            // Initialization failed
            eprintln!("Failed to initialize logging: {}", e);
            // error!("Failed to initialize logging: {}", e);
            // Handle the error, e.g., fallback to a default logger or exit
        }
    }

    match run() {
        Ok(_) => (),
        Err(err) => {
            log::error!("{}", err);
            std::process::exit(1);
        }
    }
}
