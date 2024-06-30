use bitstream_io::{BitWrite, Endianness, Numeric, Primitive, SignedNumeric};
use std::io;

/// Mock struct used to test the coding schemes.
/// Log bits to a string rather than writing them to a sink.
pub struct BitWriterMock {
    content: String,
}

impl BitWriterMock {
    pub fn new() -> BitWriterMock {
        BitWriterMock {
            content: String::new(),
        }
    }
    pub fn content(self) -> String {
        self.content
    }
}

impl BitWrite for BitWriterMock {
    fn write_bit(&mut self, bit: bool) -> io::Result<()> {
        match bit {
            true => self.content.push('1'),
            false => self.content.push('0'),
        }
        Ok(())
    }

    fn write<U>(&mut self, bits: u32, mut value: U) -> io::Result<()>
    where
        U: Numeric,
    {
        for _ in 0..bits {
            let bit = value % (U::ONE << 1);
            value >>= 1;

            self.write_bit(bit == U::ONE)?;
        }
        Ok(())
    }

    fn byte_align(&mut self) -> io::Result<()> {
        while self.content.len() % 8 != 0 {
            self.write_bit(false)?;
        }
        Ok(())
    }

    fn write_signed<S>(&mut self, _bits: u32, _value: S) -> io::Result<()>
    where
        S: SignedNumeric,
    {
        todo!();
    }

    fn write_as_from<F, V>(&mut self, _value: V) -> io::Result<()>
    where
        F: Endianness,
        V: Primitive,
    {
        todo!();
    }

    fn byte_aligned(&self) -> bool {
        todo!();
    }

    fn write_from<V>(&mut self, _value: V) -> io::Result<()>
    where
        V: Primitive,
    {
        todo!();
    }

    fn write_out<const BITS: u32, U>(&mut self, _value: U) -> io::Result<()>
    where
        U: Numeric,
    {
        todo!();
    }

    fn write_signed_out<const BITS: u32, S>(&mut self, _value: S) -> io::Result<()>
    where
        S: SignedNumeric,
    {
        todo!();
    }
}
