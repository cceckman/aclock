use std::time::Duration;

use chrono::Local;
use server::{context::Context, Renderer};

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
    let mut displays = server::led_displays::LedDisplays::new().unwrap();

    let renderer = Renderer::default();
    while !ctx.is_cancelled() {
        let t = Local::now();
        tracing::info!("rendering clock at {}", t);
        renderer.render(&mut displays, t);

        // Sleep until _almost_ the next second.
        let frac = (1000 - t.timestamp_subsec_millis()) as i32;
        let sleep = std::cmp::max(frac - 10, 10);
        ctx.wait_timeout(Duration::from_millis(sleep as u64));
    }
    ctx.cancel();

    tracing::info!("shut down");
}
