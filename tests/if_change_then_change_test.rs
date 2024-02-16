use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;
use std::path::PathBuf;

use horton::rules::if_change_then_change::find_ictc_blocks;
use horton::rules::if_change_then_change::ThenChange;

fn assert_no_expected_changes(before: &str, after: &str) -> anyhow::Result<()> {
    let test_repo = TestRepo::make().unwrap();

    test_repo.write("constant.foo", "lorem ipsum".as_bytes());
    test_repo.write("revision.foo", before.as_bytes());
    test_repo.git_commit_all("create constant.foo and revision.foo");

    test_repo.write("revision.foo", after.as_bytes());
    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout.contains("Expected change")).is_false();
    assert_that(&horton.stdout.contains("constant.foo")).is_false();
    assert_that(&horton.stdout.contains("revision.foo")).is_false();

    Ok(())
}

fn assert_expected_change_in_constant_foo(before: &str, after: &str) -> anyhow::Result<()> {
    let test_repo = TestRepo::make().unwrap();

    test_repo.write("constant.foo", "lorem ipsum".as_bytes());
    test_repo.write("revision.foo", before.as_bytes());
    test_repo.git_commit_all("create constant.foo and revision.foo");

    test_repo.write("revision.foo", after.as_bytes());
    let horton = test_repo.run_horton()?;

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.stdout)
        .contains("Expected change in constant.foo because revision.foo was modified");

    Ok(())
}

#[test]
fn unmodified_block_and_preceding_lines_unchanged() -> anyhow::Result<()> {
    let before = r#"
        a
        b
        c
        // IfChange
        d
        // ThenChange constant.foo
        e
        f
    "#;

    let after = r#"
        a
        b
        c
        // IfChange
        d
        // ThenChange constant.foo
        e
        f
    "#;

    assert_no_expected_changes(&before, &after)
}

#[test]
fn unmodified_block_and_preceding_lines_changed() -> anyhow::Result<()> {
    let before = r#"
        a
        b
        c
        // IfChange
        d
        // ThenChange constant.foo
        e
        f
    "#;

    let after = r#"
        a
        b
        c
        x
        y
        z
        // IfChange
        d
        // ThenChange constant.foo
        e
        f
    "#;

    assert_no_expected_changes(&before, &after)
}

#[test]
fn unmodified_block_and_preceding_lines_deleted() -> anyhow::Result<()> {
    let before = r#"
        a
        b
        c
        // IfChange
        d
        // ThenChange constant.foo
        e
        f
    "#;

    let after = r#"
        a
        // IfChange
        d
        // ThenChange constant.foo
        e
        f
    "#;

    assert_no_expected_changes(&before, &after)
}

#[test]
fn unmodified_block_and_otehr_lines_modified() -> anyhow::Result<()> {
    let before = r#"
        a
        b
        c
        // IfChange
        d
        // ThenChange constant.foo
        e
        f
    "#;

    let after = r#"
        aaaa
        bbb
        ccc
        // IfChange
        d
        // ThenChange constant.foo
        eeeeeee
        ffff
    "#;

    assert_no_expected_changes(&before, &after)
}

#[test]
fn modified_block_and_preceding_lines_unchanged() -> anyhow::Result<()> {
    let before = r#"
        a
        // IfChange
        b
        // ThenChange constant.foo
        c
    "#;

    let after = r#"
        a
        // IfChange
        bbbbbbbb
        // ThenChange constant.foo
        c
    "#;

    assert_expected_change_in_constant_foo(&before, &after)
}

#[test]
fn modified_block_and_preceding_line_count_unchanged() -> anyhow::Result<()> {
    let before = r#"
        aaaaaaaaaa        
        // IfChange
        b
        // ThenChange constant.foo
        c
    "#;

    let after = r#"
        a
        // IfChange
        bbbbbbbb
        // ThenChange constant.foo
        c
    "#;

    assert_expected_change_in_constant_foo(&before, &after)
}

