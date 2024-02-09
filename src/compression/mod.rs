use crate::{
    bitvector::{self, BitVector},
    coding::{phase_in_coding::PhaseInCoder, rice_coding::RiceCoder},
};
use image::{GrayImage, Luma};
use misc::RasterScan;
use parameter_selection::KEstimator;
use std::cmp;

mod misc;
mod parameter_selection;

/// A grayscale image that was compressed using the felics algorithm.
pub struct CompressedGrayscaleImage {
    width: u32,
    height: u32,
    // The first two pixels in the image must be stored unencoded.
    pixel1: u8,
    pixel2: u8,
    data: BitVector,
}

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

pub fn compress(image: &GrayImage) -> CompressedGrayscaleImage {
    // Check for edge-case image dimensions.
    match image.dimensions() {
        (width @ 0, height) | (width, height @ 0) => {
            return CompressedGrayscaleImage {
                width,
                height,
                pixel1: 0,
                pixel2: 0,
                data: BitVector::new(),
            }
        }
        (width @ 1, height @ 1) => {
            let &Luma([pixel1]) = image.get_pixel(0, 0);
            return CompressedGrayscaleImage {
                width,
                height,
                pixel1,
                pixel2: 0,
                data: BitVector::new(),
            };
        }
        _ => (),
    };

    let mut pixels = RasterScan::new(image.width(), image.height());
    // We now know that we have at least 2 pixels.
    let (x, y) = pixels.next().unwrap();
    let Luma([pixel1]) = *image.get_pixel(x, y);

    let (x, y) = pixels.next().unwrap();
    let Luma([pixel2]) = *image.get_pixel(x, y);

    let mut bitvec = BitVector::new();
    let mut estimator = KEstimator::new(vec![0, 1, 2, 3, 4, 5]);

    // Proceed in raster-scan order.
    for (x, y) in pixels {
        let ((x1, y1), (x2, y2)) = misc::nearest_neighbours((x, y), image).unwrap();

        let Luma([p]) = *image.get_pixel(x, y);
        let Luma([v1]) = *image.get_pixel(x1, y1);
        let Luma([v2]) = *image.get_pixel(x2, y2);

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let context = h - l;
        let k = estimator.get_k(context);
        let rice_coder = RiceCoder::new(k);

        if p >= l && p <= h {
            encode_intensity(&mut bitvec, PixelIntensity::InRange);
            // Encode p-l in the range [0, context]
            let to_encode = (p - l) as u32;
            let encoder = PhaseInCoder::new(context as u32 + 1);
            encoder.encode(&mut bitvec, to_encode);
        } else if p < l {
            encode_intensity(&mut bitvec, PixelIntensity::BelowRange);
            let to_encode = (l - p - 1) as u32;
            rice_coder.encode_rice(&mut bitvec, to_encode);
            estimator.update(context, to_encode);
        } else {
            encode_intensity(&mut bitvec, PixelIntensity::AboveRange);
            let to_encode = (p - h - 1) as u32;
            rice_coder.encode_rice(&mut bitvec, to_encode);
            estimator.update(context, to_encode);
        }
    }
    println!("Took: {} bytes", bitvec.as_raw_bytes().len());

    CompressedGrayscaleImage {
        width: image.width(),
        height: image.height(),
        pixel1,
        pixel2,
        data: bitvec,
    }
}

