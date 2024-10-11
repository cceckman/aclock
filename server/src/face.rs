//! Drawing routines for the face of the clock.

use chrono::{DateTime, Local, Timelike};
use embedded_graphics::Drawable;
use embedded_graphics::{
    geometry::{Point, Size},
    mono_font::{ascii::FONT_4X6, MonoTextStyle},
    primitives::{Primitive, PrimitiveStyleBuilder, Rectangle},
    text::Text,
};
use embedded_graphics_core::pixelcolor::Rgb888;
use embedded_graphics_core::pixelcolor::RgbColor;

use crate::Displays;

/// Render the face of the clock onto the provided DrawTarget.
pub fn get_clock(time: DateTime<Local>, canvas: &mut impl Displays) {
    let minute = time.minute();
    let hour = time.hour();
    let second = time.second();
    let time = format!("{hour:02}:{minute:02}:{second:02}");

    let mut canvas = canvas.face();
    Rectangle::new(Point::new(0, 0), Size::new(32, 16))
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(Rgb888::BLACK)
                .build(),
        )
        .draw(&mut canvas)
        .expect("infallible");
    // five 6x10 characters in a 32x16 space:
    // 30 pixels wide, 1 on each side;
    // I don't know how they're handling the vertical but this looks right.
    //
    // Using a smaller one so the seconds show up...
    let mono_text_style = MonoTextStyle::new(&FONT_4X6, Rgb888::WHITE);
    let style = mono_text_style;
    Text::new(&time, Point::new(1, 11), style)
        .draw(&mut canvas)
        .expect("infallible");
}
