#!/usr/bin/env bash
set -uo pipefail

source /home/xuanwo/.cargo/env

source_dir=$1
evidence_dir=$2
suite=$3
mkdir -p "$evidence_dir"
cd "$source_dir" || exit 2
exec > >(tee "$evidence_dir/run.log") 2>&1

export CARGO_BUILD_JOBS=8
overall=0
run() {
    printf '\nCOMMAND:'
    printf ' %q' "$@"
    printf '\n'
    "$@"
    result=$?
    printf 'COMMAND_EXIT=%s\n' "$result"
    if [ "$result" -ne 0 ]; then
        overall=1
    fi
}

date -u
hostname
git rev-parse HEAD
git status --short
cargo --version
rustc -Vv
printf 'SUITE=%s\n' "$suite"

case "$suite" in
    standard)
        run cargo fmt --all -- --check
        run cargo build --workspace --all-features
        run cargo test --workspace --all-features
        run cargo clippy --workspace --all-features --all-targets -- -D warnings
        ;;
    msrv)
        run cargo +1.85.0 build --workspace
        ;;
    feature)
        run cargo test -p backon --no-default-features --features std-blocking-sleep
        run cargo check -p backon --no-default-features
        ;;
    backoff)
        run cargo test -p backon --all-features backoff::
        ;;
    wasm)
        cd backon || exit 2
        run wasm-pack test --node --all-features -- --skip blocking
        ;;
    nightly)
        export RUSTDOCFLAGS='--cfg docsrs'
        run cargo +nightly test --workspace --all-features
        ;;
    *)
        printf 'Unknown suite: %s\n' "$suite"
        overall=2
        ;;
esac

date -u
printf 'SUITE_EXIT=%s\n' "$overall"
printf '%s\n' "$overall" > "$evidence_dir/exit.tmp"
mv "$evidence_dir/exit.tmp" "$evidence_dir/exit"
exit "$overall"
