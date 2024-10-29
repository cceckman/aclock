use std::convert::Infallible;

use embedded_graphics::{
    draw_target::{DrawTarget, DrawTargetExt},
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::{Rgb888, RgbColor},
    primitives::Rectangle,
};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};

use crate::{drawing::render_edge, Displays, NeoPixelColor};

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

    /// Flush to a screenshot instead of a display.
    pub fn screenshot(&mut self) -> embedded_graphics_simulator::OutputImage<Rgb888> {
        let settings = OutputSettingsBuilder::new().scale(20).build();
        render_edge(&self.edge, &mut self.display);
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

impl Displays for SimDisplays {
    fn edge(&mut self) -> &mut [NeoPixelColor] {
        &mut self.edge
    }

    fn face(
        &mut self,
    ) -> impl embedded_graphics_core::draw_target::DrawTarget<Color = Rgb888, Error = Infallible>
    {
        // Cropped translates; clipped ensures that OOB writes get dropped.
        // But clipped borrows from cropped, so we can't chain them, alas.
        self.display
            .cropped(&Rectangle::new(Point::new(2, 2), Size::new(32, 16)))
    }

    fn flush(&mut self) -> Result<(), String> {
        render_edge(&self.edge, &mut self.display);
        if let Some(window) = &mut self.window {
            window.update(&self.display);
        }
        // self.clear();
        Ok(())
    }
}
