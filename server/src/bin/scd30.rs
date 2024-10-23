//! Demo of reading the SCD30.

use linux_embedded_hal::I2cdev;
use server::atmosphere::AtmosphereSampler;
use std::thread;
use std::time::Duration;

const PERIOD: Duration = Duration::from_secs(2);

fn main() {
    let device = I2cdev::new("/dev/i2c-1").expect("could not open i2c device");

    let mut scd30 = scd30::SCD30::new(device, scd30::SCD30Settings::default()).unwrap();

    let mut samples = 0;
    println!("starting measurement...");
    while samples < 20 {
        println!("getting sample...");
        let v = (&mut scd30 as &mut dyn AtmosphereSampler).sample();
        if let Some(co2) = v.co2_ppm {
            println!("co2: {:.1}", co2)
        } else {
            println!("waiting...")
        }

        thread::sleep(PERIOD);
    }
}
