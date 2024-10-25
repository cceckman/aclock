redo-ifchange Cargo.toml Cargo.lock $(find src/)

set -eu

# Build normally first, to show errors in the stderr stream
cargo build --release \
    --no-default-features --features=video \
    --bin video

cargo build --release \
    --no-default-features --features=video \
    --message-format=json  \
    --bin video \
| jq -r "select(.target.name == \"video\") | select(.executable) | .executable" \
>"$3"

OUTPUT="$(cat "$3")"
rm "$3"
cp "$OUTPUT" "$3"
