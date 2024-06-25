// trunk-ignore-all(trunk-toolbox/todo)
use spectral::prelude::*;

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

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout).contains("Found 'toDO'");

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

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout).contains("Found 'FIXME'");

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

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout.contains("Found 'todo'")).is_false();

    Ok(())
}

#[test]
fn default_disabled_in_config() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("alpha.foo", "todo\n".as_bytes());
    test_repo.git_add_all()?;

    {
        let horton = test_repo.run_horton()?;
        assert_that(&horton.stdout.contains("Found 'todo'")).is_false();
    }

    Ok(())
}
