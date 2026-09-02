#!/usr/bin/env bats

setup() {
    source "${BATS_TEST_DIRNAME}/../body.sh"
}

@test "default: keeps 1-line header, sorts the rest" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; printf "header\nc\na\nb\n" | body 2>/dev/null'
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "header" ]
    [ "${lines[1]}" = "a" ]
    [ "${lines[2]}" = "b" ]
    [ "${lines[3]}" = "c" ]
}

@test "N header lines are preserved, custom command is used" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; printf "h1\nh2\nc\na\nb\n" | body 2 sort -r 2>/dev/null'
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "h1" ]
    [ "${lines[1]}" = "h2" ]
    [ "${lines[2]}" = "c" ]
    [ "${lines[3]}" = "b" ]
    [ "${lines[4]}" = "a" ]
}

@test "0 header lines passes everything straight through to the command (regression)" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; printf "c\na\nb\n" | body 0 sort 2>/dev/null'
    [ "$status" -eq 0 ]
    [ "${#lines[@]}" -eq 3 ]
    [ "${lines[0]}" = "a" ]
    [ "${lines[1]}" = "b" ]
    [ "${lines[2]}" = "c" ]
}

@test "0 header lines does not consume two lines like the old {1..0} brace-expansion bug" {
    # wc -l only sees lines actually forwarded to the command's stdin, so it
    # exposes the old bug (which siphoned off 2 lines as "header" for N=0)
    # in a way that `cat` alone cannot (identical output either way).
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; printf "one\ntwo\nthree\n" | body 0 wc -l 2>/dev/null'
    [ "$status" -eq 0 ]
    [ "${#lines[@]}" -eq 1 ]
    [[ "${lines[0]}" =~ ^[[:space:]]*3$ ]]
}

@test "fewer lines than requested header count does not hang or error" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; printf "only-one\n" | body 5 sort 2>/dev/null'
    [ "$status" -eq 0 ]
    [ "${#lines[@]}" -eq 1 ]
    [ "${lines[0]}" = "only-one" ]
}

@test "command with its own arguments works alongside a preserved header" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; printf "header\nbanana\napple\ncherry\n" | body 1 grep -v banana 2>/dev/null'
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "header" ]
    [ "${lines[1]}" = "apple" ]
    [ "${lines[2]}" = "cherry" ]
}

@test "no piped input prints an error and usage, exits non-zero" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; body < /dev/tty' </dev/null
    # No /dev/tty in CI, so simulate via an explicit terminal check instead:
    skip "requires a real controlling terminal to exercise the -t 0 branch"
}

@test "-h prints help and exits 0 when input is piped" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; printf "a\nb\n" | body -h'
    [ "$status" -eq 0 ]
    [[ "$output" == *"USAGE:"* ]]
}

@test "missing command falls back to sort and announces it on stderr" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; printf "header\nb\na\n" | body 2>&1 1>/dev/null'
    [[ "$output" == *"running sort by default"* ]]
}

@test "BODY_DEFAULT_COMMAND is announced and used when no command is given" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; export BODY_DEFAULT_COMMAND="rev"; printf "header\nabc\n" | body 2>&1'
    [ "$status" -eq 0 ]
    [[ "$output" == *"running rev by default"* ]]
    [[ "$output" == *"cba"* ]]
}

@test "BODY_DEFAULT_COMMAND is ignored when a command is given explicitly" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; export BODY_DEFAULT_COMMAND="rev"; printf "header\nc\na\nb\n" | body 1 sort'
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "header" ]
    [ "${lines[1]}" = "a" ]
    [ "${lines[2]}" = "b" ]
    [ "${lines[3]}" = "c" ]
}

@test "BODY_DEFAULT_COMMAND with arguments is word-split, not run as one command name" {
    run bash -c 'source "'"${BATS_TEST_DIRNAME}"'/../body.sh"; export BODY_DEFAULT_COMMAND="sort -r"; printf "header\na\nc\nb\n" | body 2>/dev/null'
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "header" ]
    [ "${lines[1]}" = "c" ]
    [ "${lines[2]}" = "b" ]
    [ "${lines[3]}" = "a" ]
}
