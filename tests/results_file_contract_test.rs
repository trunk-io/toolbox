mod integration_testing;

use integration_testing::TestRepo;
use serde_sarif::sarif::Sarif;

/// Contract check (aligned with the other `read_output_from: tmp_file` linters
/// in trunk-io/plugins like eslint, semgrep, gitleaks): when trunk-check invokes
/// toolbox with `--results=${tmpfile}`, the tmpfile does not yet exist and
/// toolbox is responsible for creating it. A successful run must leave a
/// parseable SARIF document at that path.
#[test]
fn results_file_is_created_on_clean_run() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("src/file.txt", "hello\n".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("initial");
    test_repo.write("src/file.txt", "goodbye\n".as_bytes());

    let tmp = tempfile::tempdir()?;
    let results_path = tmp.path().join("out.sarif");
    assert!(
        !results_path.exists(),
        "precondition: tmpfile must not pre-exist"
    );

    let horton = test_repo.run_horton_customized("HEAD", "sarif", Some(&results_path), None)?;

    assert_eq!(
        horton.exit_code,
        Some(0),
        "clean run should exit 0; stderr:\n{}",
        horton.stderr
    );
    assert!(
        results_path.exists(),
        "toolbox must create the --results tmpfile on a clean run; stderr:\n{}",
        horton.stderr
    );
    let _: Sarif = serde_json::from_str(&horton.results).unwrap_or_else(|e| {
        panic!(
            "results file is not valid SARIF: {}\nbody:\n{}",
            e, horton.results
        )
    });

    Ok(())
}

/// Regression: if the toolbox config file fails to load, the previous
/// implementation called `process::exit(1)` directly without ever writing
/// the `--results` file. trunk-check then reports the linter as failed with
/// "failed to read output file". The fix is to always produce a valid
/// (possibly minimal) SARIF document at the `--results` path, even on
/// catastrophic failure, and carry the non-zero exit separately.
#[test]
fn malformed_config_still_writes_results_file() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("src/file.txt", "hello\n".as_bytes());
    // Deliberately broken TOML - unterminated string.
    test_repo.set_toolbox_toml("this is = \"not valid toml\n[invalid");
    test_repo.git_add_all()?;
    test_repo.git_commit_all("initial");
    test_repo.write("src/file.txt", "goodbye\n".as_bytes());

    let tmp = tempfile::tempdir()?;
    let results_path = tmp.path().join("out.sarif");

    let horton = test_repo.run_horton_customized("HEAD", "sarif", Some(&results_path), None)?;

    assert_ne!(
        horton.exit_code,
        Some(0),
        "toolbox should exit non-zero when config is malformed; stderr:\n{}",
        horton.stderr
    );
    assert!(
        results_path.exists(),
        "results file must be created even when toolbox fails; stderr:\n{}",
        horton.stderr
    );
    let sarif: Sarif = serde_json::from_str(&horton.results).unwrap_or_else(|e| {
        panic!(
            "results file must contain valid SARIF even on failure: {}\nbody:\n{}",
            e, horton.results
        )
    });
    // The error should be represented as at least one result in the SARIF so
    // downstream tooling can surface it to the user (instead of the
    // linter-level "tmpfile missing" error that the old code produced).
    let has_error_result = sarif.runs.iter().any(|run| {
        run.results
            .as_ref()
            .map(|rs| {
                rs.iter().any(|r| {
                    r.rule_id
                        .as_deref()
                        .map_or(false, |id| id.starts_with("toolbox-"))
                })
            })
            .unwrap_or(false)
    });
    assert!(
        has_error_result,
        "fallback SARIF should describe the failure; body:\n{}",
        horton.results
    );

    Ok(())
}

/// Text-format counterpart to the SARIF contract: `--output-format text`
/// with `--results` must also produce the file (empty or populated), not
/// leave the path untouched.
#[test]
fn text_format_results_file_is_created() -> anyhow::Result<()> {
    let test_repo = TestRepo::make()?;
    test_repo.write("src/file.txt", "hello\n".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("initial");
    test_repo.write("src/file.txt", "goodbye\n".as_bytes());

    let tmp = tempfile::tempdir()?;
    let results_path = tmp.path().join("out.txt");

    let horton = test_repo.run_horton_customized("HEAD", "text", Some(&results_path), None)?;

    assert_eq!(
        horton.exit_code,
        Some(0),
        "clean run should exit 0; stderr:\n{}",
        horton.stderr
    );
    assert!(
        results_path.exists(),
        "toolbox must create the --results file in text mode too; stderr:\n{}",
        horton.stderr
    );

    Ok(())
}
