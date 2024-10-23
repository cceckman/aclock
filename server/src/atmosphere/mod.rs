//! Types for surfacing atmospheric data.

use chrono::{DateTime, Utc};
use scd30::I2cBus;

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

#[cfg(feature = "atmo-usgov")]
pub mod usgov;

/// Fake atmosphere sampler: repeatedly provides the indicated sample.
pub struct FakeAtmosphereSampler {
    pub sample: AtmosphereSample,
}

impl AtmosphereSampler for FakeAtmosphereSampler {
    fn sample(&mut self) -> AtmosphereSample {
        self.sample
    }
}

impl<I> AtmosphereSampler for scd30::SCD30<I>
where
    I: I2cBus,
{
    fn sample(&mut self) -> AtmosphereSample {
        if let Ok(s) = self.sample() {
            AtmosphereSample {
                timestamp: Utc::now(),
                temperature: Some(s.temperature),
                relative_humidity: Some(s.humidity),
                co2_ppm: Some(s.co2),
            }
        } else {
            AtmosphereSample {
                timestamp: Utc::now(),
                ..Default::default()
            }
        }
    }
}
