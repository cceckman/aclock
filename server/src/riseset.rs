//! Sunrise / sunset calculations.
//!
//! Equations are derived from two resources from
//! [NOAA](https://gml.noaa.gov/grad/solcalc/calcdetails.html).
//! I started with [this equations sheet](https://gml.noaa.gov/grad/solcalc/solareqns.PDF),
//! but got results inconsistent with other sources.
//!
//! The spreadsheets on [this page](https://gml.noaa.gov/grad/solcalc/calcdetails.html)
//! align with other sources.
//!

/*
Spreadsheet computation:

Sunrise is (X2 * 1440 - W2 * 4) / 1440 (fraction of a day)
Sunset is (X2 * 1440 + W2 * 4) / 1440

W2: HA of sunrise (in degrees)
X2: Solar noon:
    720 - 4 * longitude + tzoffset - V2
V2: equation of time

*/

use std::f32::consts::PI;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

/// Compute the next year's rise and set times.
#[cfg_attr(feature = "web", wasm_bindgen::prelude::wasm_bindgen)]
pub fn ephemerides(latitude: f64, longitude: f64) -> String {
    let day = Local::now();
    let dates = day.naive_utc().date().iter_days();
    let mut out = "".to_owned();
    for date in dates.take(365) {
        // tracing::info!("computing {}", date);
        let (rise, noon, set) = riseset(date, latitude as f32, longitude as f32, Local);
        out += &format!(
            "{}: {} / {} / {}\n",
            date,
            rise.time(),
            noon.time(),
            set.time()
        );
    }
    out
}

/// Compute sun rise/noon/set times.
pub fn riseset<T: TimeZone>(
    date: NaiveDate,
    latitude: f32,
    longitude: f32,
    tz: T,
) -> (DateTime<T>, DateTime<T>, DateTime<T>) {
    let yr = date.year();
    // The NOAA equations produce rise and set times in minutes past UTC midnight.
    // We'll complete the NOAA equations then convert back to DateTime.
    let (rise, snoon, set) = {
        // START OF NOAA EQUATIONS
        let leap_year = yr % 4 == 0 && yr % 100 != 0;

        let days = if leap_year { 366 } else { 365 };

        // Fractional year in radians. We don't include a fractional day from the hour.
        let ordinal_day = date.ordinal(); // - 1 + (date.hour() - 12) / 24;
        let gamma = (2.0 * PI) * (ordinal_day as f32) / (days as f32);

        // equation of time, relating mean solar time and true solar time
        let eqtime = 229.18
            * (0.000075 + 0.001868 * gamma.cos()
                - 0.032077 * gamma.sin()
                - 0.014615 * (2.0 * gamma).cos()
                - 0.040849 * (2.0 * gamma).sin());

        // solar declination angle (in radians):
        let decl = 0.006918 - 0.399912 * (gamma).cos() + 0.070257 * (gamma).sin()
            - 0.006758 * (2.0 * gamma).cos()
            + 0.000907 * (2.0 * gamma).sin()
            - 0.002697 * (3.0 * gamma).cos()
            + 0.00148 * (3.0 * gamma).sin();

        // The hour angle of the sunrise and sunset is:
        let zenith: f32 = (90.833f32).to_radians();
        let lat = latitude.to_radians();

        // We diverge from the PDF here and use the spreadsheet's form:
        // rise and set are computed as a difference from solar noon.
        // TODO: Is this giving the right answer under WASM?
        // See https://github.com/ssmichael1/satkit/issues/3-
        // also showing discontinuities, under WASM only.
        let ha = (zenith.cos() / (lat.cos() * decl.cos()) - lat.tan() * decl.tan()).acos();

        let snoon = 720.0 - 4.0 * longitude - eqtime;
        let rise = snoon - 4.0 * ha.to_degrees();
        let set = snoon + 4.0 * ha.to_degrees();

        (rise, snoon, set)
        // END OF NOAA EQUATIONS
    };
    let [rise, snoon, set] = [rise, snoon, set].map(|f| {
        let offset = Duration::seconds(f.round() as i64 * 60);
        let d = NaiveDateTime::new(date, NaiveTime::MIN) + offset;
        tz.from_utc_datetime(&d)
    });

    (rise, snoon, set)
}
