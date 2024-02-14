use crate::{
    bitvector::{self, BitVector},
    coding::{phase_in_coding::PhaseInCoder, rice_coding::RiceCoder},
};
use error::DecompressionError;
pub use format::CompressedImage;
use image::{ImageBuffer, Luma};
use misc::RasterScan;
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

impl<T> CompressDecompress for ImageBuffer<Luma<T>, Vec<T>>
where
    T: Intensity,
{
    fn compress(&self) -> CompressedImage {
        // Check for edge-case image dimensions.
        match self.dimensions() {
            (width @ 0, height) | (width, height @ 0) => {
                return CompressedImage {
                    format: T::COLOR_FORMAT,
                    width,
                    height,
                    pixel1: 0,
                    pixel2: 0,
                    data: BitVector::new(),
                };
            }
            (width @ 1, height @ 1) => {
                let &Luma([pixel1]) = self.get_pixel(0, 0);
                return CompressedImage {
                    format: T::COLOR_FORMAT,
                    width,
                    height,
                    pixel1: pixel1.into(),
                    pixel2: 0,
                    data: BitVector::new(),
                };
            }
            _ => (),
        };

        let mut pixels = RasterScan::new(self.width(), self.height());
        // We now know that we have at least 2 pixels.
        let (x, y) = pixels.next().unwrap();
        let Luma([pixel1]) = *self.get_pixel(x, y);

        let (x, y) = pixels.next().unwrap();
        let Luma([pixel2]) = *self.get_pixel(x, y);

        let mut bitvec = BitVector::new();
        let mut estimator: KEstimator<T> = KEstimator::new(true);

        // Proceed in raster-scan order.
        for (x, y) in pixels {
            let ((x1, y1), (x2, y2)) = misc::nearest_neighbours((x, y), self).unwrap();

            let Luma([p]) = *self.get_pixel(x, y);
            let Luma([v1]) = *self.get_pixel(x1, y1);
            let Luma([v2]) = *self.get_pixel(x2, y2);

            let h = cmp::max(v1, v2);
            let l = cmp::min(v1, v2);
            let context = h - l;
            let k = estimator.get_k(context);
            let rice_coder = RiceCoder::new(k);

            if p >= l && p <= h {
                encode_intensity(&mut bitvec, PixelIntensity::InRange);
                // Encode p-l in the range [0, context]
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

        CompressedImage {
            format: T::COLOR_FORMAT,
            width: self.width(),
            height: self.height(),
            pixel1: pixel1.into(),
            pixel2: pixel2.into(),
            data: bitvec,
        }
    }

    fn decompress(compressed: &CompressedImage) -> Result<Self, DecompressionError> {
        let mut image = ImageBuffer::new(compressed.width, compressed.height);

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
        match (compressed.width, compressed.height) {
            (0, _) | (_, 0) => {
                return Ok(image);
            }
            (1, 1) => {
                image.put_pixel(0, 0, Luma([pixel1]));
                return Ok(image);
            }
            _ => (),
        };

        // We now know that we have at least 2 pixels.
        let mut pixels = RasterScan::new(image.width(), image.height());
        let (x, y) = pixels.next().unwrap();
        image.put_pixel(x, y, Luma([pixel1]));

        let (x, y) = pixels.next().unwrap();
        image.put_pixel(x, y, Luma([pixel2]));

        let mut data_iter = compressed.data.iter();
        let mut estimator: KEstimator<T> = KEstimator::new(true);

        // Proceed in raster-scan order.
        for (x, y) in pixels {
            let ((x1, y1), (x2, y2)) = misc::nearest_neighbours((x, y), &image).unwrap();

            let Luma([v1]) = *image.get_pixel(x1, y1);
            let Luma([v2]) = *image.get_pixel(x2, y2);

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
            image.put_pixel(x, y, Luma([pixel_value]));
        }
        Ok(image)
    }
}

#[cfg(test)]
mod test {
    use super::{encode_intensity, CompressDecompress, Intensity, PixelIntensity};
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
        assert_eq!(compressed.pixel1, 0);
        assert_eq!(compressed.pixel2, 0);
        assert_eq!(compressed.data.len(), 0);

        let decompressed = GrayImage::decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    fn test_compression_zero_height() {
        let image = GrayImage::new(12, 0);
        let compressed = image.compress();

        assert_eq!(compressed.width, 12);
        assert_eq!(compressed.height, 0);
        assert_eq!(compressed.pixel1, 0);
        assert_eq!(compressed.pixel2, 0);
        assert_eq!(compressed.data.len(), 0);

        let decompressed = GrayImage::decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    fn test_compression_zero_width_and_height() {
        let image = GrayImage::new(0, 0);
        let compressed = image.compress();

        assert_eq!(compressed.width, 0);
        assert_eq!(compressed.height, 0);
        assert_eq!(compressed.pixel1, 0);
        assert_eq!(compressed.pixel2, 0);
        assert_eq!(compressed.data.len(), 0);

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
        assert_eq!(compressed.pixel1, 243);
        assert_eq!(compressed.pixel2, 0);
        assert_eq!(compressed.data.len(), 0);

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
        T: Intensity,
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

        for width in 0..100 {
            for height in 0..100 {
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
