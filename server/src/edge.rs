//! Routine for computing neopixel brightnesses.
use std::f32::consts::PI;

use chrono::{DateTime, Local};
use chrono::{Timelike, Utc};
use satkit::{lpephem::sun::riseset, AstroTime, ITRFCoord};

use crate::Displays;

const MIN_DAYLIGHT: f32 = 0.25;
// MIN_DAYLIGHT is a 3- or 4-channel value; NIGHTLIGHT is not.
// Scale NIGHTLIGHT accordingly.
const MAX_NIGHTLIGHT: f32 = MIN_DAYLIGHT * 1.8;

/// Alias for a color of NeoPixel.
pub type NeoPixelColor = [u8; 4];

/// Compute the pixel colors for the given date.
/// (The time component is ignored.)
pub fn get_pixels(time: DateTime<Local>, displays: &mut impl Displays) -> Result<(), String> {
    let output = displays.edge();
    // Test version:
    //let len = output.len();
    //for (i, px) in output.iter_mut().enumerate() {
    //    let v = ((i * 255) / len).clamp(0, 255) as u8;
    //    *px = [v, v, v, v];
    //}

    let astro = AstroTime::from_unixtime(time.to_utc().timestamp() as f64);
    // Wilmington, DE
    let coord = ITRFCoord::from_geodetic_deg(39.7447, -75.539787, 28.0);
    // "standard" rise and set are slightly off 90 degrees; ignoring that for now.
    let (a, b) = riseset(&astro, &coord, Some(90.0)).map_err(|err| format!("{}", err))?;

    // The above function returns the next two times the sun hits the horizon,
    // but they may be (rise, set) or (set, rise) depending on the specified time.

    tracing::trace!("next: {} after: {}", a, b);
    let [a, b] = [a, b].map(|v| {
        DateTime::from_timestamp(v.to_unixtime() as i64, 0).expect("could not convert datetime")
    });
    let (rise, set) = if a.hour() < b.hour() { (a, b) } else { (b, a) };
    // Convert both of them to coordinates around the face.
    let [rise, set] = [rise, set].map(|v: DateTime<Utc>| {
        let time = v.with_timezone(&Local).time();
        tracing::trace!("local: {}", time);
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
            let sin = (day_fraction * PI).sin();
            // But then make sure it meets a minimum brightness:
            let f = MIN_DAYLIGHT + sin * (1.0 - MIN_DAYLIGHT);

            // Then re-range to 0..=255.
            let amt = (f * 255.0).clamp(0.0, 255.0) as u8;
            tracing::trace!(
                "point {i:03}:   day fraction {day_fraction:.2}, sin {sin:.2}, amt {amt:0}",
            );
            // TODO: Using RGB so it shows up on the simulator.
            // How do we use / render W channel?
            *px = [amt, amt, amt, amt];
        } else {
            // Normalize to "tomorrow night"
            let night_point = if date_fraction < rise {
                date_fraction + 1.0
            } else {
                date_fraction
            };
            let night_fraction = (night_point - set) / ((rise + 1.0) - set);
            let sin = (night_fraction * PI).sin();
            // and subtract that out from the maximum:
            let f = MAX_NIGHTLIGHT - (MAX_NIGHTLIGHT * sin);
            let amt = (f * 255.0).clamp(0.0, 255.0) as u8;
            tracing::trace!(
                "point {i:03}: night fraction {night_fraction:.2}, sin {sin:.2}, amt {amt:0}",
            );
            // Night is only blue, for now.
            *px = [0, 0, amt, 0];
        }
    }
    Ok(())
}
