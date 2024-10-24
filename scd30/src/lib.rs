//! Driver for the SCD30 atmospheric sensor package.

use embedded_hal::i2c::{I2c, SevenBitAddress};
use std::time::Duration;

mod i2c;

/// An error in communicating with the SCD30.
pub enum Error<I2cError> {
    I2cWrite(I2cError),
    I2cRead(I2cError),
    Crc(i2c::InvalidCRC),
    InvalidArgument(&'static str),
    NotReady(),
}

impl<I2cError> core::fmt::Debug for Error<I2cError>
where
    I2cError: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::I2cWrite(e) => write!(f, "Error::I2cWrite({:?})", e),
            Error::I2cRead(e) => write!(f, "Error::I2cRead({:?})", e),
            Error::Crc(c) => write!(f, "Error::Crc({:?})", c),
            Error::InvalidArgument(s) => write!(f, "Error::InvalidArgument({:?})", s),
            Error::NotReady() => write!(f, "Error::NotReady"),
        }
    }
}

impl<I2cError> core::fmt::Display for Error<I2cError>
where
    I2cError: core::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::I2cWrite(e) => write!(f, "in SCD30 write to I2c: {}", e),
            Error::I2cRead(e) => write!(f, "in SCD30 read from I2c: {}", e),
            Error::Crc(c) => write!(f, "in SCD30 CRC computation: {}", c),
            Error::InvalidArgument(s) => write!(f, "invalid argument for SCD30 setup: {}", s),
            Error::NotReady() => write!(f, "SCD30 reports no data ready"),
        }
    }
}

impl<I2cError> core::error::Error for Error<I2cError>
where
    I2cError: core::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::I2cWrite(e) => Some(e),
            Error::I2cRead(e) => Some(e),
            _ => None,
        }
    }
}

/// Handle to an SCD30 atmospheric sensor package.
pub struct SCD30<I> {
    comm: i2c::I2cComm<I>,
    continuous_enabled: bool,
}

/// Settings when starting to use an SCD30.
#[non_exhaustive]
pub struct SCD30Settings {
    /// Internal polling period of the SCD30.
    /// Defaults to 10 seconds; ranges from 2 to 1800 seconds.
    pub period: Duration,
}

impl Default for SCD30Settings {
    fn default() -> Self {
        Self {
            period: Duration::from_secs(10),
        }
    }
}

impl<I> SCD30<I>
where
    I: I2c<SevenBitAddress>,
{
    /// Attach to an SCD30 on the bus and configure it as specified.
    pub fn new(bus: I, settings: SCD30Settings) -> Result<Self, Error<I::Error>> {
        let mut s = SCD30 {
            comm: i2c::I2cComm::new(bus),
            continuous_enabled: false,
        };
        let period = settings.period.as_secs();
        if !(2..=1800).contains(&period) {
            return Err(Error::InvalidArgument(
                "period must be between 2 and 1800 seconds",
            ));
        }

        s.run_command(Command::SetContinuousInterval(period as u16))?;
        // TODO: Support pressure adjustment?
        s.run_command(Command::StartContinuous())?;
        s.continuous_enabled = true;

        Ok(s)
    }

    /// Stop continuous measurement.
    pub fn stop(mut self) -> Result<(), (Error<I::Error>, Self)> {
        match self.run_command(Command::StopContinuous()) {
            Ok(_) => {
                self.continuous_enabled = false;
                Ok(())
            }
            Err(e) => Err((e, self)),
        }
    }

    /// Acquire a sample from the SCD30.
    ///
    /// If a sample is not immediately available, returns a NotReady error.
    pub fn sample(&mut self) -> Result<Sample, Error<I::Error>> {
        const READY: Command = Command::GetDataReady();
        let mut ready = [0u8; READY.data_bytes()];
        self.run_command(READY)?;
        self.comm.read(&mut ready)?;
        if u16::from_be_bytes(ready) != 1 {
            return Err(Error::NotReady());
        }

        const SAMPLE: Command = Command::ReadMeasurement();
        let mut data = [0u8; SAMPLE.data_bytes()];
        self.run_command(SAMPLE)?;
        self.comm.read(&mut data)?;

        // Interpret the sample.
        // Per the datasheet (table 2), the order is "CO2", "temp", "humidity",
        // each as a big-endian u32; we've already removed CRCs.
        let mut co2_bytes = [0u8; 4];
        let mut temp_bytes = [0u8; 4];
        let mut humid_bytes = [0u8; 4];
        co2_bytes.copy_from_slice(&data[0..4]);
        temp_bytes.copy_from_slice(&data[4..8]);
        humid_bytes.copy_from_slice(&data[8..12]);

        Ok(Sample {
            co2: f32::from_be_bytes(co2_bytes),
            temperature: f32::from_be_bytes(temp_bytes),
            humidity: f32::from_be_bytes(humid_bytes),
        })
    }

    /// Run a command, without getting any data back.
    fn run_command(&mut self, cmd: Command) -> Result<(), Error<I::Error>> {
        self.comm.send(cmd.command(), cmd.data())
    }
}

