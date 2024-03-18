use bitstream_io::{BitRead, BitWrite};
use std::io;

/// A struct that is used to encode and decode phase-in codes for the numbers in the `[0, n-1]` range.
///
/// Phased-in codes are for symbols with equal probabilities.
/// If the size of the set of symbols is not a power of two, we may assign some symbols less bits.
///
/// Check out [Phased-In Codes](https://www.davidsalomon.name/VLCadvertis/phasedin.pdf) for more information.
pub struct PhaseInCoder {
    n: u32,
    m: u32,
    left_p: u32,
    right_p: u32,
}

impl PhaseInCoder {
    /// Constructs a phase-in coder for the given range: `[0, n-1]`.
    ///
    /// # Panics
    ///
    /// Panics if `n` is 0 or greater or equal to 2^31.
    pub fn new(n: u32) -> PhaseInCoder {
        let m = n.checked_ilog2().expect("n is 0!");

        // Compute neighbouring powers of two.
        let lpw: u32 = 1 << m;
        let rpw: u32 = 1u32.checked_shl(m + 1).expect("n is too big!");

        PhaseInCoder {
            n,
            m,
            left_p: n - lpw,
            right_p: rpw - n,
        }
    }

    /// Rotates all numbers in the domain `[0, n-1] to the right p positions.
    /// This is used so that values with shorter codewords end up
    /// near the middle of the range.
    ///
    /// For example, coding of the values [0, 4] is: `00, 01, 10, 110, 111`.
    /// If we rotate the values to the right p = 1 positions, we will
    /// end up with: `111, 00, 01, 10, 110`
    fn rotate_right(&self, number: u32) -> u32 {
        (number + self.n - self.left_p) % self.n
    }

    /// Opposite of `rotate_right`.
    fn rotate_left(&self, number: u32) -> u32 {
        (number + self.left_p) % self.n
    }

    /// Writes the phase-in coding of a number in the range `[0, n-1]` to the given `BitWrite`.
    ///
    /// # Panics
    ///
    /// Panics if `number` is out of range.
    pub fn encode<T>(&self, bitwrite: &mut T, number: u32) -> io::Result<()>
    where
        T: BitWrite,
    {
        assert!(number < self.n);

        let number = self.rotate_right(number);

        // The first P integers: [0, P - 1] receive short codewords (m bits).
        if number < self.right_p {
            bitwrite.write(self.m, number)?;
        }
        // The remaining 2*p integers [P, n-1] are assigned the long codewords (m+1 bits).
        // The long codewords consist of p pairs, where each pair starts with the m-bit value
        // P, P+1, ...., P+p-1, followed by an extra bit (0 or 1) to distinguish between the two
        // codewords of a pair.
        else {
            let pair = (number - self.right_p) / 2;
            let last_bit = (number - self.right_p) % 2;
            let to_encode = pair + self.right_p;

            bitwrite.write(self.m, to_encode)?;
            bitwrite.write_bit(if last_bit == 1 { true } else { false })?;
        }
        Ok(())
    }

    /// Decodes the phase-in coding of a number in the range `[0, n-1]` by reading from the
    /// provided `BitRead`.
    ///
    /// Returns `None` if the decoding process failed.
    pub fn decode<T>(&self, bitread: &mut T) -> io::Result<u32>
    where
        T: BitRead,
    {
        // Read m bits.
        let first_m = bitread.read(self.m)?;

        if first_m < self.right_p {
            return Ok(self.rotate_left(first_m));
        }

        // It then must be a long codeword, get the corresponding pair.
        let pair = first_m - self.right_p;
        let mut number = pair * 2 + self.right_p;

        // Then read the next bit to get the actual number.
        let bit = bitread.read_bit()?;
        if bit {
            number += 1;
        }

        Ok(self.rotate_left(number))
    }
}
