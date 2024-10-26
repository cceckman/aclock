//! Set LEDs to max-brightness to test shinethrough.

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{Dimensions, DrawTarget, Point},
    Pixel,
};
use server::{context::Context, Displays, NeoPixelColor};
use std::time::Duration;

fn face_color(state: &mut u8) -> Rgb888 {
    *state = (*state + 1) % 4;
    match state {
        1 => Rgb888::new(255, 0, 0),
        2 => Rgb888::new(0, 255, 0),
        3 => Rgb888::new(0, 0, 255),
        _ => Rgb888::new(255, 255, 255),
    }
}

fn edge_color(state: &mut u8) -> NeoPixelColor {
    *state = (*state + 1) % 5;
    match state {
        1 => [255, 0, 0, 0],
        2 => [0, 255, 0, 0],
        3 => [0, 0, 255, 0],
        4 => [0, 0, 0, 255],
        _ => [255, 255, 255, 255],
    }
}

pub fn main() {
    tracing_subscriber::fmt::init();
    #[cfg(feature = "simulator")]
    let mut displays = server::simulator::SimDisplays::new();

    #[cfg(not(feature = "simulator"))]
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

    let mut face_channel = 0;
    let mut edge_channel = 0;

    tracing::info!("starting loop");
    while !ctx.is_cancelled() {
        {
            let mut disp = displays.face();
            let dims = disp.bounding_box();
            let xs = dims.top_left.x..(dims.top_left.x + dims.size.width as i32);
            let ys = dims.top_left.y..(dims.top_left.y + dims.size.height as i32);
            let color = face_color(&mut face_channel);
            let pixels = xs
                .flat_map(|x| {
                    let ys = ys.clone();
                    ys.map(move |y| (x, y))
                })
                .map(|(x, y)| Pixel(Point::new(x, y), color));
            disp.draw_iter(pixels).expect("infallible");
        }
        {
            let color = edge_color(&mut edge_channel);
            for px in displays.edge() {
                *px = color;
            }
        }

        displays.flush().expect("infallible?");
        ctx.wait_timeout(Duration::from_secs(1));
    }
    tracing::info!("exiting loop");
}
