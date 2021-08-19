use clap::{AppSettings, Clap};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

#[derive(Clap)]
#[clap(version = "0.1", author = "Trunk Technologies Inc.")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short, long)]
    file: String,
}

fn lines_view(filename: &Path) -> Vec<String> 
{
    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => panic!("Unable to open file {}", filename.display()),
    };
    let buffer = BufReader::new(file);
    return buffer.lines().map( |l| l.expect("Could not parse line"))
    .collect();
    
}

fn main() {
    let opts: Opts = Opts::parse();
    let filename = opts.file;

    // Assume all rules are enabled. 

    // Read the contents of the file into a string before we pass it along to all the rules

    let lines = lines_view(Path::new(&filename));
    for line in lines {
        println!("{:?}", line);
    }
}
