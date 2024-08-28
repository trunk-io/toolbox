use spectral::prelude::*;

mod integration_testing;
use integration_testing::TestRepo;
use std::path::PathBuf;

// use horton::rules::if_change_then_change::find_ictc_blocks;
// use horton::rules::if_change_then_change::ThenChange;

#[test]
fn assert_missing_thenchange() {
    let test_repo: TestRepo = TestRepo::make().unwrap();

    test_repo.write("write_once.txt", "immutable text".as_bytes());
    test_repo.git_commit_all("create write once file");
}
