redo-ifchange Cargo.toml Cargo.lock $(find src/)

set -eu

if uname -m | grep -q x86
then
    export CXX=aarch64-linux-gnu-g++
    export CC=aarch64-linux-gnu-gcc
    TARGET="--target aarch64-unknown-linux-gnu"
    TARGET_DIR="target/aarch64-unknown-linux/"
else
    TARGET_DIR="target/"
fi


# Build normally first, to show errors in the stderr stream
cargo build --release $TARGET --no-default-features --features=hardware

cargo build --release $TARGET --message-format=json --no-default-features --features=hardware \
| jq -r "select(.target.name == \"$2\") | select(.executable) | .executable" \
>"$3"

OUTPUT="$(cat "$3")"
rm "$3"
cp "$OUTPUT" "$3"
