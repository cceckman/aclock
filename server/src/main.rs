use server::context::Context;

fn main() {
    tracing_subscriber::fmt::init();

    let ctx = Context::new();
    {
        let ctx = ctx.clone();
        ctrlc::set_handler(move || {
            tracing::info!("got SIGINT, closing context");
            ctx.cancel();
        })
        .expect("could not set SIGINT handler");
    }

    #[cfg(feature = "simulator")]
    let mut displays = server::simulator::SimDisplays::new();

    #[cfg(not(feature = "simulator"))]
    let mut displays = server::LedDisplays::new();

    server::run(&ctx, &mut displays);
    tracing::info!("shut down");
}
