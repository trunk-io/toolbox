mod integration_testing;

use integration_testing::TestRepo;

/// trunk-check invokes toolbox with `--results` pointing at a tmpfile whose
/// parent directory may not have been pre-created. Matching the behavior of
/// every other `read_output_from: tmp_file` linter (eslint, semgrep,
/// gitleaks, ...), toolbox creates any missing ancestor directories and
/// writes the results file rather than bailing - otherwise the linter looks
/// "failed" to the caller before it ever gets a chance to run.
#[test]
fn results_path_with_missing_parent_dir_is_created() -> anyhow::Result<()> {
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
        Some(0),
        "toolbox should create missing parent dirs and write results; stderr:\n{}",
        horton.stderr
    );
    assert!(
        missing_parent.exists(),
        "results file should have been created alongside the new parent dir"
    );

    Ok(())
}

/// Pointing `--results` at an existing directory remains a genuine usage
/// bug - there's no sane recovery - so the pre-flight check still catches
/// it with a clear error.
#[test]
fn results_path_pointing_at_directory_bails_with_clear_error() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("src/file.txt", "hello".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("initial");
    test_repo.write("src/file.txt", "goodbye".as_bytes());

    let tmp = tempfile::tempdir()?;
    let dir_as_results = tmp.path().to_path_buf();
    assert!(dir_as_results.is_dir());

    let horton = test_repo.run_horton_customized("HEAD", "sarif", Some(&dir_as_results), None)?;

    assert_eq!(
        horton.exit_code,
        Some(1),
        "toolbox should fail when --results points at a directory; stderr:\n{}",
        horton.stderr
    );
    assert!(
        horton.stderr.contains("--results"),
        "stderr should mention --results; got:\n{}",
        horton.stderr
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
