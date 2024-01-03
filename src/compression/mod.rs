use crate::{
    bitvector::{self, BitVector},
    coding::rice_coding,
    coding::{phase_in_coding::PhaseInCoder, rice_coding::decode_rice},
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

pub fn compress(image: GrayImage) -> Option<CompressedGrayscaleImage> {
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
        let ((x1, y1), (x2, y2)) = misc::nearest_neighbours((x, y), &image).unwrap();

        let Luma([p]) = *image.get_pixel(x, y);
        let Luma([v1]) = *image.get_pixel(x1, y1);
        let Luma([v2]) = *image.get_pixel(x2, y2);

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);
        let k = 2;
        let context = h - l;

        if p >= l && p <= h {
            encode_intensity(&mut bitvec, PixelIntensity::InRange);
            // Encode p-l in the range [0, context]
            let to_encode = (p - l) as u32;
            let encoder = PhaseInCoder::new(context as u32 + 1);
            encoder.encode(&mut bitvec, to_encode);
        } else if p < l {
            encode_intensity(&mut bitvec, PixelIntensity::BelowRange);
            let to_encode = (l - p - 1) as u32;
            rice_coding::encode_rice(&mut bitvec, to_encode, k);
        } else {
            encode_intensity(&mut bitvec, PixelIntensity::AboveRange);
            let to_encode = (p - h - 1) as u32;
            rice_coding::encode_rice(&mut bitvec, to_encode, k);
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

pub fn decompress(compressed: CompressedGrayscaleImage) -> Option<GrayImage> {
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

        let intensity = decode_intensity(&mut data_iter)?;

        let pixel_value = match intensity {
            PixelIntensity::InRange => {
                let coder = PhaseInCoder::new(context as u32 + 1);
                let p = coder.decode(&mut data_iter)?;
                (p as u8).checked_add(l)?
            }
            PixelIntensity::BelowRange => {
                let encoded = decode_rice(&mut data_iter, k)?;
                // The encoded value is l-p-1.
                // To get p back, we must compute: l-encoded-1.
                l.checked_sub(encoded as u8)?.checked_sub(1)?
            }
            PixelIntensity::AboveRange => {
                let encoded = decode_rice(&mut data_iter, k)?;
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
    use super::{encode_intensity, PixelIntensity};
    use crate::{bitvector::BitVector, compression::decode_intensity};

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
}
