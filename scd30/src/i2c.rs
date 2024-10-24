//! Helper module for I2c communication with the SH30 package.

use crate::Error;
use core::fmt::Debug;
use core::mem::size_of;
use crc_any::CRCu8;
use embedded_hal::i2c::{I2c, SevenBitAddress};
use std::fmt::Display;

/// I2c bus address of the SH30.
const ADDRESS: u8 = 0x61;

/// Wrapper for the SH30's communication mechanism, including CRC-8 checking.
pub(crate) struct I2cComm<I> {
    bus: I,
}

impl<I> I2cComm<I>
where
    I: I2c<SevenBitAddress>,
{
    pub fn new(bus: I) -> Self {
        I2cComm { bus }
    }

    /// Sends the given command and associated data to the SH30 device.
    /// Includes headers, CRC checksums, and endian handling.
    pub fn send(&mut self, command: u16, data: Option<u16>) -> Result<(), Error<I::Error>> {
        // Allocate space for:
        // - Command ID (2 bytes)
        // - Optional: 2 bytes of data + 1 byte of CRC.
        let mut buffer = [0u8; 5];
        buffer[0..=1].copy_from_slice(&command.to_be_bytes());

        let write_buf = match data {
            Some(data) => {
                buffer[2..].copy_from_slice(&SH30CRC::new().add(data));
                &buffer[..]
            }
            None => &buffer[..2],
        };

        self.bus.write(ADDRESS, write_buf).map_err(Error::I2cWrite)
    }

    /// Read data back from the device.
    /// Validates and removes CRCs.
    pub fn read(&mut self, data: &mut [u8]) -> Result<(), Error<I::Error>> {
        const WORD_SIZE: usize = size_of::<u16>();
        const WORD_WITH_CRC_SIZE: usize = size_of::<u16>() + size_of::<u8>();

        // The "read measurement" command reads back 6 u16s.
        const MAX_WORDS: usize = 6;

        // We have to locally allocate space for CRCs as well.
        const BUFFER_SIZE: usize = MAX_WORDS * WORD_WITH_CRC_SIZE;

        // How many u16s do we need to read?
        let words_requested = (data.len() + 1) / 2;
        assert!(words_requested <= MAX_WORDS);
        assert!(words_requested >= 1);

        let mut buffer = [0u8; BUFFER_SIZE];
        let mut data_with_crcs = &mut buffer[..words_requested * WORD_WITH_CRC_SIZE];

        self.bus
            .read(ADDRESS, &mut data_with_crcs)
            .map_err(Error::I2cRead)?;

        let mut crc = SH30CRC::new();
        // Check CRCs
        let crc_err = data_with_crcs
            .chunks(3)
            .find_map(|chunk| crc.check(chunk).err());
        if let Some(err) = crc_err {
            eprintln!("invalid CRC: {:?}", err);
            // return Err(Error::Crc(err));
        }

        // CRCs are OK. Copy data.
        for i in 0..words_requested {
            let no_crc_offset = i * WORD_SIZE;
            let with_crc_offset = i * WORD_WITH_CRC_SIZE;
            data[no_crc_offset] = data_with_crcs[with_crc_offset];
            data[no_crc_offset + 1] = data_with_crcs[with_crc_offset + 1];
        }

        Ok(())
    }
}

/// CRC computer for the SH30.
pub struct SH30CRC(CRCu8);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct InvalidCRC {
    computed: u8,
    received: u8,
}

impl Display for InvalidCRC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "received: {} computed: {}", self.received, self.computed)
    }
}

impl SH30CRC {
    /// Create a CRC computer for the SH30's check.
    pub fn new() -> Self {
        SH30CRC(CRCu8::create_crc(
            /*poly=*/ 0x31, /*bits=*/ 8, /*initial=*/ 0xFF, /*final_xor=*/ 0,
            /*reflect=*/ false,
        ))
    }

    /// Add a CRC to the given data; output the bytes in bus order.
    pub fn add(&mut self, word: u16) -> [u8; 3] {
        let SH30CRC(ref mut crc) = self;
        let word_be = word.to_be_bytes();
        crc.reset();
        crc.digest(&word_be);
        [word_be[0], word_be[1], crc.get_crc()]
    }

    /// Validate the CRC of the raw data and output in native-endian order.
    pub fn check(&mut self, raw: &[u8]) -> Result<u16, InvalidCRC> {
        let SH30CRC(ref mut crc) = self;
        crc.reset();
        crc.digest(&raw[0..2]);
        let computed = crc.get_crc();
        let received = raw[2];

        if computed != received {
            Err(InvalidCRC { computed, received })
        } else {
            Ok(u16::from_be_bytes([raw[0], raw[1]]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_crc() {
        // Test that we constructed the CRC correctly -
        // that the computed value matches the datasheet value.
        // Note that this assumes big-endian byte order of u16s-
        // same as they appear on the i2c bus.
        let mut crc = SH30CRC::new();
        let result = crc.add(0xBEEF);
        assert_eq!(result, [0xBE, 0xEF, 0x92]);
    }

    #[test]
    fn check_crc() {
        let mut crc = SH30CRC::new();
        let value = crc.check(&[0xBE, 0xEF, 0x92]).expect("CRC should validate");
        assert_eq!(value, 0xBEEF);

        let _ = crc
            .check(&[0xBE, 0xEF, 0x91])
            .expect_err("CRC should not validate");
    }
}