/// An atmospheric sensor sample from the SCD30.
#[derive(Debug, Copy, Clone)]
pub struct Sample {
    /// CO2 concentration in parts per million
    pub co2: f32,
    /// Temperature in degrees celcius
    pub temperature: f32,
    /// Relative humidity
    pub humidity: f32,
}

impl core::fmt::Display for Sample {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "CO2: {:.1} ppm\nT: {:.1} °C\nRH: {:.1}%",
            self.co2, self.temperature, self.humidity
        )
    }
}

#[allow(dead_code)]
enum Command {
    /// Start continous measurement.
    /// TODO:Consider pressure adjustment?
    StartContinuous(),

    /// Stop continuous measurement.
    StopContinuous(),

    /// Set the period for continuous measurements, in seconds
    SetContinuousInterval(u16),

    /// Check whether a measurement is ready from the internal buffer.
    GetDataReady(),

    /// Check whether a measurement is ready from the internal buffer.
    ReadMeasurement(),

    /// Enable or disable automatic self-calibration.
    /// See datasheet for physical requirements for calibration.
    SetAutomaticSelfCalibration(bool),

    /// Set forced recalibration value, in PPM.
    /// See datasheet for physical requirements for calibration.
    SetForcedRecalibration(u16),

    /// Set a temperature offset, in units of 0.01°C.
    SetTemperatureOffset(u16),

    /// Set a configured altitude, as an alternative to an ambient pressure measurement.
    /// The unit is in meters above sea level.
    SetAltitude(u16),

    /// Trigger a soft reset, forcing the sensor into its power-up state
    /// without clearing nonvolatile memory.
    SoftReset(),
}

impl Command {
    /// Command identifier for this command.
    fn command(&self) -> u16 {
        match self {
            Command::StartContinuous() => 0x0010,
            Command::StopContinuous() => 0x0104,
            Command::SetContinuousInterval(_) => 0x4600,
            Command::GetDataReady() => 0x0202,
            Command::ReadMeasurement() => 0x0300,
            Command::SetAutomaticSelfCalibration(_) => 0x5306,
            Command::SetForcedRecalibration(_) => 0x5204,
            Command::SetTemperatureOffset(_) => 0x5403,
            Command::SetAltitude(_) => 0x5102,
            Command::SoftReset() => 0xD304,
        }
    }

    /// Data to be sent with this command (if any).
    fn data(&self) -> Option<u16> {
        match self {
            Command::StartContinuous() => Some(0x0000),
            Command::SetContinuousInterval(period) => Some(*period),
            Command::SetAutomaticSelfCalibration(false) => Some(0),
            Command::SetAutomaticSelfCalibration(true) => Some(1),
            Command::SetForcedRecalibration(value) => Some(*value),
            Command::SetTemperatureOffset(value) => Some(*value),
            Command::SetAltitude(ref value) => Some(*value),
            _ => None,
        }
    }

    /// Number of bytes that should be read for this command, excluding CRCs.
    const fn data_bytes(&self) -> usize {
        use core::mem::size_of;

        let words = match self {
            Command::GetDataReady() => 1,
            Command::ReadMeasurement() => 6,
            _ => 0,
        };

        words * size_of::<u16>()
    }
}
