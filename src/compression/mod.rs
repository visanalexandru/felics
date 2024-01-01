use crate::bitvector::{self, BitVector};
use image::{GenericImageView, GrayImage, Luma};
use std::cmp;

/// A grayscale image that was compressed using the felics algorithm.
pub struct CompressedGrayscaleImage {
    width: u32,
    height: u32,
    pixel1: Luma<u8>,
    pixel2: Luma<u8>,
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

/// Errors that may occur when compressing an image.
#[derive(Debug)]
pub enum CompressionError {
    ImageTooSmall,
}

pub fn compress(image: GrayImage) -> Result<CompressedGrayscaleImage, CompressionError> {
    let mut pixels = image.enumerate_pixels();

    let pixel1 = match pixels.next() {
        Some((_, _, &luma)) => luma,
        None => return Err(CompressionError::ImageTooSmall),
    };

    let pixel2 = match pixels.next() {
        Some((_, _, &luma)) => luma,
        None => return Err(CompressionError::ImageTooSmall),
    };

    let mut bitvec = BitVector::new();
    for (x, y, Luma([p])) in pixels {
        let ((x1, y1), (x2, y2)) = nearest_neighbours((x, y), &image).unwrap();
        let Luma([v1]) = image.get_pixel(x1, y1);
        let Luma([v2]) = image.get_pixel(x2, y2);

        let h = cmp::max(v1, v2);
        let l = cmp::min(v1, v2);

        if p >= l && p <= h {
            encode_intensity(&mut bitvec, PixelIntensity::InRange);
        } else if p < l {
            encode_intensity(&mut bitvec, PixelIntensity::BelowRange);
        } else {
            encode_intensity(&mut bitvec, PixelIntensity::AboveRange);
        }
    }

    Ok(CompressedGrayscaleImage {
        width: image.width(),
        height: image.height(),
        pixel1,
        pixel2,
        data: bitvec,
    })
}

/// Returns the two nearest neighbours of a pixel in a given image, that have already been coded.
///
/// Except along the top and left edges, these are the pixel above and the pixel
/// to the left of the pixel.
fn nearest_neighbours<T>((x, y): (u32, u32), img: &T) -> Option<((u32, u32), (u32, u32))>
where
    T: GenericImageView,
{
    assert!(img.in_bounds(x, y));
    if x > 0 && y > 0 {
        Some(((x - 1, y), (x, y - 1)))
    } else if y == 0 {
        if x >= 2 {
            Some(((x - 1, y), (x - 2, y)))
        } else {
            None
        }
    } else {
        if y >= 2 {
            Some(((x, y - 1), (x, y - 2)))
        } else if img.in_bounds(x, y - 1) && img.in_bounds(x + 1, y - 1) {
            Some(((x, y - 1), (x + 1, y - 1)))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::{encode_intensity, nearest_neighbours, PixelIntensity};
    use crate::{bitvector::BitVector, compression::decode_intensity};
    use image::{GenericImageView, Luma};

    struct ImageMock {
        width: u32,
        height: u32,
    }

    impl ImageMock {
        fn new(width: u32, height: u32) -> ImageMock {
            assert_ne!(width, 0);
            assert_ne!(height, 0);
            ImageMock { width, height }
        }
    }

    impl GenericImageView for ImageMock {
        type Pixel = Luma<u8>;

        fn dimensions(&self) -> (u32, u32) {
            (self.width, self.height)
        }

        fn bounds(&self) -> (u32, u32, u32, u32) {
            (0, 0, self.width, self.height)
        }
        fn get_pixel(&self, _: u32, _: u32) -> Self::Pixel {
            Luma([0])
        }
    }

    #[test]
    fn test_nearest_neighbours() {
        let image = ImageMock::new(23, 25);
        assert_eq!(nearest_neighbours((5, 8), &image), Some(((4, 8), (5, 7))));
        assert_eq!(nearest_neighbours((0, 8), &image), Some(((0, 7), (0, 6))));
        assert_eq!(nearest_neighbours((2, 0), &image), Some(((1, 0), (0, 0))));
        assert_eq!(nearest_neighbours((1, 1), &image), Some(((0, 1), (1, 0))));
        assert_eq!(nearest_neighbours((1, 0), &image), None);
        assert_eq!(nearest_neighbours((0, 1), &image), Some(((0, 0), (1, 0))));

        let image = ImageMock::new(5, 1);
        assert_eq!(nearest_neighbours((1, 0), &image), None);
        assert_eq!(nearest_neighbours((2, 0), &image), Some(((1, 0), (0, 0))));
        assert_eq!(nearest_neighbours((4, 0), &image), Some(((3, 0), (2, 0))));

        let image = ImageMock::new(1, 30);
        assert_eq!(nearest_neighbours((0, 0), &image), None);
        assert_eq!(nearest_neighbours((0, 1), &image), None);
        assert_eq!(nearest_neighbours((0, 2), &image), Some(((0, 1), (0, 0))));
        assert_eq!(nearest_neighbours((0, 10), &image), Some(((0, 9), (0, 8))));

        let image = ImageMock::new(2, 2);
        assert_eq!(nearest_neighbours((0, 1), &image), Some(((0, 0), (1, 0))));
        assert_eq!(nearest_neighbours((1, 1), &image), Some(((0, 1), (1, 0))));
    }

    #[test]
    #[should_panic]
    fn test_nearest_neighbours_out_of_bounds() {
        let image = ImageMock::new(100, 80);
        nearest_neighbours((90, 80), &image);
    }

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
