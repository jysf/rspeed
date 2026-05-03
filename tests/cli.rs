#![allow(clippy::unwrap_used, clippy::expect_used)] // test helpers; panicking is correct here

use assert_cmd::Command;

// --- exit code tests ---

#[test]
fn unknown_flag_exits_2() {
    Command::cargo_bin("rspeed")
        .unwrap()
        .arg("--definitely-not-a-real-flag")
        .assert()
        .code(2);
}

#[test]
fn ipv4_ipv6_conflict_exits_2() {
    Command::cargo_bin("rspeed")
        .unwrap()
        .args(["-4", "-6"])
        .assert()
        .code(2);
}

#[test]
fn no_upload_no_download_conflict_exits_2() {
    Command::cargo_bin("rspeed")
        .unwrap()
        .args(["--no-upload", "--no-download"])
        .assert()
        .code(2);
}

#[test]
fn connections_zero_rejected() {
    Command::cargo_bin("rspeed")
        .unwrap()
        .args(["-c", "0"])
        .assert()
        .code(2);
}

#[test]
fn connections_too_many_rejected() {
    Command::cargo_bin("rspeed")
        .unwrap()
        .args(["-c", "33"])
        .assert()
        .code(2);
}

#[test]
fn invalid_url_rejected() {
    Command::cargo_bin("rspeed")
        .unwrap()
        .args(["-s", "not a url"])
        .assert()
        .code(2);
}

// Config::validate() rejects --server URLs without a trailing slash (AC-10).
// Note: bare host URLs like http://example.com are normalised to http://example.com/
// by the url crate. A path component without a trailing slash (e.g. /api) is needed
// to exercise the validation branch.
#[test]
fn server_without_trailing_slash_exits_2() {
    Command::cargo_bin("rspeed")
        .unwrap()
        .args(["-s", "http://example.com/api"])
        .assert()
        .code(2);
}

// --- snapshot tests (insta) ---

#[test]
fn snapshot_help() {
    let output = Command::cargo_bin("rspeed")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(stdout);
}

#[test]
fn snapshot_version() {
    let output = Command::cargo_bin("rspeed")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(stdout);
}
