// trunk-ignore-all(trunk-toolbox/todo)
mod integration_testing;
use integration_testing::TestRepo;

fn write_enable_config(test_repo: &TestRepo) -> anyhow::Result<()> {
    let config = r#"
    [todo]
    enabled = true

    [donotland]
    enabled = false
"#;
    test_repo.write("toolbox.toml", config.as_bytes());
    Ok(())
}

#[test]
fn basic_todo() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write(
        "alpha.foo",
        "lorem ipsum dolor\ntoDO\nsit amet\n".as_bytes(),
    );
    write_enable_config(&test_repo)?;
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton()?;

    assert_eq!(horton.exit_code, Some(0));
    assert!(horton.has_result_with_rule_id("todo"));

    Ok(())
}

#[test]
fn basic_fixme() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write(
        "alpha.foo",
        "lorem ipsum dolor\nFIXME: fix this\nsit amet\n".as_bytes(),
    );
    write_enable_config(&test_repo)?;
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton()?;

    assert_eq!(horton.exit_code, Some(0));
    assert!(horton.has_result("todo", "Found 'FIXME'", Some("alpha.foo")));

    Ok(())
}

#[test]
fn basic_mastodon() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write(
        "alpha.foo",
        "lorem ipsum dolor\n// Mastodons are cool\nsit amet\n".as_bytes(),
    );
    write_enable_config(&test_repo)?;
    test_repo.git_add_all()?;
    let horton = test_repo.run_horton()?;

    assert_eq!(horton.exit_code, Some(0));
    assert!(!horton.stdout.contains("Found 'todo'"));

    Ok(())
}

#[test]
fn default_disabled_in_config() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("alpha.foo", "todo\n".as_bytes());
    test_repo.git_add_all()?;

    {
        let horton = test_repo.run_horton()?;
        assert!(!horton.stdout.contains("Found 'todo'"));
    }

    Ok(())
}
