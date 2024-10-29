redo-always

wasm-pack build \
    --target web \
    --dev \
    --no-default-features \
    --features web \

sha256sum pkg/* | tee "$3" | redo-stamp

