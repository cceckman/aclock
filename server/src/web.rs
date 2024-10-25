use std::{convert::Infallible, str::FromStr};

use chrono::NaiveDateTime;
use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{DrawTarget, DrawTargetExt, OriginDimensions, Point, RgbColor, Size},
    primitives::Rectangle,
    Pixel,
};
/// Set up logging for the WASM simulator.
use log::MakeConsoleWriter;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use crate::{
    atmosphere::{AtmosphereSample, FakeAtmosphereSampler},
    drawing::render_edge,
    Displays, NeoPixelColor, Renderer, RendererSettings,
};

/// Renderer that renders to a web display.
#[wasm_bindgen]
#[allow(unused)]
pub struct WebRenderer {
    renderer: Renderer,
    displays: WebDisplays,
    atmo: FakeAtmosphereSampler,
}

#[wasm_bindgen]
pub fn new_web_render(
    scale: i32,
    settings: RendererSettings,
    canvas: CanvasRenderingContext2d,
) -> WebRenderer {
    let renderer: Renderer = settings.into();
    WebRenderer {
        renderer,
        displays: WebDisplays::new(scale, canvas),
        atmo: FakeAtmosphereSampler::default(),
    }
}

impl WebRenderer {
    pub fn update(&mut self, time: &str, co2: f64, temperature: f64, humidity: f64) {
        let time = match NaiveDateTime::from_str(time) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("{}", e);
                return;
            }
        }
        .and_utc();
        self.atmo.sample = AtmosphereSample {
            timestamp: time,
            temperature: Some(temperature as f32),
            relative_humidity: Some(humidity as f32),
            co2_ppm: Some(co2 as f32),
        };
        self.renderer
            .render(&mut self.displays, &mut self.atmo, time);
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
/// TODO: Extract this to its own crate?
struct CanvasTarget {
    scale: i32,
    size: Size,
    canvas: CanvasRenderingContext2d,
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
        // Keep a 1-slot cache of color strings, as we're likely to get swaths of the same color
        // repeatedly.
        let mut last_fill_color: Rgb888 = Rgb888::new(0, 0, 0);
        let mut last_fill_style: String = fill_color(last_fill_color);

        for Pixel(pt, color) in pixels.into_iter() {
            if last_fill_color != color {
                last_fill_style = fill_color(color);
                last_fill_color = color;
            }
            self.canvas.set_fill_style_str(&last_fill_style);
            self.canvas.rect(
                pt.x as f64,
                pt.y as f64,
                (pt.x + self.scale) as f64,
                (pt.y + self.scale) as f64,
            );
        }
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
    fn new(scale: i32, canvas: CanvasRenderingContext2d) -> Self {
        // 2 pixels on each edge; get the perimeter
        let perimeter = ((32 + 4) + (16 + 4)) * 2;
        let mut edge = Vec::new();
        edge.resize(perimeter, [0, 0, 0, 0]);
        WebDisplays {
            canvas: CanvasTarget {
                scale,
                canvas,
                size: Size::new(36, 20),
            },
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
        // Clear the previous image:
        self.canvas.clear(Rgb888::new(0, 0, 0)).expect("infallible");

        // Draw the border:
        render_edge(&self.edge, &mut self.canvas);

        let pixels = {
            let mut new = Vec::with_capacity(self.display.len());
            std::mem::swap(&mut self.display, &mut new);
            new
        };
        // Cropped translates; clipped ensures that OOB writes get dropped.
        let mut crop = self
            .canvas
            .cropped(&Rectangle::new(Point::new(2, 2), Size::new(32, 16)));
        let mut clip = crop.clipped(&Rectangle::new(Point::new(0, 0), Size::new(32, 16)));
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
