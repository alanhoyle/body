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

## Installation

### Rust (recommended)

```bash
cargo install --path .
```

This installs a `body` binary to `~/.cargo/bin` (make sure it's on your
`PATH`). It works as a normal command — no sourcing needed:

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
  is valid and means "no header").
- `COMMAND_TO_PROCESS_BODY` — the command the remaining lines are piped to
  (default: `sort`). Any arguments after it are passed along.

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
