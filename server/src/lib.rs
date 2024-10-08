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
    f64::consts::PI,
    sync::atomic::{AtomicBool, Ordering::Relaxed},
    thread,
    time::Duration,
};

use rpi_led_matrix::{LedColor, LedMatrix, LedMatrixOptions};
use rs_ws281x::{ChannelBuilder, ControllerBuilder};

const INTERVAL: Duration = Duration::from_millis(100);

pub fn run_display(run: &AtomicBool) -> Result<(), &'static str> {
    let mut options = LedMatrixOptions::new();
    options.set_rows(16);
    options.set_cols(32);
    options.set_refresh_rate(false);
    // TODO: Consider shorting pin 18, using PWM
    options.set_hardware_mapping("adafruit-hat");
    // Default runtime options, for now.
    let matrix = LedMatrix::new(Some(options), None)?;

    let color = LedColor {
        red: 64,
        green: 64,
        blue: 64,
    };

    let mut r = 0;
    let mut c = 0;
    let mut canvas = matrix.offscreen_canvas();
    tracing::info!("starting display loop");
    while run.load(Relaxed) {
        for r_ in 0..r {
            for c_ in 0..c {
                canvas.draw_circle(r_, c_, 1, &color);
            }
        }
        canvas = matrix.swap(canvas);
        r = (r + 1) % 16;
        c = (c + 1) % 32;

        thread::sleep(INTERVAL);
    }
    tracing::info!("ending display loop");
    Ok(())
}

/// Run a neopixel display.
pub fn run_neopixels(run: &AtomicBool) -> Result<(), rs_ws281x::WS2811Error> {
    const STRIP_SIZE: i32 = 60; // 1 meter at 60/meter

    let mut controller = ControllerBuilder::new()
        .freq(800_000)
        .dma(10)
        .channel(
            0,
            ChannelBuilder::new()
                .pin(10) // SPI MOSI
                .count(STRIP_SIZE)
                .strip_type(rs_ws281x::StripType::Sk6812Rgbw)
                .brightness(20)
                .build(),
        )
        .build()?;

    let mut offset = 0;
    tracing::info!("starting neopixel loop");
    while run.load(Relaxed) {
        offset += 1;

        let leds = controller.leds_mut(0);
        for (i, led) in leds.iter_mut().enumerate() {
            let brightness_fraction = (i + offset) as f64 / STRIP_SIZE as f64;
            let brightness_sin = (brightness_fraction * PI * 2.0).sin() * 0.5 + 0.5;
            // Normalize between 0 and 1: add 1 to get range (0, 2), then divide
            let brightness_sin = (brightness_sin + 1.0) / 2.0;
            let brigntess_int = ((brightness_sin * 255.0) as usize).clamp(0, 255) as u8;
            *led = [0, 0, 0, brigntess_int];
        }
        controller.render()?;
        thread::sleep(INTERVAL);
    }
    tracing::info!("ending neopixel loop");
    Ok(())
}
