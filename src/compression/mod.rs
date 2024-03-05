use crate::{
    bitvector::{self, BitVector},
    coding::{phase_in_coding::PhaseInCoder, rice_coding::RiceCoder},
};
use error::DecompressionError;
pub use format::{ColorType, CompressedChannel, CompressedImage, PixelDepth};
use image::{ImageBuffer, Luma, Pixel, Rgb};
use parameter_selection::KEstimator;
use std::cmp;
pub use traits::{CompressDecompress, Intensity};

mod error;
mod format;
mod misc;
mod parameter_selection;
mod traits;

/// The possible intensity of a pixel relative to the context induced by its two
/// nearest neighbours: `[L, H]`.
#[derive(PartialEq, Debug)]
enum PixelIntensity {
    InRange,
    BelowRange,
    AboveRange,
}

/// Appends the encoded pixel intensity indicator to the given `BitVector`.
fn encode_intensity(bitvec: &mut BitVector, intensity: PixelIntensity) {
    match intensity {
        PixelIntensity::InRange => bitvec.push(true),
        PixelIntensity::AboveRange => {
            bitvec.push(false);
            bitvec.push(true)
        }
        PixelIntensity::BelowRange => {
            bitvec.push(false);
            bitvec.push(false)
        }
    }
}

/// Decodes a pixel intensity indicator by advancing the `BitVector` iterator.
///
/// Returns `None` if the decoding process failed.
fn decode_intensity(iter: &mut bitvector::Iter) -> Option<PixelIntensity> {
    let in_range = iter.next()?;
    if in_range {
        return Some(PixelIntensity::InRange);
    }
    let above = iter.next()?;
    if above {
        return Some(PixelIntensity::AboveRange);
    }
    Some(PixelIntensity::BelowRange)
}

fn compress_channel<T>(channel: &[T], width: u32, height: u32) -> CompressedChannel
where
    T: Intensity,
{
    // Check for edge-case image dimensions.
    match (width, height) {
        (0, _) | (_, 0) => {
            return CompressedChannel {
                pixel1: 0,
                pixel2: 0,
                data: BitVector::new(),
            };
        }
        (1, 1) => {
            return CompressedChannel {
                pixel1: channel[0].into(),
                pixel2: 0,
                data: BitVector::new(),
            };
        }
        _ => (),
    };

    // We now know that we have at least 2 pixels.
    let (pixel1, pixel2) = (channel[0], channel[1]);

    let mut bitvec = BitVector::new();
    let mut estimator: KEstimator<T> = KEstimator::new(true);

    let total_size: usize = width.checked_mul(height).unwrap().try_into().unwrap();

    // Proceed in raster-scan order.
    for i in 2..total_size {
        let (a, b) = misc::nearest_neighbours(i, width as usize).unwrap();

        let p = channel[i];
        let v1 = channel[a];
        let v2 = channel[b];

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let context = h - l;
        let k = estimator.get_k(context);
        let rice_coder = RiceCoder::new(k);

        if p >= l && p <= h {
            encode_intensity(&mut bitvec, PixelIntensity::InRange);
            let to_encode = p - l;
            let phase_in_coder = PhaseInCoder::new(Into::<u32>::into(context) + 1);
            phase_in_coder.encode(&mut bitvec, to_encode.into());
        } else if p < l {
            encode_intensity(&mut bitvec, PixelIntensity::BelowRange);
            let to_encode = l - p - T::one();
            rice_coder.encode_rice(&mut bitvec, to_encode.into());
            estimator.update(context, to_encode);
        } else {
            encode_intensity(&mut bitvec, PixelIntensity::AboveRange);
            let to_encode = p - h - T::one();
            rice_coder.encode_rice(&mut bitvec, to_encode.into());
            estimator.update(context, to_encode);
        }
    }

    CompressedChannel {
        pixel1: pixel1.into(),
        pixel2: pixel2.into(),
        data: bitvec,
    }
}

