//! Routine for computing neopixel brightnesses.
use std::f32::consts::PI;

use chrono::{DateTime, Local};
use chrono::{Timelike, Utc};
use rs_ws281x::Controller;
use satkit::{lpephem::sun::riseset, AstroTime, ITRFCoord};
use simulator::SimEdge;

use crate::Edge;

const MIN_DAYLIGHT: f32 = 0.05;

const STRIP_SIZE: usize = 60; // 1 meter at 60/meter

/// Alias for a color of NeoPixel.
pub type NeoPixelColor = [u8; 4];

pub fn get_edge() -> Result<impl Edge, String> {
    #[cfg(feature = "simulator")]
    {
        Ok(SimEdge::new())
    }
    #[cfg(not(feature = "simulator"))]
    {
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

        Ok(controller)
    }
}

impl Edge for Controller {
    fn data(&mut self) -> &mut [self::NeoPixelColor] {
        self.leds_mut(0)
    }

    fn flush(&mut self) -> Result<(), String> {
        self.render().map_err(|e| e.to_string())
    }
}

#[cfg(feature = "simulator")]
mod simulator {
    use embedded_graphics::{
        draw_target::DrawTarget,
        geometry::{Point, Size},
        pixelcolor::Rgb888,
        Pixel,
    };
    use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};

    use crate::Edge;

    use super::{NeoPixelColor, STRIP_SIZE};

    pub struct SimEdge {
        display: SimulatorDisplay<Rgb888>,
        window: Window,
        data: [NeoPixelColor; STRIP_SIZE],
    }

    impl SimEdge {
        pub fn new() -> Self {
            let display = SimulatorDisplay::new(Size::new(STRIP_SIZE.try_into().unwrap(), 1));

            let settings = OutputSettingsBuilder::new().scale(20).build();

            let window = Window::new("Strip", &settings);
            SimEdge {
                window,
                display,
                data: [Default::default(); 60],
            }
        }
    }

    impl Edge for SimEdge {
        fn data(&mut self) -> &mut [super::NeoPixelColor] {
            &mut self.data
        }

        fn flush(&mut self) -> Result<(), String> {
            let points = self
                .data
                .iter()
                .map(|color| {
                    let [r, g, b, _w] = *color;
                    // TODO: Incorporate W channel
                    Rgb888::new(r, g, b)
                })
                .enumerate()
                .map(|(i, c)| Pixel(Point { x: i as i32, y: 0 }, c));
            self.display.draw_iter(points).expect("infallible");
            self.window.update(&self.display);
            Ok(())
        }
    }
}

/// Compute the pixel colors for the given date.
/// (The time component is ignored.)
pub fn get_pixels(time: DateTime<Local>, edge: &mut dyn Edge) -> Result<(), String> {
    let output = edge.data();
    let len = output.len();
    for (i, px) in output.iter_mut().enumerate() {
        let v = ((i * 255) / len).clamp(0, 255) as u8;
        *px = [v, v, v, v];
    }

    let astro = AstroTime::from_unixtime(time.to_utc().timestamp() as f64);
    // Wilmington, DE
    let coord = ITRFCoord::from_geodetic_deg(39.7447, -75.539787, 28.0);
    // "standard" rise and set are slightly off 90 degrees; ignoring that for now.
    let (a, b) = riseset(&astro, &coord, Some(90.0)).map_err(|err| format!("{}", err))?;

    // The above function returns the next two times the sun hits the horizon,
    // but they may be (rise, set) or (set, rise) depending on the specified time.

    tracing::trace!("next: {} after: {}", a, b);
    // Convert both of them to coordinates around the face.
    let [a, b]: [f32; 2] = [a, b]
        .map(|v| {
            DateTime::from_timestamp(v.to_unixtime() as i64, 0).expect("could not convert datetime")
        })
        .map(|v: DateTime<Utc>| {
            let time = v.with_timezone(&Local).time();
            tracing::trace!("local: {}", time);
            let h = time.hour();
            let m = time.minute();
            // Convert to a fraction of the day, at a minute granualirty.
            (h * 60 + m) as f32 / (24 * 60) as f32
        });
    let (rise, set) = if a > b { (b, a) } else { (a, b) };

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
            let f = MIN_DAYLIGHT + sin * (1.0 - MIN_DAYLIGHT);

            // Then re-range to 0..=255.
            let amt = (f * 255.0).clamp(0.0, 255.0) as u8;
            tracing::trace!(
                "point {i:02}:   day fraction {day_fraction:.2}, sin {sin:.2}, amt {amt:0}",
            );
            *px = [0, 0, 0, amt];
        } else {
            // Normalize to "tomorrow night"
            let night_point = if date_fraction < rise {
                date_fraction + 1.0
            } else {
                date_fraction
            };
            let night_fraction = (night_point - set) / ((rise + 1.0) - set);
            let sin = (night_fraction * PI).sin();
            // and subtract that out from the daylight:
            let f = MIN_DAYLIGHT - (MIN_DAYLIGHT * sin);
            let amt = (f * 255.0).clamp(0.0, 255.0) as u8;
            tracing::trace!(
                "point {i:02}: night fraction {night_fraction:.2}, sin {sin:.2}, amt {amt:0}",
            );
            // Night is only blue, for now.
            *px = [0, 0, amt, 0];
        }
    }

    edge.flush()
}
