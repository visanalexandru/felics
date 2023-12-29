use crate::bitvector::BitVector;
/// A struct that is used to encode and decode phase-in codes for the numbers in the `[left, right]` range.
///
/// Phased-in codes are for symbols with equal probabilities.
/// If the size of the set of symbols is not a power of two, we may assign some symbols less bits.
///
/// Check out [Phased-In Codes](https://www.davidsalomon.name/VLCadvertis/phasedin.pdf) for more information.
pub struct PhaseInCoder {
    left: u32,
    right: u32,
    m: u32,
    left_p: u32,
    right_p: u32,
}

impl PhaseInCoder {
    /// Constructs a phase-in coder for the given range: `[left, right]`.
    pub fn new(left: u32, right: u32) -> PhaseInCoder {
        assert!(right >= left);

        let range_length: u32 = right - left + 1;
        let m = range_length.ilog2();

        // Compute neighbouring powers of two.
        let lpw: u32 = 1 << m;
        let rpw: u32 = 1 << (m + 1);

        PhaseInCoder {
            left,
            right,
            m,
            left_p: range_length - lpw,
            right_p: rpw - range_length,
        }
    }
    pub fn encode(&self, bitvector: &mut BitVector, number: u32) {
        assert!(self.left <= number && number <= self.right);
        let to_encode = number - self.left;

        // The first P integers: [0, P - 1] receive short codewords (m bits).
        if to_encode < self.right_p {
            for bit in (0..self.m).rev() {
                let mask = 1 << bit;
                if (to_encode & mask) == mask {
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
            let pair = (to_encode - self.right_p) / 2;
            let last_bit = (to_encode - self.right_p) % 2;
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
}

#[cfg(test)]
mod test {
    use crate::bitvector::BitVector;

    use super::PhaseInCoder;

    // Taken from the dummy chapter in the phase-in coding article.
    #[test]
    fn test_new_coder() {
        // 7 values
        let coder = PhaseInCoder::new(0, 6);
        assert_eq!(coder.left, 0);
        assert_eq!(coder.right, 6);
        assert_eq!(coder.m, 2);
        assert_eq!(coder.left_p, 3);
        assert_eq!(coder.right_p, 1);

        // 15 values
        let coder = PhaseInCoder::new(24, 38);
        assert_eq!(coder.left, 24);
        assert_eq!(coder.right, 38);
        assert_eq!(coder.m, 3);
        assert_eq!(coder.left_p, 7);
        assert_eq!(coder.right_p, 1);

        // 32 values
        let coder = PhaseInCoder::new(100, 131);
        assert_eq!(coder.left, 100);
        assert_eq!(coder.right, 131);
        assert_eq!(coder.m, 5);
        assert_eq!(coder.left_p, 0);
        assert_eq!(coder.right_p, 32);
    }

    // Utility function to compute the phase in codes of the set [left, right]
    fn get_phase_in_codes(left: u32, right: u32) -> Vec<String> {
        let coder = PhaseInCoder::new(left, right);
        let mut codes = Vec::new();

        for number in left..=right {
            let mut bitvec = BitVector::new();
            coder.encode(&mut bitvec, number);
            codes.push(bitvec.to_string());
        }
        codes
    }

    #[test]
    fn test_phase_in_encoding() {
        assert_eq!(
            get_phase_in_codes(0, 6),
            vec!["00", "010", "011", "100", "101", "110", "111"]
        );
        // Same range length as above, but shifted range.
        assert_eq!(
            get_phase_in_codes(23, 29),
            vec!["00", "010", "011", "100", "101", "110", "111"]
        );

        assert_eq!(
            get_phase_in_codes(0, 7),
            vec!["000", "001", "010", "011", "100", "101", "110", "111"]
        );
        assert_eq!(
            get_phase_in_codes(230, 237),
            vec!["000", "001", "010", "011", "100", "101", "110", "111"]
        );

        assert_eq!(
            get_phase_in_codes(0, 8),
            vec!["000", "001", "010", "011", "100", "101", "110", "1110", "1111"]
        );
        assert_eq!(
            get_phase_in_codes(44, 52),
            vec!["000", "001", "010", "011", "100", "101", "110", "1110", "1111"]
        );

        assert_eq!(
            get_phase_in_codes(0, 14),
            vec![
                "000", "0010", "0011", "0100", "0101", "0110", "0111", "1000", "1001", "1010",
                "1011", "1100", "1101", "1110", "1111"
            ]
        );
        assert_eq!(
            get_phase_in_codes(2314, 2328),
            vec![
                "000", "0010", "0011", "0100", "0101", "0110", "0111", "1000", "1001", "1010",
                "1011", "1100", "1101", "1110", "1111"
            ]
        );

        assert_eq!(
            get_phase_in_codes(0, 15),
            vec![
                "0000", "0001", "0010", "0011", "0100", "0101", "0110", "0111", "1000", "1001",
                "1010", "1011", "1100", "1101", "1110", "1111"
            ]
        );
        assert_eq!(
            get_phase_in_codes(1246, 1261),
            vec![
                "0000", "0001", "0010", "0011", "0100", "0101", "0110", "0111", "1000", "1001",
                "1010", "1011", "1100", "1101", "1110", "1111"
            ]
        );
    }
}
