use confique::{Config, Partial};
use horton::config::Conf;
use horton::diagnostic::{Position, Range};
use horton::run::Run;
use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;
use std::collections::HashSet;
use std::path::PathBuf;

use horton::rules::if_change_then_change::{find_ictc_blocks, IctcBlock, TextBlock};
use horton::rules::if_change_then_change::{Ictc, IfChange, RemoteLocation, ThenChange};

fn assert_no_expected_changes(before: &str, after: &str) -> anyhow::Result<()> {
    let test_repo = TestRepo::make().unwrap();

    test_repo.write("constant.foo", "lorem ipsum".as_bytes());
    test_repo.write("revision.foo", before.as_bytes());
    test_repo.git_commit_all("create constant.foo and revision.foo");

    test_repo.write("revision.foo", after.as_bytes());
    let horton = test_repo.run_horton()?;

    print!("{}", horton.stdout);

    assert_that(&horton.exit_code).contains_value(0);
    assert_that(&horton.has_result(
        "if-change-then-change-this",
        "Expected change in constant.foo because revision.foo was modified",
        Some("revision.foo"),
    ))
    .is_false();

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
    assert_that(&horton.has_result(
        "if-change-then-change-this",
        "Expected change in constant.foo because revision.foo was modified",
        Some("revision.foo"),
    ))
    .is_true();
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
        assert_that(&horton.has_result(
            "if-change-mismatched",
            "Expected matching ThenChange tag",
            None,
        ))
        .is_true();
    }

    {
        test_repo.write("revision.foo", multi_tag.as_bytes());
        let horton = test_repo.run_horton().unwrap();
        assert_that(&horton.exit_code).contains_value(0);
        assert_that(&horton.has_result(
            "if-change-mismatched",
            "Expected matching ThenChange tag",
            None,
        ))
        .is_true();
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
        assert_that(&horton.has_result(
            "if-change-mismatched",
            "Expected preceding IfChange tag",
            None,
        ))
        .is_true();
        assert_that(&horton.stdout).contains("");
    }

    {
        test_repo.write("revision.foo", multi_tag.as_bytes());
        let horton = test_repo.run_horton().unwrap();
        assert_that(&horton.exit_code).contains_value(0);
        assert_that(&horton.has_result(
            "if-change-mismatched",
            "Expected preceding IfChange tag",
            None,
        ))
        .is_true();
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
        assert_that(&horton.has_result(
            "if-change-file-does-not-exist",
            "ThenChange zee.foo does not exist",
            None,
        ))
        .is_true();
    }
}

#[test]
fn find_local_ictc_blocks() {
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
    assert_eq!(first.begin, Some(6));
    assert_eq!(first.end, Some(10));
    match &first.thenchange {
        Some(ThenChange::RepoFile(path)) => {
            assert_eq!(*path, PathBuf::from("foo.bar"));
        }
        _ => {
            panic!("wrong thenchange type");
        }
    };

    let second = &list[1];
    assert_eq!(second.begin, Some(16));
    assert_eq!(second.end, Some(18));
    match &second.thenchange {
        Some(ThenChange::RepoFile(path)) => {
            assert_eq!(*path, PathBuf::from("path/to/file/something.else"));
        }
        _ => {
            panic!("wrong thenchange type");
        }
    };
}

#[test]
fn find_repo_ictc_blocks() {
    let result = find_ictc_blocks(&PathBuf::from(
        "tests/if_change_then_change/basic_ictc_remote.file",
    ));
    assert!(result.is_ok());

    let list = result.unwrap();
    assert!(list.len() == 1, "should find 1 ictc block");

    let first = &list[0];
    assert_eq!(first.begin, Some(6));
    assert_eq!(first.end, Some(10));
    match &first.ifchange {
        Some(IfChange::RemoteFile(remote)) => {
            assert_that(&remote.repo).contains("github.com:eslint/eslint.git");
            assert_that(&remote.path).contains("LICENSE");
        }
        _ => {
            panic!("wrong ifchange type");
        }
    };
}

#[test]
fn remote_repo_insert_hash_fix() {
    type PartialConf = <Conf as Config>::Partial;
    let config = Conf::from_partial(PartialConf::default_values()).unwrap();

    let run: Run = Run {
        paths: HashSet::new(),
        config,
        cache_dir: "".to_string(),
        config_path: "fake/config/path".to_string(),
    };

    let remote = RemoteLocation {
        repo: "git@github.com:eslint/eslint.git".to_string(),
        path: "LICENSE".to_string(),
        lock_hash: "".to_string(),
        block: TextBlock {
            location: Range {
                start: Position {
                    line: 1,
                    character: 1,
                },
                end: Position {
                    line: 1,
                    character: 2,
                },
            },
            text: "git@github.com:eslint/eslint.git LICENSE".to_string(),
        },
    };

    let block = IctcBlock {
        path: PathBuf::from("tests/if_change_then_change/basic_ictc_remote.file"),
        begin: Some(6),
        end: Some(10),
        ifchange: Some(IfChange::RemoteFile(remote.clone())),
        thenchange: Some(ThenChange::RepoFile(PathBuf::from("f2oo.bar"))),
    };

    let mut ictc = Ictc::new(&run, "no-upstream");
    ictc.ifchange_remote(&remote, &block);
    assert!(ictc.diagnostics.len() == 1, "should have 1 diagnostic");

    let diag = ictc.diagnostics.get(0).unwrap();
    let replacements = diag
        .replacements
        .as_ref()
        .expect("should have replacements");
    let replacement = replacements
        .get(0)
        .expect("should have at least one replacement");

    assert_that(&replacement.inserted_content).starts_with("#");
}

#[test]
fn remote_repo_update_hash() {
    type PartialConf = <Conf as Config>::Partial;
    let config = Conf::from_partial(PartialConf::default_values()).unwrap();

    let run: Run = Run {
        paths: HashSet::new(),
        config,
        cache_dir: "".to_string(),
        config_path: "fake/config/path".to_string(),
    };

    let remote = RemoteLocation {
        repo: "git@github.com:eslint/eslint.git".to_string(),
        path: "LICENSE".to_string(),
        lock_hash: "ABCDEFG".to_string(),
        block: TextBlock {
            location: Range {
                start: Position {
                    line: 1,
                    character: 1,
                },
                end: Position {
                    line: 1,
                    character: 47,
                },
            },
            text: "git@github.com:eslint/eslint.git LICENSE#ABCDEFG".to_string(),
        },
    };

    let block = IctcBlock {
        path: PathBuf::from("tests/if_change_then_change/basic_ictc_remote.file"),
        begin: Some(6),
        end: Some(10),
        ifchange: Some(IfChange::RemoteFile(remote.clone())),
        thenchange: Some(ThenChange::RepoFile(PathBuf::from("f2oo.bar"))),
    };

    let mut ictc = Ictc::new(&run, "no-upstream");
    ictc.ifchange_remote(&remote, &block);
    assert!(ictc.diagnostics.len() == 1, "should have 1 diagnostic");

    let diag = ictc.diagnostics.get(0).unwrap();
    let replacements = diag
        .replacements
        .as_ref()
        .expect("should have replacements");
    let replacement = replacements
        .get(0)
        .expect("should have at least one replacement");

    assert_that(&replacement.deleted_region.start.character).is_equal_to(40);
    assert_that(&replacement.deleted_region.end.character).is_equal_to(47);
    assert_that(&replacement.inserted_content.len()).is_equal_to(10);

    let sarif = diag.to_sarif();
    assert!(
        sarif.fixes.as_ref().map_or(0, |fixes| fixes.len()) == 1,
        "should have 1 fix"
    );
}
