//! Demo of the NeoPixel rendering routine.

use chrono::Local;
use embedded_graphics::{
    pixelcolor::{self, Rgb888},
    prelude::*,
};
use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};
use server::edge::{get_pixels, NeoPixelColor};

fn main() -> Result<(), core::convert::Infallible> {
    let mut display = SimulatorDisplay::<pixelcolor::Rgb888>::new(Size::new(60, 1));
    let mut pixels: [NeoPixelColor; 60] = [[0; 4]; 60];
    get_pixels(Local::now(), &mut pixels).expect("could not prepare neopixels");
    display
        .draw_iter(pixels.iter().enumerate().map(|(i, px)| {
            let pt = Point { x: i as i32, y: 0 };
            let c = Rgb888::new(px[0], px[1], px[2]);
            Pixel(pt, c)
        }))
        .expect("could not draw");

    let output_settings = OutputSettingsBuilder::new().scale(20).build();
    Window::new("Hello World", &output_settings).show_static(&display);

    Ok(())
}
