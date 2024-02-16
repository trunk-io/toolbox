use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;
use std::path::PathBuf;

use horton::rules::if_change_then_change::find_ictc_blocks;

fn assert_no_expected_changes(revisions: [&str; 2]) -> anyhow::Result<()> {
    let test_repo = TestRepo::make().unwrap();

    test_repo.write(
        "constant.foo",
        "// IfChange\nlorem ipsum\nThenChange revision.foo".as_bytes(),
    )?;
    test_repo.write("revision.foo", revisions[0].as_bytes())?;
    test_repo.git_commit_all("create constant.foo and revision.foo")?;

    test_repo.write("revision.foo", revisions[1].as_bytes())?;
    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout.contains("Expected change")).is_false();
    assert_that(&horton.stdout.contains("constant.foo")).is_false();
    assert_that(&horton.stdout.contains("revision.foo")).is_false();

    Ok(())
}

fn assert_expected_change_in_constant_foo(revisions: [&str; 2]) -> anyhow::Result<()> {
    let test_repo = TestRepo::make().unwrap();

    test_repo.write(
        "constant.foo",
        "// IfChange\nlorem ipsum\nThenChange revision.foo".as_bytes(),
    )?;
    test_repo.write("revision.foo", revisions[0].as_bytes())?;
    test_repo.git_commit_all("create constant.foo and revision.foo")?;

    test_repo.write("revision.foo", revisions[1].as_bytes())?;
    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout)
        .contains("Expected change in constant.foo because revision.foo was modified");

    Ok(())
}

#[test]
fn unmodified_block_and_preceding_lines_unchanged() -> anyhow::Result<()> {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_no_expected_changes(revisions)
}

#[test]
fn modified_block_and_preceding_lines_unchanged() -> anyhow::Result<()> {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
initial
preceding
lines
// IfChange
delta = "new value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_expected_change_in_constant_foo(revisions)
}

#[test]
fn unmodified_block_and_preceding_line_count_unchanged() -> anyhow::Result<()> {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
preceding lines
have been changed
but same number of them
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_no_expected_changes(revisions)
}

#[test]
fn modified_block_and_preceding_line_count_unchanged() -> anyhow::Result<()> {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
preceding lines
have been changed
but same number of them
// IfChange
delta = "new value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_expected_change_in_constant_foo(revisions)
}

#[test]
fn unmodified_block_and_preceding_line_count_decreased() -> anyhow::Result<()> {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
fewer preceding lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_no_expected_changes(revisions)
}

#[test]
fn modified_block_and_preceding_line_count_decreased() -> anyhow::Result<()> {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
fewer preceding lines
// IfChange
delta = "new value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_expected_change_in_constant_foo(revisions)
}

#[test]
fn unmodified_block_and_preceding_line_count_increased() -> anyhow::Result<()> {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
now
there
are
more
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_no_expected_changes(revisions)
}

#[test]
fn modified_block_and_preceding_line_count_increased() -> anyhow::Result<()> {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
now
there
are
more
preceding
lines
// IfChange
delta = "new value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_expected_change_in_constant_foo(revisions)
}

#[test]
fn honor_disabled_in_config() {
    let revisions = [
        r#"
initial
preceding
lines
// IfChange
delta = "initial value"
// ThenChange constant.foo
and then
trailing lines
"#,
        r#"
now
there
are
more
preceding
lines
// IfChange
delta = "new value"
// ThenChange constant.foo
and then
trailing lines
"#,
    ];

    assert_expected_change_in_constant_foo(revisions)
}

#[test]
fn verify_find_ictc_blocks() {
    let result = find_ictc_blocks(&PathBuf::from(
        "tests/if_change_then_change/basic_ictc.file",
    ));
    assert!(result.is_ok());
    assert!(result.unwrap().len() == 1, "should find 1 ictc block");

    let result = find_ictc_blocks(&PathBuf::from("tests/if_change_then_change/no_ictc.file"));
    assert!(result.is_ok());
    assert!(result.unwrap().len() == 0, "should find no ictc block");

    let result = find_ictc_blocks(&PathBuf::from(
        "tests/if_change_then_change/multiple_ictc.file",
    ));
    assert!(result.is_ok());
    let list = result.unwrap();
    assert!(list.len() == 2, "should find two ictc block");
    // assert!(list[0].begin == 1, "first block should point to 2");
    let first = &list[0];
    assert_eq!(first.begin, 6);
    assert_eq!(first.end, 10);
    assert_eq!(first.thenchange, PathBuf::from("foo.bar"));

    let second = &list[1];
    assert_eq!(second.begin, 16);
    assert_eq!(second.end, 18);
    assert_eq!(
        second.thenchange,
        PathBuf::from("path/to/file/something.else")
    );
}
