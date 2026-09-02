#!/usr/bin/env bash
# body: prints the header line(s) from STDIN untouched, then pipes the remaining
# body of the input to another command for further processing.  Useful for
# sort/grep/etc. when you want to preserve a header row.
#
# Source this file (or add it to your .bashrc/.zshrc) to make `body` available
# as a shell function, e.g.:
#     source /path/to/body.sh
#
# USAGE:
#     COMMAND | body [-h|--help] [N] [COMMAND_TO_PROCESS_BODY [ARGS...]]
#
# Improvements over the original version:
#   - Fixed a bug where `body 0 ...` (zero header lines) actually consumed and
#     printed TWO lines as header, because `{1..$N}` brace expansion with N=0
#     expands to "1 0" (a descending two-element range) rather than nothing.
#     Header-line iteration now uses a plain counted loop instead of an
#     eval'd brace expansion, so N=0 correctly means "no header".
#   - Removed the `eval` call entirely (avoids an unnecessary code-injection
#     surface and is easier to reason about).
#   - Stops reading header lines early (via `read || break`) if STDIN has
#     fewer lines than requested, instead of emitting extra blank lines.
#   - Uses `$#` to detect whether a body-command was supplied instead of
#     joining "$@" into a string and testing it for emptiness, which could
#     mis-detect arguments that are empty or all-whitespace.
#   - The default command (used when COMMAND_TO_PROCESS_BODY is omitted) can
#     be overridden by setting the BODY_DEFAULT_COMMAND environment variable;
#     it falls back to "sort" if unset or empty. It may include arguments
#     (e.g. BODY_DEFAULT_COMMAND="sort -r"), which are word-split before
#     running.
body() {
    local HEADER_LINES=1
    local DEFAULT_COMMAND="${BODY_DEFAULT_COMMAND:-sort}"

    if [[ -t 0 || "$1" == "-h" || "$1" == "--help" ]]; then
        if [ -t 0 ]; then
            echo "ERROR:  body requires piped input!" >&2
            echo "" >&2
        fi

        echo "body: prints the header from a STDIN and sends the 'body' to another command for" >&2
        echo "    additional processing.  Useful for sort/grep when you want to keep headers." >&2
        echo "" >&2
        echo "USAGE:  COMMAND | body [ N ] [ COMMAND_TO_PROCESS_OUTPUT ]" >&2
        echo "    if the first parameter N is a whole number (0 or more), it prints that many" >&2
        echo "        lines before proceeding  [ default: skip $HEADER_LINES ]" >&2
        echo "    if the [ COMMAND_TO_PROCESS_OUTPUT ] is omitted, '$DEFAULT_COMMAND' is used" >&2
        echo "        (override via the BODY_DEFAULT_COMMAND environment variable)" >&2
        echo "" >&2
        echo "EXAMPLES:" >&2
        echo "    Sort a file, but maintain a one-line header:" >&2
        echo "        cat file_with_one_line_header.txt | body" >&2
        echo "    Sort a file in reverse, but keep the top two lines of header in place:" >&2
        echo "        cat file_with_two_line_header.txt | body 2 sort -r" >&2
        echo "    Pass through with no header at all:" >&2
        echo "        cat file_with_no_header.txt | body 0 sort" >&2

        if [ -t 0 ]; then
            return 1
        else
            return 0
        fi
    fi

    local re='^[0-9]+$'

    if [[ $1 =~ $re ]]; then
        HEADER_LINES=$1
        shift
    fi

    if [ $# -eq 0 ]; then
        echo "body: running $DEFAULT_COMMAND by default" >&2
    fi

    local i header
    for (( i = 0; i < HEADER_LINES; i++ )); do
        IFS= read -r header || break
        printf '%s\n' "$header"
    done

    if [ $# -eq 0 ]; then
        # BODY_DEFAULT_COMMAND may contain arguments (e.g. "sort -r"), so it
        # must be word-split rather than run as a single (quoted) command name.
        local -a default_command_words
        read -ra default_command_words <<< "$DEFAULT_COMMAND"
        "${default_command_words[@]}"
    else
        "$@"
    fi
}
