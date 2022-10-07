use clap::Parser;
use horton::diagnostic;
use horton::git;
use horton::rules::if_change_then_change::ictc;
use horton::rules::pls_no_land::pls_no_land;
use serde_sarif::sarif;

#[derive(Parser, Debug)]
#[clap(version = "0.1", author = "Trunk Technologies Inc.")]
struct Opts {
    #[clap(long)]
    // #[arg(default_value_t = String::from("refs/heads/main"))]
    #[arg(default_value_t = String::from("HEAD"))]
    upstream: String,
}

fn run() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();

    let mut ret = diagnostic::Diagnostics::default();
    let modified = git::modified_since(&opts.upstream)?;

    log::debug!("Modified stats, per libgit2:\n{:#?}", modified);

    ret.diagnostics.extend(pls_no_land(&modified.paths)?);
    ret.diagnostics.extend(ictc(&modified.hunks)?);

    // TODO(sam): figure out how to stop using unwrap() inside the map() calls below
    let results: Vec<sarif::Result> = ret
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

    let run = sarif::RunBuilder::default()
        .tool(
            sarif::ToolBuilder::default()
                .driver(
                    sarif::ToolComponentBuilder::default()
                        .name("horton")
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
    println!("{}", sarif);

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
