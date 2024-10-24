use std::time::Duration;

use chrono::Local;
use server::{context::Context, Renderer, RendererSettings};

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
    let (mut displays, mut atmo) = {
        tracing::info!("using simulated hardware");
        (
            server::simulator::SimDisplays::new(),
            server::atmosphere::NullAtmosphereSampler {},
        )
    };

    #[cfg(not(feature = "simulator"))]
    let (mut displays, mut atmo) = {
        use linux_embedded_hal::I2cdev;
        let device = I2cdev::new("/dev/i2c-1").expect("could not open i2c device");
        let atmo = scd30::SCD30::new(device, scd30::SCD30Settings::default()).unwrap();
        let displays = server::led_displays::LedDisplays::new().unwrap();

        (displays, atmo)
    };

    let mut renderer: Renderer = RendererSettings::default().into();

    // let mut atmo = NullAtmosphereSampler {};
    while !ctx.is_cancelled() {
        let t = Local::now();
        tracing::info!("rendering clock at {}", t);
        renderer.render(&mut displays, &mut atmo, t);

        // Sleep until _almost_ the next second.
        let frac = (1000 - t.timestamp_subsec_millis()) as i32;
        let sleep = std::cmp::max(frac - 10, 10);
        ctx.wait_timeout(Duration::from_millis(sleep as u64));
    }
    ctx.cancel();

    tracing::info!("shut down");
}
