use clap::Parser;
use horton::diagnostic;
use horton::rules::if_change_then_change::ictc;
use horton::rules::pls_no_land::pls_no_land;
use serde_sarif::sarif;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;
#[derive(Parser, Debug)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "Trunk Technologies Inc.")]
struct Opts {
    // #[arg(short, long, num_args = 1..)]
    files: Vec<String>,

    #[clap(long)]
    #[arg(default_value_t = String::from("HEAD"))]
    upstream: String,

    #[clap(long)]
    #[arg(default_value_t = String::from(""))]
    results: String,
}

fn run() -> anyhow::Result<()> {
    let start = Instant::now();
    let opts: Opts = Opts::parse();

    let mut ret = diagnostic::Diagnostics::default();

    // Convert to PathBufs
    let paths: HashSet<PathBuf> = opts.files.into_iter().map(PathBuf::from).collect();

    let (pls_no_land_result, ictc_result): (Result<_, _>, Result<_, _>) =
        rayon::join(|| pls_no_land(&paths), || ictc(&paths, &opts.upstream));

    match pls_no_land_result {
        Ok(result) => ret.diagnostics.extend(result),
        Err(e) => return Err(e),
    }

    match ictc_result {
        Ok(result) => ret.diagnostics.extend(result),
        Err(e) => return Err(e),
    }

    // TODO(sam): figure out how to stop using unwrap() inside the map() calls below
    let mut results: Vec<sarif::Result> = ret
        .diagnostics
        .iter()
        .map(|d| {
            sarif::ResultBuilder::default()
                .level("error")
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
                    paths.len(),
                    start.elapsed()
                ))
                .build()
                .unwrap(),
        )
        .rule_id("toolbox-perf")
        .level("note")
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

    if opts.results.is_empty() {
        println!("{}", sarif);
    } else {
        std::fs::write(opts.results, sarif)?;
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
