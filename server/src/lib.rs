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
pub mod riseset;

#[cfg(feature = "simulator")]
pub mod simulator;

use chrono::Local;
use context::Context;
use edge::get_pixels;
use embedded_graphics_core::pixelcolor::Rgb888;
use face::get_clock;
use rpi_led_matrix::{LedMatrix, LedMatrixOptions};
use rs_ws281x::{ChannelBuilder, Controller, ControllerBuilder};

pub trait Displays {
    /// Access to the edge data
    fn edge(&mut self) -> &mut [edge::NeoPixelColor];

    /// Access to the face display
    fn face(
        &mut self,
    ) -> impl embedded_graphics_core::draw_target::DrawTarget<Color = Rgb888, Error = Infallible>;

    /// Flush any pending pixels (i.e. v-sync)
    fn flush(&mut self) -> Result<(), String>;
}

/// Displays implementation for real hardware.
/// Backed by a 32x16 LED matrix and a strip of NeoPixels.
pub struct LedDisplays {
    strip: Controller,
    matrix: LedMatrix,
}

impl Displays for LedDisplays {
    fn edge(&mut self) -> &mut [edge::NeoPixelColor] {
        self.strip.leds_mut(0)
    }

    fn face(
        &mut self,
    ) -> impl embedded_graphics_core::draw_target::DrawTarget<Color = Rgb888, Error = Infallible>
    {
        self.matrix.offscreen_canvas()
    }

    fn flush(&mut self) -> Result<(), String> {
        let off = self.matrix.offscreen_canvas();
        let _ = self.matrix.swap(off);
        self.strip.render().map_err(|e| e.to_string())
    }
}

impl LedDisplays {
    const STRIP_SIZE: i32 = 60;

    /// Create a new handler for hardware LED displays.
    pub fn new() -> Result<Self, String> {
        let strip = Self::new_controller()?;
        let matrix = Self::new_matrix()?;
        Ok(Self { strip, matrix })
    }

    fn new_controller() -> Result<Controller, String> {
        ControllerBuilder::new()
            .freq(800_000)
            .dma(10)
            .channel(
                0,
                ChannelBuilder::new()
                    .pin(10) // SPI MOSI
                    .count(Self::STRIP_SIZE)
                    // Datasheet says RGBW, but this is what
                    // I've got.
                    .strip_type(rs_ws281x::StripType::Sk6812Gbrw)
                    .brightness(100)
                    .build(),
            )
            .build()
            .map_err(|v| v.to_string())
    }

    fn new_matrix() -> Result<LedMatrix, String> {
        let mut options = LedMatrixOptions::new();
        // This matrix presents as two 16x16 panels.
        const ROWS: u32 = 16;
        const COLS: u32 = 16;
        options.set_rows(ROWS);
        options.set_cols(COLS);
        options.set_chain_length(2);
        options.set_parallel(1);
        options.set_refresh_rate(false);

        // TODO: Consider shorting pin 18, using PWM
        options.set_hardware_mapping("adafruit-hat");
        LedMatrix::new(Some(options), None).map_err(|e| e.to_owned())
    }
}

/// Display routine.
pub fn run(ctx: &Context, displays: &mut impl Displays) {
    while !ctx.is_cancelled() {
        let t = Local::now();
        tracing::info!("rendering clock at {}", t);
        get_clock(t, displays);
        get_pixels(t, displays).expect("");
        displays.flush().expect("failed to write to displays");

        // Sleep until _almost_ the next second.
        let frac = (1000 - t.timestamp_subsec_millis()) as i32;
        let sleep = std::cmp::max(frac - 10, 10);
        ctx.wait_timeout(Duration::from_millis(sleep as u64));
    }
    ctx.cancel()
}
