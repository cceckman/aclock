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

pub mod edge;

use chrono::{DateTime, Local};
use edge::get_pixels;
use rpi_led_matrix::{LedColor, LedMatrix, LedMatrixOptions};
use rs_ws281x::{ChannelBuilder, ControllerBuilder, WS2811Error};

const INTERVAL: Duration = Duration::from_millis(10);

pub fn run_display(run: &AtomicBool) -> Result<(), &'static str> {
    let mut options = LedMatrixOptions::new();
    const ROWS: u32 = 16;
    const COLS: u32 = 16;
    options.set_rows(ROWS);
    options.set_cols(COLS);
    options.set_refresh_rate(false);

    // 16R16C accurately covers the low rows,
    // but duplicates them to the high.
    // 16R32C keeps going along a row, to columns that don't exist.
    // 32R16C loops back over columns that already exist, and mirrors.
    //
    // In the 16C32 case, our grid is:
    // row 0: col 0-16 (of 32)
    // row 1: col 0-16 (of 32)
    // as if there's a missing additional 16-column panel.
    // That kinda makes sense: there is, if this was a 32x32.
    //
    // If we use 32x32... the addresses don't work out.
    // It's zig-zaggy? does half a row, then the whole row, then the second half
    // options.set_row_addr_type(2);
    // options.set_multiplexing(4);
    // So: 16x32 is "the right" way to get the addresses.
    //
    // ... how did I have this set up in Python? 16x32, one chain, one parallel.
    // But I don't know how that worked :D
    //
    // "cols * chain length is the total length of the display..."
    // per docstring. So we *should* use 32R16C.
    // With "chain length 2", 32R16C only covers the first 8 columns (twice each),
    // but it does cover all 32 rows.
    // With "chain length 2", 16R16C goes through the first half of the board, once....
    // With "chain length 1", 16R16C goes through the first half of the board, once,
    // but also ghosts to the second half.
    options.set_chain_length(2);
    options.set_parallel(1);

    // Ah!
    // With "chain length 2", 16R16C goes through the first half of the board, once....
    // because we stop when we get to the first half!
    // So our answer is that this behaves as two chained 16x16 panels,
    // and we have to handle going "past the edge" of a single panel.

    // TODO: Consider shorting pin 18, using PWM
    options.set_hardware_mapping("adafruit-hat");
    // Default runtime options, for now.
    let matrix = LedMatrix::new(Some(options), None)?;

    let color = LedColor {
        red: 64,
        green: 64,
        blue: 64,
    };
    let off = LedColor {
        red: 0,
        green: 0,
        blue: 0,
    };

    let mut r: u32 = 0;
    let mut c: u32 = 0;
    let mut canvas = matrix.offscreen_canvas();
    tracing::info!("starting display loop");
    while run.load(Relaxed) {
        tracing::info!("{},{}", r, c);
        canvas.fill(&off);
        canvas.set(r as i32, c as i32, &color);
        canvas = matrix.swap(canvas);
        c = (c + 1) % COLS;
        if c == 0 {
            r = (r + 1) % (ROWS * 2);
        }
        if r == 0 && c == 0 {
            break;
        }

        thread::sleep(INTERVAL);
    }
    tracing::info!("ending display loop");
    Ok(())
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
                .strip_type(rs_ws281x::StripType::Sk6812Rgbw)
                .brightness(20)
                .build(),
        )
        .build()
        .map_err(|v| v.to_string())?;

    tracing::info!("starting neopixel loop");
    while run.load(Relaxed) {
        let leds = controller.leds_mut(0);
        get_pixels(Local::now(), leds)?;
        controller.render().map_err(|v| v.to_string())?;
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
