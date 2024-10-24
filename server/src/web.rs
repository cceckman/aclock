/// Set up logging for the WASM simulator.
use log::MakeConsoleWriter;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn run() {
    tracing_subscriber::fmt::fmt()
        .with_writer(MakeConsoleWriter)
        .init();

    tracing::info!("hi");
}

mod log {

    use tracing_subscriber::fmt::MakeWriter;
    use wasm_bindgen::JsValue;
    /// Makes a writer to the web_sys console.
    pub struct MakeConsoleWriter;

    impl MakeWriter<'_> for MakeConsoleWriter {
        type Writer = MakeConsoleWriter;

        fn make_writer(&'_ self) -> Self::Writer {
            MakeConsoleWriter
        }
    }

    impl std::io::Write for MakeConsoleWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let s = core::str::from_utf8(buf)
                .map(JsValue::from_str)
                .ok()
                .unwrap_or_else(|| {
                    JsValue::from_str(&format!("non-string log message: {:?}", buf))
                });
            let a = js_sys::Array::new_with_length(1);
            a.set(0, s);

            web_sys::console::log(&a);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
