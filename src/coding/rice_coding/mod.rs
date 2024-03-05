use crate::bitvector::{self, BitVector};

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

    /// Appends the rice encoded number to the given bitvector.
    pub fn encode_rice(&self, bitvector: &mut BitVector, number: u32) {
        let quotient = number >> self.k;
        let remainder = number & self.mask_first_k;

        // Encode the quotient in unary.
        bitvector.pushn_toggled(quotient);
        bitvector.push(false);

        // Now encode the remainder using k bits.
        bitvector.pushn(self.k, remainder);
    }
    /// Decodes an encoded rice number by advancing the `BitVector` iterator.
    ///
    /// Returns `None` if the decoding process failed, caused by a truncated input.
    pub fn decode_rice(&self, iter: &mut bitvector::Iter) -> Option<u32> {
        let mut quotient: u32 = 0;
        // Loop to decode the unary quotient.
        loop {
            let bit = iter.next()?;
            if !bit {
                break;
            }
            quotient += 1;
        }

        let remainder: u32 = iter.nextn(self.k)?;

        let result = quotient.checked_mul(self.m).unwrap() + remainder;
        Some(result)
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
    use super::{BitVector, RiceCoder};
    use rand::seq::SliceRandom;
    #[test]
    fn test_rice_encoding() {
        let mut bitvec = BitVector::new();

        RiceCoder::new(4).encode_rice(&mut bitvec, 7);
        let contained: Vec<u32> = bitvec.iter().map(|bit| bit as u32).collect();
        assert_eq!(contained, vec![0, 1, 1, 1, 0]);

        bitvec.clear();

        RiceCoder::new(0).encode_rice(&mut bitvec, 12);
        let contained: Vec<u32> = bitvec.iter().map(|bit| bit as u32).collect();
        assert_eq!(contained, vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0]);

        bitvec.clear();

        RiceCoder::new(3).encode_rice(&mut bitvec, 10);
        let contained: Vec<u32> = bitvec.iter().map(|bit| bit as u32).collect();
        assert_eq!(contained, vec![1, 0, 0, 1, 0]);
    }
    #[test]
    #[should_panic]
    fn test_rice_panic() {
        let _coder = RiceCoder::new(32);
    }

    #[test]
    fn test_rice_decoding() {
        let mut bitvec = BitVector::new();

        let (a, b, c) = (RiceCoder::new(4), RiceCoder::new(0), RiceCoder::new(3));

        a.encode_rice(&mut bitvec, 7);
        b.encode_rice(&mut bitvec, 12);
        c.encode_rice(&mut bitvec, 10);

        let mut iter = bitvec.iter();

        assert_eq!(a.decode_rice(&mut iter), Some(7));
        assert_eq!(b.decode_rice(&mut iter), Some(12));
        assert_eq!(c.decode_rice(&mut iter), Some(10));
        assert_eq!(b.decode_rice(&mut iter), None);
    }

    #[test]
    #[ignore]
    fn test_rice_decoding_extensive() {
        let mut bitvec = BitVector::new();
        let mut numbers: Vec<u32> = (0..3000).collect();
        numbers.shuffle(&mut rand::thread_rng());

        for number in &numbers {
            for k in 0..32 {
                let coder = RiceCoder::new(k);
                coder.encode_rice(&mut bitvec, *number);
            }
        }

        let mut iter = bitvec.iter();
        for number in &numbers {
            for k in 0..32 {
                let coder = RiceCoder::new(k);
                let decoded = coder.decode_rice(&mut iter);
                assert_eq!(decoded, Some(*number));
            }
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
                let mut bitvec = BitVector::new();
                coder.encode_rice(&mut bitvec, number);
                assert_eq!(bitvec.len(), coder.code_length(number) as usize);
            }
        }
    }
}
