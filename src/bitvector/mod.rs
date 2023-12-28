use std::fmt;
const BITS_PER_BYTE: usize = 8;

/// A data structure that supports storing individual bits.
pub struct BitVector {
    data: Vec<u8>,
    len: usize,
}

impl BitVector {
    /// Constructs a new, empty `BitVector`
    pub fn new() -> BitVector {
        BitVector {
            data: Vec::new(),
            len: 0,
        }
    }

    /// Returns the bitmask associated to the given position in a byte.
    fn bitmask(position: u32) -> Option<u8> {
        1u8.checked_shl(position)
    }

    /// Returns the number of bits stored in the `BitVector`
    pub fn len(&self) -> usize {
        self.len
    }

    /// Pushes a new bit at the end of the `BitVector`.
    pub fn push(&mut self, bit: bool) {
        let bit_position = (self.len % BITS_PER_BYTE) as u32;
        if bit_position == 0 {
            self.data.push(0);
        }

        if bit {
            let bitmask = BitVector::bitmask(bit_position).unwrap();
            let last = self.data.last_mut().unwrap();
            *last |= bitmask;
        }

        self.len += 1;
    }

    /// Constructs a new iterator over the bits in the `BitVector`.
    pub fn iter(&self) -> Iter {
        return Iter {
            v: self,
            position: 0,
        };
    }

    /// Clears the `BitVector`, removing all bits.
    pub fn clear(&mut self) {
        self.data.clear();
        self.len = 0;
    }
}

impl fmt::Display for BitVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for bit in self.iter() {
            if bit {
                write!(f, "1")?;
            } else {
                write!(f, "0")?;
            }
        }
        return Ok(());
    }
}

/// Iterator over the `BitVector`
pub struct Iter<'a> {
    v: &'a BitVector,
    position: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.v.len {
            return None;
        }

        let byte = self.position / BITS_PER_BYTE;
        let bit_position = (self.position % BITS_PER_BYTE) as u32;
        let bitmask: u8 = BitVector::bitmask(bit_position).unwrap();
        let bit = self.v.data[byte] & bitmask;

        self.position += 1;

        if bit != 0 {
            Some(true)
        } else {
            Some(false)
        }
    }
}

#[cfg(test)]
mod test {
    use super::BitVector;
    #[test]
    fn test_bitmask() {
        assert_eq!(BitVector::bitmask(0), Some(0b00000001));
        assert_eq!(BitVector::bitmask(1), Some(0b00000010));
        assert_eq!(BitVector::bitmask(2), Some(0b00000100));
        assert_eq!(BitVector::bitmask(3), Some(0b00001000));
        assert_eq!(BitVector::bitmask(4), Some(0b00010000));
        assert_eq!(BitVector::bitmask(5), Some(0b00100000));
        assert_eq!(BitVector::bitmask(6), Some(0b01000000));
        assert_eq!(BitVector::bitmask(7), Some(0b10000000));
        assert_eq!(BitVector::bitmask(8), None);
    }

    #[test]
    fn test_push_three_bits() {
        let mut bitvector = BitVector::new();
        assert!(bitvector.len() == 0);
        bitvector.push(true);
        bitvector.push(false);
        bitvector.push(true);
        assert!(bitvector.len() == 3);

        let bits: Vec<bool> = bitvector.iter().collect();
        assert_eq!(bits, vec![true, false, true]);
    }

    #[test]
    fn test_push_multiple_bytes() {
        let mut bitvector = BitVector::new();

        let bits = vec![
            true, true, false, true, true, true, false, false, true, false, true, true, false,
            false, false, true, true, true, false, false,
        ];
        bits.iter().for_each(|x| bitvector.push(*x));

        let contained: Vec<bool> = bitvector.iter().collect();
        assert_eq!(bits, contained);
    }

    #[test]
    fn test_clear() {
        let mut bitvector = BitVector::new();
        let bits = vec![
            true, true, false, true, true, true, false, false, true, false, true, true, false,
            false, false, true, true, true, false, false,
        ];
        bits.iter().for_each(|x| bitvector.push(*x));
        bitvector.clear();

        assert_eq!(bitvector.len(), 0);
        let contained: Vec<bool> = bitvector.iter().collect();
        assert!(contained.is_empty());
    }

    #[test]
    fn test_to_string() {
        let mut bitvector = BitVector::new();
        bitvector.push(true);
        bitvector.push(false);
        bitvector.push(false);
        bitvector.push(true);
        bitvector.push(false);
        bitvector.push(false);
        assert_eq!(bitvector.to_string(), "100100");
    }
}
