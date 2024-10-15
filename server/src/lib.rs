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
use std::{convert::Infallible, f32::consts::PI};

pub mod context;
pub mod riseset;

#[cfg(feature = "simulator")]
pub mod simulator;

pub mod led_displays;

use chrono::{DateTime, Datelike, Timelike};
use embedded_graphics::pixelcolor::Rgb888;

/// Alias for a color of NeoPixel.
pub type NeoPixelColor = [u8; 4];

/// Displays are the output of the clock
/// Notionally, these are a 32x16 LED matrix, and a strip of NeoPixels of indefinite length;
/// in practice, we provide support for a virtualized display as well, for testing and development.
pub trait Displays {
    /// Access to the edge data
    fn edge(&mut self) -> &mut [NeoPixelColor];

    /// Access to the face display
    fn face(
        &mut self,
    ) -> impl embedded_graphics_core::draw_target::DrawTarget<Color = Rgb888, Error = Infallible>;

    /// Flush any pending pixels (i.e. v-sync)
    fn flush(&mut self) -> Result<(), String>;
}

/// Provides the core rendering setting(s).
pub struct Renderer {
    /// Minimum edge pixel brightness during daylight
    pub min_daylight: f32,
    /// Maximum edge pixel brightness during night
    pub max_nightlight: f32,
    /// Latitude for sun position calculation
    pub latitude: f32,
    /// Longitude for sun position calculation
    pub longitude: f32,
}

impl Default for Renderer {
    fn default() -> Self {
        const DEFAULT_MIN_DAYLIGHT: f32 = 0.2;
        // Approximate location of Washington, DC
        Self {
            min_daylight: DEFAULT_MIN_DAYLIGHT,
            max_nightlight: DEFAULT_MIN_DAYLIGHT * 1.8,
            latitude: 39.0,
            longitude: -77.0,
        }
    }
}

impl Renderer {
    /// Update the displays with the current time.
    pub fn render<Tz, D>(&self, displays: &mut D, now: DateTime<Tz>)
    where
        Tz: chrono::TimeZone,
        D: Displays,
    {
        self.render_edge(displays, now.clone());
        self.render_face(displays, now.clone());
        displays.flush().expect("failed to render to output");
    }

    fn render_edge<Tz, D>(&self, displays: &mut D, now: DateTime<Tz>)
    where
        Tz: chrono::TimeZone,
        D: Displays,
    {
        let output = displays.edge();
        // Test version:
        //let len = output.len();
        //for (i, px) in output.iter_mut().enumerate() {
        //    let v = ((i * 255) / len).clamp(0, 255) as u8;
        //    *px = [v, v, v, v];
        //}

        // Wilmington, DE
        let (rise, _noon, set) = riseset::riseset(now, self.latitude, self.longitude);

        // Convert both of them to coordinates around the face.
        let [rise, set] = [rise, set].map(|v: DateTime<Tz>| {
            let time = v.time();
            tracing::trace!("local: {}", time);
            let h = time.hour();
            let m = time.hour();
            // Convert to a fraction of the day, at a minute granualirty.
            (h * 60 + m) as f32 / (24 * 60) as f32
        });

        let daylight = set - rise;

        let len = output.len() as f32;
        for (i, px) in output.iter_mut().enumerate() {
            // The [0, 1)-bounded fraction of the day this point is at.
            let date_fraction = i as f32 / len;
            // What fraction of _daylight_ has passed at this point?
            // (May be negative or greater than 1)
            let day_fraction = (date_fraction - rise) / daylight;
            if (0.0..=1.0).contains(&day_fraction) {
                // During daylight hours.
                // Make a nice curve via sin:
                let sin = (day_fraction * PI).sin();
                // But then make sure it meets a minimum brightness:
                let f = self.min_daylight + sin * (1.0 - self.min_daylight);

                // Then re-range to 0..=255.
                let amt = (f * 255.0).clamp(0.0, 255.0) as u8;
                tracing::trace!(
                    "point {i:03}:   day fraction {day_fraction:.2}, sin {sin:.2}, amt {amt:0}",
                );
                // TODO: Using RGB so it shows up on the simulator.
                // How do we use / render W channel?
                *px = [amt, amt, amt, amt];
            } else {
                // Normalize to "tomorrow night"
                let night_point = if date_fraction < rise {
                    date_fraction + 1.0
                } else {
                    date_fraction
                };
                let night_fraction = (night_point - set) / ((rise + 1.0) - set);
                let sin = (night_fraction * PI).sin();
                // and subtract that out from the maximum:
                let f = self.max_nightlight - (self.max_nightlight * sin);
                let amt = (f * 255.0).clamp(0.0, 255.0) as u8;
                tracing::trace!(
                    "point {i:03}: night fraction {night_fraction:.2}, sin {sin:.2}, amt {amt:0}",
                );
                // Night is only blue, for now.
                *px = [0, 0, amt, 0];
            }
        }
    }

    fn render_face<Tz, D>(&self, displays: &mut D, time: DateTime<Tz>)
    where
        Tz: chrono::TimeZone,
        D: Displays,
    {
        use embedded_graphics::{
            geometry::{Point, Size},
            mono_font::{
                ascii::{FONT_4X6, FONT_6X9},
                MonoTextStyle,
            },
            pixelcolor::RgbColor,
            primitives::{Primitive, PrimitiveStyleBuilder, Rectangle},
            text::{Alignment, Baseline, Text, TextStyleBuilder},
            Drawable,
        };

        let minute = time.minute();
        let hour = time.hour();
        let day = time.day();
        let month = month_en3(time.month());
        let year = time.year() % 100;
        let time = format!("{hour:02}:{minute:02}");
        let date = format!("{day:02}{month}{year:02}");

        let mut canvas = displays.face();
        Rectangle::new(Point::new(0, 0), Size::new(32, 16))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb888::BLACK)
                    .build(),
            )
            .draw(&mut canvas)
            .expect("infallible");

        let time_style = MonoTextStyle::new(&FONT_6X9, Rgb888::WHITE);
        let date_style = MonoTextStyle::new(&FONT_4X6, Rgb888::WHITE);
        let style = TextStyleBuilder::new()
            .alignment(Alignment::Center)
            .baseline(Baseline::Top)
            .build();

        Text::with_text_style(&time, Point::new(15, 0), time_style, style)
            .draw(&mut canvas)
            .expect("infallible");
        Text::with_text_style(&date, Point::new(15, 11), date_style, style)
            .draw(&mut canvas)
            .expect("infallible");
    }
}

/// Enblish 3-character month abbreviations.
fn month_en3(number: u32) -> &'static str {
    match number {
        1 => "JAN",
        2 => "FEB",
        3 => "MAR",
        4 => "APR",
        5 => "MAY",
        6 => "JUN",
        7 => "JUL",
        8 => "AUG",
        9 => "SEP",
        10 => "OCT",
        11 => "NOV",
        12 => "DEC",
        _ => "???",
    }
}
