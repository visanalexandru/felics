use crate::bitvector::{self, BitVector};
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

/// Decodes an encoded rice number by advancing the `BitVector` iterator.
///
/// Returns `None` if the decoding process failed, caused by a truncated input.
pub fn decode_rice(iter: &mut bitvector::Iter, k: u32) -> Option<u32> {
    let m = 1u32.checked_shl(k).expect("k is too big!");

    let mut quotient: u32 = 0;
    // Loop to decode the unary quotient.
    loop {
        let bit = iter.next()?;
        if !bit {
            break;
        }
        quotient += 1;
    }

    let mut remainder: u32 = 0;
    for bit in (0..k).rev() {
        let mask = 1 << bit;
        let is_toggled = iter.next()?;
        if is_toggled {
            remainder += mask;
        }
    }

    let result = quotient.checked_mul(m).unwrap() + remainder;
    Some(result)
}

#[cfg(test)]
mod test {
    use super::{decode_rice, encode_rice, BitVector};
    use rand::seq::SliceRandom;
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

    #[test]
    fn test_rice_decoding() {
        let mut bitvec = BitVector::new();

        encode_rice(&mut bitvec, 7, 4);
        encode_rice(&mut bitvec, 12, 0);
        encode_rice(&mut bitvec, 10, 3);

        let mut iter = bitvec.iter();
        let a = decode_rice(&mut iter, 4);
        let b = decode_rice(&mut iter, 0);
        let c = decode_rice(&mut iter, 3);

        assert_eq!(a, Some(7));
        assert_eq!(b, Some(12));
        assert_eq!(c, Some(10));
    }

    #[test]
    #[ignore]
    fn test_rice_decoding_extensive() {
        let mut bitvec = BitVector::new();
        let mut numbers: Vec<u32> = (0..3000).collect();
        numbers.shuffle(&mut rand::thread_rng());

        for number in &numbers {
            for k in 0..32 {
                encode_rice(&mut bitvec, *number, k);
            }
        }

        let mut iter = bitvec.iter();
        for number in &numbers {
            for k in 0..32 {
                let decoded = decode_rice(&mut iter, k);
                assert_eq!(decoded, Some(*number));
            }
        }
    }
}
