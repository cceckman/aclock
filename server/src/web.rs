use std::{convert::Infallible, f64::consts::PI, str::FromStr};

use chrono::{DateTime, FixedOffset, MappedLocalTime, NaiveDateTime, TimeDelta, TimeZone};
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{DrawTarget, OriginDimensions, RgbColor, Size},
    Pixel,
};
/// Set up logging for the WASM simulator.
use log::MakeConsoleWriter;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement};

use crate::{
    atmosphere::{AtmosphereSample, AtmosphereSampler},
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

impl Default for WebRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WebRenderer {
    #[allow(
        clippy::too_many_arguments,
        reason = "items are otherwise not passable over ABI boundary"
    )]
    pub fn update(
        &mut self,
        canvas: HtmlCanvasElement,
        time: HtmlInputElement,
        tz: HtmlInputElement,
        scale: HtmlInputElement,
        latitude: HtmlInputElement,
        longitude: HtmlInputElement,
        co2: Option<HtmlInputElement>,
        temperature: Option<HtmlInputElement>,
        humidity: Option<HtmlInputElement>,
    ) -> Result<(), String> {
        let latitude = latitude.value_as_number();
        let longitude = longitude.value_as_number();
        if (-90.0..90.0).contains(&latitude) {
            self.renderer.settings().latitude = latitude as f32;
        }
        if (-180.0..180.0).contains(&latitude) {
            self.renderer.settings().longitude = longitude as f32;
        }

        let mut inputs = WebInputs {
            time,
            tz,
            co2,
            temperature,
            humidity,
        };
        let mut displays = WebDisplays::new(scale, canvas)
            .inspect_err(|e| tracing::error!("error in setting up displays: {}", e))?;
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

/// Displays implementation for a 2d canvas
struct WebDisplays {
    canvas: HtmlCanvasElement,
    scale: u32,
    display: Vec<Pixel<Rgb888>>,
    edge: Vec<NeoPixelColor>,
}

impl WebDisplays {
    fn new(scale: HtmlInputElement, canvas: HtmlCanvasElement) -> Result<Self, String> {
        let scale = scale.value_as_number();
        // Must be at least 4 to allow spacing between matrix pixels.
        if !(4.0..4096.0).contains(&scale) {
            return Err(format!("invalid scale: {}", scale));
        }
        let scale = scale as u32;

        // TODO: Render the central matrix as dots (circles, with space).
        // TODO: Render the edge as arcs.
        // OK to emulate the SDL version for now.
        // 2 pixels on each edge; get the perimeter
        let perimeter = 60;
        let mut edge = Vec::new();
        edge.resize(perimeter, [0, 0, 0, 0]);
        Ok(WebDisplays {
            canvas,
            scale,
            display: Vec::new(),
            edge,
        })
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
        // Our drawing area is a nominal NxN with the matrix centered.
        const SIM_MATRIX: u32 = 50;
        let d = SIM_MATRIX * self.scale;
        self.canvas.set_width(d);
        self.canvas.set_height(d);
        let center = d as f64 / 2.0;

        let ctx = self
            .canvas
            .get_context("2d")
            .map_err(|e| format!("could not obtain canvas context: {:?}", e))?
            .ok_or_else(|| "obtained empty 2d context".to_owned())?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|e| format!("canvas rendering context was not a 2d context: {:?}", e))?;

        ctx.clear_rect(0.0, 0.0, d as f64, d as f64);

        let face_radius = ((SIM_MATRIX - 10) * self.scale) as f64 / 2.0;
        {
            let radius = d as f64 / 2.0;
            // Draw the edge display:
            let arc_size = 2.0 * PI / self.edge.len() as f64;
            // Javascript by default measures arcs in clockwise radians? Eh?
            const DOWN: f64 = PI / 2.0;
            for (i, it) in self.edge.iter().enumerate() {
                let start_angle = DOWN + (i as f64 * arc_size);
                let end_angle = start_angle + arc_size;
                let mid_angle = (start_angle + end_angle) / 2.0;

                let (x_outer, y_outer) = (
                    center + mid_angle.cos() * radius,
                    center + mid_angle.sin() * radius,
                );
                let (x_inner, y_inner) = (
                    center + mid_angle.cos() * face_radius,
                    center + mid_angle.sin() * face_radius,
                );

                let gradient = ctx.create_linear_gradient(x_inner, y_inner, x_outer, y_outer);
                gradient
                    .add_color_stop(0.0, &fill_color(Rgb888::new(it[0], it[1], it[2])))
                    .map_err(|e| format!("failed to stop gradient: {e:?}"))?;
                gradient
                    .add_color_stop(1.0, "black")
                    .map_err(|e| format!("failed to stop gradient: {e:?}"))?;

                // let fill = fill_color(Rgb888::WHITE);
                // Begins a new path
                ctx.begin_path();
                ctx.move_to(center, center);
                ctx.ellipse(center, center, radius, radius, 0.0, start_angle, end_angle)
                    .map_err(|e| format!("could not draw edge arc: {e:?}"))?;
                ctx.move_to(center, center);
                ctx.close_path();
                ctx.set_fill_style_canvas_gradient(&gradient);
                ctx.fill();
            }
        }
        // Draw an inner arc to mask off the face.
        {
            ctx.begin_path();
            ctx.set_fill_style_str(&fill_color(Rgb888::BLACK));
            ctx.ellipse(center, center, face_radius, face_radius, 0.0, 0.0, 2.0 * PI)
                .map_err(|e| format!("could not draw center mask: {e:?}"))?;
            ctx.close_path();
            ctx.fill();
        }
        {
            // Finally, draw each pixel in the matrix.
            // We extend the matrix out to the full dimensions,
            // and here compute the edges.
            let matrix_offset_top = (SIM_MATRIX - DEFAULT_SIZE.height) / 2;
            let matrix_offset_left = (SIM_MATRIX - DEFAULT_SIZE.width) / 2;
            // Radius must be at least 1.
            let r = std::cmp::max(self.scale / 4, 1) as f64;
            // Since most colors will be the same, we only update the fill color if it changes.
            let mut last_color = Rgb888::BLACK;
            for Pixel(pt, color) in self.display.drain(0..) {
                let (x, y) = (
                    pt.x as u32 + matrix_offset_left,
                    pt.y as u32 + matrix_offset_top,
                );
                let (x, y) = (x * self.scale, y * self.scale);
                let (x, y) = (x as f64, y as f64);
                ctx.begin_path();
                ctx.move_to(x, y);
                ctx.arc(x, y, r, 0.0, 2.0 * PI)
                    .map_err(|e| format!("could not draw matrix pixel: {e:?}"))?;
                ctx.close_path();

                if color != last_color {
                    ctx.set_fill_style_str(&fill_color(color));
                    last_color = color;
                }
                ctx.fill();
            }
        }

        tracing::trace!("done drawing frame");

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
