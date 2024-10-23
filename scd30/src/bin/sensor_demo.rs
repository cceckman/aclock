use linux_embedded_hal::I2cdev;
use scd30::Error;
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
        match scd30.sample() {
            Err(Error::NotReady()) => println!("not ready"),
            Ok(s) => {
                println!("{}", s);
                samples += 1;
            }
            Err(e) => {
                println!("error:{}", e);
                samples += 1;
            }
        }

        thread::sleep(PERIOD);
    }
}
