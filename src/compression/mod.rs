use crate::{
    bitvector::{self, BitVector},
    coding::{phase_in_coding::PhaseInCoder, rice_coding::RiceCoder},
};
use image::{GrayImage, Luma};
use std::cmp;
mod misc;
use misc::RasterScan;

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

pub fn compress(image: &GrayImage) -> Option<CompressedGrayscaleImage> {
    let mut pixels = RasterScan::new(image.width(), image.height());

    let Luma([pixel1]) = match pixels.next() {
        Some((x, y)) => *image.get_pixel(x, y),
        None => return None,
    };

    let Luma([pixel2]) = match pixels.next() {
        Some((x, y)) => *image.get_pixel(x, y),
        None => return None,
    };

    let mut bitvec = BitVector::new();

    // Proceed in raster-scan order.
    for (x, y) in pixels {
        let ((x1, y1), (x2, y2)) = misc::nearest_neighbours((x, y), image).unwrap();

        let Luma([p]) = *image.get_pixel(x, y);
        let Luma([v1]) = *image.get_pixel(x1, y1);
        let Luma([v2]) = *image.get_pixel(x2, y2);

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let k = 2;
        let context = h - l;
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
        } else {
            encode_intensity(&mut bitvec, PixelIntensity::AboveRange);
            let to_encode = (p - h - 1) as u32;
            rice_coder.encode_rice(&mut bitvec, to_encode);
        }
    }
    println!("Took: {} bytes", bitvec.as_raw_bytes().len());

    Some(CompressedGrayscaleImage {
        width: image.width(),
        height: image.height(),
        pixel1,
        pixel2,
        data: bitvec,
    })
}

pub fn decompress(compressed: &CompressedGrayscaleImage) -> Option<GrayImage> {
    let mut image: image::ImageBuffer<Luma<u8>, Vec<u8>> =
        GrayImage::new(compressed.width, compressed.height);

    let mut pixels = RasterScan::new(image.width(), image.height());

    match pixels.next() {
        Some((x, y)) => image.put_pixel(x, y, Luma([compressed.pixel1])),
        None => return None,
    };

    match pixels.next() {
        Some((x, y)) => image.put_pixel(x, y, Luma([compressed.pixel2])),
        None => return None,
    };

    let mut data_iter = compressed.data.iter();

    // Proceed in raster-scan order.
    for (x, y) in pixels {
        let ((x1, y1), (x2, y2)) = misc::nearest_neighbours((x, y), &image).unwrap();

        let Luma([v1]) = *image.get_pixel(x1, y1);
        let Luma([v2]) = *image.get_pixel(x2, y2);

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let k = 2;
        let context = h - l;
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
                // The encoded value is l-p-1.
                // To get p back, we must compute: l-encoded-1.
                l.checked_sub(encoded as u8)?.checked_sub(1)?
            }
            PixelIntensity::AboveRange => {
                let encoded = rice_coder.decode_rice(&mut data_iter)?;
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
    fn test_compression_invalid_dimensions() {
        let image = GrayImage::new(1, 1);
        assert!(compress(&image).is_none());
    }

    #[test]
    fn test_compression_two_pixels() {
        let mut image = GrayImage::new(1, 2);
        image.put_pixel(0, 0, Luma([10]));
        image.put_pixel(0, 1, Luma([3]));
        let compressed = compress(&image).unwrap();
        assert_eq!(compressed.pixel1, 10);
        assert_eq!(compressed.pixel2, 3);
        assert_eq!(compressed.data.len(), 0);

        let mut image = GrayImage::new(2, 1);
        image.put_pixel(0, 0, Luma([4]));
        image.put_pixel(1, 0, Luma([42]));
        let compressed = compress(&image).unwrap();
        assert_eq!(compressed.pixel1, 4);
        assert_eq!(compressed.pixel2, 42);
        assert_eq!(compressed.data.len(), 0);
    }

    #[test]
    fn test_compression_decompression() {
        let mut image = GrayImage::new(240, 480);
        for x in 0..240 {
            for y in 0..480 {
                let value = ((x ^ y) % 256) as u8;
                image.put_pixel(x, y, Luma([value]));
            }
        }
        let compressed = compress(&image).unwrap();
        let decompressed = decompress(&compressed).unwrap();

        assert_eq!(decompressed.width(), image.width());
        assert_eq!(decompressed.height(), image.height());
        assert_eq!(decompressed.as_raw(), image.as_raw());
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

    /// Generate a bunch of random images.
    /// Check that for each image, Decompress(Compress(image)) = image
    #[test]
    #[ignore]
    fn test_compression_decompression_intensive() {
        let num_images = 100;
        let mut rng = rand::thread_rng();

        for _ in 0..num_images {
            let width: u32 = rng.gen_range(1..400);
            let height: u32 = rng.gen_range(1..400);

            let image = random_image(width, height, &mut rng);

            let compressed = compress(&image).unwrap();
            let decompressed = decompress(&compressed).unwrap();

            assert_eq!(image.as_raw(), decompressed.as_raw());
        }
    }
}
