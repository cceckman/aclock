use std::{convert::Infallible, mem::swap, str::FromStr};

use chrono::{DateTime, FixedOffset, MappedLocalTime, NaiveDateTime, TimeDelta, TimeZone, Utc};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{DrawTarget, DrawTargetExt, OriginDimensions, Point, RgbColor, Size},
    primitives::Rectangle,
    Pixel,
};
/// Set up logging for the WASM simulator.
use log::MakeConsoleWriter;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement};

use crate::{
    atmosphere::{AtmosphereSample, AtmosphereSampler},
    drawing::render_edge,
    Displays, NeoPixelColor, Renderer, RendererSettings,
};

const DEFAULT_SIZE: Size = Size::new(32, 16);

/// Inputs from the browser context.
///
/// This is not public because we can't (apparently) pass JS elements from WASM objects.
/// Selectors for HTML elements.
/// We can't (apparently) store JS elements in WASM objects.
struct WebInputs {
    time: HtmlInputElement,
    tz: HtmlInputElement,
    co2: Option<HtmlInputElement>,
    temperature: Option<HtmlInputElement>,
    humidity: Option<HtmlInputElement>,
}

impl WebInputs {
    /// Read the current time.
    fn get_time(&self) -> Result<DateTime<FixedOffset>, String> {
        // NaiveDateTime takes seconds, datetime-local provides 1-minute granularity
        let naive = NaiveDateTime::from_str(&(self.time.value() + ":00")).map_err(|e| {
            format!(
                "error in timestamp {}: format error: {}",
                self.time.value(),
                e
            )
        })?;
        let tz = self.tz.value();
        let tz_offset: TimeDelta = {
            let int_err = |e| format!("invalid timezone offset {}: {}", tz, e);
            let str_err = |e| format!("invalid timezone offset {}: {}", tz, e);

            if let Some((tz_h, tz_m)) = tz.split_once(":") {
                let tz_h: i64 = tz_h.parse().map_err(int_err)?;
                let tz_m: i64 = tz_m.parse().map_err(int_err)?;
                let tz_m = if tz_h.signum() != 0 {
                    tz_m * tz_h.signum()
                } else {
                    tz_m
                };
                let tz_h =
                    TimeDelta::try_hours(tz_h).ok_or_else(|| str_err("overflow in hours"))?;
                let tz_m =
                    TimeDelta::try_hours(tz_m).ok_or_else(|| str_err("overflow in minutes"))?;
                tz_h + tz_m
            } else {
                let off: i64 = tz.parse().map_err(int_err)?;
                TimeDelta::try_hours(off).ok_or_else(|| str_err("overflow in hours"))?
            }
        };
        let tz = FixedOffset::east_opt(tz_offset.num_seconds() as i32)
            .ok_or_else(|| "invalid timezone offset ".to_owned() + &tz)?;
        match tz.from_local_datetime(&naive) {
            MappedLocalTime::Single(local) => Ok(local),
            MappedLocalTime::Ambiguous(a, b) => {
                tracing::info!("in mapping imestamp {} with UTC offset {} to absolute (UTC) time: could be {} or {}, assuming the former", self.time.value(), tz, a, b);
                Ok(b)
            }
            MappedLocalTime::None => {
                let msg = format!("in mapping imestamp {} with UTC offset {} to absolute (UTC) time: no possible result!", self.time.value(), tz);
                tracing::error!(msg);
                Err(msg)
            }
        }
    }
}

impl AtmosphereSampler for WebInputs {
    fn sample(&mut self) -> AtmosphereSample {
        let timestamp = self.get_time().unwrap_or_default();
        let parse = |v: &HtmlInputElement| v.value().parse().ok();
        let co2_ppm: Option<f32> = self.co2.as_ref().and_then(parse);
        let temperature: Option<f32> = self.temperature.as_ref().and_then(parse);
        let relative_humidity: Option<f32> = self.humidity.as_ref().and_then(parse);

        AtmosphereSample {
            timestamp: timestamp.to_utc(),
            co2_ppm,
            temperature,
            relative_humidity,
        }
    }
}

/// Renderer that renders to a web display.
#[wasm_bindgen]
#[allow(unused)]
pub struct WebRenderer {
    renderer: Renderer,
}

#[wasm_bindgen]
impl WebRenderer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WebRenderer {
        WebRenderer {
            renderer: RendererSettings::default().into(),
        }
    }
}

#[wasm_bindgen]
impl WebRenderer {
    // TODO: Allow mutating location
    #[allow(
        clippy::too_many_arguments,
        reason = "passing a lot of items over ABI boundary; temporary"
    )]
    pub fn update(
        &mut self,
        canvas: HtmlCanvasElement,
        time: HtmlInputElement,
        tz: HtmlInputElement,
        scale: HtmlInputElement,
        co2: Option<HtmlInputElement>,
        temperature: Option<HtmlInputElement>,
        humidity: Option<HtmlInputElement>,
    ) -> Result<(), String> {
        let mut inputs = WebInputs {
            time,
            tz,
            co2,
            temperature,
            humidity,
        };
        let canvas = CanvasTarget::new(scale, canvas)?;
        let mut displays = WebDisplays::new(canvas);
        let time = inputs.get_time()?;

        self.renderer.render(&mut displays, &mut inputs, time);

        Ok(())
    }
}

