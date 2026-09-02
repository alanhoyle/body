# body

[![CI](https://github.com/alanhoyle/body/actions/workflows/ci.yml/badge.svg)](https://github.com/alanhoyle/body/actions/workflows/ci.yml)

Prints the header line(s) from piped STDIN untouched, then hands the
remaining "body" of the input to another command (`sort` by default) for
further processing. Handy for `sort`/`grep`/etc. when you want to keep a
header row in place.

This started as [a gist](https://gist.github.com/alanhoyle/7ec6bd445a790b62567d8b1ff6941c66)
(a bash shell function) and now has two implementations that are kept
behaviorally identical and tested against each other:

- **Rust** (`src/main.rs`) — a standalone binary, no sourcing required. This
  is the recommended version.
- **bash** (`bash/body.sh`) — the original shell-function version, kept as a
  lightweight alternative for anyone who'd rather not build a binary, and as
  a reference the Rust port is tested against.

**Unix only** (Linux, macOS, BSD, ...) — there is no Windows support. The
Rust binary relies on Unix-specific APIs (raw file descriptor access via
`std::os::unix::io::FromRawFd`, and reading a child's exit signal via
`std::os::unix::process::ExitStatusExt`) and will not compile on Windows;
the bash version requires a POSIX-style shell (bash or zsh) that Windows
doesn't provide natively. CI only tests Linux and macOS.

## Installation

### Rust (recommended)

From a local clone:

```bash
cargo install --path .
```

Or directly from GitHub, without cloning first:

```bash
cargo install --git https://github.com/alanhoyle/body.git
```

Either way, this installs a `body` binary to `~/.cargo/bin` (make sure it's
on your `PATH`). It works as a normal command — no sourcing needed:

```bash
cat file_with_one_line_header.txt | body
```

### bash

`bash/body.sh` defines a shell function — source it into your shell:

```bash
source /path/to/bash/body.sh
```

To make it available in every new shell, add that line to your `~/.bashrc`
or `~/.zshrc`.

## Usage

Both versions share the same interface:

```
COMMAND | body [-h|--help] [N] [COMMAND_TO_PROCESS_BODY [ARGS...]]
```

- `N` — number of header lines to pass through untouched (default: `1`; `0`
  is valid and means "no header"). Always parsed as plain decimal, even with
  leading zeros (e.g. `007` means seven, not octal); a value greater than
  `9223372036854775807` is rejected with an error rather than silently
  reinterpreted.
- `COMMAND_TO_PROCESS_BODY` — the command the remaining lines are piped to
  (default: `sort`). Any arguments after it are passed along.

The default command (used when `COMMAND_TO_PROCESS_BODY` is omitted) can be
changed by setting the `BODY_DEFAULT_COMMAND` environment variable, e.g.:

```bash
BODY_DEFAULT_COMMAND="sort -r" body <<< $'header\na\nc\nb'
```

`BODY_DEFAULT_COMMAND` must contain an actual command once word-split; a
whitespace-only value is rejected with an error rather than silently
falling back to `sort` or silently discarding the piped body.

`bash/body.sh` can be sourced into either bash or zsh (see Installation
above); both are covered by the test suite.

### Examples

Sort a file, keeping a one-line header in place:

```bash
cat file_with_one_line_header.txt | body
```

Sort a file in reverse, keeping the top two header lines in place:

```bash
cat file_with_two_line_header.txt | body 2 sort -r
```

No header at all, still explicit about it:

```bash
cat file_with_no_header.txt | body 0 sort
```

Use a completely different command, e.g. filtering out a row:

```bash
cat file.txt | body 1 grep -v banana
```

## Testing

### Rust

```bash
cargo test
```

This runs the CLI behavior tests (`tests/cli.rs`, ported from the bash bats
suite) plus `tests/compare_with_bash.rs`, which runs the same inputs through
`bash/body.sh` and the compiled binary and asserts identical stdout and exit
codes.

### bash

Tests use [bats-core](https://github.com/bats-core/bats-core):

```bash
brew install bats-core   # one-time setup
bats bash/test/body.bats
```

## Porting notes: bash → Rust

The trickiest part of the port: after printing N header lines, the remaining
STDIN bytes must reach the child command completely untouched. Rust's
`std::io::Stdin` wraps a shared, buffered reader that reads ahead and would
swallow bytes the child needs. The Rust version instead reads header lines
one byte at a time directly from file descriptor 0 via an unbuffered `File`,
mirroring how bash's `read` builtin avoids over-consuming a pipe — see the
comment in `src/main.rs` for details.

## Changes from the original gist

- Fixed a bug where `body 0 ...` (zero header lines) actually consumed and
  emitted *two* lines as header. The original code built the header loop with
  `for line in $(eval echo "{1..$HEADER_LINES}")`, and bash's `{1..0}` brace
  expansion produces the two-element descending sequence `1 0` rather than
  nothing — so `N=0` behaved like `N=2`. Fixed in both the bash version (a
  plain counted `for ((...))` loop, no `eval`) and, by construction, in the
  Rust port.
- The header-reading loop stops early if STDIN has fewer lines than
  requested, instead of printing extra blank lines.
- bash: command-supplied detection now uses `$#` instead of joining `"$@"`
  into a string and checking it for emptiness (avoids mis-detecting
  arguments that are empty or all-whitespace).
- Rust: unknown commands exit `127` and signal-terminated commands exit
  `128 + signal`, matching typical shell conventions.
- Fixed a divergence where a header-count argument with a leading zero
  (e.g. `body 010 ...`) was interpreted as octal by bash's `(( ))`
  arithmetic (`010` → 8, and `018` was a hard arithmetic error since `8`/`9`
  aren't valid octal digits) but as decimal by the Rust port (`010` → 10).
  Both now always treat `N` as plain decimal, matching the documented "N is
  a whole number" contract; bash normalizes by stripping leading zeros from
  the argument before it ever reaches arithmetic.
- Fixed a divergence in how an out-of-range header count (e.g.
  `body 99999999999999999999999999 ...`) was handled: bash's `(( ))`
  arithmetic silently overflowed/wrapped (ending up behaving like `N=0`),
  while the Rust port silently fell back to the default header count
  (`N=1`) on parse failure with no indication anything was wrong. Both now
  explicitly reject any `N` greater than `9223372036854775807` (the largest
  value both bash's 64-bit signed arithmetic and Rust's `i64` can
  represent) with a clear error on stderr and exit code `1`, instead of
  silently doing something surprising.
- Fixed a bug where an unterminated final header line (STDIN ends mid-line,
  with no trailing newline, exactly where the header ends) was silently
  dropped in bash: `read || break` discards the partial line `read` still
  populates on that kind of EOF, since the failure short-circuits before the
  `printf` that would print it. Bash now prints that partial line before
  stopping, matching the Rust port (which already handled this correctly).
- Fixed a bash/zsh incompatibility: the default-command path used bash's
  `read -a` to word-split `BODY_DEFAULT_COMMAND` into an array, but zsh's
  `read` uses `-A` for the same purpose (`-a` is a different, invalid
  option in zsh), so sourcing `body.sh` into zsh and hitting the
  default-command path — even the ordinary case of just running `body` with
  no arguments — failed outright. `body.sh` is documented to also be
  sourceable into zsh (see Installation); it's now tested under both
  shells.
- Fixed a three-way divergence for a whitespace-only `BODY_DEFAULT_COMMAND`
  (e.g. `BODY_DEFAULT_COMMAND="   "`): bash silently word-split it to zero
  words and ran a no-op, silently discarding the piped body with exit `0`;
  zsh's attempt to run the same zero-word command instead failed with an
  unrelated-looking "permission denied" (exit `126`); and the Rust port
  silently substituted `sort` as if the variable were unset. All three now
  reject it explicitly, before touching STDIN, with a clear error on
  stderr and exit code `1`.

## License

MIT — see [LICENSE](LICENSE).
