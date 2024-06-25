use clap::Parser;
use confique::Config;
use horton::config::Conf;
use horton::diagnostic;
use horton::rules::if_change_then_change::ictc;
use horton::rules::pls_no_land::pls_no_land;
use horton::run::{Cli, OutputFormat, Run, Subcommands};

use serde_sarif::sarif;
use std::path::PathBuf;
use std::time::Instant;

fn generate_line_string(original_results: &diagnostic::Diagnostics) -> String {
    return original_results
        .diagnostics
        .iter()
        .map(|d| {
            format!(
                "{}:{}:{}: {} ({})",
                d.range.path, d.range.start.line, d.range.start.character, d.message, d.severity
            )
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
            sarif::ResultBuilder::default()
                .level(d.severity.to_string())
                .locations([sarif::LocationBuilder::default()
                    .physical_location(
                        sarif::PhysicalLocationBuilder::default()
                            .artifact_location(
                                sarif::ArtifactLocationBuilder::default()
                                    .uri(d.range.path.clone())
                                    .build()
                                    .unwrap(),
                            )
                            .region(
                                sarif::RegionBuilder::default()
                                    .start_line(d.range.start.line as i64 + 1)
                                    .start_column(d.range.start.character as i64 + 1)
                                    .end_line(d.range.end.line as i64 + 1)
                                    .end_column(d.range.end.character as i64 + 1)
                                    .build()
                                    .unwrap(),
                            )
                            .build()
                            .unwrap(),
                    )
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
                    start_time.elapsed()
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

fn run() -> anyhow::Result<()> {
    let start = Instant::now();
    let cli: Cli = Cli::parse();

    if let Some(Subcommands::Genconfig {}) = &cli.subcommand {
        Conf::print_default();
        return Ok(());
    }

    let mut ret = diagnostic::Diagnostics::default();

    let config = Conf::builder()
        .env()
        .file("toolbox.toml")
        .file(".config/toolbox.toml")
        .file(".trunk/config/toolbox.toml")
        .file(".trunk/configs/toolbox.toml")
        .load()
        .unwrap_or_else(|err| {
            eprintln!("Toolbox cannot run: {}", err);
            std::process::exit(1);
        });

    let run = Run {
        paths: cli.files.into_iter().map(PathBuf::from).collect(),
        config,
    };

    let (pls_no_land_result, ictc_result): (Result<_, _>, Result<_, _>) =
        rayon::join(|| pls_no_land(&run), || ictc(&run, &cli.upstream));

    match pls_no_land_result {
        Ok(result) => ret.diagnostics.extend(result),
        Err(e) => return Err(e),
    }

    match ictc_result {
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
    env_logger::init();

    match run() {
        Ok(_) => (),
        Err(err) => {
            log::error!("{}", err);
            std::process::exit(1);
        }
    }
}
