use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;
use std::path::PathBuf;

// use horton::rules::if_change_then_change::find_ictc_blocks;
// use horton::rules::if_change_then_change::ThenChange;

#[test]
fn assert_missing_thenchange() {
    let test_repo = TestRepo::make().unwrap();

    test_repo.write("write_once.txt", "immutable text".as_bytes());
    test_repo.git_commit_all("create write once file");

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
