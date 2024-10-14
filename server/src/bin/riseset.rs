//! Reasonableness check on the satkit rise/set times.
//!
//! Print a table of ephemerides: for each hour of the next year, the next rise and set times.

use chrono::{Datelike, Local};
use server::riseset::riseset;

fn main() {
    tracing_subscriber::fmt::init();
    let mut now = Local::now().with_month(1).unwrap().with_day(1).unwrap();
    let end = now + chrono::Duration::days(366);

    while now < end {
        let (rise, snoon, set) = riseset(now, 39.0, -77.0);
        println!("{rise} // {snoon} // {set}");
        now += chrono::Duration::days(1);
    }
}
