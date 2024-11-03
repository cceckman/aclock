//! Set LEDs to max-brightness to test shinethrough.

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point, RgbColor},
    primitives::Rectangle,
    Pixel,
};
use server::{context::Context, Displays};
use std::{iter::once, time::Duration};

/// Generate an iterator over the pixels in the rectangle.
fn points(area: Rectangle) -> impl Iterator<Item = Point> {
    let xs = area.top_left.x..(area.top_left.x + area.size.width as i32);
    let ys = area.top_left.y..(area.top_left.y + area.size.height as i32);
    xs.flat_map(move |x| {
        let ys = ys.clone();
        ys.map(move |y| Point::new(x, y))
    })
}

pub fn main() {
    tracing_subscriber::fmt::init();
    #[cfg(feature = "simulator")]
    let mut displays = server::simulator::SimDisplays::new();

    #[cfg(feature = "hardware")]
    let mut displays = server::led_displays::LedDisplays::new().unwrap();

    let ctx = Context::new();
    {
        let ctx = ctx.clone();
        ctrlc::set_handler(move || {
            tracing::info!("got SIGINT, closing context");
            ctx.cancel();
        })
        .expect("could not set SIGINT handler");
    }

    let mut edge_channel = 0;
    let mut face_channel = 0;
    let mut edge_color = std::iter::from_fn(move || {
        let colors = [
            [0, 0, 0, 255],
            [255, 0, 0, 0],
            [0, 255, 0, 0],
            [0, 0, 255, 0],
            [255, 255, 255, 255],
        ];
        edge_channel = (edge_channel + 1) % colors.len();
        Some(colors[edge_channel])
    });
    let mut face_color = std::iter::from_fn(move || {
        let colors = [
            Rgb888::WHITE,
            Rgb888::RED,
            Rgb888::GREEN,
            Rgb888::BLUE,
            Rgb888::BLACK,
        ];
        face_channel = (face_channel + 1) % colors.len();
        Some(colors[face_channel])
    });

    tracing::info!("starting loop");
    while !ctx.is_cancelled() {
        {
            let color = edge_color.next().expect("infallible");
            for px in displays.edge() {
                *px = color;
            }
        }

        let face_color = face_color.next().expect("infallible");
        let area = displays.face().bounding_box();
        let pixels = points(area).map(|pt| Pixel(pt, face_color));

        if face_color == Rgb888::BLACK {
            for pt in points(area) {
                displays
                    .face()
                    .draw_iter(once(Pixel(pt, Rgb888::WHITE)))
                    .expect("infallible");
                displays.flush().expect("infallible");
                if ctx.wait_timeout(Duration::from_millis(10)) {
                    break;
                }
                displays
                    .face()
                    .draw_iter(once(Pixel(pt, Rgb888::BLACK)))
                    .expect("infallible");
            }
        } else {
            displays.face().draw_iter(pixels).expect("infallible");
            displays.flush().expect("infallible?");
            ctx.wait_timeout(Duration::from_secs(1));
        }
    }
    tracing::info!("exiting loop");
}
