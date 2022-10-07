use predicates::prelude::*; // Used for writing assertions

mod integration_testing;
use integration_testing::TestRepo;

#[test]
fn do_not_land() {
    let test_repo = TestRepo::make().unwrap();

    test_repo
        .write("alpha.foo", "lorem ipsum dolor\ndo-NOT-lAnD\nsit amet\n")
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