pub fn decompress(compressed: &CompressedGrayscaleImage) -> Option<GrayImage> {
    let mut image = GrayImage::new(compressed.width, compressed.height);

    // Handle edge-case dimensions.
    match (compressed.width, compressed.height) {
        (0, _) | (_, 0) => {
            return Some(image);
        }
        (1, 1) => {
            image.put_pixel(0, 0, Luma([compressed.pixel1]));
            return Some(image);
        }
        _ => (),
    };

    // We now know that we have at least 2 pixels.
    let mut pixels = RasterScan::new(image.width(), image.height());
    let (x, y) = pixels.next().unwrap();
    image.put_pixel(x, y, Luma([compressed.pixel1]));

    let (x, y) = pixels.next().unwrap();
    image.put_pixel(x, y, Luma([compressed.pixel2]));

    let mut data_iter = compressed.data.iter();
    let mut estimator = KEstimator::new(vec![0, 1, 2, 3, 4, 5]);

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

        let intensity = decode_intensity(&mut data_iter)?;

        let pixel_value = match intensity {
            PixelIntensity::InRange => {
                let coder = PhaseInCoder::new(context as u32 + 1);
                let p = coder.decode(&mut data_iter)?;
                (p as u8).checked_add(l)?
            }
            PixelIntensity::BelowRange => {
                let encoded = rice_coder.decode_rice(&mut data_iter)?;
                estimator.update(context, encoded);
                // The encoded value is l-p-1.
                // To get p back, we must compute: l-encoded-1.
                l.checked_sub(encoded as u8)?.checked_sub(1)?
            }
            PixelIntensity::AboveRange => {
                let encoded = rice_coder.decode_rice(&mut data_iter)?;
                estimator.update(context, encoded);
                // The encoded value is p-h-1.
                // To get p back, we must compute: encoded + h + 1.
                (encoded as u8).checked_add(h)?.checked_add(1)?
            }
        };
        image.put_pixel(x, y, Luma([pixel_value]));
    }
    Some(image)
}

#[cfg(test)]
mod test {
    use super::{compress, decompress, encode_intensity, PixelIntensity};
    use crate::{bitvector::BitVector, compression::decode_intensity};
    use image::{GrayImage, Luma};
    use rand::{self, rngs::ThreadRng, Rng};

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
        let compressed = compress(&image);

        assert_eq!(compressed.width, 0);
        assert_eq!(compressed.height, 3);
        assert_eq!(compressed.pixel1, 0);
        assert_eq!(compressed.pixel2, 0);
        assert_eq!(compressed.data.len(), 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    fn test_compression_zero_height() {
        let image = GrayImage::new(12, 0);
        let compressed = compress(&image);

        assert_eq!(compressed.width, 12);
        assert_eq!(compressed.height, 0);
        assert_eq!(compressed.pixel1, 0);
        assert_eq!(compressed.pixel2, 0);
        assert_eq!(compressed.data.len(), 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    fn test_compression_zero_width_and_height() {
        let image = GrayImage::new(0, 0);
        let compressed = compress(&image);

        assert_eq!(compressed.width, 0);
        assert_eq!(compressed.height, 0);
        assert_eq!(compressed.pixel1, 0);
        assert_eq!(compressed.pixel2, 0);
        assert_eq!(compressed.data.len(), 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    #[test]
    fn test_compression_single_pixel() {
        let mut image = GrayImage::new(1, 1);
        image.put_pixel(0, 0, Luma([243]));

        let compressed = compress(&image);
        assert_eq!(compressed.width, 1);
        assert_eq!(compressed.height, 1);
        assert_eq!(compressed.pixel1, 243);
        assert_eq!(compressed.pixel2, 0);
        assert_eq!(compressed.data.len(), 0);

        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(image, decompressed);
    }

    // Returns a random image with the given dimensions.
    fn random_image(width: u32, height: u32, rng: &mut ThreadRng) -> GrayImage {
        let mut image = GrayImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let pixel_intensity: u8 = rng.gen();
                image.put_pixel(x, y, Luma([pixel_intensity]));
            }
        }
        image
    }

    #[test]
    fn test_compression_decompression() {
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
            let image = random_image(width, height, &mut rng);
            let compressed = compress(&image);
            let decompressed = decompress(&compressed).unwrap();
            assert_eq!(image, decompressed);
        }
    }

    #[test]
    #[ignore]
    fn test_compression_decompression_intensive() {
        let mut rng = rand::thread_rng();

        for width in 0..100 {
            for height in 0..100 {
                let image = random_image(width, height, &mut rng);
                let compressed = compress(&image);
                let decompressed = decompress(&compressed).unwrap();
                assert_eq!(image.as_raw(), decompressed.as_raw());
            }
        }
    }
}
