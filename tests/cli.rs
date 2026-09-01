//! Integration tests for the `body` binary, ported 1:1 from bash/test/body.bats
//! so the Rust port is held to the same behavioral contract as the original.

use assert_cmd::Command;

fn body() -> Command {
    Command::cargo_bin("body").unwrap()
}

#[test]
fn default_keeps_one_line_header_and_sorts_the_rest() {
    let assert = body().write_stdin("header\nc\na\nb\n").assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "header\na\nb\nc\n");
}

#[test]
fn n_header_lines_are_preserved_with_a_custom_command() {
    let assert = body()
        .args(["2", "sort", "-r"])
        .write_stdin("h1\nh2\nc\na\nb\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "h1\nh2\nc\nb\na\n");
}

#[test]
fn zero_header_lines_passes_everything_straight_through() {
    let assert = body()
        .args(["0", "sort"])
        .write_stdin("c\na\nb\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "a\nb\nc\n");
}

#[test]
fn zero_header_lines_does_not_steal_lines_from_the_body() {
    // wc -l only sees lines actually forwarded to the command's stdin, so it
    // would expose the original bash bug (N=0 behaving like N=2) in a way
    // that a pass-through command like `cat` cannot (identical output either way).
    let assert = body()
        .args(["0", "wc", "-l"])
        .write_stdin("one\ntwo\nthree\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out.trim(), "3");
}

#[test]
fn fewer_lines_than_requested_header_count_does_not_hang_or_error() {
    let assert = body()
        .args(["5", "sort"])
        .write_stdin("only-one\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "only-one\n");
}

#[test]
fn command_with_its_own_arguments_works_alongside_a_preserved_header() {
    let assert = body()
        .args(["1", "grep", "-v", "banana"])
        .write_stdin("header\nbanana\napple\ncherry\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "header\napple\ncherry\n");
}

#[test]
#[ignore = "requires a real controlling terminal to exercise the no-piped-input branch"]
fn no_piped_input_prints_an_error_and_usage_and_exits_non_zero() {}

#[test]
fn help_prints_usage_and_exits_zero_when_input_is_piped() {
    let assert = body().arg("-h").write_stdin("a\nb\n").assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("USAGE:"));
}

#[test]
fn missing_command_falls_back_to_sort_and_announces_it_on_stderr() {
    let assert = body().write_stdin("header\nb\na\n").assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("running sort by default"));
}

#[test]
fn unknown_command_exits_127_like_a_shell_would() {
    body()
        .args(["1", "totally_not_a_real_command"])
        .write_stdin("a\n")
        .assert()
        .code(127);
}