fn decompress_channel<T>(
    compressed: &CompressedChannel,
    width: u32,
    height: u32,
) -> Result<Vec<T>, DecompressionError>
where
    T: Intensity,
{
    // Parse the first two pixels.
    let pixel1: T = compressed
        .pixel1
        .try_into()
        .map_err(|_| DecompressionError::InvalidValue)?;

    let pixel2: T = compressed
        .pixel2
        .try_into()
        .map_err(|_| DecompressionError::InvalidValue)?;

    // Handle edge-case dimensions.
    match (width, height) {
        (0, _) | (_, 0) => {
            return Ok(vec![]);
        }
        (1, 1) => {
            return Ok(vec![pixel1]);
        }
        _ => (),
    };

    // Create the pixel buffer.
    let total_size: usize = width
        .checked_mul(height)
        .ok_or(DecompressionError::ValueOverflow)?
        .try_into()
        .map_err(|_| DecompressionError::InvalidDimensions)?;

    let mut buf = vec![T::zero(); total_size];
    buf[0] = pixel1;
    buf[1] = pixel2;

    let mut data_iter = compressed.data.iter();
    let mut estimator: KEstimator<T> = KEstimator::new(true);

    // Proceed in raster-scan order.
    for i in 2..total_size {
        let (a, b) = misc::nearest_neighbours(i, width as usize).unwrap();

        let v1 = buf[a];
        let v2 = buf[b];

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let context = h - l;
        let k = estimator.get_k(context);
        let rice_coder = RiceCoder::new(k);

        let intensity = match decode_intensity(&mut data_iter) {
            Some(intensity) => intensity,
            None => return Err(DecompressionError::Truncated),
        };

        let pixel_value = match intensity {
            PixelIntensity::InRange => {
                let phase_in_coder = PhaseInCoder::new(Into::<u32>::into(context) + 1);
                let p: T = phase_in_coder
                    .decode(&mut data_iter)
                    .ok_or(DecompressionError::Truncated)?
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;
                p.checked_add(&l).ok_or(DecompressionError::ValueOverflow)?
            }
            PixelIntensity::BelowRange => {
                let encoded: T = rice_coder
                    .decode_rice(&mut data_iter)
                    .ok_or(DecompressionError::Truncated)?
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;
                estimator.update(context, encoded);
                // The encoded value is l-p-1.
                // To get p back, we must compute: l-encoded-1.
                l.checked_sub(&encoded)
                    .ok_or(DecompressionError::ValueOverflow)?
                    .checked_sub(&T::one())
                    .ok_or(DecompressionError::ValueOverflow)?
            }
            PixelIntensity::AboveRange => {
                let encoded: T = rice_coder
                    .decode_rice(&mut data_iter)
                    .ok_or(DecompressionError::Truncated)?
                    .try_into()
                    .map_err(|_| DecompressionError::InvalidValue)?;
                estimator.update(context, encoded);
                // The encoded value is p-h-1.
                // To get p back, we must compute: encoded + h + 1.
                encoded
                    .checked_add(&h)
                    .ok_or(DecompressionError::ValueOverflow)?
                    .checked_add(&T::one())
                    .ok_or(DecompressionError::ValueOverflow)?
            }
        };
        buf[i] = pixel_value;
    }
    Ok(buf)
}

impl<T> CompressDecompress for ImageBuffer<Luma<T>, Vec<T>>
where
    Luma<T>: Pixel<Subpixel = T>,
    T: Intensity,
{
    fn compress(&self) -> CompressedImage {
        let (width, height) = self.dimensions();
        let compressed_channel = compress_channel(self.as_raw(), width, height);
        CompressedImage {
            color_type: ColorType::Gray,
            pixel_depth: T::PIXEL_DEPTH,
            width: self.width(),
            height: self.height(),
            channels: vec![compressed_channel],
        }
    }

    fn decompress(compressed: &CompressedImage) -> Result<Self, DecompressionError> {
        if compressed.color_type != ColorType::Gray {
            return Err(DecompressionError::InvalidColorType);
        }

        if compressed.pixel_depth != T::PIXEL_DEPTH {
            return Err(DecompressionError::InvalidPixelDepth);
        }

        debug_assert_eq!(compressed.channels.len(), Luma::CHANNEL_COUNT as usize);

        let (width, height) = (compressed.width, compressed.height);
        let compressed_channel = &compressed.channels[0];
        let channel = decompress_channel(compressed_channel, width, height)?;
        let image = ImageBuffer::from_raw(width, height, channel).unwrap();

        Ok(image)
    }
}

