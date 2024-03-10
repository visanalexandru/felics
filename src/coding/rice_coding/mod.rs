use bitstream_io::{BitRead, BitWrite};
use std::io;

/// A struct that is used to encode numbers using rice coding.
///
/// For more information on rice coding, see: [Golumb Coding](https://en.wikipedia.org/wiki/Golomb_coding)
pub struct RiceCoder {
    k: u8,
    m: u32,
    mask_first_k: u32,
}

impl RiceCoder {
    /// Creates a new RiceCoder for m = 2^k.
    ///
    /// # Panics
    ///
    /// Panics if k is greater than 31.
    pub fn new(k: u8) -> RiceCoder {
        let m = 1u32.checked_shl(k as u32).expect("k is too big!");
        let mask_first_k = m - 1;
        RiceCoder { k, m, mask_first_k }
    }

    /// Writes the rice encoded number to the given `BitWrite`.
    pub fn encode<T>(&self, bitwrite: &mut T, number: u32) -> io::Result<()>
    where
        T: BitWrite,
    {
        let quotient = number >> self.k;
        let remainder = number & self.mask_first_k;

        // Encode the quotient in unary.
        bitwrite.write_unary0(quotient)?;
        // Now encode the remainder using k bits.
        bitwrite.write(self.k as u32, remainder)?;

        Ok(())
    }

    /// Decodes an encoded rice number by reading from the provided the `BitRead`.
    pub fn decode<T>(&self, bitread: &mut T) -> io::Result<u32>
    where
        T: BitRead,
    {
        let quotient: u32 = bitread.read_unary0()?;
        let remainder: u32 = bitread.read(self.k as u32)?;

        let result = quotient.checked_mul(self.m).unwrap() + remainder;
        Ok(result)
    }

    /// Returns the length of the rice code of the given number
    /// The method doesn't actually encode the number to count the bitsize,
    /// so it's fast.
    pub fn code_length(&self, number: u32) -> u32 {
        (number >> self.k) + 1 + (self.k as u32)
    }
}
