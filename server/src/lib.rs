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

#[cfg(feature = "web")]
pub mod web;

pub mod context;
pub mod riseset;

#[cfg(feature = "simulator")]
pub mod simulator;

pub mod atmosphere;

pub(crate) mod drawing;

#[cfg(feature = "hardware")]
pub mod led_displays;

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

use atmosphere::{AtmosphereSampler, LastMeasurement};
use chrono::{DateTime, Datelike, Timelike};
use embedded_graphics::{
    draw_target::{DrawTarget, DrawTargetExt},
    pixelcolor::Rgb888,
};

/// Alias for a color of NeoPixel.
pub type NeoPixelColor = [u8; 4];

/// Displays are the output of the clock.
///
/// Notionally, the displays are (a) a 32x16 LED matrix, and (b) a strip of NeoPixels of indefinite length.
/// In practice, we provide support for a virtualized display as well, for testing and development.
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
#[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Copy, Clone)]
pub struct RendererSettings {
    /// Minimum edge pixel brightness during daylight
    pub min_daylight: f32,
    /// Maximum edge pixel brightness during night
    pub max_nightlight: f32,
    /// Latitude for sun position calculation
    pub latitude: f32,
    /// Longitude for sun position calculation
    pub longitude: f32,

    /// How many cycles (frames) to display each piece of auxiliary data.
    pub display_cycles: usize,

    /// Color to use for rendering the matrix.
    pub matrix_r: u8,
    pub matrix_g: u8,
    pub matrix_b: u8,
}

/// State of a renderer.
pub struct Renderer {
    settings: RendererSettings,
    display_cycle: usize,

    last_co2_ppm: Option<LastMeasurement>,
    last_temperature: Option<LastMeasurement>,
    last_relative_humidity: Option<LastMeasurement>,
}

impl From<RendererSettings> for Renderer {
    fn from(value: RendererSettings) -> Self {
        Renderer {
            settings: value,
            display_cycle: 0,
            last_relative_humidity: None,
            last_co2_ppm: None,
            last_temperature: None,
        }
    }
}

#[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen)]
impl RendererSettings {
    #[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self::default()
    }
}

impl RendererSettings {
    pub fn with_color(self, color: Rgb888) -> Self {
        Self {
            matrix_r: color.r(),
            matrix_g: color.g(),
            matrix_b: color.b(),
            ..self
        }
    }
}

impl Default for RendererSettings {
    fn default() -> Self {
        const DEFAULT_MIN_DAYLIGHT: f32 = 0.2;
        // Approximate location of Washington, DC
        Self {
            min_daylight: DEFAULT_MIN_DAYLIGHT,
            max_nightlight: DEFAULT_MIN_DAYLIGHT * 1.8,
            latitude: 39.0,
            longitude: -77.0,
            display_cycles: 60,
            matrix_r: 255,
            matrix_g: 255,
            matrix_b: 255,
        }
    }
}

impl Renderer {
    /// Access the settings, for update purposes.
    pub fn settings_mut(&mut self) -> &mut RendererSettings {
        &mut self.settings
    }

    /// Update the displays with the current data.
    pub fn render<Tz, D, A>(&mut self, displays: &mut D, atmosphere: &mut A, now: DateTime<Tz>)
    where
        Tz: chrono::TimeZone,
        D: Displays,
        A: AtmosphereSampler,
    {
        tracing::debug!("rendering edge");
        self.render_edge(displays, now.clone());
        tracing::debug!("rendering face");
        self.render_face(displays, atmosphere, now.clone());
        tracing::debug!("flushing displays");
        displays.flush().expect("failed to render to output");
        tracing::debug!("completed frame");
    }

    fn matrix_color(&self) -> Rgb888 {
        Rgb888::new(
            self.settings.matrix_r,
            self.settings.matrix_g,
            self.settings.matrix_b,
        )
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
        let (rise, _noon, set) = riseset::riseset(
            now.date_naive(),
            self.settings.latitude,
            self.settings.longitude,
            now.timezone(),
        );

        // Convert both of them to coordinates around the face.
        let [rise, set] = [rise, set].map(|v: DateTime<Tz>| {
            let time = v.time();
            tracing::trace!("local: {}", time);
            let h = time.hour();
            let m = time.minute();
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
                let f = self.settings.min_daylight + sin * (1.0 - self.settings.min_daylight);

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
                let f = self.settings.max_nightlight - (self.settings.max_nightlight * sin);
                let amt = (f * 255.0).clamp(0.0, 255.0) as u8;
                tracing::trace!(
                    "point {i:03}: night fraction {night_fraction:.2}, sin {sin:.2}, amt {amt:0}",
                );
                // Night is only blue, for now.
                *px = [0, 0, amt, 0];
            }
        }
    }

