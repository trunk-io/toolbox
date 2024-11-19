use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn honor_disabled_in_config() -> anyhow::Result<()> {
    let has_curly_quotes = r#"
        //
        Line 2 - bad double quotes “here”
        Line 3 - bad single quotes ‘here’
        //
    "#;

    let test_repo: TestRepo = TestRepo::make().unwrap();

    test_repo.write("src/curly.txt", "empty_file".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("create curly quote file");
    test_repo.write("src/curly.txt", has_curly_quotes.as_bytes());

    let toml_on = r#"
    [nocurlyquotes]
    enabled = true
"#;

    // disable nocurlyquotes
    let toml_off = r#"
    [nocurlyquotes]
    enabled = false
"#;

    test_repo.set_toolbox_toml(toml_on);
    let mut horton = test_repo.run_horton()?;
    assert_that(&horton.has_result("no-curly-quotes", "", Some("src/curly.txt"))).is_true();

    test_repo.set_toolbox_toml(toml_off);
    horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.has_result("no-curly-quotes", "", Some("src/curly.txt"))).is_false();
    assert_that(&horton.has_result("toolbox-perf", "1 files processed", None)).is_true();

    Ok(())
}

#[test]
fn assert_find_curly_quotes() {
    let has_curly_quotes = r#"     1
        2 - bad double quotes “here”
        3 - bad single quotes ‘here’
        4 nothing bad here
    "#;

    let test_repo = TestRepo::make().unwrap();
    test_repo.write("revision.foo", "//".as_bytes());
    test_repo.git_commit_all("create revision.foo");

    {
        test_repo.write("revision.foo", has_curly_quotes.as_bytes());
        let horton = test_repo.run_horton().unwrap();
        assert_that(&horton.exit_code).contains_value(0);
        assert_that(&horton.has_result(
            "no-curly-quotes",
            "Found curly quote on line 2",
            Some("revision.foo"),
        ))
        .is_true();
        assert_that(&horton.has_result(
            "no-curly-quotes",
            "Found curly quote on line 3",
            Some("revision.foo"),
        ))
        .is_true();
    }
}
