//! Reasonableness check on the satkit rise/set times.
//!
//! Print a table of ephemerides: for each hour of the next year, the next rise and set times.

use chrono::{DateTime, Datelike, Local, Timelike};
use satkit::{lpephem::sun::riseset, AstroTime, ITRFCoord};

/// Get the rise and set times from the satkit library,
/// converted to chrono time units
fn get_riseset(time: DateTime<Local>) -> (DateTime<Local>, DateTime<Local>) {
    // Washington, DC, approximately
    let coord = ITRFCoord::from_geodetic_deg(39.0, -77.0, 10.0);

    // The docstring for riseset says "time is at location, and should have hours, minutes, and seconds set to zero"
    // which... is a little confusing, since there's a "unix time" constructor,
    // but whatever.
    // The rise/set tables are nonsensical if I provide fractional days (Unix timestamps),
    // shifting over the course of a day.
    let time = &AstroTime::from_date(time.year(), time.month(), time.day());

    let (a, b) = riseset(time, &coord, None).unwrap();
    let [a, b] = [a, b].map(|v| {
        DateTime::from_timestamp(v.to_unixtime() as i64, 0)
            .unwrap()
            .with_timezone(&Local)
    });
    let (rise, set) = if a.hour() < b.hour() { (a, b) } else { (b, a) };
    (rise, set)
}

fn main() {
    let mut now = Local::now();
    let end = now + chrono::Duration::days(365);

    while now < end {
        let (rise, set) = get_riseset(now);
        println!("{rise} // {set}");
        now += chrono::Duration::days(1);
    }
}
