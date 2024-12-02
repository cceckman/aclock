use std::time::Duration;

use chrono::Local;
use linux_embedded_hal::I2cdev;
use server::{
    atmosphere::{AtmosphereSampler, NullAtmosphereSampler},
    context::Context,
    Renderer, RendererSettings,
};

fn get_i2c_atmosphere() -> Result<scd30::SCD30<I2cdev>, scd30::Error<linux_embedded_hal::I2CError>>
{
    let device = linux_embedded_hal::I2cdev::new("/dev/i2c-1").map_err(|e| {
        tracing::error!("could not open device at /dev/i2c-1: {}", e);
        scd30::Error::NotReady()
    })?;
    scd30::SCD30::new(device, scd30::SCD30Settings::default())
}

fn get_atmosphere() -> Box<dyn AtmosphereSampler> {
    match get_i2c_atmosphere() {
        Ok(v) => Box::new(v),
        Err(e) => {
            tracing::error!("could not set up SCD30: {e}");
            Box::new(NullAtmosphereSampler {})
        }
    }
}

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
    let mut displays = {
        tracing::info!("using simulated displays");
        server::simulator::SimDisplays::new()
    };

    #[cfg(not(feature = "simulator"))]
    let mut displays = server::led_displays::LedDisplays::new().unwrap();

    let mut atmo = get_atmosphere();

    let mut renderer: Renderer = RendererSettings::default().into();

    // let mut atmo = NullAtmosphereSampler {};
    while !ctx.is_cancelled() {
        let t = Local::now();
        tracing::trace!("rendering clock at {}", t);
        renderer.render(&mut displays, &mut atmo, t);

        // Sleep until _almost_ the next second.
        let frac = (1000 - t.timestamp_subsec_millis()) as i32;
        let sleep = std::cmp::max(frac - 10, 10);
        ctx.wait_timeout(Duration::from_millis(sleep as u64));
    }
    ctx.cancel();

    tracing::info!("shut down");
}