    fn update_atmo<A>(&mut self, atmosphere: &mut A)
    where
        A: AtmosphereSampler,
    {
        let s = atmosphere.sample();
        LastMeasurement::update(&mut self.last_co2_ppm, s.timestamp, s.co2_ppm);
        LastMeasurement::update(
            &mut self.last_relative_humidity,
            s.timestamp,
            s.relative_humidity,
        );
        LastMeasurement::update(&mut self.last_temperature, s.timestamp, s.temperature);
    }

    fn render_face<Tz, D, A>(&mut self, displays: &mut D, atmosphere: &mut A, time: DateTime<Tz>)
    where
        Tz: chrono::TimeZone,
        D: Displays,
        A: AtmosphereSampler,
    {
        let minute = time.minute();
        let hour = time.hour();
        let time_str = format!("{hour:02}:{minute:02}");

        let mut canvas = displays.face();
        Rectangle::new(Point::new(0, 0), Size::new(32, 16))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb888::BLACK)
                    .build(),
            )
            .draw(&mut canvas)
            .expect("infallible");

        // The time always goes into the upper half of the display;
        // auxiliary data into the bottom.
        {
            let time_style = MonoTextStyle::new(&FONT_6X9, self.matrix_color());
            let style = TextStyleBuilder::new()
                .alignment(Alignment::Center)
                .baseline(Baseline::Top)
                .build();
            Text::with_text_style(&time_str, Point::new(15, 0), time_style, style)
                .draw(&mut canvas)
                .expect("infallible");
        }
        let aux_size = Size::new(32, 7);
        let mut aux_crop = canvas.cropped(&Rectangle::new(Point::new(0, 9), aux_size));
        let mut aux = aux_crop.clipped(&Rectangle::new(Point::new(0, 0), aux_size));

        self.update_atmo(atmosphere);
        if !self.render_atmo(&mut aux) {
            // Fall back to rendering date
            self.render_date(&mut aux, time);
        }
    }

    /// Render the date into the provided space.
    fn render_date<Tz>(
        &self,
        canvas: &mut impl DrawTarget<Color = Rgb888, Error = Infallible>,
        time: DateTime<Tz>,
    ) where
        Tz: chrono::TimeZone,
    {
        let date = format!(
            "{:02}{}{:02}",
            time.day(),
            month_en3(time.month()),
            time.year() % 100
        );
        let date_style = MonoTextStyle::new(&FONT_4X6, self.matrix_color());
        let style = TextStyleBuilder::new()
            .alignment(Alignment::Right)
            .baseline(Baseline::Top)
            .build();

        Text::with_text_style(&date, Point::new(31, 0), date_style, style)
            .draw(canvas)
            .expect("infallible");
    }

    fn render_atmo(&self, aux: &mut impl DrawTarget<Color = Rgb888, Error = Infallible>) -> bool {
        if let (Some(temp), Some(humid), Some(co2)) = (
            self.last_temperature,
            self.last_relative_humidity,
            self.last_co2_ppm,
        ) {
            // In a 4x6 font, we have (32/4=) 8 characters to work with.
            // 3 for temperature (NNC), four for CO2 (NNNN),
            // and one in the middle for a box-drawing character of humidity.
            const BAR: &[char] = &[' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

            // Consider 90% "full", this is supposed to be indoors.
            let rh_symbol = f32::floor(humid.value * (BAR.len() as f32) / 90.0) as usize;
            let v = BAR[rh_symbol];

            let s = format!("{:<2.0}C{v}{:>4.0}", temp.value, co2.value);
            let temp_style = MonoTextStyle::new(&FONT_4X6, self.matrix_color());
            let style = TextStyleBuilder::new()
                .alignment(Alignment::Right)
                .baseline(Baseline::Top)
                .build();

            Text::with_text_style(&s, Point::new(31, 0), temp_style, style)
                .draw(aux)
                .expect("infallible");

            true
        } else {
            false
        }
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
