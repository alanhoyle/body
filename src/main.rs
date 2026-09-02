//! `body`: print the header line(s) from STDIN untouched, then hand the
//! remaining "body" of the input to another command (`sort` by default) for
//! further processing. Useful for `sort`/`grep`/etc. when you want to keep a
//! header row in place.
//!
//! Ported from the original bash function (see bash/body.sh). The behavior
//! here is intended to match that version line-for-line; see bash/body.sh's
//! header comment for the bugs that were fixed along the way, which apply
//! equally to this port.
//!
//! Implementation note: after printing N header lines, the remaining STDIN
//! bytes must reach the child command completely untouched (no bytes stolen
//! by internal buffering). Rust's `std::io::Stdin` wraps a shared, buffered
//! reader that would read ahead past the header and swallow bytes the child
//! needs. To avoid that, header lines are read one byte at a time directly
//! from file descriptor 0 via an unbuffered `File`, mirroring how bash's
//! `read` builtin avoids over-consuming a pipe.

use std::env;
use std::fs::File;
use std::io::{self, IsTerminal, Read, Write};
use std::mem::forget;
use std::os::unix::io::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitCode, Stdio};

const DEFAULT_COMMAND: &str = "sort";
const DEFAULT_HEADER_LINES: usize = 1;

/// Default command to hand the body to when none is given on the command
/// line. Overridable via the `BODY_DEFAULT_COMMAND` environment variable,
/// falling back to `sort` if unset or empty (matching bash's `${VAR:-default}`).
fn default_command() -> String {
    match env::var("BODY_DEFAULT_COMMAND") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => DEFAULT_COMMAND.to_string(),
    }
}

fn print_help(default_command: &str) {
    eprintln!("body: prints the header from a STDIN and sends the 'body' to another command for");
    eprintln!("    additional processing.  Useful for sort/grep when you want to keep headers.");
    eprintln!();
    eprintln!("USAGE:  COMMAND | body [ N ] [ COMMAND_TO_PROCESS_OUTPUT ]");
    eprintln!("    if the first parameter N is a whole number (0 or more), it prints that many");
    eprintln!(
        "        lines before proceeding  [ default: skip {} ]",
        DEFAULT_HEADER_LINES
    );
    eprintln!(
        "    if the [ COMMAND_TO_PROCESS_OUTPUT ] is omitted, '{}' is used",
        default_command
    );
    eprintln!("        (override via the BODY_DEFAULT_COMMAND environment variable)");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    Sort a file, but maintain a one-line header:");
    eprintln!("        cat file_with_one_line_header.txt | body");
    eprintln!("    Sort a file in reverse, but keep the top two lines of header in place:");
    eprintln!("        cat file_with_two_line_header.txt | body 2 sort -r");
    eprintln!("    Pass through with no header at all:");
    eprintln!("        cat file_with_no_header.txt | body 0 sort");
}

/// Reads one line (without the trailing newline) directly from `file`,
/// one byte at a time, so no bytes beyond the line itself are consumed
/// from the underlying pipe. Returns `None` at EOF with nothing read.
fn read_line_unbuffered(file: &mut File) -> Option<Vec<u8>> {
    let mut line = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match file.read(&mut byte) {
            Ok(0) => return if line.is_empty() { None } else { Some(line) },
            Ok(_) => {
                if byte[0] == b'\n' {
                    return Some(line);
                }
                line.push(byte[0]);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return if line.is_empty() { None } else { Some(line) },
        }
    }
}

fn is_whole_number(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let stdin_is_tty = io::stdin().is_terminal();
    let first_is_help = matches!(args.first().map(String::as_str), Some("-h" | "--help"));

    if stdin_is_tty || first_is_help {
        if stdin_is_tty && !first_is_help {
            eprintln!("ERROR:  body requires piped input!");
            eprintln!();
        }
        print_help(&default_command());
        return if stdin_is_tty && !first_is_help {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        };
    }

    let mut args = args.into_iter();
    let mut header_lines = DEFAULT_HEADER_LINES;
    let mut pending_first = args.next();

    if let Some(first) = &pending_first {
        if is_whole_number(first) {
            header_lines = first.parse().unwrap_or(DEFAULT_HEADER_LINES);
            pending_first = args.next();
        }
    }

    let default_command = default_command();
    let command_args: Vec<String> = pending_first.into_iter().chain(args).collect();
    if command_args.is_empty() {
        eprintln!("body: running {} by default", default_command);
    }

    // SAFETY: fd 0 (stdin) is open for the lifetime of the process; wrapping
    // it here does not take exclusive ownership away from anything else,
    // and `forget` below ensures it is never closed by this `File`'s Drop.
    let mut raw_stdin = unsafe { File::from_raw_fd(0) };
    {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for _ in 0..header_lines {
            match read_line_unbuffered(&mut raw_stdin) {
                Some(line) => {
                    let _ = out.write_all(&line);
                    let _ = out.write_all(b"\n");
                }
                None => break,
            }
        }
        let _ = out.flush();
    }
    forget(raw_stdin); // the child still needs fd 0 open

    // BODY_DEFAULT_COMMAND may contain arguments (e.g. "sort -r"); split it
    // on whitespace the same way bash word-splits an unquoted variable.
    let final_command: Vec<String> = if command_args.is_empty() {
        default_command
            .split_whitespace()
            .map(String::from)
            .collect()
    } else {
        command_args
    };
    let (program, prog_args) = final_command
        .split_first()
        .map(|(program, rest)| (program.as_str(), rest))
        .expect("BODY_DEFAULT_COMMAND is never empty or all-whitespace");

    let status = Command::new(program)
        .args(prog_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match status {
        Ok(status) => {
            if let Some(code) = status.code() {
                ExitCode::from(code as u8)
            } else if let Some(signal) = status.signal() {
                ExitCode::from((128 + signal) as u8)
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            eprintln!("body: {}: command not found", program);
            ExitCode::from(127)
        }
        Err(e) => {
            eprintln!("body: failed to run '{}': {}", program, e);
            ExitCode::FAILURE
        }
    }
}
