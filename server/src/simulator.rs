use std::convert::Infallible;

use embedded_graphics::{
    draw_target::{DrawTarget, DrawTargetExt},
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::{Rgb888, RgbColor},
    primitives::Rectangle,
    Pixel,
};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};

use crate::{Displays, NeoPixelColor};

pub struct SimDisplays {
    display: SimulatorDisplay<Rgb888>,
    window: Option<Window>,
    edge: Vec<NeoPixelColor>,
}

impl SimDisplays {
    pub fn new() -> Self {
        let settings = OutputSettingsBuilder::new().scale(20).build();
        let window = Window::new("A Clock", &settings);
        let n = Self::new_hidden();
        SimDisplays {
            window: Some(window),
            ..n
        }
    }

    /// Creates a new SimDisplays, but without generating a window.
    pub fn new_hidden() -> Self {
        let size = Size::new(32 + 2 * 2, 16 + 2 * 2);
        let display = SimulatorDisplay::new(size);

        // This gives us more edge lights than we normally have,
        // but that's OK; the drawing routines account for any number.
        let count = (size.width * 2 + size.height * 2) as usize;
        let mut edge = Vec::with_capacity(count);
        edge.resize(count, NeoPixelColor::default());

        SimDisplays {
            window: None,
            display,
            edge,
        }
    }

    /// Render the edge pixels onto the screen.
    fn render_edge(&mut self) {
        let points = PerimiterTracer::new(self.display.size()).take(self.edge.len());
        let edge_pixels = self
            .edge
            .iter()
            .map(|color| {
                let [r, g, b, _w] = *color;
                // TODO: Incorporate W channel
                Rgb888::new(r, g, b)
            })
            .zip(points)
            .map(|(c, p)| Pixel(p, c));
        self.display.draw_iter(edge_pixels).expect("infallible");
    }

    /// Flush to a screenshot instead of a display.
    pub fn screenshot(&mut self) -> embedded_graphics_simulator::OutputImage<Rgb888> {
        let settings = OutputSettingsBuilder::new().scale(20).build();
        self.render_edge();
        let img = self.display.to_rgb_output_image(&settings);
        self.clear();
        img
    }

    fn clear(&mut self) {
        self.display
            .fill_solid(
                &Rectangle::new(Point::new(0, 0), self.display.size()),
                Rgb888::BLACK,
            )
            .expect("infallible");
    }
}

impl Default for SimDisplays {
    fn default() -> Self {
        Self::new()
    }
}

/// Enumerates the points along the perimeter, from 6 o'clock to 6 o'clock, clockwise.
/// Note: This is an infinite iterator.
struct PerimiterTracer {
    next: Point,
    bounds: Size,
}

impl PerimiterTracer {
    pub fn new(size: Size) -> Self {
        let y = size.height - 1;
        let x = size.width / 2;
        Self {
            next: Point::new(x as i32, y as i32),
            bounds: size,
        }
    }
}

impl Iterator for PerimiterTracer {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        let v = self.next;
        let mut x = v.x as u32;
        let mut y = v.y as u32;

        let right_edge = x == self.bounds.width - 1;
        let left_edge = x == 0;
        let top_edge = y == 0;
        let bottom_edge = y == self.bounds.height - 1;

        if right_edge && !bottom_edge {
            // Move down.
            y += 1;
        }
        if bottom_edge && !left_edge {
            // Move left.
            x -= 1;
        }
        if left_edge && !top_edge {
            // Move up.
            y -= 1;
        }
        if top_edge && !right_edge {
            // Move right.
            x += 1;
        }

        // Move down along the right edge

        self.next = Point {
            x: x as i32,
            y: y as i32,
        };
        Some(v)
    }
}

impl Displays for SimDisplays {
    fn edge(&mut self) -> &mut [NeoPixelColor] {
        &mut self.edge
    }

    fn face(
        &mut self,
    ) -> impl embedded_graphics_core::draw_target::DrawTarget<Color = Rgb888, Error = Infallible>
    {
        self.display
            .cropped(&Rectangle::new(Point::new(2, 2), Size::new(32, 16)))
    }

    fn flush(&mut self) -> Result<(), String> {
        self.render_edge();
        if let Some(window) = &mut self.window {
            window.update(&self.display);
        }
        // self.clear();
        Ok(())
    }
}
