//! Display implementation on LED screens.
use std::convert::Infallible;

use crate::{Displays, NeoPixelColor};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget},
};
use rpi_led_matrix::{LedCanvas, LedMatrix, LedMatrixOptions};
use rs_ws281x::{ChannelBuilder, Controller, ControllerBuilder};

/// Displays implementation for real hardware.
/// Backed by a 32x16 LED matrix and a strip of NeoPixels.
pub struct LedDisplays {
    strip: Controller,
    matrix: LedMatrix,
    offscreen: Option<LedCanvas>,
}

struct OffscreenCanvas<'a> {
    canvas: &'a mut LedCanvas,
}

impl Dimensions for OffscreenCanvas<'_> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        self.canvas.bounding_box()
    }
}

impl DrawTarget for OffscreenCanvas<'_> {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        self.canvas.draw_iter(pixels)
    }
}

impl Displays for LedDisplays {
    fn edge(&mut self) -> &mut [NeoPixelColor] {
        self.strip.leds_mut(0)
    }

    fn face(
        &mut self,
    ) -> impl embedded_graphics_core::draw_target::DrawTarget<Color = Rgb888, Error = Infallible>
    {
        if self.offscreen.is_none() {
            self.offscreen = Some(self.matrix.offscreen_canvas());
        }
        OffscreenCanvas {
            canvas: self.offscreen.as_mut().unwrap(),
        }
    }

    fn flush(&mut self) -> Result<(), String> {
        if let Some(offscreen) = self.offscreen.take() {
            self.offscreen = Some({
                let mut off = self.matrix.swap(offscreen);
                off.clear();
                off
            });
        }
        self.strip.render().map_err(|e| e.to_string())
    }
}

impl Drop for LedDisplays {
    fn drop(&mut self) {
        for px in self.strip.leds_mut(0) {
            *px = [0, 0, 0, 0];
        }
        let _ = self.flush();
    }
}

impl LedDisplays {
    const STRIP_SIZE: i32 = 60;

    /// Create a new handler for hardware LED displays.
    pub fn new() -> Result<Self, String> {
        let strip = Self::new_controller()?;
        let matrix = Self::new_matrix()?;
        Ok(Self {
            strip,
            matrix,
            offscreen: None,
        })
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
