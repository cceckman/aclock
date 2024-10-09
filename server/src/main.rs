//! Main binary: try to run both loops.

use std::{
    sync::atomic::{self, AtomicBool, Ordering},
    thread,
    time::{self, Duration, Instant},
};

use server::{run_display, run_neopixels};

pub fn main() {
    tracing_subscriber::fmt().init();

    let run = AtomicBool::new(true);
    std::thread::scope(|s| {
        let neopixel = s.spawn(|| {
            let r = run_neopixels(&run);
            run.store(false, Ordering::SeqCst);
            r.unwrap();
        });
        let matrix = s.spawn(|| {
            let r = run_display(&run);
            run.store(false, Ordering::SeqCst);
            r.unwrap();
        });
        let timer = s.spawn(|| {
            let now = time::Instant::now();
            let deadline = now + Duration::from_secs(30);
            tracing::info!("starting timer loop");
            while run.load(Ordering::Relaxed) && Instant::now() < deadline {
                // Responsive, but not too busy
                thread::sleep(Duration::from_millis(100));
            }
            tracing::info!("ending timer");
            run.store(false, atomic::Ordering::SeqCst);
        });

        neopixel.join().expect("could not join neopixel thread");
        matrix.join().expect("could not join matrix thread");
        timer.join().expect("could not join timer thread");
    });
    tracing::info!("all done!");
}
