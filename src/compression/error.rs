/// Errors that can occur when decompressing images.
#[derive(Debug)]
pub enum DecompressionError {
    /// We have reached the end of the buffer prematurely.
    Truncated,
    /// A value that was decoded does not fit the image bit-depth.
    InvalidValue,
    /// An overflow occured during an arithmetic operation.
    ValueOverflow,
    /// The channel dimensions are invalid.
    InvalidDimensions,
}
