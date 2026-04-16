// trunk-ignore-all(trunk-toolbox/do-not-land)
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

    assert_eq!(horton.exit_code, Some(0));
    assert!(horton.has_result("do-not-land", "Found 'do-NOT-lAnD'", Some("alpha.foo")));

    Ok(())
}

#[test]
fn binary_files_ignored() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write("alpha.foo.binary", include_bytes!("trunk-logo.png"));
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton()?;

    assert_eq!(horton.runs().len(), 1);
    assert!(!horton.has_result_with_rule_id("do-not-land"));

    Ok(())
}

#[test]
fn honor_disabled_in_config() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("alpha.foo", "do-not-land\n".as_bytes());
    test_repo.git_add_all()?;

    {
        let horton = test_repo.run_horton()?;
        assert!(horton.has_result_with_rule_id("do-not-land"));
    }

    let config = r#"
        [donotland]
        enabled = false
    "#;

    // Now disable the rule
    test_repo.write("toolbox.toml", config.as_bytes());
    {
        let horton = test_repo.run_horton().unwrap();
        assert!(!horton.has_result_with_rule_id("do-not-land"));
    }

    Ok(())
}
