use assert_cmd::prelude::*; // Add methods on commands

use std::error::Error;
use std::process::Command;
use std::result::Result;

pub fn run_horton(file: &str) -> Result<String, Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("horton")?;

    cmd.arg("--file").arg(file);

    let output = cmd.output()?;

    return Ok(String::from_utf8(output.stdout)?);
}
