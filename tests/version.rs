#![allow(clippy::unwrap_used, clippy::expect_used)] // test helpers; panicking is correct here

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_version_on_version_flag() {
    Command::cargo_bin("rspeed")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("rspeed"))
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
