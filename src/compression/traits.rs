use super::error::DecompressionError;
use super::format::PixelDepth;
use bitstream_io::{BitRead, BitWrite};
use num::{CheckedAdd, CheckedSub, One, Zero};
use std::cmp::Ord;
use std::io::{self, Read, Write};

/// This trait is implemented by all types that can
/// represent a pixel intensity in a grayscale image.
pub trait Intensity:
    CheckedAdd + CheckedSub + Ord + Into<u32> + Into<usize> + TryFrom<u32> + One + Zero + Copy
{
    /// The list of reasonable k values we can use to encode
    /// this pixel intensity using rice coding.
    const K_VALUES: &'static [u8];

    /// The maximum context, as specified in the article.
    /// A pixel's context is defined as: `context = H - L`,
    /// so `MAX_CONTEXT` will be the maximum possible difference
    /// between two pixel intensities.
    const MAX_CONTEXT: usize;

    /// Halve all code lengths when the smallest value reaches this threshold.
    const COUNT_SCALING_THRESHOLD: u32;

    /// The pixel depth of this pixel intensity.
    const PIXEL_DEPTH: PixelDepth;

    fn write<T>(&self, bitwrite: &mut T) -> io::Result<()>
    where
        T: BitWrite;

    fn read<T>(bitread: &mut T) -> io::Result<Self>
    where
        T: BitRead;
}

impl Intensity for u8 {
    const K_VALUES: &'static [u8] = &[0, 1, 2, 3, 4, 5];

    const MAX_CONTEXT: usize = u8::MAX as usize;

    const COUNT_SCALING_THRESHOLD: u32 = 1024;

    const PIXEL_DEPTH: PixelDepth = PixelDepth::Eight;

    fn write<T>(&self, bitwrite: &mut T) -> io::Result<()>
    where
        T: BitWrite,
    {
        bitwrite.write(u8::BITS, *self)
    }

    fn read<T>(bitread: &mut T) -> io::Result<Self>
    where
        T: BitRead,
    {
        bitread.read(u8::BITS)
    }
}

impl Intensity for u16 {
    const K_VALUES: &'static [u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

    const MAX_CONTEXT: usize = u16::MAX as usize;

    const COUNT_SCALING_THRESHOLD: u32 = 1024;

    const PIXEL_DEPTH: PixelDepth = PixelDepth::Sixteen;

    fn write<T>(&self, bitwrite: &mut T) -> io::Result<()>
    where
        T: BitWrite,
    {
        bitwrite.write(u16::BITS, *self)
    }

    fn read<T>(bitread: &mut T) -> io::Result<Self>
    where
        T: BitRead,
    {
        bitread.read(u16::BITS)
    }
}

/// This trait is implemented by all image types that are supported by the felics
/// compression algorithm.
pub trait CompressDecompress {
    fn compress<W>(&self, to: W) -> io::Result<()>
    where
        W: Write;

    fn decompress<R>(from: R) -> Result<Self, DecompressionError>
    where
        Self: Sized,
        R: Read;
}
