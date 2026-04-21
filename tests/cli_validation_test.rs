mod integration_testing;

use integration_testing::TestRepo;

/// Regression: trunk-check invokes toolbox with `--results` pointing at a
/// path whose parent directory does not yet exist. Toolbox must fail fast
/// with an actionable error rather than a raw `No such file or directory
/// (os error 2)` emitted from the final write after all the rules have run.
#[test]
fn results_path_with_missing_parent_dir_bails_with_clear_error() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("src/file.txt", "hello".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("initial");
    test_repo.write("src/file.txt", "goodbye".as_bytes());

    let tmp = tempfile::tempdir()?;
    let missing_parent = tmp.path().join("does/not/exist/yet/results.json");
    assert!(!missing_parent.parent().unwrap().exists());

    let horton = test_repo.run_horton_customized("HEAD", "sarif", Some(&missing_parent), None)?;

    assert_eq!(
        horton.exit_code,
        Some(1),
        "toolbox should fail when --results parent dir is missing; stderr:\n{}",
        horton.stderr
    );
    // The error message should name the offending flag and path so the caller
    // (trunk-check, a human, etc.) can actually diagnose the problem.
    assert!(
        horton.stderr.contains("--results"),
        "stderr should mention --results; got:\n{}",
        horton.stderr
    );
    assert!(
        horton.stderr.contains("results.json"),
        "stderr should mention the offending path; got:\n{}",
        horton.stderr
    );
    assert!(
        !missing_parent.exists(),
        "no results file should be written when validation fails"
    );

    Ok(())
}

/// `--cache-dir` is optional. If the caller passes a directory that doesn't
/// exist we should not explode - we should warn on stderr and continue
/// running without a cache.
#[test]
fn missing_cache_dir_warns_and_continues() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("src/file.txt", "hello".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("initial");
    test_repo.write("src/file.txt", "goodbye".as_bytes());

    let tmp = tempfile::tempdir()?;
    let results_file = tmp.path().join("out.json");
    let bogus_cache = tmp.path().join("no-such-cache-dir");
    assert!(!bogus_cache.exists());

    let horton = test_repo.run_horton_customized(
        "HEAD",
        "sarif",
        Some(&results_file),
        Some(bogus_cache.to_str().unwrap()),
    )?;

    assert_eq!(
        horton.exit_code,
        Some(0),
        "toolbox should continue when --cache-dir is missing; stderr:\n{}",
        horton.stderr
    );
    assert!(
        results_file.exists(),
        "results file should still be written when cache dir is invalid"
    );
    assert!(
        horton.stderr.to_lowercase().contains("cache"),
        "stderr should mention the cache; got:\n{}",
        horton.stderr
    );
    assert!(
        horton.stderr.contains("no-such-cache-dir"),
        "stderr should name the bogus cache dir; got:\n{}",
        horton.stderr
    );

    Ok(())
}
