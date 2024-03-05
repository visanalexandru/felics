use std::cmp;
use std::fmt;
const BITS_PER_BYTE: usize = u8::BITS as usize;

/// A data structure that supports inserting individual bits and iterating over them.
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

    /// Returns the number of bytes used.
    pub fn num_bytes(&self) -> usize {
        self.data.len()
    }

    /// Returns the number of bits stored in the `BitVector`
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the `BitVector` contains no bits.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Pushes a new bit at the end of the `BitVector`.
    pub fn push(&mut self, bit: bool) {
        let bit_position = self.len % BITS_PER_BYTE;
        if bit_position == 0 {
            self.data.push(0);
        }

        if bit {
            let bitmask = 1 << bit_position;
            let last = self.data.last_mut().unwrap();
            *last |= bitmask;
        }

        self.len += 1;
    }

    /// Push the last `n` significant bits of the given bitmask at the
    /// end of the bitvector.
    ///
    /// # Panics
    ///
    /// Panics if `n` exceedes `u32::BITS`.
    pub fn pushn(&mut self, mut n: u8, mut bitmask: u32) {
        assert!(n <= u32::BITS as u8, "n is too big!");

        let mut bit_position = self.len % BITS_PER_BYTE;
        while n > 0 {
            if bit_position == 0 {
                self.data.push(0);
            }

            let remaining_in_chunk = BITS_PER_BYTE - bit_position;
            let num_bits_to_mask = cmp::min(remaining_in_chunk as u8, n);

            let mask_just_enough = (1u32 << num_bits_to_mask) - 1;
            let to_append = bitmask & mask_just_enough;

            let last = self.data.last_mut().unwrap();
            *last |= (to_append as u8) << bit_position;

            n -= num_bits_to_mask;
            bitmask >>= num_bits_to_mask;
            self.len += num_bits_to_mask as usize;
            bit_position = 0;
        }
    }

    /// Appends `n` toggled bits at the end of the `BitVector`.
    pub fn pushn_toggled(&mut self, mut n: u32) {
        let mut bit_position = self.len % BITS_PER_BYTE;
        while n > 0 {
            if bit_position == 0 {
                self.data.push(0);
            }

            let remaining_in_chunk = (BITS_PER_BYTE - bit_position) as u32;
            let num_ones_to_add = cmp::min(remaining_in_chunk, n);

            let start = bit_position as u8;
            let end = start + (num_ones_to_add as u8) - 1;

            let mask_segment = bitmask_segment(start, end);

            let last = self.data.last_mut().unwrap();
            *last |= mask_segment;

            n -= num_ones_to_add;
            bit_position = 0;
            self.len += num_ones_to_add as usize;
        }
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

    /// Returns the underlying raw buffer.
    pub fn as_raw_bytes(&self) -> &Vec<u8> {
        &self.data
    }

    /// Returns the underlying raw buffer.
    pub fn into_raw_bytes(self) -> Vec<u8> {
        self.data
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
        self.next()
    }
}

/// Returns an `u8` bitmask masking bits in the range `start..=end`.
///
/// # Panics
///
/// Panics if `start` or `end` are greater or equal to `u8::BITS`.
/// Also panic if `start > end`.
fn bitmask_segment(start: u8, end: u8) -> u8 {
    assert!(start < u8::BITS as u8);
    assert!(end < u8::BITS as u8);
    assert!(start <= end);

    let segment_length = (end - start + 1) as u32;
    let bitmask_segment = ((1u32 << segment_length) - 1) as u8;
    bitmask_segment << start
}

impl<'a> Iter<'a> {
    /// Returns the next bit in the `BitVector`, or `None` if the iterator
    /// reached the end of the `BitVector`. Also advances the iterator by
    /// a bit.
    pub fn next(&mut self) -> Option<bool> {
        if self.position >= self.v.len {
            return None;
        }

        let byte = self.position / BITS_PER_BYTE;
        let bit_position = self.position % BITS_PER_BYTE;
        let bitmask: u8 = 1 << bit_position;
        let bit = self.v.data[byte] & bitmask;

        self.position += 1;

        if bit != 0 {
            Some(true)
        } else {
            Some(false)
        }
    }

    /// If there are more than `n` bits to iterate on, reads the next `n`
    /// bits in the bitvector and returns a bitmask containing
    /// the `n` bits, while advancing the iterator by `n` positions.
    /// Otherwise, returns `None`.
    ///
    /// # Panics
    ///
    /// Panics if `n` exceedes `u32::BITS`.
    pub fn nextn(&mut self, mut n: u8) -> Option<u32> {
        assert!(n <= u32::BITS as u8, "n is too big!");

        if n == 0 {
            return Some(0);
        }

        if self.position + (n as usize) - 1 >= self.v.len {
            return None;
        }

        let mut byte = self.position / BITS_PER_BYTE;
        let mut bit_position = self.position % BITS_PER_BYTE;

        let mut result = 0;
        let mut masked_count = 0;

        while n > 0 {
            let remaining_in_chunk = BITS_PER_BYTE - bit_position;
            let num_bits_to_mask = cmp::min(remaining_in_chunk as u8, n);

            let start = bit_position as u8;
            let end = start + num_bits_to_mask - 1;

            let mask_segment = bitmask_segment(start, end);
            let to_append = ((self.v.data[byte] & mask_segment) >> start) as u32;

            result |= to_append << masked_count;
            masked_count += num_bits_to_mask;

            n -= num_bits_to_mask;
            byte += 1;
            self.position += num_bits_to_mask as usize;
            bit_position = 0;
        }

        Some(result)
    }
}

