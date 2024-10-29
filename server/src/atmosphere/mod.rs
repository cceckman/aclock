//! Types for surfacing atmospheric data.

use chrono::{DateTime, Utc};

/// A sample of local atmospheric conditions.
///
/// For instance,
/// consider the US [National Weather Service](https://www.weather.gov/documentation/services-web-api) for outdoor conditions,
/// or an [SCD30](https://www.adafruit.com/product/4867) for indoor conditions
#[derive(Default, Clone, Copy, Debug)]
pub struct AtmosphereSample {
    /// Time at which the data in this sample was acquired.
    pub timestamp: DateTime<Utc>,

    /// Temperature in degrees Celsius.
    pub temperature: Option<f32>,

    /// Relative humidity as a percentage (i.e. range 0.0 to 100.0).
    pub relative_humidity: Option<f32>,

    /// Carbom dioxide concentration in parts per million
    /// (i.e. 1.0 = 1 ppm).
    pub co2_ppm: Option<f32>,
}

/// A historical measurement, timestamped.
#[derive(Copy, Clone, Debug)]
pub struct LastMeasurement {
    pub at: DateTime<Utc>,
    pub value: f32,
}

impl LastMeasurement {
    pub fn update(old: &mut Option<LastMeasurement>, time: DateTime<Utc>, new: Option<f32>) {
        let new = new.map(|value| LastMeasurement { at: time, value });
        *old = match (*old, new) {
            (Some(o), Some(new)) => Some(if new.at > o.at { new } else { o }),
            (Some(o), None) => Some(o),
            (None, Some(new)) => Some(new),
            _ => None,
        }
    }
}

/// A type that can get local atmospheric conditions.
pub trait AtmosphereSampler {
    /// Get a current / latest sample of atmospheric conditions.
    ///
    /// While this will always return _an_ AtmosphereSample,
    /// it may return stale or partial data.
    fn sample(&mut self) -> AtmosphereSample;
}

/// The nullary AtmosphereSampler: provides no data.
pub struct NullAtmosphereSampler {}

impl AtmosphereSampler for NullAtmosphereSampler {
    fn sample(&mut self) -> AtmosphereSample {
        Default::default()
    }
}

/// Fake atmosphere sampler: repeatedly provides the indicated sample.
#[derive(Default)]
pub struct FakeAtmosphereSampler {
    pub sample: AtmosphereSample,
}

impl AtmosphereSampler for FakeAtmosphereSampler {
    fn sample(&mut self) -> AtmosphereSample {
        self.sample
    }
}

#[cfg(feature = "hardware")]
mod scd30 {
    use chrono::Utc;
    use embedded_hal::i2c::SevenBitAddress;
    use scd30::SCD30;

    use super::{AtmosphereSample, AtmosphereSampler};

    impl<I> AtmosphereSampler for SCD30<I>
    where
        I: embedded_hal::i2c::I2c<SevenBitAddress>,
    {
        fn sample(&mut self) -> AtmosphereSample {
            match self.sample() {
                Ok(s) => AtmosphereSample {
                    timestamp: Utc::now(),
                    temperature: Some(s.temperature),
                    relative_humidity: Some(s.humidity),
                    co2_ppm: Some(s.co2),
                },
                Err(e) => {
                    if let scd30::Error::NotReady() = e {
                        tracing::debug!("scd30 not ready with new sample");
                    } else {
                        tracing::warn!("error in communicating with scd30: {:?}", e);
                    }
                    AtmosphereSample {
                        timestamp: Utc::now(),
                        ..Default::default()
                    }
                }
            }
        }
    }
}
