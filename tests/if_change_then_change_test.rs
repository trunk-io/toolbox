use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;

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
