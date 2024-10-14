//! Generates a video of the display for a whole day/year.

use std::{
    ops::Range,
    path::Path,
    time::{Duration, Instant},
};

use chrono::{DateTime, Local};
use server::{context::Context, edge::get_pixels, face::get_clock};
use tempfile::NamedTempFile;

/// Make screen samples starting from the start time, stepping by the duration, and put them in the
/// output path.
///
/// Only produces the (offset)th samples of (parallel_count);
/// e.g. with offset=1 and parallel_count = 4, produces the 1th, 5th, 9th, etc. multiples of step.
fn make_samples(
    ctx: &Context,
    when: Range<DateTime<Local>>,
    step: Duration,
    offset: u32,
    parallel_count: u32,
    outdir: &Path,
) {
    let mut displays = server::simulator::SimDisplays::new_hidden();

    let end = when.end;

    for i in 0.. {
        let t = when.start + step * (i * parallel_count + offset);
        if t > end || ctx.is_cancelled() {
            break;
        }
        let start = Instant::now();
        get_clock(t, &mut displays);
        get_pixels(t, &mut displays).unwrap();
        let buffer = displays.screenshot();
        let rendered = Instant::now();

        let path = outdir.join(format!("{i:04}.png"));
        buffer.save_png(&path).unwrap();
        let saved = Instant::now();
        // Note: the --release build of the PNG writer is _much_ faster.
        tracing::trace!(
            "{:04} -- {:03} rendering, {:03} saving",
            i,
            (rendered - start).as_millis(),
            (saved - rendered).as_millis()
        );
    }
}

pub fn main() {
    tracing_subscriber::fmt::init();

    let outfile = {
        let outfile = NamedTempFile::with_suffix(".webp").unwrap();
        outfile.path().to_owned()
    };
    let ctx = server::context::Context::new();
    {
        let ctx = ctx.clone();
        ctrlc::set_handler(move || {
            tracing::info!("got SIGINT, closing context");
            ctx.cancel();
        })
        .expect("could not set SIGINT handler");
    }

    let output = tempfile::Builder::new().keep(false).tempdir().unwrap();
    let n: u32 = num_cpus::get().try_into().unwrap();
    let start = Local::now();
    let end = start + Duration::from_secs(365 * 24 * 60 * 60);
    let step = Duration::from_secs(24 * 60 * 60);
    tracing::info!("starting frame generation...");
    std::thread::scope(|scope| {
        for i in 0u32..n {
            let ctx = &ctx;
            let outdir = output.path();
            scope.spawn(move || make_samples(ctx, start..end, step, i, n, outdir));
        }
    });

    tracing::info!("output frames in {}", output.path().display());
    let c = std::process::Command::new("ffmpeg")
        .arg("-i")
        .arg(format!("{}/%04d.png", output.path().display()))
        .arg("-loop")
        .arg("0") // infinite loop
        .arg("-filter:v")
        .arg("fps=15")
        .arg("-y") // OK to overwrite
        .arg(&outfile)
        .output()
        .expect("could not run ffmpeg");
    if !c.status.success() {
        tracing::error!("ffmpeg failed: {}", c.status);
        if let Ok(s) = std::str::from_utf8(&c.stderr) {
            tracing::error!("ffmpeg output: {}", s);
        }
        std::process::exit(2);
    }
    tracing::info!("video in {}", outfile.display());
    tracing::info!("file://{}", outfile.display());
}
