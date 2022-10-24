use predicates::prelude::*; // Used for writing assertions

mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn do_not_land() {
    let test_repo = TestRepo::make().unwrap();

    test_repo
        .write(
            "alpha.foo",
            "lorem ipsum dolor\ndo-NOT-lAnD\nsit amet\n".as_bytes(),
        )
        .unwrap();
    test_repo.git_add_all().unwrap();
    let horton = test_repo.run_horton().unwrap();

    // TODO(sam): the stdlib assertion framework is useless. look into fluent assertions for rust.
    // maybe spectral or google/assertor?
    assert_eq!(
        true,
        predicates::str::contains("Found 'do-NOT-lAnD'").eval(&horton)
    );
}

#[test]
fn do_not_land_ignores_binary_files() {
    let test_repo = TestRepo::make().unwrap();

    test_repo
        .write("alpha.foo.binary", include_bytes!("trunk-logo.png"))
        .unwrap();
    test_repo.git_add_all().unwrap();
    let horton = test_repo.run_horton().unwrap();
    let result: serde_json::Value = serde_json::from_str(&horton).unwrap();
    let runs = result
        .as_object()
        .unwrap()
        .get("runs")
        .unwrap()
        .as_array()
        .unwrap();

    assert_eq!(runs.len(), 1);
    assert_eq!(
        runs.get(0)
            .unwrap()
            .get("results")
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty(),
        true
    );
}
