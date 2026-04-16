mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn binary_file_untracked() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write("picture.binary", include_bytes!("trunk-logo.png"));

    let horton = test_repo.run_horton()?;

    assert_eq!(horton.exit_code, Some(0));
    assert!(!horton.stdout.contains("Expected change"));
    Ok(())
}

#[test]
fn binary_file_committed() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;

    test_repo.write("picture.binary", include_bytes!("trunk-logo.png"));
    test_repo.git_commit_all("commit a picture");

    let horton = test_repo.run_horton_with("HEAD^", "sarif", false)?;

    print!("{}", horton.stdout);

    assert_eq!(horton.exit_code, Some(0));
    assert!(!horton.stdout.contains("Expected change"));

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

    assert_eq!(horton.exit_code, Some(0));
    assert!(!horton.stdout.contains("Expected change"));

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

    let horton = test_repo.run_horton_with("HEAD^", "sarif", false)?;

    assert_eq!(horton.exit_code, Some(0));
    assert!(!horton.stdout.contains("Expected change"));

    Ok(())
}
