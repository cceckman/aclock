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

use std::f64::consts::PI;

use chrono::{DateTime, Datelike, Duration, NaiveDateTime, NaiveTime, Timelike};

pub fn riseset<T: chrono::TimeZone>(
    date: DateTime<T>,
    latitude: f64,
    longitude: f64,
) -> (DateTime<T>, DateTime<T>, DateTime<T>) {
    let yr = date.year();
    // The NOAA equations produce rise and set times in minutes past UTC midnight.
    // We'll complete the NOAA equations then convert back to DateTime.
    let (rise, snoon, set) = {
        // START OF NOAA EQUATIONS
        let leap_year = yr % 4 == 0 && yr % 100 != 0;

        let days = if leap_year { 366 } else { 365 };

        // Fractional year in radians
        let ordinal_day = date.ordinal() - 1 + (date.hour() - 12) / 24;
        let gamma = (2.0 * PI) * (ordinal_day as f64) / (days as f64);

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
        let zenith: f64 = (90.833f64).to_radians();
        let lat = latitude.to_radians();

        // We diverge from the PDF here and use the spreadsheet's form:
        // rise and set are computed as a difference from solar noon.
        let ha = (zenith.cos() / (lat.cos() * decl.cos()) - lat.tan() * decl.tan()).acos();

        let snoon = 720.0 - 4.0 * longitude - eqtime;
        let rise = snoon - 4.0 * ha.to_degrees();
        let set = snoon + 4.0 * ha.to_degrees();

        (rise, snoon, set)
        // END OF NOAA EQUATIONS
    };
    let just_date = date.naive_utc().date();
    let [rise, snoon, set] = [rise, snoon, set].map(|f| {
        let offset = Duration::seconds(f.round() as i64 * 60);
        let d = NaiveDateTime::new(just_date, NaiveTime::MIN) + offset;
        date.timezone().from_utc_datetime(&d)
    });

    (rise, snoon, set)
}