#[test]
fn modified_block_and_preceding_line_count_decreased() -> anyhow::Result<()> {
    let before = r#"
        a
        aaa
        aaaaa        
        // IfChange
        b
        // ThenChange constant.foo
        c
    "#;

    let after = r#"
        a
        // IfChange
        bbbbbbbb
        // ThenChange constant.foo
        c
    "#;

    assert_expected_change_in_constant_foo(&before, &after)
}

#[test]
fn modified_block_and_preceding_line_count_increased() -> anyhow::Result<()> {
    let before = r#"
        a        
        // IfChange
        b
        // ThenChange constant.foo
        c
    "#;

    let after = r#"
        a
        aa
        aaa
        // IfChange
        bbbbbbbb
        // ThenChange constant.foo
        c
    "#;

    assert_expected_change_in_constant_foo(&before, &after)
}

#[test]
fn assert_missing_thenchange() {
    let single_tag = r#"
        aaaa
        bbb
        cc
        // IfChange
        d
    "#;

    let multi_tag = r#"
        aaaa
        bbb
        cc
        // IfChange
        c
        // IfChange
        d
        // ThenChange constants.foo
    "#;

    let test_repo = TestRepo::make().unwrap();

    test_repo.write("constant.foo", "foo-bar".as_bytes());
    test_repo.write("revision.foo", "".as_bytes());
    test_repo.git_commit_all("create constant.foo and revision.foo");

    {
        test_repo.write("revision.foo", single_tag.as_bytes());
        let horton = test_repo.run_horton().unwrap();
        assert_that(&horton.exit_code).contains_value(0);
        assert_that(&horton.stdout).contains("Expected matching ThenChange tag");
    }

    {
        test_repo.write("revision.foo", multi_tag.as_bytes());
        let horton = test_repo.run_horton().unwrap();
        assert_that(&horton.exit_code).contains_value(0);
        assert_that(&horton.stdout).contains("Expected matching ThenChange tag");
    }
}

#[test]
fn assert_missing_ifchange() {
    let single_tag = r#"
        aaaa
        bbb
        cc
        // ThenChange
        d
    "#;

    let multi_tag = r#"
        aaaa
        bbb
        cc
        // IfChange
        c
        // ThenChange constants.foo
        d
        // ThenChange constants.foo
    "#;

    let test_repo = TestRepo::make().unwrap();

    test_repo.write("constant.foo", "foo-bar".as_bytes());
    test_repo.write("revision.foo", "".as_bytes());
    test_repo.git_commit_all("create constant.foo and revision.foo");

    {
        test_repo.write("revision.foo", single_tag.as_bytes());
        let horton = test_repo.run_horton().unwrap();
        assert_that(&horton.exit_code).contains_value(0);
        assert_that(&horton.stdout).contains("Expected preceding IfChange tag");
    }

    {
        test_repo.write("revision.foo", multi_tag.as_bytes());
        let horton = test_repo.run_horton().unwrap();
        assert_that(&horton.exit_code).contains_value(0);
        assert_that(&horton.stdout).contains("Expected preceding IfChange tag");
    }
}

#[test]
fn assert_localfile_notfound() {
    let missing_file = r#"
        aaaa
        bbb
        // IfChange
        cc
        // ThenChange zee.foo
        d
    "#;

    let test_repo = TestRepo::make().unwrap();

    test_repo.write("revision.foo", "".as_bytes());
    test_repo.git_commit_all("create constant.foo and revision.foo");

    {
        test_repo.write("revision.foo", missing_file.as_bytes());
        let horton = test_repo.run_horton().unwrap();
        assert_that(&horton.exit_code).contains_value(0);
        assert_that(&horton.stdout).contains("ThenChange zee.foo does not exist");
    }
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
    match &first.thenchange {
        ThenChange::RepoFile(path) => {
            assert_eq!(*path, PathBuf::from("foo.bar"));
        }
        _ => {
            panic!("wrong thenchange type");
        }
    };

    let second = &list[1];
    assert_eq!(second.begin, 16);
    assert_eq!(second.end, 18);
    match &second.thenchange {
        ThenChange::RepoFile(path) => {
            assert_eq!(*path, PathBuf::from("path/to/file/something.else"));
        }
        _ => {
            panic!("wrong thenchange type");
        }
    };
}
