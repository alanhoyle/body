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
fn body_default_command_env_var_overrides_the_default() {
    let assert = body()
        .env("BODY_DEFAULT_COMMAND", "rev")
        .write_stdin("header\nabc\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert_eq!(out, "header\ncba\n");
    assert!(stderr.contains("running rev by default"));
}

#[test]
fn body_default_command_env_var_with_arguments_is_word_split() {
    // Regression test: BODY_DEFAULT_COMMAND="sort -r" must run `sort -r`,
    // not fail looking for a single binary literally named "sort -r".
    let assert = body()
        .env("BODY_DEFAULT_COMMAND", "sort -r")
        .write_stdin("header\na\nc\nb\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "header\nc\nb\na\n");
}

#[test]
fn body_default_command_env_var_is_ignored_when_a_command_is_given() {
    let assert = body()
        .env("BODY_DEFAULT_COMMAND", "rev")
        .args(["1", "sort"])
        .write_stdin("header\nc\na\nb\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "header\na\nb\nc\n");
}

#[test]
fn unknown_command_exits_127_like_a_shell_would() {
    body()
        .args(["1", "totally_not_a_real_command"])
        .write_stdin("a\n")
        .assert()
        .code(127);
}

#[test]
fn leading_zero_header_count_is_interpreted_as_decimal_not_octal() {
    // "010" must mean ten header lines (decimal), not eight (octal) -- a
    // divergence that used to exist between bash's `(( ))` arithmetic
    // (octal) and Rust's decimal `parse`.
    let assert = body()
        .args(["010", "sort"])
        .write_stdin("h01\nh02\nh03\nh04\nh05\nh06\nh07\nh08\nh09\nh10\ncharlie\nalpha\nbravo\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(
        out,
        "h01\nh02\nh03\nh04\nh05\nh06\nh07\nh08\nh09\nh10\nalpha\nbravo\ncharlie\n"
    );
}

#[test]
fn header_count_at_the_i64_boundary_is_accepted() {
    // 9223372036854775807 (i64::MAX) is the largest value both
    // implementations accept; anything larger is a hard error (see
    // `header_count_that_overflows_is_rejected_with_a_clear_error`).
    let assert = body()
        .args(["9223372036854775807", "sort"])
        .write_stdin("only-one\n")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "only-one\n");
}

#[test]
fn header_count_that_overflows_is_rejected_with_a_clear_error() {
    let assert = body()
        .args(["99999999999999999999999999", "sort"])
        .write_stdin("a\nb\n")
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("too large"));
}

#[test]
fn unterminated_final_header_line_is_still_printed() {
    // STDIN ends mid-line (no trailing newline) exactly where the header
    // ends. bash's `read || break` used to silently drop this partial line
    // instead of printing it; both sides must print it.
    let assert = body()
        .args(["1", "sort"])
        .write_stdin("header")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out, "header\n");
}

#[test]
fn body_default_command_that_is_only_whitespace_is_rejected_with_a_clear_error() {
    // A whitespace-only BODY_DEFAULT_COMMAND word-splits to nothing usable.
    // It must be rejected explicitly, not silently substituted with `sort`
    // (the old behavior here) nor silently run as a no-op that discards the
    // piped body (bash's old behavior).
    let assert = body()
        .env("BODY_DEFAULT_COMMAND", "   ")
        .write_stdin("header\na\n")
        .assert()
        .failure()
        .code(1);
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("has no command after word-splitting"));
}

#[test]
fn a_genuine_stdin_read_error_is_reported_not_silently_treated_as_eof() {
    // Redirecting STDIN from a directory makes the underlying read(2) fail
    // with EISDIR instead of returning EOF, exercising the real I/O-error
    // path (distinct from ordinary EOF, which is not an error).
    use std::process::{Command, Stdio};
    let dir = std::fs::File::open(".").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_body"))
        .args(["1", "sort"])
        .stdin(Stdio::from(dir))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("error reading from stdin"));
}
