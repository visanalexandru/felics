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

#[cfg(test)]
mod test {
    use super::*;
    use crate::coding::bitwrite_mock::BitWriterMock;
    use bitstream_io::{BigEndian, BitCounter, BitReader, BitWriter};
    use rand::seq::SliceRandom;
    use std::io::Cursor;

    #[test]
    fn test_rice_encoding() {
        let mut bitwriter = BitWriterMock::new();
        RiceCoder::new(4).encode(&mut bitwriter, 7).unwrap();
        assert_eq!(bitwriter.content(), "01110");

        let mut bitwriter = BitWriterMock::new();
        RiceCoder::new(0).encode(&mut bitwriter, 12).unwrap();
        assert_eq!(bitwriter.content(), "1111111111110");

        let mut bitwriter = BitWriterMock::new();
        RiceCoder::new(3).encode(&mut bitwriter, 10).unwrap();
        assert_eq!(bitwriter.content(), "10010");
    }

    #[test]
    #[should_panic]
    fn test_rice_panic() {
        let _ = RiceCoder::new(32);
    }

    #[test]
    fn test_rice_decoding() {
        let mut to = Vec::new();
        let mut bitwriter = BitWriter::<_, BigEndian>::new(&mut to);

        let (a, b, c) = (RiceCoder::new(4), RiceCoder::new(0), RiceCoder::new(3));

        a.encode(&mut bitwriter, 7).unwrap();
        b.encode(&mut bitwriter, 12).unwrap();
        c.encode(&mut bitwriter, 10).unwrap();
        bitwriter.byte_align().unwrap();

        let mut from = BitReader::<_, BigEndian>::new(Cursor::new(&to));

        assert_eq!(a.decode(&mut from).unwrap(), 7);
        assert_eq!(b.decode(&mut from).unwrap(), 12);
        assert_eq!(c.decode(&mut from).unwrap(), 10);
    }

    #[test]
    #[ignore]
    fn test_rice_decoding_extensive() {
        let mut to = Vec::new();
        let mut bitwriter = BitWriter::<_, BigEndian>::new(&mut to);

        let mut numbers: Vec<u32> = (0..(u16::MAX as u32 * 2)).collect();
        numbers.shuffle(&mut rand::thread_rng());

        let k = 8;
        let coder = RiceCoder::new(k);

        for number in &numbers {
            coder.encode(&mut bitwriter, *number).unwrap();
        }

        bitwriter.byte_align().unwrap();

        let mut from = BitReader::<_, BigEndian>::new(Cursor::new(&to));
        for number in &numbers {
            let decoded = coder.decode(&mut from).unwrap();
            assert_eq!(decoded, *number);
        }
    }

    // Encode some numbers using multiple k values and check
    // if the length of the encoding matches the fast
    // code length method.
    #[test]
    fn test_rice_code_length() {
        for number in 0..3000 {
            for k in 0..32 {
                let coder = RiceCoder::new(k);
                let mut bitcounter = BitCounter::<u32, BigEndian>::new();

                coder.encode(&mut bitcounter, number).unwrap();
                assert_eq!(bitcounter.written(), coder.code_length(number));
            }
        }
    }
}
