// trunk-ignore-all(trunk-toolbox/do-not-land)
extern crate regex;

use serde_json::Error;
use serde_sarif::sarif::{Run, Sarif};
use spectral::prelude::*;
use std::fs;
use std::path::Path;

use regex::Regex;
mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn default_sarif() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    let expected_file = Path::new(file!()).parent().unwrap().join("output.sarif");
    let expected_sarif = fs::read_to_string(expected_file)?;

    test_repo.write(
        "alpha.foo",
        "lorem ipsum dolor\ndo-NOT-lAnD\nDONOTLAND sit amet\n".as_bytes(),
    );
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton_with("HEAD", "sarif", false)?;

    let re = Regex::new(r"\d+\.\d+ms").unwrap();
    let normalized_output = re.replace(&horton.stdout, "ms");

    let sarif: Result<Sarif, Error> = serde_json::from_str(&normalized_output);
    assert_that(&sarif.is_ok()).is_true();
    assert_that(&sarif.unwrap().runs).has_length(1);

    Ok(())
}

#[test]
fn default_print() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write(
        "alpha.foo",
        "lorem ipsum dolor\ndo-NOT-lAnD\nDONOTLAND sit amet\n".as_bytes(),
    );
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton_with("HEAD", "text", false)?;
    let expected_text = String::from(
        "alpha.foo:1:0: Found 'do-NOT-lAnD' (error)\nalpha.foo:2:0: Found 'DONOTLAND' (error)\n",
    );

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout).is_equal_to(&expected_text);

    Ok(())
}
