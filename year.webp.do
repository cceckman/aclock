
set -eu

OUTPUT="$(realpath "$3")"

cd server

find . -name '*.rs' | xargs redo-ifchange

cargo run --release --bin video --features="video" -- "$OUTPUT" 1>&2

