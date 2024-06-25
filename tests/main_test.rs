use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn binary_file_untracked() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write("picture.binary", include_bytes!("trunk-logo.png"));

    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout.contains("Expected change")).is_false();
    assert_that(&horton.stdout.contains("picture.binary")).is_false();

    Ok(())
}

#[test]
fn binary_file_committed() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write("picture.binary", include_bytes!("trunk-logo.png"));
    test_repo.git_commit_all("commit a picture");

    let horton = test_repo.run_horton_with("HEAD^", "sarif")?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout.contains("Expected change")).is_false();
    assert_that(&horton.stdout.contains("picture.binary")).is_false();

    Ok(())
}

#[test]
fn lfs_file_untracked() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write(
        ".gitattributes",
        "*.binary filter=lfs diff=lfs merge=lfs -text\n".as_bytes(),
    );
    test_repo.git_commit_all("create .gitattributes");

    test_repo.write("picture.binary", include_bytes!("trunk-logo.png"));

    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout.contains("Expected change")).is_false();
    assert_that(&horton.stdout.contains("picture.binary")).is_false();

    Ok(())
}

#[test]
fn lfs_file_committed() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write(
        ".gitattributes",
        "*.binary filter=lfs diff=lfs merge=lfs -text\n".as_bytes(),
    );
    test_repo.git_commit_all("create .gitattributes");

    test_repo.write("picture.binary", include_bytes!("trunk-logo.png"));

    let horton = test_repo.run_horton_with("HEAD^", "sarif")?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout.contains("Expected change")).is_false();
    assert_that(&horton.stdout.contains("picture.binary")).is_false();

    Ok(())
}
