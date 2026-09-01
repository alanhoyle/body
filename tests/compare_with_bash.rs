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
}

const CASES: &[Case] = &[
    Case {
        name: "default 1-line header, sort",
        args: &[],
        stdin: "header\nc\na\nb\n",
    },
    Case {
        name: "2-line header, sort -r",
        args: &["2", "sort", "-r"],
        stdin: "h1\nh2\nc\na\nb\n",
    },
    Case {
        name: "0 header lines, sort",
        args: &["0", "sort"],
        stdin: "c\na\nb\n",
    },
    Case {
        name: "0 header lines, wc -l",
        args: &["0", "wc", "-l"],
        stdin: "one\ntwo\nthree\n",
    },
    Case {
        name: "header count exceeds input length",
        args: &["5", "sort"],
        stdin: "only-one\n",
    },
    Case {
        name: "grep -v with a preserved header",
        args: &["1", "grep", "-v", "banana"],
        stdin: "header\nbanana\napple\ncherry\n",
    },
    Case {
        name: "no trailing newline on last line",
        args: &["1", "sort"],
        stdin: "header\nc\na\nb",
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
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
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
    let assert = AssertCommand::cargo_bin("body")
        .unwrap()
        .args(case.args)
        .write_stdin(case.stdin)
        .assert();
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