impl<T> CompressDecompress for ImageBuffer<Rgb<T>, Vec<T>>
where
    Rgb<T>: Pixel<Subpixel = T>,
    T: Intensity,
{
    fn compress(&self) -> CompressedImage {
        let (width, height) = self.dimensions();
        let num_pixels = (width as usize) * (height as usize);
        let pixels = self.as_raw();

        let (mut red, mut green, mut blue) = (
            vec![T::zero(); num_pixels],
            vec![T::zero(); num_pixels],
            vec![T::zero(); num_pixels],
        );

        for i in 0..num_pixels {
            let current = i * 3;
            red[i] = pixels[current];
            green[i] = pixels[current + 1];
            blue[i] = pixels[current + 2];
        }

        let (c_red, c_green, c_blue) = (
            compress_channel(&red, width, height),
            compress_channel(&green, width, height),
            compress_channel(&blue, width, height),
        );

        CompressedImage {
            color_type: ColorType::Rgb,
            pixel_depth: T::PIXEL_DEPTH,
            width: self.width(),
            height: self.height(),
            channels: vec![c_red, c_green, c_blue],
        }
    }

    fn decompress(compressed: &CompressedImage) -> Result<Self, DecompressionError> {
        if compressed.color_type != ColorType::Rgb {
            return Err(DecompressionError::InvalidColorType);
        }

        if compressed.pixel_depth != T::PIXEL_DEPTH {
            return Err(DecompressionError::InvalidPixelDepth);
        }

        debug_assert_eq!(compressed.channels.len(), Rgb::CHANNEL_COUNT as usize);

        let (width, height) = (compressed.width, compressed.height);

        let (c_red, c_green, c_blue) = (
            &compressed.channels[0],
            &compressed.channels[1],
            &compressed.channels[2],
        );

        let (red, green, blue) = (
            decompress_channel(c_red, width, height)?,
            decompress_channel(c_green, width, height)?,
            decompress_channel(c_blue, width, height)?,
        );

        let num_pixels = (width as usize) * (height as usize);
        let buf_size = num_pixels
            .checked_mul(Rgb::CHANNEL_COUNT as usize)
            .ok_or(DecompressionError::InvalidDimensions)?;

        let mut buf = vec![T::zero(); buf_size];

        for i in 0..num_pixels {
            buf[i * 3] = red[i];
            buf[i * 3 + 1] = green[i];
            buf[i * 3 + 2] = blue[i];
        }

        Ok(ImageBuffer::from_raw(width, height, buf).unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::{encode_intensity, CompressDecompress, Pixel, PixelIntensity};
    use crate::{bitvector::BitVector, compression::decode_intensity};
    use image::{GrayImage, ImageBuffer, Luma};
    use rand::{
        self,
        distributions::{Distribution, Standard},
        rngs::ThreadRng,
        Rng,
    };

    #[test]
    fn test_intensity_indicator_encoding() {
        let mut bitvec = BitVector::new();
        encode_intensity(&mut bitvec, PixelIntensity::InRange);
        assert_eq!(bitvec.to_string(), "1");

        let mut bitvec = BitVector::new();
        encode_intensity(&mut bitvec, PixelIntensity::AboveRange);
        assert_eq!(bitvec.to_string(), "01");

        let mut bitvec = BitVector::new();
        encode_intensity(&mut bitvec, PixelIntensity::BelowRange);
        assert_eq!(bitvec.to_string(), "00");
    }

    #[test]
    fn test_intensity_indicator_decoding() {
        let mut bitvec = BitVector::new();
        encode_intensity(&mut bitvec, PixelIntensity::InRange);
        encode_intensity(&mut bitvec, PixelIntensity::InRange);
        encode_intensity(&mut bitvec, PixelIntensity::AboveRange);
        encode_intensity(&mut bitvec, PixelIntensity::BelowRange);
        encode_intensity(&mut bitvec, PixelIntensity::InRange);
        encode_intensity(&mut bitvec, PixelIntensity::AboveRange);

        let mut iter = bitvec.iter();

        assert_eq!(decode_intensity(&mut iter), Some(PixelIntensity::InRange));
        assert_eq!(decode_intensity(&mut iter), Some(PixelIntensity::InRange));
        assert_eq!(
            decode_intensity(&mut iter),
            Some(PixelIntensity::AboveRange)
        );
        assert_eq!(
            decode_intensity(&mut iter),
            Some(PixelIntensity::BelowRange)
        );
        assert_eq!(decode_intensity(&mut iter), Some(PixelIntensity::InRange));
        assert_eq!(
            decode_intensity(&mut iter),
            Some(PixelIntensity::AboveRange)
        );
        assert_eq!(decode_intensity(&mut iter), None);
    }

    #[test]
    fn test_compression_zero_width() {
        let image = GrayImage::new(0, 3);
        let compressed = image.compress();

        assert_eq!(compressed.width, 0);
        assert_eq!(compressed.height, 3);
        assert_eq!(compressed.channels.len(), 1);

        let channel = &compressed.channels[0];
        assert_eq!(channel.pixel1, 0);
        assert_eq!(channel.pixel2, 0);
        assert_eq!(channel.data.len(), 0);

        let decompressed = GrayImage::decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    fn test_compression_zero_height() {
        let image = GrayImage::new(12, 0);
        let compressed = image.compress();

        assert_eq!(compressed.width, 12);
        assert_eq!(compressed.height, 0);

        let channel = &compressed.channels[0];
        assert_eq!(channel.pixel1, 0);
        assert_eq!(channel.pixel2, 0);
        assert_eq!(channel.data.len(), 0);

        let decompressed = GrayImage::decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    fn test_compression_zero_width_and_height() {
        let image = GrayImage::new(0, 0);
        let compressed = image.compress();

        assert_eq!(compressed.width, 0);
        assert_eq!(compressed.height, 0);

        let channel = &compressed.channels[0];
        assert_eq!(channel.pixel1, 0);
        assert_eq!(channel.pixel2, 0);
        assert_eq!(channel.data.len(), 0);

        let decompressed = GrayImage::decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    fn test_compression_single_pixel() {
        let mut image = GrayImage::new(1, 1);
        image.put_pixel(0, 0, Luma([243]));

        let compressed = image.compress();
        assert_eq!(compressed.width, 1);
        assert_eq!(compressed.height, 1);

        let channel = &compressed.channels[0];
        assert_eq!(channel.pixel1, 243);
        assert_eq!(channel.pixel2, 0);
        assert_eq!(channel.data.len(), 0);

        let decompressed = GrayImage::decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    // Returns a random image with the given dimensions.
    fn random_grayscale<T>(
        width: u32,
        height: u32,
        rng: &mut ThreadRng,
    ) -> ImageBuffer<Luma<T>, Vec<T>>
    where
        Luma<T>: Pixel<Subpixel = T>,
        Standard: Distribution<T>,
    {
        let mut image = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let pixel_intensity: T = rng.gen();
                image.put_pixel(x, y, Luma([pixel_intensity]));
            }
        }
        image
    }

    #[test]
    fn test_compression_decompression_grayscale() {
        let dimensions = vec![
            (2, 1),
            (1, 2),
            (1, 1),
            (4, 7),
            (100, 40),
            (124, 274),
            (1447, 8),
            (44, 1),
            (1, 100),
            (680, 480),
        ];
        let mut rng = rand::thread_rng();

        for (width, height) in dimensions {
            let image = random_grayscale::<u8>(width, height, &mut rng);
            let compressed = image.compress();
            let decompressed = CompressDecompress::decompress(&compressed).unwrap();
            assert_eq!(image, decompressed);

            let image = random_grayscale::<u16>(width, height, &mut rng);
            let compressed = image.compress();
            let decompressed = CompressDecompress::decompress(&compressed).unwrap();
            assert_eq!(image, decompressed);
        }
    }

    #[test]
    #[ignore]
    fn test_compression_decompression_intensive() {
        let mut rng = rand::thread_rng();

        for width in 0..20 {
            for height in 0..20 {
                let image = random_grayscale::<u8>(width, height, &mut rng);
                let compressed = image.compress();
                let decompressed = CompressDecompress::decompress(&compressed).unwrap();
                assert_eq!(image, decompressed);

                let image = random_grayscale::<u16>(width, height, &mut rng);
                let compressed = image.compress();
                let decompressed = CompressDecompress::decompress(&compressed).unwrap();
                assert_eq!(image, decompressed);
            }
        }
    }
}