#[cfg(test)]
mod test {
    use super::{bitmask_segment, BitVector};

    #[test]
    fn test_push_three_bits() {
        let mut bitvector = BitVector::new();
        assert!(bitvector.len() == 0);
        assert!(bitvector.is_empty());
        bitvector.push(true);
        bitvector.push(false);
        bitvector.push(true);
        assert!(bitvector.len() == 3);
        assert!(!bitvector.is_empty());

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

    #[test]
    fn test_pushn() {
        let mut bitvector = BitVector::new();
        bitvector.push(true);
        bitvector.push(false);
        bitvector.push(false);
        bitvector.push(true);
        bitvector.push(false);
        bitvector.pushn(12, 0b110100100110);
        assert_eq!(bitvector.to_string(), "10010011001001011");
        bitvector.clear();

        bitvector.push(false);
        bitvector.push(true);
        bitvector.push(false);
        bitvector.pushn(32, u32::MAX);
        assert_eq!(bitvector.to_string(), "01011111111111111111111111111111111");
        bitvector.clear();

        bitvector.pushn(0, 100);
        assert!(bitvector.is_empty());

        bitvector.pushn(1, 123);
        assert_eq!(bitvector.to_string(), "1");
        bitvector.clear();

        bitvector.pushn(8, 0b10100110);
        assert_eq!(bitvector.to_string(), "01100101");
        bitvector.pushn(7, 0b0100101);
        assert_eq!(bitvector.to_string(), "011001011010010");
        bitvector.pushn(16, 0b1101011110111010);
        assert_eq!(bitvector.to_string(), "0110010110100100101110111101011");
    }

    #[test]
    fn test_bitmask_segment() {
        assert_eq!(bitmask_segment(0, 3), 0b00001111);
        assert_eq!(bitmask_segment(1, 3), 0b00001110);
        assert_eq!(bitmask_segment(1, 7), 0b11111110);
        assert_eq!(bitmask_segment(2, 7), 0b11111100);
        assert_eq!(bitmask_segment(3, 7), 0b11111000);
        assert_eq!(bitmask_segment(3, 6), 0b01111000);
        assert_eq!(bitmask_segment(3, 5), 0b00111000);
        assert_eq!(bitmask_segment(3, 4), 0b00011000);
        assert_eq!(bitmask_segment(3, 3), 0b00001000);
        assert_eq!(bitmask_segment(7, 7), 0b10000000);
        assert_eq!(bitmask_segment(0, 7), 0b11111111);
    }

    #[test]
    fn test_read_nextn() {
        let mut bitvector = BitVector::new();
        bitvector.pushn(11, 0b11000101001);
        bitvector.pushn(29, 0b01110110110111010011101111010);
        bitvector.pushn(1, 0b0);

        let mut i = bitvector.iter();
        assert_eq!(Some(0b101001), i.nextn(6));
        assert_eq!(Some(0b11000), i.nextn(5));
        assert_eq!(Some(0b011101111010), i.nextn(12));
        assert_eq!(Some(0b01110110110111010), i.nextn(17));
        assert_eq!(Some(0b0), i.nextn(1));
        assert_eq!(Some(0), i.nextn(0));
        assert_eq!(None, i.nextn(1));
        bitvector.clear();

        bitvector.pushn(17, 0b001);
        let mut i = bitvector.iter();
        assert_eq!(Some(0b001), i.nextn(3));
        assert_eq!(Some(0b000000000000), i.nextn(12));
        assert_eq!(None, i.nextn(3));
        assert_eq!(Some(0b00), i.nextn(2));

        bitvector.clear();
        let mut i = bitvector.iter();
        assert_eq!(Some(0), i.nextn(0));
    }

    #[test]
    fn test_pushn_toggled() {
        let mut bitvector = BitVector::new();
        bitvector.pushn_toggled(3);
        assert_eq!(bitvector.len(), 3);
        assert_eq!(bitvector.data, vec![7]);

        bitvector.pushn_toggled(6);
        assert_eq!(bitvector.len(), 9);
        assert_eq!(bitvector.data, vec![255, 1]);

        bitvector.pushn_toggled(7);
        assert_eq!(bitvector.len(), 16);
        assert_eq!(bitvector.data, vec![255, 255]);

        bitvector.clear();
        bitvector.pushn_toggled(45);
        assert_eq!(bitvector.len(), 45);
        assert_eq!(bitvector.data, vec![255, 255, 255, 255, 255, 31]);

        bitvector.pushn_toggled(2);
        assert_eq!(bitvector.data, vec![255, 255, 255, 255, 255, 127]);
        assert_eq!(bitvector.len(), 47);

        bitvector.pushn_toggled(1);
        assert_eq!(bitvector.data, vec![255, 255, 255, 255, 255, 255]);
        assert_eq!(bitvector.len(), 48);

        bitvector.pushn_toggled(0);
        assert_eq!(bitvector.data, vec![255, 255, 255, 255, 255, 255]);
        assert_eq!(bitvector.len(), 48);
    }
}
