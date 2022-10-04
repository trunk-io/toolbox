use std::fs::File;
use std::io::{BufRead, BufReader};

use anyhow::Context;
use clap::Parser;
use horton::lsp_json;
use regex::Regex;

#[derive(Parser, Debug)]
#[clap(version = "0.1", author = "Trunk Technologies Inc.")]
struct Opts {
    #[clap(short, long)]
    file: String,
}

type LinesView = Vec<String>;

fn lines_view<R: BufRead>(reader: R) -> anyhow::Result<LinesView> {
    let mut ret: LinesView = LinesView::default();
    for line in reader.lines() {
        let line = line?;
        ret.push(line);
    }
    Ok(ret)
}

fn run() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();
    let re = Regex::new(r"(?i)(DO[\s_-]+NOT[\s_-]+LAND)").unwrap();

    let in_file =
        File::open(&opts.file).with_context(|| format!("failed to open: {}", opts.file))?;
    let in_buff = BufReader::new(in_file);
    let lines_view = lines_view(in_buff).context("failed to build lines view")?;
    let mut ret = lsp_json::LspJson::default();

    for (i, line) in lines_view.iter().enumerate() {
        // trunk-ignore(horton/do-not-land)
        if line.contains("trunk-ignore(horton/do-not-land)") {
            continue;
        }
        let m = if let Some(m) = re.find(line) {
            m
        } else {
            continue;
        };

        ret.diagnostics.push(lsp_json::Diagnostic {
            range: lsp_json::Range {
                start: lsp_json::Position {
                    line: i as u64,
                    character: m.start() as u64,
                },
                end: lsp_json::Position {
                    line: i as u64,
                    character: m.end() as u64,
                },
            },
            severity: lsp_json::Severity::Error,
            // trunk-ignore(horton/do-not-land)
            code: "do-not-land".to_string(),
            message: format!("Found '{}'", m.as_str()),
        });
    }

    let diagnostics_str = ret.to_string()?;
    println!("{}", diagnostics_str);

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
