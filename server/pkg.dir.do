redo-always

wasm-pack build --target web \
    --no-default-features \
    --features web

sha256sum pkg/* | tee "$3" | redo-stamp

