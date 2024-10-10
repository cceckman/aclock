fn main() {
    tracing_subscriber::fmt::init();

    server::run();
}
