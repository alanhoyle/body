//! Compares the Rust port against the original bash implementation
//! (bash/body.sh) for a range of inputs, so the two stay behaviorally
//! identical as either one changes.

use assert_cmd::Command as AssertCommand;
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Case {
    name: &'static str,
    args: &'static [&'static str],
    stdin: &'static str,
    default_command_env: Option<&'static str>,
}

const CASES: &[Case] = &[
    Case {
        name: "default 1-line header, sort",
        args: &[],
        stdin: "header\nc\na\nb\n",
        default_command_env: None,
    },
    Case {
        name: "2-line header, sort -r",
        args: &["2", "sort", "-r"],
        stdin: "h1\nh2\nc\na\nb\n",
        default_command_env: None,
    },
    Case {
        name: "0 header lines, sort",
        args: &["0", "sort"],
        stdin: "c\na\nb\n",
        default_command_env: None,
    },
    Case {
        name: "0 header lines, wc -l",
        args: &["0", "wc", "-l"],
        stdin: "one\ntwo\nthree\n",
        default_command_env: None,
    },
    Case {
        name: "header count exceeds input length",
        args: &["5", "sort"],
        stdin: "only-one\n",
        default_command_env: None,
    },
    Case {
        name: "grep -v with a preserved header",
        args: &["1", "grep", "-v", "banana"],
        stdin: "header\nbanana\napple\ncherry\n",
        default_command_env: None,
    },
    Case {
        name: "no trailing newline on last line",
        args: &["1", "sort"],
        stdin: "header\nc\na\nb",
        default_command_env: None,
    },
    Case {
        name: "BODY_DEFAULT_COMMAND overrides the default command",
        args: &[],
        stdin: "header\nabc\n",
        default_command_env: Some("rev"),
    },
    Case {
        name: "BODY_DEFAULT_COMMAND with arguments is word-split",
        args: &[],
        stdin: "header\na\nc\nb\n",
        default_command_env: Some("sort -r"),
    },
    Case {
        name: "leading zero header count is decimal, not octal",
        args: &["010", "sort"],
        stdin: "h01\nh02\nh03\nh04\nh05\nh06\nh07\nh08\nh09\nh10\ncharlie\nalpha\nbravo\n",
        default_command_env: None,
    },
    Case {
        name: "header count at the i64 boundary is accepted",
        args: &["9223372036854775807", "sort"],
        stdin: "only-one\n",
        default_command_env: None,
    },
    Case {
        name: "header count overflow is rejected on both sides",
        args: &["99999999999999999999999999", "sort"],
        stdin: "a\nb\n",
        default_command_env: None,
    },
    Case {
        name: "unterminated final header line is still printed",
        args: &["1", "sort"],
        stdin: "header",
        default_command_env: None,
    },
    Case {
        name: "whitespace-only BODY_DEFAULT_COMMAND is rejected on both sides",
        args: &[],
        stdin: "header\na\n",
        default_command_env: Some("   "),
    },
];

fn bash_body_sh_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bash/body.sh")
}

fn run_bash(case: &Case) -> (String, i32) {
    let script = format!(
        "source '{}'; body {}",
        bash_body_sh_path().display(),
        case.args.join(" ")
    );
    let mut command = Command::new("bash");
    command
        .arg("-c")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(value) = case.default_command_env {
        command.env("BODY_DEFAULT_COMMAND", value);
    }
    let output = command
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(case.stdin.as_bytes())?;
            child.wait_with_output()
        })
        .expect("failed to run bash/body.sh");
    (
        String::from_utf8(output.stdout).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

fn run_rust(case: &Case) -> (String, i32) {
    let mut command = AssertCommand::cargo_bin("body").unwrap();
    command.args(case.args);
    if let Some(value) = case.default_command_env {
        command.env("BODY_DEFAULT_COMMAND", value);
    }
    let assert = command.write_stdin(case.stdin).assert();
    let output = assert.get_output();
    (
        String::from_utf8(output.stdout.clone()).unwrap(),
        output.status.code().unwrap_or(-1),
    )
}

#[test]
fn rust_port_matches_bash_original_across_cases() {
    for case in CASES {
        let (bash_out, bash_code) = run_bash(case);
        let (rust_out, rust_code) = run_rust(case);
        assert_eq!(
            bash_out, rust_out,
            "stdout mismatch for case '{}'",
            case.name
        );
        assert_eq!(
            bash_code, rust_code,
            "exit code mismatch for case '{}'",
            case.name
        );
    }
}
