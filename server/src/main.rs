use server::context::{self, Context};

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

    server::run(&ctx);
    tracing::info!("shut down");
}
