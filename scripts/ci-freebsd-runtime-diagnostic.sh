#!/bin/sh
# Temporary hosted diagnostic for the full x86_64 FreeBSD pilot, not a test filter.
set -eu
command -v timeout >/dev/null
export RUST_BACKTRACE=1
export CARGO_PROFILE_DEV_DEBUG=0

# Only fixed read-only fields: never print process arguments or environment.
(
    sample=0
    while [ "$sample" -lt 300 ]; do
        echo "=== FreeBSD runtime sample $sample ==="
        date -u
        timeout 10s ps -axo pid,ppid,state,wchan,rss,comm || :
        timeout 10s swapinfo -k || :
        timeout 10s vmstat 1 2 || :
        sample=$((sample + 1))
        sleep 60
    done
) &
monitor_pid=$!
cleanup() {
    status=$?
    trap - 0
    kill "$monitor_pid" 2>/dev/null || :
    wait "$monitor_pid" 2>/dev/null || :
    exit "$status"
}
trap cleanup 0

# Keep the original four selections, features, profiles, serial integration/GPU
# settings, fail-fast semantics and assertions. Only expose buffered test output.
cargo test --workspace --locked --features heif --exclude photocraft-gpu --lib --bins -- --nocapture
cargo test --workspace --locked --features heif --exclude photocraft-gpu --doc -- --nocapture
cargo test --workspace --locked --features heif --exclude photocraft-gpu --test '*' -- --test-threads=1 --nocapture
cargo test -p photocraft-gpu --locked -- --test-threads=1 --nocapture