#[wasm_bindgen(start)]
fn run() {
    // Does not have a time accessible in the wasm environment.
    tracing_subscriber::fmt::fmt()
        .with_writer(MakeConsoleWriter)
        .without_time()
        .with_ansi(false)
        .init();
}

/// DrawTarget implementation for a web canvas.
///
/// TODO: Draw edge pixels as arcs rather than squares.
/// We don't have the embedded-graphics constraints on web.
struct CanvasTarget {
    size: Size,
    scale: i32,
    canvas: HtmlCanvasElement,
}

impl CanvasTarget {
    fn new(scale: HtmlInputElement, canvas: HtmlCanvasElement) -> Result<Self, String> {
        let scale: u32 = scale
            .value()
            .parse()
            .map_err(|e| format!("invalid scale: {}", e))?;
        if !(1..4096).contains(&scale) {
            // Arbitrary choice of max.
            return Err(format!("invalid scale (out of range): {}", scale));
        }
        Ok(CanvasTarget {
            scale: scale as i32,
            size: DEFAULT_SIZE + Size::new(4, 4),
            canvas,
        })
    }
}

impl OriginDimensions for CanvasTarget {
    fn size(&self) -> Size {
        self.size
    }
}

impl DrawTarget for CanvasTarget {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        tracing::info!("drawing to canvas");
        let canvas = self
            .canvas
            .get_context("2d")
            .expect("failed to get canvas rendering context")
            .expect("unwraped empty canvas rendering object")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("canvas rendering context was not the expected type");
        // Keep a 1-slot cache of color strings, as we're likely to get swaths of the same color
        // repeatedly.
        let mut last_fill_color: Rgb888 = Rgb888::new(0, 0, 0);
        let mut last_fill_style: String = fill_color(last_fill_color);

        for Pixel(pt, color) in pixels.into_iter() {
            if last_fill_color != color {
                last_fill_style = fill_color(color);
                last_fill_color = color;
            }
            canvas.set_fill_style_str(&last_fill_style);
            canvas.fill_rect(
                (pt.x * self.scale) as f64,
                (pt.y * self.scale) as f64,
                self.scale as f64,
                self.scale as f64,
            );
        }
        tracing::info!("drawing to canvas");
        Ok(())
    }
}

/// Displays implementation for a 2d canvas
struct WebDisplays {
    canvas: CanvasTarget,
    display: Vec<Pixel<Rgb888>>,
    edge: Vec<NeoPixelColor>,
}

impl WebDisplays {
    fn new(canvas: CanvasTarget) -> Self {
        // TODO: Render the central matrix as dots (circles, with space).
        // TODO: Render the edge as arcs.
        // OK to emulate the SDL version for now.
        // 2 pixels on each edge; get the perimeter
        let perimeter = ((32 + 4) + (16 + 4)) * 2;
        let mut edge = Vec::new();
        edge.resize(perimeter, [0, 0, 0, 0]);
        WebDisplays {
            canvas,
            display: Vec::new(),
            edge,
        }
    }
}

impl OriginDimensions for &mut WebDisplays {
    fn size(&self) -> Size {
        Size::new(32, 16)
    }
}

/// Get a fillStyle value for a given color.
fn fill_color(color: Rgb888) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

impl DrawTarget for &mut WebDisplays {
    type Color = Rgb888;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        tracing::trace!("drawing to inner matrix");
        self.display.extend(pixels);

        Ok(())
    }
}

impl Displays for WebDisplays {
    fn edge(&mut self) -> &mut [crate::NeoPixelColor] {
        &mut self.edge
    }

    fn face(
        &mut self,
    ) -> impl embedded_graphics_core::draw_target::DrawTarget<
        Color = embedded_graphics::pixelcolor::Rgb888,
        Error = std::convert::Infallible,
    > {
        self
    }

    fn flush(&mut self) -> Result<(), String> {
        tracing::trace!("clearning canvas");
        self.canvas.clear(Rgb888::new(0, 0, 0)).expect("infallible");

        // Draw the border:
        render_edge(&self.edge, &mut self.canvas);

        // Draw the middle:
        let pixels = {
            let mut alt = Vec::new();
            swap(&mut alt, &mut self.display);
            alt
        };

        // Cropped translates; clipped ensures that OOB writes get dropped.
        let mut crop = self
            .canvas
            .cropped(&Rectangle::new(Point::new(2, 2), DEFAULT_SIZE));
        let mut clip = crop.clipped(&Rectangle::new(Point::new(0, 0), DEFAULT_SIZE));
        clip.draw_iter(pixels).expect("infallible");

        Ok(())
    }
}

mod log {

    use tracing_subscriber::fmt::MakeWriter;
    use wasm_bindgen::JsValue;
    /// Makes a writer to the web_sys console.
    pub struct MakeConsoleWriter;

    impl MakeWriter<'_> for MakeConsoleWriter {
        type Writer = MakeConsoleWriter;

        fn make_writer(&'_ self) -> Self::Writer {
            MakeConsoleWriter
        }
    }

    impl std::io::Write for MakeConsoleWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let s = core::str::from_utf8(buf)
                .map(JsValue::from_str)
                .ok()
                .unwrap_or_else(|| {
                    JsValue::from_str(&format!("non-string log message: {:?}", buf))
                });
            let a = js_sys::Array::new_with_length(1);
            a.set(0, s);

            web_sys::console::log(&a);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
