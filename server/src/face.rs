//! Drawing routines for the face of the clock.

use std::convert::Infallible;

use chrono::{DateTime, Local, Timelike};
use embedded_graphics::Drawable;
use embedded_graphics::{
    geometry::{Point, Size},
    mono_font::{ascii::FONT_4X6, MonoTextStyle},
    primitives::{Primitive, PrimitiveStyleBuilder, Rectangle},
    text::Text,
};
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::pixelcolor::Rgb888;
use embedded_graphics_core::pixelcolor::RgbColor;
use rpi_led_matrix::{LedCanvas, LedMatrix, LedMatrixOptions};

use crate::Face;

/// Render the face of the clock onto the provided DrawTarget.
pub fn get_clock(time: DateTime<Local>, canvas: &mut impl Face) {
    let minute = time.minute();
    let hour = time.hour();
    let second = time.second();
    let time = format!("{hour:02}:{minute:02}:{second:02}");

    {
        let canvas = canvas.drawable();
        Rectangle::new(Point::new(0, 0), Size::new(32, 16))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb888::BLACK)
                    .build(),
            )
            .draw(canvas)
            .expect("infallible");
        // five 6x10 characters in a 32x16 space:
        // 30 pixels wide, 1 on each side;
        // I don't know how they're handling the vertical but this looks right.
        //
        // Using a smaller one so the seconds show up...
        let mono_text_style = MonoTextStyle::new(&FONT_4X6, Rgb888::WHITE);
        let style = mono_text_style;
        Text::new(&time, Point::new(1, 11), style)
            .draw(canvas)
            .expect("infallible");
    }
    canvas.flush();
}

#[cfg(feature = "simulator")]
pub type FaceImpl = simulator::SimFace;

#[cfg(feature = "simulator")]
pub fn get_face() -> Result<FaceImpl, &'static str> {
    Ok(simulator::SimFace::new())
}

#[cfg(not(feature = "simulator"))]
pub type FaceImpl = MatrixFace;

#[cfg(not(feature = "simulator"))]
pub fn get_face() -> Result<FaceImpl, &'static str> {
    MatrixFace::new()
}

/// Implements Face by drawing to an LED matrix.
pub struct MatrixFace {
    matrix: LedMatrix,
    offscreen_canvas: LedCanvas,
}

impl MatrixFace {
    pub fn new() -> Result<Self, &'static str> {
        let mut options = LedMatrixOptions::new();
        // This matrix presents as two 16x16 panels.
        const ROWS: u32 = 16;
        const COLS: u32 = 16;
        options.set_rows(ROWS);
        options.set_cols(COLS);
        options.set_chain_length(2);
        options.set_parallel(1);
        options.set_refresh_rate(false);

        // TODO: Consider shorting pin 18, using PWM
        options.set_hardware_mapping("adafruit-hat");
        let matrix = LedMatrix::new(Some(options), None)?;
        let offscreen_canvas = matrix.offscreen_canvas();
        let mut f = MatrixFace {
            matrix,
            offscreen_canvas,
        };
        Rectangle::new(Point::new(0, 0), Size::new(32, 16))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb888::BLACK)
                    .build(),
            )
            .draw(f.drawable())
            .expect("infallible");
        f.flush();
        Ok(f)
    }
}

impl Face for MatrixFace {
    fn drawable(&mut self) -> &mut impl DrawTarget<Color = Rgb888, Error = Infallible> {
        &mut self.offscreen_canvas
    }

    fn flush(&mut self) {
        // We have to swap pointers too...
        let mut spare = self.matrix.offscreen_canvas();
        std::mem::swap(&mut spare, &mut self.offscreen_canvas);
        spare = self.matrix.swap(spare);
        std::mem::swap(&mut spare, &mut self.offscreen_canvas);
    }
}

#[cfg(feature = "simulator")]
mod simulator {
    use std::convert::Infallible;

    use embedded_graphics::{draw_target::DrawTarget, geometry::Size, pixelcolor::Rgb888};
    use embedded_graphics_simulator::{OutputSettingsBuilder, SimulatorDisplay, Window};

    use crate::Face;

    /// Simulator of a Face.

    pub struct SimFace {
        display: SimulatorDisplay<Rgb888>,
        window: Window,
    }

    impl SimFace {
        pub fn new() -> Self {
            let display = SimulatorDisplay::new(Size::new(32, 16));

            let settings = OutputSettingsBuilder::new().scale(10).build();

            let window = Window::new("Matrix", &settings);
            SimFace { window, display }
        }
    }

    impl Default for SimFace {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Face for SimFace {
        fn drawable(&mut self) -> &mut impl DrawTarget<Color = Rgb888, Error = Infallible> {
            &mut self.display
        }

        fn flush(&mut self) {
            self.window.update(&self.display)
        }
    }
}

//pub fn get_(run: &AtomicBool) -> Result<(), &'static str> {
//    let color = LedColor {
//        red: 64,
//        green: 64,
//        blue: 64,
//    };
//
//    let mut r: u32 = 0;
//    let mut c: u32 = 0;
//    let mut canvas = matrix.offscreen_canvas();
//    tracing::info!("starting display loop");
//    while run.load(Relaxed) {
//        canvas.fill(&off);
//        canvas.set(r as i32, c as i32, &color);
//        canvas = matrix.swap(canvas);
//        c = (c + 1) % COLS;
//        if c == 0 {
//            r = (r + 1) % (ROWS * 2);
//        }
//        if r == 0 && c == 0 {
//            break;
//        }
//
//        thread::sleep(INTERVAL);
//    }
//    tracing::info!("ending display loop");
//    Ok(())
//}
