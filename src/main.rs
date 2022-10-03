use clap::Parser;

use regex::Regex;

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use horton::lsp_json;

#[derive(Parser, Debug)]
#[clap(version = "0.1", author = "Trunk Technologies Inc.")]
struct Opts {
    file: String,
}

fn lines_view(filename: &Path) -> Vec<String> {
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => panic!("Unable to open file {}", filename.display()),
    };
    let buffer = BufReader::new(file);
    return buffer
        .lines()
        .map(|l| l.expect("Could not parse line"))
        .collect();
}

fn main() {
    let opts: Opts = Opts::parse();

    let re = Regex::new(r"(?i)(DO[\s_-]+NOT[\s_-]+LAND)").unwrap();

    let mut ret = lsp_json::LspJson {
        diagnostics: Vec::new(),
    };

    for (i, line) in lines_view(Path::new(&opts.file)).iter().enumerate() {
        // trunk-ignore(horton/do-not-land)
        if line.contains("trunk-ignore(horton/do-not-land)") {
            continue;
        }
        let maybe_match = re.find(&line);
        if maybe_match.is_none() {
            continue;
        }
        let m = maybe_match.unwrap();
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

    match ret.to_string() {
        Ok(s) => {
            println!("{}", s)
        }
        Err(err) => {
            panic!("Failed to serialize diagnostics, error was: {}", err)
        }
    }
}
