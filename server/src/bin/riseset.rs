//! Reasonableness check on the rise/set computations.
//!
//! Print a table of ephemerides: for each day of the next year, the next rise and set times.

use server::riseset::ephemerides;

fn main() {
    tracing_subscriber::fmt::init();
    let eph = ephemerides(39.0, -77.0);
    print!("{}", eph);
}
