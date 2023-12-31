use crate::bitvector::{self, BitVector};

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

    /// Appends the phase-in coding of a number in the range `[0, n-1]` to the given bitvector.
    pub fn encode(&self, bitvector: &mut BitVector, number: u32) {
        assert!(number < self.n);

        // The first P integers: [0, P - 1] receive short codewords (m bits).
        if number < self.right_p {
            for bit in (0..self.m).rev() {
                let mask = 1 << bit;
                if (number & mask) == mask {
                    bitvector.push(true);
                } else {
                    bitvector.push(false);
                }
            }
        }
        // The remaining 2*p integers [P, n-1] are assigned the long codewords (m+1 bits).
        // The long codewords consist of p pairs, where each pair starts with the m-bit value
        // P, P+1, ...., P+p-1, followed by an extra bit (0 or 1) to distinguish between the two
        // codewords of a pair.
        else {
            let pair = (number - self.right_p) / 2;
            let last_bit = (number - self.right_p) % 2;
            let to_encode = pair + self.right_p;

            for bit in (0..self.m).rev() {
                let mask = 1 << bit;
                if (to_encode & mask) == mask {
                    bitvector.push(true);
                } else {
                    bitvector.push(false);
                }
            }
            bitvector.push(if last_bit == 1 { true } else { false });
        }
    }

    /// Decodes the phase-in coding of a number in the range `[0, n-1]` by advancing the `BitVector` iterator.
    ///
    /// Returns `None` if the decoding process failed.
    pub fn decode(&self, iter: &mut bitvector::Iter) -> Option<u32> {
        // Read m bits.
        let mut first_m = 0;
        for bit in (0..self.m).rev() {
            let mask = 1 << bit;
            let is_toggled = iter.next()?;
            if is_toggled {
                first_m += mask;
            }
        }

        if first_m < self.right_p {
            return Some(first_m);
        }

        // It then must be a long codeword, get the corresponding pair.
        let pair = first_m - self.right_p;
        let mut number = pair * 2 + self.right_p;

        // Then read the next bit to get the actual number.
        let bit = iter.next()?;
        if bit {
            number += 1;
        }
        Some(number)
    }
}

#[cfg(test)]
mod test {
    use super::PhaseInCoder;
    use crate::bitvector::BitVector;
    use rand::seq::SliceRandom;

    #[test]
    #[should_panic]
    fn test_zero_n_should_panic() {
        PhaseInCoder::new(0);
    }

    // Taken from the dummy chapter in the phase-in coding article.
    #[test]
    fn test_new_coder() {
        let coder = PhaseInCoder::new(7);
        assert_eq!(coder.n, 7);
        assert_eq!(coder.m, 2);
        assert_eq!(coder.left_p, 3);
        assert_eq!(coder.right_p, 1);

        let coder = PhaseInCoder::new(15);
        assert_eq!(coder.n, 15);
        assert_eq!(coder.m, 3);
        assert_eq!(coder.left_p, 7);
        assert_eq!(coder.right_p, 1);

        let coder = PhaseInCoder::new(32);
        assert_eq!(coder.n, 32);
        assert_eq!(coder.m, 5);
        assert_eq!(coder.left_p, 0);
        assert_eq!(coder.right_p, 32);
    }

    // Utility function to compute the phase in codes of the set [0, n-1]
    fn get_phase_in_codes(n: u32) -> Vec<String> {
        let coder = PhaseInCoder::new(n);
        let mut codes = Vec::new();

        for number in 0..n {
            let mut bitvec: BitVector = BitVector::new();
            coder.encode(&mut bitvec, number);
            codes.push(bitvec.to_string());
        }
        codes
    }

    #[test]
    fn test_phase_in_encoding() {
        assert_eq!(
            get_phase_in_codes(7),
            vec!["00", "010", "011", "100", "101", "110", "111"]
        );

        assert_eq!(
            get_phase_in_codes(8),
            vec!["000", "001", "010", "011", "100", "101", "110", "111"]
        );

        assert_eq!(
            get_phase_in_codes(9),
            vec!["000", "001", "010", "011", "100", "101", "110", "1110", "1111"]
        );

        assert_eq!(
            get_phase_in_codes(15),
            vec![
                "000", "0010", "0011", "0100", "0101", "0110", "0111", "1000", "1001", "1010",
                "1011", "1100", "1101", "1110", "1111"
            ]
        );

        assert_eq!(
            get_phase_in_codes(16),
            vec![
                "0000", "0001", "0010", "0011", "0100", "0101", "0110", "0111", "1000", "1001",
                "1010", "1011", "1100", "1101", "1110", "1111"
            ]
        );

        assert_eq!(
            get_phase_in_codes(17),
            vec![
                "0000", "0001", "0010", "0011", "0100", "0101", "0110", "0111", "1000", "1001",
                "1010", "1011", "1100", "1101", "1110", "11110", "11111"
            ]
        );
    }

    #[test]
    fn test_phase_in_decoding() {
        let coder: PhaseInCoder = PhaseInCoder::new(15);
        let mut bitvec = BitVector::new();
        coder.encode(&mut bitvec, 0);
        coder.encode(&mut bitvec, 6);
        coder.encode(&mut bitvec, 12);
        coder.encode(&mut bitvec, 14);
        coder.encode(&mut bitvec, 1);

        let mut iter = bitvec.iter();
        assert_eq!(coder.decode(&mut iter), Some(0));
        assert_eq!(coder.decode(&mut iter), Some(6));
        assert_eq!(coder.decode(&mut iter), Some(12));
        assert_eq!(coder.decode(&mut iter), Some(14));
        assert_eq!(coder.decode(&mut iter), Some(1));
        assert_eq!(coder.decode(&mut iter), None);
    }

    // Enumerate possible values for n. For each domain `[0, n-1]`, shuffle the values in the domain
    // and encode them using phase-in coding. Then, decode them and check if we get the same values.
    #[test]
    #[ignore]
    fn test_phase_in_decoding_extensive() {
        for n in 1..2000 {
            let coder = PhaseInCoder::new(n);
            let mut domain: Vec<u32> = (0..n).collect();
            domain.shuffle(&mut rand::thread_rng());
            let mut bitvector = BitVector::new();

            // Encode all the values in the domain.
            for value in &domain {
                coder.encode(&mut bitvector, *value);
            }

            // Decode them an check if they are correct.
            let mut iter = bitvector.iter();
            for value in &domain {
                assert_eq!(coder.decode(&mut iter), Some(*value));
            }
        }
    }
}
