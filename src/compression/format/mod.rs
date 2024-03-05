use crate::bitvector::BitVector;

/// Supported color formats by the felics compression algorithm.
#[derive(Debug, PartialEq, Eq)]
pub enum ColorFormat {
    Gray8,
    Gray16,
}

/// Holds actual compressed pixel data of a color channel.
pub struct CompressedChannel {
    pub pixel1: u32,
    pub pixel2: u32,
    pub data: BitVector,
}

/// A compressed representation of an image that was encoded using the felics
/// compression algorithm.
pub struct CompressedImage {
    pub format: ColorFormat,
    pub width: u32,
    pub height: u32,
    pub channels: Vec<CompressedChannel>,
}

impl CompressedImage {
    pub fn size(&self) -> usize {
        self.channels.iter().map(|chan| chan.data.num_bytes()).sum()
    }
}
