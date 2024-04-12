use super::error::DecompressionError;
use super::format::{read_header, Header, PixelDepth};
use std::io::{self, Read, Write};

/// This trait is implemented by all types that can
/// represent a pixel intensity in an image.
pub trait Intensity: Into<i32> + TryFrom<i32> + Default + Clone + Copy {
    /// The list of reasonable k values we can use to encode
    /// this pixel intensity using rice coding.
    const K_VALUES: &'static [u8];

    /// The maximum context, as specified in the article.
    /// A pixel's context is defined as: `context = H - L`,
    /// so `MAX_CONTEXT` will be the maximum possible difference
    /// between two pixel intensities after the RGB -> YCoCg transform.
    const MAX_CONTEXT: u32;

    /// Halve all code lengths when the smallest value reaches this threshold.
    const COUNT_SCALING: Option<u32>;

    /// The pixel depth of this pixel intensity.
    const PIXEL_DEPTH: PixelDepth;
}

impl Intensity for u8 {
    const K_VALUES: &'static [u8] = &[0, 1, 2, 3, 4, 5];

    const MAX_CONTEXT: u32 = u8::MAX as u32 * 2;

    const COUNT_SCALING: Option<u32> = Some(1024);

    const PIXEL_DEPTH: PixelDepth = PixelDepth::Eight;
}

impl Intensity for u16 {
    const K_VALUES: &'static [u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

    const MAX_CONTEXT: u32 = u16::MAX as u32 * 2;

    const COUNT_SCALING: Option<u32> = Some(1024);

    const PIXEL_DEPTH: PixelDepth = PixelDepth::Sixteen;
}

/// This trait is implemented by all image types that are supported by the felics
/// compression algorithm.
pub trait CompressDecompress {
    fn compress<W>(&self, to: W) -> io::Result<()>
    where
        W: Write;

    fn decompress_with_header<R>(from: R, header: &Header) -> Result<Self, DecompressionError>
    where
        Self: Sized,
        R: Read;

    fn decompress<R>(mut from: R) -> Result<Self, DecompressionError>
    where
        Self: Sized,
        R: Read,
    {
        let header = read_header(&mut from)?;
        Self::decompress_with_header(from, &header)
    }
}
