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
    assert_that(&horton.has_result(
        "never-edit-modified",
        "file is protected and should not be modified",
        Some("src/write_once.txt"),
    ))
    .is_true();
    assert_that(&horton.has_result("never-edit-modified", "", Some("src/write_many.txt")))
        .is_false();

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
    assert_that(&horton.has_result("never-edit-deleted", "", Some("src/locked/file.txt")))
        .is_true();

    Ok(())
}

#[test]
fn honor_disabled_in_config() -> anyhow::Result<()> {
    let test_repo: TestRepo = TestRepo::make().unwrap();

    test_repo.write("src/locked/file.txt", "immutable text".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("create locked and editable files");

    let toml_on = r#"
    [neveredit]
    enabled = true
    paths = ["src/locked/**"]
"#;

    // enable and configure never_edit
    let toml_off = r#"
    [neveredit]
    enabled = false
    paths = ["src/locked/**"]
"#;

    // write to the protected file
    test_repo.delete("src/locked/file.txt");

    test_repo.set_toolbox_toml(toml_on);
    let mut horton = test_repo.run_horton()?;
    assert_that(&horton.has_result("never-edit-deleted", "", Some("src/locked/file.txt")))
        .is_true();

    test_repo.set_toolbox_toml(toml_off);
    horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.has_result("never-edit-deleted", "", Some("src/locked/file.txt")))
        .is_false();
    assert_that(&horton.has_result("toolbox-perf", "1 files processed", None)).is_true();

    Ok(())
}

#[test]
fn warn_for_config_not_protecting_anything() -> anyhow::Result<()> {
    let test_repo: TestRepo = TestRepo::make().unwrap();

    // enable and configure never_edit
    let toml = r#"
    [neveredit]
    enabled = true
    paths = ["bad_path/**"]
"#;
    test_repo.set_toolbox_toml(toml);

    let horton: integration_testing::HortonOutput = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.has_result_with_rule_id("never-edit-bad-config")).is_true();
    Ok(())
}
