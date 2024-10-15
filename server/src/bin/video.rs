//! Generates a video of the display for a whole day/year.

use std::{
    ops::Range,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Local};
use server::{context::Context, Renderer};
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
    let renderer = Renderer::default();

    let end = when.end;

    for j in 0.. {
        let i = j * parallel_count + offset;
        let t = when.start + step * i;
        if t > end || ctx.is_cancelled() {
            break;
        }
        renderer.render(&mut displays, t);
        let buffer = displays.screenshot();

        let path = outdir.join(format!("{i:04}.png"));
        buffer.save_png(&path).unwrap();
        // Note: the --release build of the PNG writer is _much_ faster.
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
        .arg("-r")
        .arg("6") // frames per second... *then* provide input frames.
        // 365 frames: 6fps = ~1 minute video
        .arg("-i")
        .arg(format!("{}/%04d.png", output.path().display()))
        .arg("-loop")
        .arg("0") // infinite loop
        .arg("-lossless")
        .arg("1")
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
    if let Some(name) = std::env::args_os().nth(1) {
        let p: PathBuf = name.into();
        std::fs::copy(&outfile, &p).expect("could not copy to destination");
        tracing::info!("video in {}", p.display());
    } else {
        tracing::info!("video in {}", outfile.display());
    }
}
