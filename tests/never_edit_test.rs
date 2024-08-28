use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn assert_modified_locked_file() -> anyhow::Result<()> {
    let test_repo: TestRepo = TestRepo::make().unwrap();

    test_repo.write("src/write_once.txt", "immutable text".as_bytes());
    test_repo.write("src/write_many.txt", "immutable text".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("create write once and write many file");

    // enable and configure never_edit
    let toml = r#"
    [neveredit]
    enabled = true
    paths = ["src/foo", "src/bar/**", "**/write_once*"]
"#;

    // write to the protected file
    test_repo.write("src/write_once.txt", "edit the text".as_bytes());
    test_repo.write("src/write_many.txt", "edit the text".as_bytes());

    test_repo.set_toolbox_toml(toml);

    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout).contains("file is protected and should not be modified");
    assert_that(&horton.stdout.contains("src/write_many.txt")).is_false();

    Ok(())
}

#[test]
fn assert_deleted_locked_file() -> anyhow::Result<()> {
    let test_repo: TestRepo = TestRepo::make().unwrap();

    test_repo.write("src/locked/file.txt", "immutable text".as_bytes());
    test_repo.write("src/editable.txt", "mutable text".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("create locked and editable files");

    // enable and configure never_edit
    let toml = r#"
    [neveredit]
    enabled = true
    paths = ["src/locked/**"]
"#;

    // write to the protected file
    test_repo.delete("src/locked/file.txt");

    test_repo.set_toolbox_toml(toml);

    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout).contains("file is protected and should not be deleted");
    assert_that(&horton.stdout).contains("src/locked/file.txt");

    Ok(())
}

#[test]
fn honor_disabled_in_config() -> anyhow::Result<()> {
    let test_repo: TestRepo = TestRepo::make().unwrap();

    test_repo.write("src/locked/file.txt", "immutable text".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("create locked and editable files");

    // enable and configure never_edit
    let toml = r#"
    [neveredit]
    enabled = false
    paths = ["src/locked/**"]
"#;

    // write to the protected file
    test_repo.delete("src/locked/file.txt");

    test_repo.set_toolbox_toml(toml);

    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout.contains("src/locked/file.txt")).is_false();
    assert_that(&horton.stdout).contains("1 files processed");

    Ok(())
}
