redo-ifchange Cargo.toml Cargo.lock $(find src/)

if uname -m | grep -q x86
then
    export CXX=aarch64-linux-gnu-g++
    export CC=aarch64-linux-gnu-gcc
    TARGET="--target aarch64-unknown-linux-gnu"
    TARGET_DIR="target/aarch64-unknown-linux/"
else
    TARGET_DIR="target/"
fi

OUTPUT="$(
    cargo build $TARGET --message-format=json --no-default-features \
        | jq -r 'select(.reason == "compiler-artifact") | select(.executable) | .executable')"
cp "$OUTPUT" "$3"
