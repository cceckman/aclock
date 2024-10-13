//! Drawing routines for the face of the clock.

use chrono::{DateTime, Datelike, Local, Timelike};
use embedded_graphics::text::{Alignment, Baseline, TextStyleBuilder};
use embedded_graphics::Drawable;
use embedded_graphics::{
    geometry::{Point, Size},
    mono_font::{ascii::FONT_4X6, ascii::FONT_6X9, MonoTextStyle},
    primitives::{Primitive, PrimitiveStyleBuilder, Rectangle},
    text::Text,
};
use embedded_graphics_core::pixelcolor::Rgb888;
use embedded_graphics_core::pixelcolor::RgbColor;

use crate::Displays;

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

/// Render the face of the clock onto the provided DrawTarget.
pub fn get_clock(time: DateTime<Local>, canvas: &mut impl Displays) {
    let minute = time.minute();
    let hour = time.hour();
    let day = time.day();
    let month = month_en3(time.month());
    let year = time.year() % 100;
    let time = format!("{hour:02}:{minute:02}");
    let date = format!("{day:02}{month}{year:02}");

    let mut canvas = canvas.face();
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
