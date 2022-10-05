use predicates::prelude::*; // Used for writing assertions

mod integration_testing;
use integration_testing::run_horton;

#[test]
fn do_not_land() {
    let mut tmpfile = tempfile::tempfile().unwrap();

    let horton = run_horton("src/main.rs").unwrap();

    assert_eq!(
        true,
        predicates::str::contains("Found 'do-not-land'").eval(&horton)
    );
}
