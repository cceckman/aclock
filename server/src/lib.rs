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
use std::{convert::Infallible, time::Duration};

pub mod context;

pub mod edge;
pub mod face;

use chrono::Local;
use context::Context;
use edge::{get_edge, get_pixels};
use embedded_graphics_core::pixelcolor::Rgb888;
use face::{get_clock, get_face};

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
pub fn run(ctx: &Context) {
    let caught = std::panic::catch_unwind(|| {
        let mut face = get_face().expect("should create matrix");
        let mut edge = get_edge().expect("should create strip");
        while !ctx.is_cancelled() {
            let t = Local::now();
            tracing::info!("rendering clock at {}", t);
            get_clock(t, &mut face);
            get_pixels(t, &mut edge).expect("");

            // Sleep until _almost_ the next second.
            let frac = (1000 - t.timestamp_subsec_millis()) as i32;
            let sleep = std::cmp::max(frac - 10, 10);
            ctx.wait_timeout(Duration::from_millis(sleep as u64));
        }
    });
    if let Err(e) = caught {
        tracing::error!("rendering thread panicked: {:?}", e);
    }
    ctx.cancel()
}
