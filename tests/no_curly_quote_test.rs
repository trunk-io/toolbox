use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;

const TOML_ON: &str = r#"
[nocurlyquotes]
enabled = true
"#;

const CURLY_QUOTES: &str = r#"
the opening double quote ( “ ) U+201C
the closing double quote ( ” ) U+201D
the opening single quote ( ‘ ) U+2018
the closing single quote ( ’) U+2019
the double low quotation ( „ ) U+201E
the double high reversed ( ‟ ) U+201F
//
"#;

#[test]
fn honor_disabled_in_config() -> anyhow::Result<()> {
    let test_repo: TestRepo = TestRepo::make().unwrap();

    test_repo.write("src/curly.txt", "empty_file".as_bytes());
    test_repo.git_add_all()?;
    test_repo.git_commit_all("create curly quote file");
    test_repo.write("src/curly.txt", CURLY_QUOTES.as_bytes());

    // disable nocurlyquotes
    let toml_off = r#"
    [nocurlyquotes]
    enabled = false
"#;

    test_repo.set_toolbox_toml(TOML_ON);
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
    let test_repo = TestRepo::make().unwrap();
    test_repo.set_toolbox_toml(TOML_ON);
    test_repo.write("revision.foo", "//".as_bytes());
    test_repo.git_commit_all("create revision.foo");

    {
        test_repo.write("revision.foo", CURLY_QUOTES.as_bytes());
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
        assert_that(&horton.has_result(
            "no-curly-quotes",
            "Found curly quote on line 4",
            Some("revision.foo"),
        ))
        .is_true();
        assert_that(&horton.has_result(
            "no-curly-quotes",
            "Found curly quote on line 5",
            Some("revision.foo"),
        ))
        .is_true();
    }
}
