//! A clock.
//!
//! Drives an LED matrix and a ring of NeoPixels.
//!
//! # Building
//! Requires:
//! - libclang, per [rs_ws281x](https://crates.io/crates/rs_ws281x)
//! - gcc-aarch64-linux-gnu for cross-compilation
//! - g++-aarch64-linux-gnu for cross-compilation
//!
use std::{
    convert::Infallible,
    mem::transmute_copy,
    sync::atomic::{
        AtomicBool,
        Ordering::{self, Relaxed},
    },
    thread,
    time::{Duration, Instant},
};

pub mod edge;
pub mod face;

use chrono::{Local, SubsecRound};
use edge::get_pixels;
use embedded_graphics_core::pixelcolor::Rgb888;
use face::{get_clock, get_face};
use rs_ws281x::{ChannelBuilder, ControllerBuilder};

const INTERVAL: Duration = Duration::from_millis(10);

/// An Edge is a renderer for the edge lights.
/// In real hardware, this is a line of RGBW NeoPixels.
pub trait Edge {
    /// Access the data buffer
    fn data(&mut self) -> &mut [edge::NeoPixelColor];

    /// Flush the most recently-written data to the lights.
    fn flush(&mut self) -> Result<(), String>;
}

/// A Face is a renderer for the face display.
/// In real hardware, this is a 32x16 LED matrix.
pub trait Face {
    /// Access the underlying drawable of this face.
    fn drawable(
        &mut self,
    ) -> &mut impl embedded_graphics_core::draw_target::DrawTarget<Color = Rgb888, Error = Infallible>;

    /// Flush any pending pixels (i.e. v-sync)
    fn flush(&mut self);
}

/// Display routine.
pub fn run() {
    let run = AtomicBool::new(true);
    std::thread::scope(|s| {
        //let neopixel = s.spawn(|| {
        //    let r = run_neopixels(run);
        //    run.store(false, Ordering::SeqCst);
        //    r.unwrap();
        //});
        let matrix = s.spawn(|| {
            let mut face = get_face().expect("should create matrix");
            while run.load(Ordering::Relaxed) {
                let t = Local::now();
                tracing::info!("rendering clock at {}", t);
                get_clock(t, &mut face);

                // Sleep until _almost_ the next second.
                let frac = (1000 - t.timestamp_subsec_millis()) as i32;
                let sleep = std::cmp::max(frac - 10, 10);
                std::thread::sleep(Duration::from_millis(sleep as u64));
            }
            run.store(false, Ordering::SeqCst);
        });
        let timer = s.spawn(|| {
            let now = Instant::now();
            let deadline = now + Duration::from_secs(60);
            tracing::info!("starting timer loop");
            while run.load(Ordering::Relaxed) && Instant::now() < deadline {
                // Responsive, but not too busy
                thread::sleep(Duration::from_millis(100));
            }
            tracing::info!("ending timer");
            run.store(false, Ordering::SeqCst);
        });

        // neopixel.join().expect("could not join neopixel thread");
        matrix.join().expect("could not join matrix thread");
        timer.join().expect("could not join timer thread");
    });
}

/// Run a neopixel display.
pub fn run_neopixels(run: &AtomicBool) -> Result<(), String> {
    const STRIP_SIZE: usize = 60; // 1 meter at 60/meter

    let mut controller = ControllerBuilder::new()
        .freq(800_000)
        .dma(10)
        .channel(
            0,
            ChannelBuilder::new()
                .pin(10) // SPI MOSI
                .count(STRIP_SIZE as i32)
                // Datasheet says RGBW, but this is what
                // I've got.
                .strip_type(rs_ws281x::StripType::Sk6812Gbrw)
                .brightness(100)
                .build(),
        )
        .build()
        .map_err(|v| v.to_string())?;

    tracing::info!("starting neopixel loop");
    let leds = controller.leds_mut(0);
    get_pixels(Local::now(), leds)?;
    controller.render().map_err(|v| v.to_string())?;

    while run.load(Relaxed) {
        // This is the interval of "poll for stop signal",
        // not necessarily the interval for "re-render".
        // TODO: better cancellation (async?)
        thread::sleep(INTERVAL);
    }
    tracing::info!("clearing neopixels");
    for led in controller.leds_mut(0) {
        *led = [0, 0, 0, 0];
    }
    controller.render().map_err(|v| v.to_string())?;
    tracing::info!("ending neopixel loop");
    Ok(())
}
