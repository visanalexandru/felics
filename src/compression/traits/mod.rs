use super::error::DecompressionError;
use super::format::{ColorFormat, CompressedImage};
use image::Primitive;
use num::{CheckedAdd, CheckedSub};
use std::cmp::Ord;

/// This trait is implemented by all types that can
/// represent a pixel intensity in a grayscale image.
pub trait Intensity:
    Primitive + CheckedAdd + CheckedSub + Ord + Into<u32> + Into<usize> + TryFrom<u32>
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

    /// The color format associated with this pixel intensity.
    const COLOR_FORMAT: ColorFormat;
}

impl Intensity for u8 {
    const K_VALUES: &'static [u8] = &[0, 1, 2, 3, 4, 5];

    const MAX_CONTEXT: usize = u8::MAX as usize;

    const COUNT_SCALING_THRESHOLD: u32 = 1024;

    const COLOR_FORMAT: ColorFormat = ColorFormat::Gray8;
}

impl Intensity for u16 {
    const K_VALUES: &'static [u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

    const MAX_CONTEXT: usize = u16::MAX as usize;

    const COUNT_SCALING_THRESHOLD: u32 = 1024;

    const COLOR_FORMAT: ColorFormat = ColorFormat::Gray16;
}

/// This trait is implemented by all image types that are supported by the felics
/// compression algorithm.
pub trait CompressDecompress {
    fn compress(&self) -> CompressedImage;

    fn decompress(img: &CompressedImage) -> Result<Self, DecompressionError>
    where
        Self: Sized;
}
