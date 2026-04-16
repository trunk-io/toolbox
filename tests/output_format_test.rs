// trunk-ignore-all(trunk-toolbox/do-not-land)
extern crate regex;

use serde_json::Error;
use serde_sarif::sarif::Sarif;

mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn default_sarif() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write(
        "alpha.foo",
        "lorem ipsum dolor\ndo-NOT-lAnD\nDONOTLAND sit amet\n".as_bytes(),
    );
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton_with("HEAD", "sarif", false)?;

    let sarif: Result<Sarif, Error> = serde_json::from_str(&horton.stdout);
    assert!(sarif.is_ok());
    assert_eq!(sarif.unwrap().runs.len(), 1);

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

    assert_eq!(horton.exit_code, Some(0));
    assert_eq!(horton.stdout, expected_text);

    Ok(())
}
