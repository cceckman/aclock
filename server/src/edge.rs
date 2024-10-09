//! Routines to render the NeoPixels.
use std::f32::consts::PI;

use chrono::{DateTime, Local};
use chrono::{Timelike, Utc};
use satkit::{lpephem::sun::riseset, AstroTime, ITRFCoord};

const MIN_DAYLIGHT: f32 = 0.2;

/// Alias for a color of NeoPixel.
pub type NeoPixelColor = [u8; 4];

/// Compute the pixel colors for the given date.
/// (The time component is ignored.)
pub fn get_pixels(time: DateTime<Local>, output: &mut [NeoPixelColor]) -> Result<(), String> {
    let len = output.len();
    for (i, px) in output.iter_mut().enumerate() {
        let v = ((i * 255) / len).clamp(0, 255) as u8;
        *px = [v, v, v, v];
    }

    let astro = AstroTime::from_unixtime(time.to_utc().timestamp() as f64);
    // Wilmington, DE
    let coord = ITRFCoord::from_geodetic_deg(39.7447, -75.539787, 28.0);
    // "standard" rise and set are slightly off 90 degrees; ignoring that for now.
    let (rise, set) = riseset(&astro, &coord, Some(90.0)).map_err(|err| format!("{}", err))?;
    // Civil Twilight: 96deg, per docstring
    // let (dawn, twilight) = riseset(&astro, &coord, Some(96.0)).map_err(|err| format!("{}", err))?;
    let [rise, set]: [f32; 2] = [rise, set]
        .map(|v| {
            DateTime::from_timestamp(v.to_unixtime() as i64, 0).expect("could not convert datetime")
        })
        .map(|v: DateTime<Utc>| {
            let time = v.with_timezone(&Local).time();
            let h = time.hour();
            let m = time.minute();
            // Convert to a fraction of the day, at a minute granualirty.
            (h * 60 + m) as f32 / (24 * 60) as f32
        });
    let daylight = set - rise;

    let len = output.len() as f32;
    for (i, px) in output.iter_mut().enumerate() {
        // The [0, 1)-bounded fraction of the day this point is at.
        let date_fraction = i as f32 / len;
        // What fraction of _daylight_ has passed at this point?
        // (May be negative or greater than 1)
        let day_fraction = (date_fraction - rise) / daylight;
        if (0.0..=1.0).contains(&day_fraction) {
            // During daylight hours.
            // Make a nice curve via sin:
            let f = (day_fraction * PI).sin();
            // But then make sure it meets a minimum brightness:
            let f = MIN_DAYLIGHT + f * (1.0 - MIN_DAYLIGHT);

            // Then re-range to 0..=255.
            let amt = (f * 255.0).clamp(0.0, 255.0) as u8;
            // Just using RGB, for now.
            // TODO: Mix in W.
            *px = [amt, amt, amt, 0];
        } else {
            let min: u8 = (MIN_DAYLIGHT * 255.0).clamp(0.0, 255.0) as u8;
            // Night hours.
            // Just blue them out, for now.
            *px = [0, min / 2, min, 0];
        }
    }

    Ok(())
}
