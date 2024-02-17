// trunk-ignore-all(trunk-toolbox/do-not-land)
use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn basic() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write(
        "alpha.foo",
        "lorem ipsum dolor\ndo-NOT-lAnD\nsit amet\n".as_bytes(),
    );
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout).contains("Found 'do-NOT-lAnD'");

    Ok(())
}

#[test]
fn binary_files_ignored() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write("alpha.foo.binary", include_bytes!("trunk-logo.png"));
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton()?;
    let result: serde_json::Value = serde_json::from_str(&horton.stdout)?;
    let runs = result
        .as_object()
        .unwrap()
        .get("runs")
        .unwrap()
        .as_array()
        .unwrap();

    assert_eq!(runs.len(), 1);

    assert_eq!(
        runs.get(0)
            .unwrap()
            .get("results")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r.get("level").unwrap() != "note")
            .collect::<Vec<_>>()
            .is_empty(),
        true
    );
    Ok(())
}
