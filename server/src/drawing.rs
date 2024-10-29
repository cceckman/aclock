//! Utilities for drawing.

use std::fmt::Debug;

use embedded_graphics::{
    pixelcolor::Rgb888,
    prelude::{DrawTarget, Point, Size},
    Pixel,
};

use embedded_graphics_core::prelude::OriginDimensions;

use crate::NeoPixelColor;

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

/// Draw an edge onto the display,
/// assuming the display has a 1px border representing the edge.
#[allow(unused)]
pub fn render_edge<D>(edge: &[NeoPixelColor], display: &mut D)
where
    D: DrawTarget<Color = Rgb888> + OriginDimensions,
    D::Error: Debug,
{
    let points = PerimiterTracer::new(display.size()).take(edge.len());
    let edge_pixels = edge
        .iter()
        .map(|color| {
            let [r, g, b, _w] = *color;
            // TODO: Incorporate W channel
            Rgb888::new(r, g, b)
        })
        .zip(points)
        .map(|(c, p)| Pixel(p, c));
    display.draw_iter(edge_pixels).expect("infallible");
}
