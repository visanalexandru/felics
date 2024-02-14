use crate::bitvector::BitVector;

/// Supported color formats by the felics compression algorithm.
pub enum ColorFormat {
    Gray8,
    Gray16,
}

/// A compressed representation of an image that was encoded using the felics
/// compression algorithm.
pub struct CompressedImage {
    pub format: ColorFormat,
    pub width: u32,
    pub height: u32,
    // The first two pixels "seed" the image.
    pub pixel1: u32,
    pub pixel2: u32,
    pub data: BitVector,
}

impl CompressedImage {
    pub fn size(&self) -> usize {
        self.data.as_raw_bytes().len()
    }
}
