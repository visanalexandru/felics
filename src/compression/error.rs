use std::convert::From;
use std::io;

#[derive(Debug)]
pub enum DecompressionError {
    IoError(io::Error),
    /// A value that was decoded does not fit the image bit-depth.
    InvalidValue,
    /// An overflow occured during an arithmetic operation.
    ValueOverflow,
    /// The channel dimensions are invalid.
    InvalidDimensions,
    /// There was an attempt to decode an image with an invalid color type.
    InvalidColorType,
    /// There was an attempt to decode an image with an invalid pixel depth.
    InvalidPixelDepth,
    /// The signature of the file does not match a felics file.
    InvalidSignature,
}

impl From<io::Error> for DecompressionError {
    fn from(err: io::Error) -> DecompressionError {
        DecompressionError::IoError(err)
    }
}
