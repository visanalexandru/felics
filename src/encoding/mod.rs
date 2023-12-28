use crate::bitvector::BitVector;

/// Appends the rice encoded number to the given bitvector.
///
/// For more information on rice coding, see: [Golumb Coding](https://en.wikipedia.org/wiki/Golomb_coding)
pub fn encode_rice(bitvector: &mut BitVector, number: u32, k: u32) {
    let m = 1u32.checked_shl(k).expect("k is too big!");

    let mask_first_k = m - 1;

    let quotient = number >> k;
    let remainder = number & (mask_first_k);

    // Encode the quotient in unary.
    for _ in 0..quotient {
        bitvector.push(true);
    }
    bitvector.push(false);

    // Now encode the remainder using k bits.
    for bit in (0..k).rev() {
        let mask = 1 << bit;
        if (remainder & mask) == mask {
            bitvector.push(true);
        } else {
            bitvector.push(false);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{encode_rice, BitVector};
    #[test]
    fn test_rice_encoding() {
        let mut bitvec = BitVector::new();

        encode_rice(&mut bitvec, 7, 4);
        let contained: Vec<u32> = bitvec.iter().map(|bit| bit as u32).collect();
        assert_eq!(contained, vec![0, 0, 1, 1, 1]);

        bitvec.clear();

        encode_rice(&mut bitvec, 12, 0);
        let contained: Vec<u32> = bitvec.iter().map(|bit| bit as u32).collect();
        assert_eq!(contained, vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0]);

        bitvec.clear();

        encode_rice(&mut bitvec, 10, 3);
        let contained: Vec<u32> = bitvec.iter().map(|bit| bit as u32).collect();
        assert_eq!(contained, vec![1, 0, 0, 1, 0]);
    }
    #[test]
    #[should_panic]
    fn test_rice_panic() {
        let mut bitvec = BitVector::new();
        encode_rice(&mut bitvec, 10, 32);
    }
}
